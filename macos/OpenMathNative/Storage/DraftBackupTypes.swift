import Foundation

struct DraftTextRange:Codable,Sendable,Equatable {
  let location:Int
  let length:Int
  init(_ range:NSRange) {location=range.location;length=range.length}
  var native:NSRange {NSRange(location:location,length:length)}
  func validate(in text:String)throws {
    let count=text.utf16.count
    guard location>=0,length>=0,location<=count,length<=count-location,
      String.Index(utf16Offset:location,in:text).samePosition(in:text.unicodeScalars) != nil,
      String.Index(utf16Offset:location+length,in:text).samePosition(in:text.unicodeScalars) != nil else {throw StorageError.corruptIdentity}
  }
}
struct DraftBackupCell:Codable,Sendable {
  let cellID:String
  let editorGeneration:UInt64
  let draftSequence:UInt64
  let baseCellRevision:UInt64
  let baseUTF8:Data
  let sourceUTF8:Data
  let wasComposing:Bool
  let conflict:Bool
  let requiresReview:Bool
  let selections:[DraftTextRange]
  let markedRange:DraftTextRange?
}
struct DraftBackupOwner:Codable,Sendable,Equatable {
  let ownerID:String
  let storeID:String
  let documentID:String
  let leaseSerial:UInt64
  let runtime:String
  let documentGeneration:UInt64
}
struct DraftBackupSnapshot:Codable,Sendable {
  let codecVersion:Int
  let owner:DraftBackupOwner
  let sequence:UInt64
  let sourceRevision:UInt64
  let sourceSnapshotHash:String
  let cells:[DraftBackupCell]
}
struct DraftBackupReceipt:Codable,Sendable {
  let codecVersion:Int
  let owner:DraftBackupOwner
  let sequence:UInt64
  let snapshotHash:String
}
struct StoredDraftBackup:Sendable {let receipt:DraftBackupReceipt;let snapshot:DraftBackupSnapshot}
enum DraftBackupValidation {
  static let maximumTextBytes=4*1024*1024
  static let maximumRecordBytes=8*1024*1024
  static func owner(_ value:DraftBackupOwner)throws {
    guard UUID(uuidString:value.ownerID) != nil,UUID(uuidString:value.storeID) != nil,UUID(uuidString:value.documentID) != nil,
      value.leaseSerial>0,value.leaseSerial<=HostSerial.maximum,SourceValidation.identity(value.runtime),
      value.documentGeneration>0,value.documentGeneration<=HostSerial.maximum else {throw StorageError.corruptIdentity}
  }
  static func snapshot(_ value:DraftBackupSnapshot)throws {
    try owner(value.owner)
    guard value.codecVersion==1,value.sequence>0,value.sequence<=HostSerial.maximum,value.sourceRevision<=HostSerial.maximum,
      KernelValidation.hash(value.sourceSnapshotHash),value.cells.count<=10000,
      Set(value.cells.map{Data($0.cellID.utf8)}).count==value.cells.count else {throw StorageError.corruptIdentity}
    var bytes=0
    for cell in value.cells {
      guard !cell.cellID.isEmpty,cell.cellID.utf8.count<=256,cell.editorGeneration<=HostSerial.maximum,
        cell.draftSequence<=HostSerial.maximum,cell.baseCellRevision<=value.sourceRevision,
        cell.baseUTF8.count<=maximumTextBytes,cell.sourceUTF8.count<=maximumTextBytes,
        let base=String(data:cell.baseUTF8,encoding:.utf8),let text=String(data:cell.sourceUTF8,encoding:.utf8),
        cell.selections.count>0,cell.selections.count<=256 else {throw StorageError.corruptIdentity}
      bytes+=base.utf8.count+text.utf8.count+cell.cellID.utf8.count
      guard bytes<=maximumTextBytes else {throw StorageError.corruptIdentity}
      for selection in cell.selections {try selection.validate(in:text)}
      if let marked=cell.markedRange {guard cell.wasComposing else {throw StorageError.corruptIdentity};try marked.validate(in:text)}
    }
  }
  static func encode<T:Encodable>(_ value:T)throws->Data {
    let bytes=try RecoveryValidation.encode(value);guard bytes.count<=maximumRecordBytes else {throw StorageError.corruptIdentity};return bytes
  }
}
