import Foundation

actor EventCollector {
  private var events: [String: EventEnvelope] = [:]
  func receive(_ batch: EventBatch) {
    for event in batch.events where event.event_kind == .operation_finished {
      if let id = event.operation_ref.value { events[id] = event }
    }
  }
  func value(_ id: String) -> EventEnvelope? { events[id] }
}
@MainActor private final class Pulse { var count = 0 }
@main struct NativeClientFixtures {
  @MainActor static func main() async throws {
    let client = try await NativeHostClient.open(runtime:"swift-client")
    let events = EventCollector()
    let pump = Task {
      while !Task.isCancelled { try await events.receive(client.nextEvents()) }
    }
    func submit(_ id:String,_ body:HostRequestBody) async throws {
      let frame = RequestEnvelope(protocol_version:1,runtime_instance_id:"swift-client",
        request_ref:id,operation_id:.init(id),document_binding:.init(nil),task_binding:.init(nil),body:body)
      let receipt = try await client.submit(frame)
      precondition(receipt.accepted)
    }
    func scratch(_ source:String)->HostRequestBody {
      .evaluate_scratch(.init(kind:.evaluate_scratch,source:source,dialect:.modern,
        definition_snapshot_ref:.init(nil),use_notebook_definitions:false,timeout_ms:10000))
    }
    func finished(_ id:String) async throws -> EventEnvelope {
      let start = ContinuousClock.now
      while ContinuousClock.now-start < .seconds(10) {
        if let event=await events.value(id) { return event }
        try await Task.sleep(for:.milliseconds(2))
      }
      throw NativeHostClientError.invalidReply
    }
    func object(_ value:HostJSONValue)->[String:HostJSONValue] {
      guard case .object(let object)=value else { fatalError("expected real object") }; return object
    }
    func string(_ value:HostJSONValue?)->String {
      guard case .string(let value)=value else { fatalError("expected real string") }; return value
    }
    try await submit("sum",scratch("2+2"))
    let payload=object(try await finished("sum").payload)
    let result=object(payload["result"]!)
    let response=object(result["response"]!)
    let output=object(response["output"]!)
    guard case .array(let items)=output["items"] else { fatalError("missing result items") }
    precondition(string(object(items[0])["input_form"])=="4")
    try await submit("long",scratch("map(fn(k)=>sin(k),range(1,100000))"))
    let pulse=Pulse()
    let pulseTask=Task {
      while !Task.isCancelled { pulse.count += 1; try await Task.sleep(for:.milliseconds(10)) }
    }
    let start=ContinuousClock.now
    try await submit("state",.read_host_state(.init(kind:.get_state)))
    _ = try await finished("state")
    precondition(ContinuousClock.now-start < .seconds(1))
    let signalled = try await client.cancel(operation:"long")
    precondition(signalled)
    let long=object(try await finished("long").payload)
    precondition(string(long["phase"])=="cancelled")
    // Keep the event poll alive while checking MainActor scheduling; the CAS has settled.
    try await Task.sleep(for:.milliseconds(300))
    precondition(pulse.count>=10,"MainActor blocked by native event polling")
    pulseTask.cancel()
    _ = try? await pulseTask.value
    pump.cancel()
    _ = try await pump.value
    try await client.close()
    try await client.close()
    do { _ = try await client.nextEvents(); fatalError("closed client admitted a poll") }
    catch NativeHostClientError.closing { }
    print("Swift actor client: real exact 4, independent state/cancel, MainActor pulses=\(pulse.count), background shutdown passed")
  }
}
