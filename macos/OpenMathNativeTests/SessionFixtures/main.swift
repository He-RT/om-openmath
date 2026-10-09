import Foundation
private final class RaceGate:@unchecked Sendable {
  let reached=DispatchSemaphore(value:0),release=DispatchSemaphore(value:0)
  func hold() async {await withCheckedContinuation {continuation in DispatchQueue.global().async {self.reached.signal();self.release.wait();continuation.resume()}}}
  func wait() async ->Bool {await withCheckedContinuation {continuation in DispatchQueue.global().async {continuation.resume(returning:self.reached.wait(timeout:.now()+5) == .success)}}}
}
@main struct SessionFixtures {
  static func main() async throws {
    let admission=RaceGate(),readback=RaceGate(),acknowledged=RaceGate()
    let probes=NativeSessionProbes(beforeAdmission:{id in if id=="race-original" {await admission.hold()}},
      afterAdmission:{id in if id=="race-original" {await acknowledged.hold()}},
      afterSnapshot:{snapshot in if snapshot.unavailable_operation_refs.contains("race-original") {await readback.hold()}})
    let race=try await NativeHostSession.open(runtime:"readback-admission-race",eventCapacity:1,eventByteBudget:128,probes:probes)
    let raceOperation=Task {try await race.perform(.evaluate_scratch(.init(kind:.evaluate_scratch,source:"2+2",dialect:.modern,definition_snapshot_ref:.init(nil),use_notebook_definitions:false,timeout_ms:10000)),operation:"race-original")}
    let admissionHeld=await admission.wait();precondition(admissionHeld)
    let refresh=Task {try await race.refreshProjection()}
    let readbackHeld=await readback.wait();precondition(readbackHeld)
    admission.release.signal()
    let actualAcknowledged=await acknowledged.wait();precondition(actualAcknowledged)
    readback.release.signal();acknowledged.release.signal()
    let originalResult=try await raceOperation.value
    try await refresh.value
    precondition(originalResult.phase == .completed && originalResult.result.value != nil)
    try await race.close()
    print("Actual pre-admission snapshot delivered after admission ACK keeps the original ID and reads its real result")
    // A 128-byte event budget intentionally cannot fit terminal frames. This exercises the
    // real ABI overflow/resync path: mathematical results can only arrive through readback.
    let session=try await NativeHostSession.open(runtime:"session-fixture",eventCapacity:1,eventByteBudget:128)
    let slow=try await session.updates()
    func scratch(_ source:String)->HostRequestBody {
      .evaluate_scratch(.init(kind:.evaluate_scratch,source:source,dialect:.modern,
        definition_snapshot_ref:.init(nil),use_notebook_definitions:false,timeout_ms:10000))
    }
    for i in 0..<12 {
      FileHandle.standardError.write(Data("request exact-\(i)\n".utf8))
      let result=try await session.perform(scratch("2+2"),operation:"exact-\(i)")
      FileHandle.standardError.write(Data("settled exact-\(i)\n".utf8))
      precondition(result.phase == .completed)
      guard case .object(let value)=result.result.value,
        case .object(let response)=value["response"],case .object(let output)=response["output"],
        case .array(let items)=output["items"],case .object(let item)=items[0],
        case .string(let exact)=item["input_form"] else { fatalError("missing real result") }
      precondition(exact=="4")
    }
    var iterator=slow.makeAsyncIterator()
    let latest=await iterator.next()!
    precondition(latest.operations["exact-11"]?.phase == .completed)
    let broken=try await session.perform(scratch("let f(x)=;"),operation:"broken")
    guard case .object(let result)=broken.result.value,case .string(let outcome)=result["run_outcome"] else { fatalError("missing actual failure") }
    precondition(outcome=="failed")
    let long=Task { try await session.perform(scratch("map(fn(k)=>solve(x^5-x-k=0,x),range(1,300))"),operation:"long") }
    let start=ContinuousClock.now
    while (await session.projection()).operations["long"]==nil && ContinuousClock.now-start < .seconds(2) {
      try await Task.sleep(for:.milliseconds(1))
    }
    await session.cancel("long")
    let stopped=try await long.value
    precondition(stopped.phase == .cancelled || stopped.phase == .completed || stopped.phase == .failed)
    // A late stop must preserve whatever the owner actually settled; never replace success.
    let stoppedPhase=stopped.phase
    await session.cancel("long")
    let confirmed=await session.projection()
    precondition(confirmed.operations["long"]?.phase==stoppedPhase)
    async let close1:Void=session.close()
    async let close2:Void=session.close()
    _ = try await (close1,close2)
    let closed=await session.projection()
    precondition(closed.hostPhase == .closed)
    do { _ = try await session.perform(scratch("2+2"));fatalError("closed admission reopened") }
    catch NativeHostSessionError.admissionClosed { }
    print("Real session: 12 exact results recovered with every terminal frame suppressed, slow UI latest snapshot, actual parse failure, stop/late stop and concurrent close passed")
  }
}
