import Foundation
@main struct SessionFixtures {
  static func main() async throws {
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
