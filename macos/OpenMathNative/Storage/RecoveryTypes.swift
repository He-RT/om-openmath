import Foundation

enum DocumentEffectKind:String,Codable,Sendable {case source,kernel,save}
/// A private host projection of a confirmed document fact. Generation is provenance, not
/// event identity: migrating an intact store must not create a second tool result.
struct DocumentEffect:Codable,Sendable {
  let codecVersion:Int
  let sourceStoreID:String
  let sourceGeneration:Int64
  let documentID:String
  let eventID:String
  let operationID:String
  let kind:DocumentEffectKind
  let rawReceiptHash:String
  let payload:Data
  var stableHash:String {
    get throws {
      struct Identity:Encodable {
        let codecVersion:Int,sourceStoreID:String,documentID:String,eventID:String,operationID:String
        let kind:DocumentEffectKind,rawReceiptHash:String,payload:Data
      }
      let value=Identity(codecVersion:codecVersion,sourceStoreID:sourceStoreID,documentID:documentID,eventID:eventID,operationID:operationID,kind:kind,rawReceiptHash:rawReceiptHash,payload:payload)
      return KernelHashes.data(try RecoveryValidation.encode(value))
    }
  }
}
struct DocumentOutboxEntry:Sendable {let rowID:Int64;let effect:DocumentEffect}
struct DocumentOutboxPage:Sendable {
  let kind:DocumentEffectKind
  let entries:[DocumentOutboxEntry]
  let lastRowID:Int64
  let hasMore:Bool
}
struct LibraryProjectionReceipt:Sendable {
  let libraryStoreID:String
  let libraryGeneration:Int64
  let librarySequence:Int64
  let sourceStoreID:String
  let sourceGeneration:Int64
  let documentID:String
  let eventID:String
  let kind:DocumentEffectKind
  let effectHash:String
  let inserted:Bool
}
struct StoredDocumentEffect:Sendable {
  let sequence:Int64
  let effect:DocumentEffect
}
enum NativeLibraryAvailability:Sendable {
  case ready(StoreOpenInfo),legacy(StoreOpenInfo),unavailable(StorageError)
  var warning:String? {
    switch self {
    case .ready:return nil
    case .legacy:return "会话库需要格式升级；笔记本仍可编辑和保存，原库已保留。"
    case .unavailable(.unsupportedVersion):return "会话库格式暂不支持；笔记本仍可编辑和保存，原库已保留。"
    case .unavailable(.unknownCommit):return "会话库写入结果待核对；笔记本仍可编辑和保存，原库已保留。"
    case .unavailable:return "会话库需要恢复；笔记本仍可编辑和保存，原库已保留。"
    }
  }
}
enum RecoveryValidation {
  static let maximumPayload=4*1024*1024
  static func encode<T:Encodable>(_ value:T)throws->Data {
    let encoder=JSONEncoder();encoder.outputFormatting=[.sortedKeys,.withoutEscapingSlashes]
    return try encoder.encode(value)
  }
  static func effect(_ value:DocumentEffect)throws {
    guard value.codecVersion==1,UUID(uuidString:value.sourceStoreID) != nil,
      value.sourceGeneration>0,value.sourceGeneration<=999999,UUID(uuidString:value.documentID) != nil,
      SourceValidation.identity(value.eventID),SourceValidation.identity(value.operationID),
      KernelValidation.hash(value.rawReceiptHash),!value.payload.isEmpty,value.payload.count<=maximumPayload else {throw StorageError.corruptIdentity}
    // A projection is an actual typed receipt, not an arbitrary JSON success object.
    switch value.kind {
    case .source:
      let receipt=try JSONDecoder().decode(NativeDurableSourceReceipt.self,from:value.payload)
      guard receipt.protocol_version==1,receipt.receipt.store_id==value.sourceStoreID,
        receipt.receipt.document_id.value==value.documentID,receipt.receipt.operation_id==value.operationID,
        receipt.outbox_event_id==value.eventID,receipt.receipt.phase == .completed,
        KernelHashes.data(value.payload)==value.rawReceiptHash else {throw StorageError.corruptIdentity}
    case .kernel:
      let receipt=try JSONDecoder().decode(NativeKernelReceipt.self,from:value.payload)
      guard receipt.protocol_version==1,receipt.store_id==value.sourceStoreID,receipt.document_id==value.documentID,
        receipt.operation_id==value.operationID,receipt.outbox_event_id==value.eventID,
        KernelHashes.data(value.payload)==value.rawReceiptHash else {throw StorageError.corruptIdentity}
    case .save:
      let receipt=try JSONDecoder().decode(ProjectedSaveOutcome.self,from:value.payload)
      guard receipt.codecVersion==1,receipt.documentID==value.documentID,receipt.operationID==value.operationID,
        value.eventID=="save-event-"+value.operationID,receipt.sourceRevision<=HostSerial.maximum,
        receipt.bindingRevision>0,receipt.bindingRevision<=HostSerial.maximum,
        KernelValidation.hash(receipt.sourceSnapshotHash),KernelValidation.hash(receipt.fileHash),
        !receipt.completedAt.isEmpty else {throw StorageError.corruptIdentity}
    }
  }
}
/// SaveReceipt contains private bookmarks and paths. They stay in the document DB;
/// neither the Library effect nor a future Agent history reader receives that authorization.
struct ProjectedSaveOutcome:Codable,Sendable {
  let codecVersion:Int
  let operationID:String
  let documentID:String
  let sourceRevision:UInt64
  let sourceSnapshotHash:String
  let bindingRevision:UInt64
  let fileHash:String
  let recovered:Bool
  let completedAt:String
  init(_ receipt:SaveReceipt) {
    codecVersion=receipt.codecVersion;operationID=receipt.operationID;documentID=receipt.documentID
    sourceRevision=receipt.sourceRevision;sourceSnapshotHash=receipt.sourceSnapshotHash
    bindingRevision=receipt.bindingRevision;fileHash=receipt.fileHash;recovered=receipt.recovered;completedAt=receipt.completedAt
  }
}
