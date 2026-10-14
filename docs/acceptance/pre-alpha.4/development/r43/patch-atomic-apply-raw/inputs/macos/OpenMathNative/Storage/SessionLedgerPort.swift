import Foundation

/// Host-owned producer. Complete events settle before the caller can clear a composer,
/// execute a tool or dispatch a request. An ACK retry reads the original event identity.
actor SessionLedgerPort {
  let session:String
  private let storage:StorageService
  private let sourceInstance:String
  private var sequence:UInt64=0
  private var pending:SessionAppendInput?
  private var writing=false
  init(storage:StorageService,session:String,sourceInstance:String="host-"+UUID().uuidString.lowercased()) {self.storage=storage;self.session=session;self.sourceInstance=sourceInstance}
  func record<T:Encodable & Sendable>(_ payload:T,id:String,kind:SessionEventKind,parent:String?=nil,turn:String?=nil,task:String?=nil,target:SessionOperationTarget?=nil,resource:BlobPublication?=nil,resourceOwner:BlobOwner?=nil,faults:StorageFaults = .init()) async throws->SessionAppendReceipt {
    guard !writing else {throw StorageError.inUse};writing=true;defer {writing=false}
    let bytes=try await Task.detached {try RecoveryValidation.encode(payload)}.value
    if let original=try await storage.sessionEvent(session,event:id) {
      let e=original.event
      guard e.event_kind==kind,e.parent_event_id.value==parent,e.agent_turn_id.value==turn,e.task_id.value==task,
        e.operation_id.value==target?.operationID,e.operation_store_id.value==target?.storeID,
        e.payload_hash==KernelHashes.data(bytes),e.payload_inline_json.value.map({Data($0.utf8)})==bytes else {throw StorageError.idempotencyConflict}
      if pending?.eventID==id {
        guard let pending,try SessionValidation.requestHash(pending)==original.receipt.requestHash else {throw StorageError.corruptIdentity}
        sequence=original.event.source_sequence;self.pending=nil
      }
      return .init(stored:original,inserted:false)
    }
    guard pending==nil || pending?.eventID==id else {throw SessionLedgerError.reconciliationRequired}
    guard sequence<HostSerial.maximum else {throw StorageError.corruptIdentity}
    let input=SessionAppendInput(session:session,eventID:id,sourceInstance:sourceInstance,sourceSequence:sequence+1,parentEvent:parent,turn:turn,task:task,kind:kind,payload:bytes,operation:target?.operationID,operationStore:target?.storeID)
    if let pending {guard try SessionValidation.requestHash(pending)==SessionValidation.requestHash(input) else {throw StorageError.idempotencyConflict}}
    do {
      let actual:SessionAppendReceipt
      if let resource,let resourceOwner {actual=try await storage.appendSessionResource(input,publication:resource,owner:resourceOwner,faults:faults)}
      else {guard resource==nil,resourceOwner==nil else {throw StorageError.corruptIdentity};actual=try await storage.appendSessionEvent(input,faults:faults)}
      sequence=actual.stored.event.source_sequence;pending=nil;return actual
    } catch StorageError.unknownCommit {pending=input;throw StorageError.unknownCommit(EIO)}
  }
}

/// The callback is deliberately absent from recovery. A repeated original claim can only
/// reconcile actual document effects; it never invokes the tool again after an ACK loss.
actor SessionToolPort {
  private let storage:StorageService,ledger:SessionLedgerPort
  private var busy=false
  init(storage:StorageService,ledger:SessionLedgerPort) {self.storage=storage;self.ledger=ledger}
  /// Read the original operation only. This path has no execution callback and cannot retry it.
  func reconcile(admission:SessionToolAdmission,admissionID:String,resultID:String,turn:String,task:String)async throws->SessionAppendReceipt? {
    if let existing=try await storage.sessionEvent(ledger.session,event:resultID) {
      guard existing.event.event_kind == .tool_result,existing.event.parent_event_id.value==admissionID,
        existing.event.operation_id.value==admission.target.operationID,existing.event.operation_store_id.value==admission.target.storeID else {throw SessionLedgerError.wrongSourceEffect}
      return .init(stored:existing,inserted:false)
    }
    guard let original=try await storage.originalOperationEffect(admission.target) else {return nil}
    _ = try await storage.projectDocumentEffect(original)
    let result=SessionToolResult(codecVersion:1,admissionEvent:admissionID,sourceEventID:original.eventID,sourceEffectHash:try original.stableHash,rawReceiptHash:original.rawReceiptHash,recovered:true,outcome:try SessionSemantics.effectOutcome(original,target:admission.target))
    return try await ledger.record(result,id:resultID,kind:.tool_result,parent:admissionID,turn:turn,task:task,target:admission.target)
  }
  func perform(admission:SessionToolAdmission,admissionID:String,startedID:String,resultID:String,turn:String,task:String,faults:StorageFaults = .init(),execute:@escaping @Sendable ()async throws->Void) async throws->SessionAppendReceipt {
    try Task.checkCancellation()
    guard !busy else {throw StorageError.inUse};busy=true;defer {busy=false}
    let document=try await storage.documentIdentity(admission.target.documentID)
    guard document.storeID==admission.target.storeID else {throw SessionLedgerError.wrongSourceEffect}
    let admitted=try await ledger.record(admission,id:admissionID,kind:.tool_admitted,parent:admission.proposalEvent,turn:turn,task:task,target:admission.target,faults:faults)
    guard admitted.inserted else {throw SessionLedgerError.reconciliationRequired}
    try faults.reach("tool_admission_acknowledged")
    try Task.checkCancellation()
    if let original=try await storage.originalOperationEffect(admission.target) {
      _ = try await storage.projectDocumentEffect(original,faults:faults)
      let recovered=SessionToolResult(codecVersion:1,admissionEvent:admissionID,sourceEventID:original.eventID,sourceEffectHash:try original.stableHash,rawReceiptHash:original.rawReceiptHash,recovered:true,outcome:try SessionSemantics.effectOutcome(original,target:admission.target))
      return try await ledger.record(recovered,id:resultID,kind:.tool_result,parent:admissionID,turn:turn,task:task,target:admission.target,faults:faults)
    }
    let started=try await ledger.record(SessionToolStarted(codecVersion:1,admissionEvent:admissionID),id:startedID,kind:.tool_started,parent:admissionID,turn:turn,task:task,target:admission.target,faults:faults)
    guard started.inserted else {throw SessionLedgerError.reconciliationRequired}
    try faults.reach("tool_execution_claimed")
    try Task.checkCancellation()
    try await execute()
    try faults.reach("tool_effect_returned")
    guard let actual=try await storage.originalOperationEffect(admission.target) else {throw SessionLedgerError.reconciliationRequired}
    _ = try await storage.projectDocumentEffect(actual,faults:faults)
    let result=SessionToolResult(codecVersion:1,admissionEvent:admissionID,sourceEventID:actual.eventID,sourceEffectHash:try actual.stableHash,rawReceiptHash:actual.rawReceiptHash,recovered:false,outcome:try SessionSemantics.effectOutcome(actual,target:admission.target))
    return try await ledger.record(result,id:resultID,kind:.tool_result,parent:admissionID,turn:turn,task:task,target:admission.target,faults:faults)
  }
}

struct SessionWireResponse:Sendable {let bytes:Data;let successful:Bool}
actor SessionRequestPort {
  private let storage:StorageService,ledger:SessionLedgerPort
  private var busy=false
  init(storage:StorageService,ledger:SessionLedgerPort) {self.storage=storage;self.ledger=ledger}
  /// Request body only. Credentials/headers are supplied later by the trusted URLSession
  /// adapter, never by this persistence API or any data reconstructed during recovery.
  func send(context:String,wire:Data,adapter:String,turn:String,task:String,faults:StorageFaults = .init(),dispatch:@escaping @Sendable (Data)async throws->SessionWireResponse) async throws->SessionWireResponse {
    try Task.checkCancellation()
    guard !busy else {throw StorageError.inUse};busy=true;defer {busy=false}
    try SessionValidation.json(wire)
    let published=try await storage.publishBlob(data:wire)
    do {
      let plan=SessionRequestPlan(codecVersion:1,contextID:context,wireBlobHash:published.descriptor.hash,wireByteLength:published.descriptor.byteLength,adapterVersion:adapter)
      let planned=try await ledger.record(SessionRequestLifecycle(kind:.request_planned,plan:plan,transition:nil),id:context+".planned",kind:.task_transition,turn:turn,task:task,resource:published,resourceOwner:.init(kind:.context,id:"request-"+ledger.session+"-"+context),faults:faults)
      try await storage.releaseBlob(published)
      guard planned.inserted else {throw SessionLedgerError.reconciliationRequired}
      try faults.reach("request_plan_acknowledged")
    } catch {try? await storage.releaseBlob(published);throw error}
    let starting=SessionRequestTransition(codecVersion:1,contextID:context,phase:.dispatch_started,responseBlobHash:nil,responseByteLength:nil)
    let started=try await ledger.record(SessionRequestLifecycle(kind:.request_transition,plan:nil,transition:starting),id:context+".dispatch",kind:.task_transition,parent:context+".planned",turn:turn,task:task,faults:faults)
    guard started.inserted else {throw SessionLedgerError.reconciliationRequired}
    try faults.reach("request_dispatch_acknowledged")
    try Task.checkCancellation()
    // No retry or inferred failure if this throws. The durable phase remains outcome_unknown.
    let reply=try await dispatch(wire)
    try faults.reach("request_response_observed")
    let response=try await storage.publishBlob(data:reply.bytes)
    do {
      let observed=SessionRequestTransition(codecVersion:1,contextID:context,phase:.response_started,responseBlobHash:response.descriptor.hash,responseByteLength:response.descriptor.byteLength)
      _ = try await ledger.record(SessionRequestLifecycle(kind:.request_transition,plan:nil,transition:observed),id:context+".response",kind:.task_transition,parent:context+".dispatch",turn:turn,task:task,resource:response,resourceOwner:.init(kind:.context,id:"response-"+ledger.session+"-"+context),faults:faults)
      try faults.reach("request_response_acknowledged")
      try await storage.releaseBlob(response)
      let finished=SessionRequestTransition(codecVersion:1,contextID:context,phase:reply.successful ? .finished : .failed,responseBlobHash:reply.successful ? response.descriptor.hash : nil,responseByteLength:reply.successful ? response.descriptor.byteLength : nil)
      _ = try await ledger.record(SessionRequestLifecycle(kind:.request_transition,plan:nil,transition:finished),id:context+".finished",kind:.task_transition,parent:context+".response",turn:turn,task:task,faults:faults)
      try faults.reach("request_finish_acknowledged")
      return reply
    } catch {try? await storage.releaseBlob(response);throw error}
  }
}
