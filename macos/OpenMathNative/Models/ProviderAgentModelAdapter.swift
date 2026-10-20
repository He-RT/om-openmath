import Foundation

struct ProviderPreparedTurn:Sendable {
  let model:ProviderModelSession,route:ProviderRoute,journal:ProviderTransportJournal
  /// The native context owner retains original usage/replay/finish facts before canonical output.
  let complete:@Sendable (ProviderModelResult)async throws->Void
}
private final class ProviderStreamLifetime:Sendable {
  let cancel:@Sendable ()->Void
  init(cancel:@escaping @Sendable ()->Void){self.cancel=cancel}
  deinit {cancel()}
}
private actor ProviderTextEmitter {
  private var offset:UInt64=0
  private var bytes=Data(),result:ProviderModelResult?,failure:(any Error)?,ended=false
  private var waiting:CheckedContinuation<Void,Never>?
  let intent:NativeAgentRequestIntent
  init(intent:NativeAgentRequestIntent){self.intent=intent}
  private func wake() {let waiter=waiting;waiting=nil;waiter?.resume()}
  func text(_ value:String)throws {
    guard !ended,result==nil,failure==nil else {throw ProviderTransportReason.stale}
    guard bytes.count+value.utf8.count<=8*1024*1024 else {throw ProviderTransportReason.limit}
    bytes.append(contentsOf:value.utf8);wake()
  }
  func end(_ result:ProviderModelResult) {self.result=result;wake()}
  func fail(_ error:any Error) {failure=error;wake()}
  func cancel() {ended=true;failure=CancellationError();wake()}
  // Pull chunks at the native consumer's pace. A burst of small provider tokens is
  // bounded by the same 8 MiB response limit, rather than an arbitrary event count.
  func next() async throws->AgentModelEventPayload? {
    while Int(offset)==bytes.count,result==nil,failure==nil,!ended {
      await withCheckedContinuation {waiting=$0}
    }
    try Task.checkCancellation()
    if Int(offset)<bytes.count,!ended {
      let start=Int(offset)
      var end=min(start+32768,bytes.count)
      while end<bytes.count,bytes[end]&0xc0==0x80 {end-=1}
      let text=String(decoding:bytes[start..<end],as:UTF8.self)
      let event=AgentModelEventPayload(agent_turn_id:intent.agent_turn_id,request_id:intent.request_id,byte_offset:try HostSerial(offset),text:text,phase:.delta,canonical_message:.init(nil))
      offset=UInt64(end);return event
    }
    if let failure {throw failure}
    guard !ended,let result else {return nil}
    ended=true
    let complete=result.finish != "incomplete"
    return AgentModelEventPayload(agent_turn_id:intent.agent_turn_id,request_id:intent.request_id,byte_offset:try HostSerial(offset),text:"",phase:complete ? .finished:.failed,canonical_message:.init(complete ? result.message:nil))
  }
}
/// Production native port supplied to AgentSessionService. Pi remains the only model/tool loop.
actor ProviderAgentModelAdapter {
  typealias Factory = @Sendable (NativeAgentRequestIntent)async throws->ProviderPreparedTurn
  private let transport:ProviderTransport,factory:Factory
  private var pending:[String:ProviderPreparedTurn]=[:],preparing=Set<String>(),closed=false
  private var active:[String:Task<Void,Never>]=[:]
  init(transport:ProviderTransport,factory:@escaping Factory){self.transport=transport;self.factory=factory}
  nonisolated func port()->AgentNativeModelPort {
    .init(prepare:{[self] in try await prepare($0)},stream:{[self] in try await stream($0)})
  }
  private func prepare(_ intent:NativeAgentRequestIntent)async throws->AgentPreparedRequest {
    guard !closed,pending.count+preparing.count+active.count<8,pending[intent.request_id]==nil,active[intent.request_id]==nil,preparing.insert(intent.request_id).inserted else {throw ProviderTransportReason.stale}
    defer {preparing.remove(intent.request_id)}
    let turn=try await factory(intent)
    guard !closed,turn.model.prepared.request_ref==intent.request_id,turn.model.prepared.agent_turn_id==intent.agent_turn_id,turn.model.binding==turn.route.binding else {await turn.model.cancel();throw ProviderTransportReason.stale}
    pending[intent.request_id]=turn
    return .init(context:turn.model.prepared.context_ref,model:turn.model.binding.model)
  }
  private func stream(_ intent:NativeAgentRequestIntent)throws->AsyncThrowingStream<AgentModelEventPayload,any Error> {
    guard !closed,let turn=pending.removeValue(forKey:intent.request_id),turn.model.prepared.agent_turn_id==intent.agent_turn_id else {throw ProviderTransportReason.stale}
    let emitter=ProviderTextEmitter(intent:intent)
    let work=Task {
      defer {active.removeValue(forKey:intent.request_id)}
      do {
        let result=try await transport.send(turn.model,route:turn.route,journal:turn.journal,onText:{try await emitter.text($0)})
        try Task.checkCancellation();try await turn.complete(result);try Task.checkCancellation()
        await emitter.end(result)
      } catch {await turn.model.cancel();await emitter.fail(error)}
    }
    active[intent.request_id]=work
    let lifetime=ProviderStreamLifetime {work.cancel();Task {await emitter.cancel()}}
    return AsyncThrowingStream(unfolding:{
      try await withTaskCancellationHandler {
        try await emitter.next()
      } onCancel:{lifetime.cancel()}
    })
  }
  func close()async {closed=true;for work in active.values {work.cancel()};let turns=Array(pending.values);pending.removeAll();for turn in turns {await turn.model.cancel()}}
}
