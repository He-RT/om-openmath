import Foundation

enum SessionEventKind:String,Codable,Sendable {
  case user_message,assistant_batch,assistant_end,tool_admitted,tool_started,tool_result
  case user_stop,task_transition,prompt_change,compaction,recovery
}
/// Canonical SessionEvent envelope from macos-storage.schema.json. Required nullable fields
/// use HostNullable so nil is encoded as an actual null, never omitted.
struct NativeSessionEvent:Codable,Sendable {
  let record_type:String
  let event_version:Int
  let agent_session_id:String
  let event_id:String
  let event_sequence:UInt64
  let source_instance_id:String
  let source_sequence:UInt64
  let parent_event_id:HostNullable<String>
  let agent_turn_id:HostNullable<String>
  let task_id:HostNullable<String>
  let event_kind:SessionEventKind
  let payload_hash:String
  let payload_inline_json:HostNullable<String>
  let payload_blob_hash:HostNullable<String>
  let operation_id:HostNullable<String>
  let operation_store_id:HostNullable<String>
  let created_at:String
}
struct SessionAppendInput:Codable,Sendable {
  let session:String,eventID:String,sourceInstance:String
  let sourceSequence:UInt64
  let parentEvent:String?,turn:String?,task:String?
  let kind:SessionEventKind
  let payload:Data
  let operation:String?,operationStore:String?
}
struct SessionEventReceipt:Codable,Sendable {
  let storeID:String
  let session:String
  let eventID:String
  let eventSequence:UInt64
  let sourceInstance:String
  let sourceSequence:UInt64
  let requestHash:String
  let recordHash:String
}
struct StoredSessionEvent:Sendable {let event:NativeSessionEvent;let receipt:SessionEventReceipt}
struct SessionAppendReceipt:Sendable {let stored:StoredSessionEvent;let inserted:Bool}
struct AgentSessionIdentity:Codable,Sendable {let codecVersion:Int,id:String;let createdAt:String}

struct SessionToolCall:Codable,Sendable {
  let callID:String
  let toolName:String
  /// Complete provider-decoded, data-only JSON. It is never executed by storage/recovery.
  let arguments:Data
}
struct SessionAssistantEnd:Codable,Sendable {
  let codecVersion:Int
  let messageID:String
  let textUTF8:Data
  let calls:[SessionToolCall]
}
struct SessionOperationTarget:Codable,Sendable,Equatable {
  let documentID:String
  let storeID:String
  let operationID:String
  let kind:DocumentEffectKind
  /// Source: frozen Rust patch request hash. Kernel: source snapshot hash frozen before
  /// execution (a result-dependent candidate hash cannot be known at admission).
  /// Save: exact frozen desired file hash.
  let expectedHash:String
}
struct SessionToolAdmission:Codable,Sendable {
  let codecVersion:Int
  let callID:String
  let proposalEvent:String
  let toolName:String
  let argumentsHash:String
  let target:SessionOperationTarget
}
struct SessionToolStarted:Codable,Sendable {let codecVersion:Int,admissionEvent:String}
struct SessionToolResult:Codable,Sendable {
  let codecVersion:Int
  let admissionEvent:String
  let sourceEventID:String
  let sourceEffectHash:String
  let rawReceiptHash:String
  let recovered:Bool
  let outcome:SessionToolOutcome
}
enum SessionToolOutcome:String,Codable,Sendable {case completed,partial}
enum RecoveredToolPhase:String,Codable,Sendable {case tool_not_started,tool_outcome_unknown,completed,partial}
struct RecoveredTool:Codable,Sendable {
  let session:String
  let proposalEvent:String
  let callID:String
  let admissionEvent:String?
  let phase:RecoveredToolPhase
  let target:SessionOperationTarget?
  let resultEvent:String?
}
struct SessionRequestPlan:Codable,Sendable {
  let codecVersion:Int
  let contextID:String
  let wireBlobHash:String
  let wireByteLength:UInt64
  let adapterVersion:String
}
enum SessionDispatchPhase:String,Codable,Sendable {case planned,dispatch_started,response_started,finished,failed,interrupted}
enum SessionRequestEventKind:String,Codable,Sendable {case request_planned,request_transition}
struct SessionRequestLifecycle:Codable,Sendable {
  let kind:SessionRequestEventKind
  let plan:SessionRequestPlan?
  let transition:SessionRequestTransition?
}
struct SessionRequestTransition:Codable,Sendable {
  let codecVersion:Int
  let contextID:String
  let phase:SessionDispatchPhase
  let responseBlobHash:String?
  let responseByteLength:UInt64?
}
enum RecoveredRequestPhase:String,Codable,Sendable {case not_dispatched,dispatch_outcome_unknown,response_observed,finished,failed}
struct RecoveredRequest:Codable,Sendable {
  let session:String
  let planEvent:String
  let contextID:String
  let phase:RecoveredRequestPhase
  let latestEvent:String
}
struct SessionRecoveryPage:Sendable {
  let tools:[RecoveredTool]
  let requests:[RecoveredRequest]
  let lastEventSequence:UInt64
  let complete:Bool
  var nextCursor:SessionRecoveryCursor?=nil
}
struct SessionRecoveryCursor:Sendable {
  let session:String
  let storeID:String
  let generation:Int64
  let upperSequence:UInt64
  let afterToolSequence:UInt64
  let afterToolID:String
  let afterRequestSequence:UInt64
}
struct SessionListRecord:Identifiable,Sendable {
  let id:String
  let createdAt:String
  let eventCount:UInt64
}
enum SessionLedgerError:Error,Sendable {case sequenceGap,invalidTransition,wrongSourceEffect,executionAlreadyClaimed,reconciliationRequired}
enum SessionValidation {
  static let maximumPayload=2*1024*1024
  static func json(_ bytes:Data)throws {
    guard !bytes.isEmpty,bytes.count<=maximumPayload,
      String(data:bytes,encoding:.utf8) != nil else {throw StorageError.corruptIdentity}
    _ = try JSONSerialization.jsonObject(with:bytes,options:[.fragmentsAllowed])
  }
  static func input(_ input:SessionAppendInput)throws {
    guard SourceValidation.identity(input.session),SourceValidation.identity(input.eventID),SourceValidation.identity(input.sourceInstance),
      input.sourceSequence>0,input.sourceSequence<=HostSerial.maximum else {throw StorageError.corruptIdentity}
    for id in [input.parentEvent,input.turn,input.task,input.operation,input.operationStore].compactMap({$0}) {guard SourceValidation.identity(id) else {throw StorageError.corruptIdentity}}
    try json(input.payload)
    if [.tool_admitted,.tool_started,.tool_result].contains(input.kind) {
      guard input.turn != nil,input.task != nil,input.operation != nil,input.operationStore != nil else {throw StorageError.corruptIdentity}
    }
  }
  static func event(_ event:NativeSessionEvent)throws {
    guard event.record_type=="session_event",event.event_version==1,event.event_sequence>0,event.event_sequence<=HostSerial.maximum,
      !event.created_at.isEmpty,event.created_at.utf8.count<=64,KernelValidation.hash(event.payload_hash),
      event.payload_blob_hash.value==nil,let payload=event.payload_inline_json.value,
      KernelHashes.data(Data(payload.utf8))==event.payload_hash else {throw StorageError.corruptIdentity}
    try input(.init(session:event.agent_session_id,eventID:event.event_id,sourceInstance:event.source_instance_id,sourceSequence:event.source_sequence,
      parentEvent:event.parent_event_id.value,turn:event.agent_turn_id.value,task:event.task_id.value,kind:event.event_kind,payload:Data(payload.utf8),operation:event.operation_id.value,operationStore:event.operation_store_id.value))
  }
  static func target(_ value:SessionOperationTarget)throws {
    guard UUID(uuidString:value.documentID) != nil,UUID(uuidString:value.storeID) != nil,
      SourceValidation.identity(value.operationID),KernelValidation.hash(value.expectedHash) else {throw StorageError.corruptIdentity}
  }
  static func requestHash(_ input:SessionAppendInput)throws->String {KernelHashes.data(try RecoveryValidation.encode(input))}
  static func payload<T:Decodable>(_ type:T.Type,_ event:NativeSessionEvent)throws->T {
    guard let text=event.payload_inline_json.value else {throw StorageError.corruptIdentity}
    return try JSONDecoder().decode(type,from:Data(text.utf8))
  }
}
