import Foundation

private struct ProviderHeaderObservation:Codable,Sendable {let codecVersion:Int,kind:String,request:String,context:String,status:UInt16}
private struct ProviderReplyReference:Codable,Sendable {let codecVersion:Int,kind:String,request:String,context:String,blobHash:String,byteLength:UInt64}
private struct ProviderStoredReply:Codable,Sendable {let codecVersion:Int,canonical:AgentCanonicalMessage,finish:String,providerReason:String,usageJSON:String,replayJSON:String?,partialCallsJSON:String}

/// Actual Library event/blob producer for one frozen request. Recovery contains no transport.
actor ProviderRequestLedger {
  private let storage:StorageService,ledger:SessionLedgerPort,model:ProviderModelSession
  private let base:String,faults:StorageFaults
  private var parent:String?,phase:SessionDispatchPhase?,status:UInt16?,response=Data(),writing=false
  init(storage:StorageService,ledger:SessionLedgerPort,model:ProviderModelSession,faults:StorageFaults = .init()) {
    self.storage=storage;self.ledger=ledger;self.model=model;self.faults=faults
    base="provider-"+KernelHashes.data(Data((ledger.session+"\u{0}"+model.prepared.context_ref+"\u{0}"+model.prepared.request_ref).utf8))
  }
  nonisolated func journal()->ProviderTransportJournal {
    .init(record:{[self] in try await record($0)},responseBytes:{[self] in try await receive($0)})
  }
  nonisolated func turn(route:ProviderRoute)->ProviderPreparedTurn {
    .init(model:model,route:route,journal:journal(),complete:{[self] in try await complete($0)})
  }
  private func plan()async throws {
    guard phase==nil else {return}
    let publication=try await storage.publishBlob(data:model.body)
    do {
      let plan=SessionRequestPlan(codecVersion:1,contextID:model.prepared.context_ref,wireBlobHash:publication.descriptor.hash,wireByteLength:publication.descriptor.byteLength,adapterVersion:model.binding.adapter)
      let receipt=try await ledger.record(SessionRequestLifecycle(kind:.request_planned,plan:plan,transition:nil),id:base+".planned",kind:.task_transition,turn:model.prepared.agent_turn_id,task:model.prepared.task_id,resource:publication,resourceOwner:.init(kind:.context,id:"request-"+ledger.session+"-"+model.prepared.context_ref),faults:faults)
      try await storage.releaseBlob(publication)
      guard receipt.inserted else {throw SessionLedgerError.reconciliationRequired}
      parent=base+".planned";phase = .planned;try faults.reach("provider_request_plan_durable")
    } catch {try? await storage.releaseBlob(publication);throw error}
  }
  private func transition(_ next:SessionDispatchPhase,data:Data?=nil,suffix:String)async throws {
    guard let parent else {throw SessionLedgerError.invalidTransition}
    let publication:BlobPublication?
    if let data {publication=try await storage.publishBlob(data:data)} else {publication=nil}
    do {
      let body=SessionRequestTransition(codecVersion:1,contextID:model.prepared.context_ref,phase:next,responseBlobHash:publication?.descriptor.hash,responseByteLength:publication?.descriptor.byteLength)
      let receipt=try await ledger.record(SessionRequestLifecycle(kind:.request_transition,plan:nil,transition:body),id:base+suffix,kind:.task_transition,parent:parent,turn:model.prepared.agent_turn_id,task:model.prepared.task_id,resource:publication,resourceOwner:publication == nil ? nil:.init(kind:.context,id:"response-"+ledger.session+"-"+model.prepared.context_ref),faults:faults)
      if let publication {try await storage.releaseBlob(publication)}
      guard receipt.inserted else {throw SessionLedgerError.reconciliationRequired}
      self.parent=base+suffix;phase=next
    } catch {if let publication {try? await storage.releaseBlob(publication)};throw error}
  }
  private func observe()async throws {
    guard phase == .dispatch_started else {return}
    let bytes:Data
    if !response.isEmpty {bytes=response}
    else {guard let status else {return};bytes=try RecoveryValidation.encode(ProviderHeaderObservation(codecVersion:1,kind:"provider_http_headers_only",request:model.prepared.request_ref,context:model.prepared.context_ref,status:status))}
    try await transition(.response_started,data:bytes,suffix:".response")
    try faults.reach("provider_response_durable")
  }
  private func record(_ fact:ProviderJournalFact)async throws {
    guard !writing,fact.request==model.prepared.request_ref,fact.context==model.prepared.context_ref else {throw SessionLedgerError.invalidTransition}
    writing=true;defer {writing=false}
    switch fact.phase {
    case .dispatchStarted:
      try await plan();guard phase == .planned else {throw SessionLedgerError.reconciliationRequired}
      try await transition(.dispatch_started,suffix:".dispatch");try faults.reach("provider_dispatch_durable")
    case .responseStarted:
      guard phase == .dispatch_started,let status=fact.status else {throw SessionLedgerError.invalidTransition};self.status=status
      // Do not call a volatile header callback durable response bytes. receive/terminal pins it.
    case .finished:
      try await observe();guard phase == .response_started,!response.isEmpty else {throw SessionLedgerError.invalidTransition}
      try await transition(.finished,data:response,suffix:".finished");try faults.reach("provider_finish_durable")
    case .failed,.interrupted:
      if phase == .finished || phase == .failed || phase == .interrupted {return}
      try await plan();status=fact.status ?? status
      if fact.outcome == .responseObserved {try await observe()}
      let terminal:SessionDispatchPhase = fact.outcome == .dispatchOutcomeUnknown || fact.phase == .interrupted || fact.outcome == .notDispatched ? .interrupted:.failed
      try await transition(terminal,suffix:".terminal")
    }
  }
  private func receive(_ bytes:Data)async throws {
    guard !writing,phase == .dispatch_started || phase == .response_started,response.count+bytes.count<=8*1024*1024 else {throw SessionLedgerError.invalidTransition}
    writing=true;defer {writing=false};response.append(bytes)
    if !bytes.isEmpty {try await observe()}
  }
  func complete(_ result:ProviderModelResult)async throws {
    guard !writing,phase == .finished,result.message.message_id==model.prepared.message_id,result.message.task_id.value==model.prepared.task_id,result.message.agent_turn_id.value==model.prepared.agent_turn_id else {throw SessionLedgerError.invalidTransition}
    writing=true;defer {writing=false}
    let bytes=try RecoveryValidation.encode(ProviderStoredReply(codecVersion:1,canonical:result.message,finish:result.finish,providerReason:result.providerReason,usageJSON:String(decoding:result.usageJSON,as:UTF8.self),replayJSON:result.replayJSON.map{String(decoding:$0,as:UTF8.self)},partialCallsJSON:String(decoding:result.partialCallsJSON,as:UTF8.self)))
    let publication=try await storage.publishBlob(data:bytes)
    do {
      _ = try await ledger.record(ProviderReplyReference(codecVersion:1,kind:"provider_model_reply",request:model.prepared.request_ref,context:model.prepared.context_ref,blobHash:publication.descriptor.hash,byteLength:publication.descriptor.byteLength),id:base+".model",kind:.recovery,parent:parent,turn:model.prepared.agent_turn_id,task:model.prepared.task_id,resource:publication,resourceOwner:.init(kind:.context,id:base+".model"),faults:faults)
      try await storage.releaseBlob(publication);try faults.reach("provider_model_reply_durable")
    } catch {try? await storage.releaseBlob(publication);throw error}
  }
}
