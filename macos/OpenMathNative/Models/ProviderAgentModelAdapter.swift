import Foundation

struct ProviderPreparedTurn:Sendable {
  let model:ProviderModelSession,route:ProviderRoute,journal:ProviderTransportJournal
  /// The native context owner retains original usage/replay/finish facts before canonical output.
  let complete:@Sendable (ProviderModelResult)async throws->Void
}
private actor ProviderTextEmitter {
  private var offset:UInt64=0
  let intent:NativeAgentRequestIntent,continuation:AsyncThrowingStream<AgentModelEventPayload,any Error>.Continuation
  init(intent:NativeAgentRequestIntent,continuation:AsyncThrowingStream<AgentModelEventPayload,any Error>.Continuation){self.intent=intent;self.continuation=continuation}
  func text(_ value:String)throws {
    var part="",bytes=0
    func emit()throws {
      guard !part.isEmpty else {return}
      let event=AgentModelEventPayload(agent_turn_id:intent.agent_turn_id,request_id:intent.request_id,byte_offset:try HostSerial(offset),text:part,phase:.delta,canonical_message:.init(nil))
      if case .dropped=continuation.yield(event) {throw ProviderTransportReason.limit}
      offset+=UInt64(bytes);part="";bytes=0
    }
    for scalar in value.unicodeScalars {
      let text=String(scalar),size=text.utf8.count
      if bytes+size>32768 {try emit()}
      part+=text;bytes+=size
    }
    try emit()
  }
  func end(_ result:ProviderModelResult)throws {
    let complete=result.finish != "incomplete"
    let event=AgentModelEventPayload(agent_turn_id:intent.agent_turn_id,request_id:intent.request_id,byte_offset:try HostSerial(offset),text:"",phase:complete ? .finished:.failed,canonical_message:.init(complete ? result.message:nil))
    if case .dropped=continuation.yield(event) {throw ProviderTransportReason.limit}
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
    let pair=AsyncThrowingStream<AgentModelEventPayload,any Error>.makeStream(bufferingPolicy:.bufferingOldest(32))
    let emitter=ProviderTextEmitter(intent:intent,continuation:pair.continuation)
    let work=Task {
      defer {active.removeValue(forKey:intent.request_id)}
      do {
        let result=try await transport.send(turn.model,route:turn.route,journal:turn.journal,onText:{try await emitter.text($0)})
        try Task.checkCancellation();try await turn.complete(result);try Task.checkCancellation()
        try await emitter.end(result);pair.continuation.finish()
      } catch {await turn.model.cancel();pair.continuation.finish(throwing:error)}
    }
    active[intent.request_id]=work
    pair.continuation.onTermination={_ in work.cancel()}
    return pair.stream
  }
  func close()async {closed=true;for work in active.values {work.cancel()};let turns=Array(pending.values);pending.removeAll();for turn in turns {await turn.model.cancel()}}
}
