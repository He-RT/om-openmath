import Foundation
import SQLite3

/// Queue-confined reader of the original same-document outboxes. Every receipt is checked
/// through its actual owner before Library sees it; recovery never evaluates source or writes a file.
final class DocumentOutbox {
  private let db:SQLiteDatabase,url:URL,identity:StoreIdentity
  private let source:DocumentStore,kernel:KernelStore?,saves:SaveStore?
  init(db:SQLiteDatabase,url:URL,identity:StoreIdentity,source:DocumentStore,kernel:KernelStore?,saves:SaveStore?) {
    self.db=db;self.url=url;self.identity=identity;self.source=source;self.kernel=kernel;self.saves=saves
  }
  private func table(_ kind:DocumentEffectKind)throws->String {
    switch kind {
    case .source:return "outbox"
    case .kernel:guard kernel != nil else {throw StorageError.unsupportedVersion};return "kernel_outbox"
    case .save:guard saves != nil else {throw StorageError.unsupportedVersion};return "save_outbox"
    }
  }
  private func validated(_ row:OpaquePointer,kind:DocumentEffectKind)throws->DocumentOutboxEntry {
    let position=sqlite3_column_int64(row,0),event=try SQLColumn.text(row,1),document=try SQLColumn.text(row,2),operation=try SQLColumn.text(row,3)
    guard sqlite3_column_bytes(row,4)>0,sqlite3_column_bytes(row,4)<=RecoveryValidation.maximumPayload else {throw StorageError.corruptIdentity}
    let bytes=try SQLColumn.data(row,4),hash=try SQLColumn.text(row,5),projected=sqlite3_column_int64(row,6)
    guard position>0,document==identity.documentID,KernelHashes.data(bytes)==hash,(0...1).contains(projected) else {throw StorageError.corruptIdentity}
    let payload:Data
    switch kind {
    case .source:
      guard let actual=try source.query(operation) else {throw StorageError.corruptIdentity}
      let decoded=try JSONDecoder().decode(NativeDurableSourceReceipt.self,from:bytes)
      guard try RecoveryValidation.encode(actual)==RecoveryValidation.encode(decoded) else {throw StorageError.corruptIdentity}
      payload=bytes
    case .kernel:
      guard let actual=try kernel?.query(operation) else {throw StorageError.corruptIdentity}
      let decoded=try JSONDecoder().decode(NativeKernelReceipt.self,from:bytes)
      guard try RecoveryValidation.encode(actual)==RecoveryValidation.encode(decoded) else {throw StorageError.corruptIdentity}
      payload=bytes
    case .save:
      guard let actual=try saves?.query(operation) else {throw StorageError.corruptIdentity}
      let decoded=try JSONDecoder().decode(SaveReceipt.self,from:bytes)
      guard try RecoveryValidation.encode(actual)==RecoveryValidation.encode(decoded) else {throw StorageError.corruptIdentity}
      payload=try RecoveryValidation.encode(ProjectedSaveOutcome(actual))
    }
    let effect=DocumentEffect(codecVersion:1,sourceStoreID:identity.storeID,sourceGeneration:identity.generation,documentID:document,eventID:event,operationID:operation,kind:kind,rawReceiptHash:hash,payload:payload)
    try RecoveryValidation.effect(effect)
    return .init(rowID:position,effect:effect)
  }
  func verify(_ effect:DocumentEffect)throws->DocumentEffect {
    try RecoveryValidation.effect(effect)
    guard effect.sourceStoreID==identity.storeID,effect.sourceGeneration==identity.generation,effect.documentID==identity.documentID else {throw StorageError.corruptIdentity}
    let name=try table(effect.kind),reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    var actual:DocumentEffect?
    try reader.statement("SELECT rowid,event_id,document_id,operation_id,payload,payload_hash,projected FROM \(name) WHERE event_id=?",[.text(effect.eventID)]) { [self] row in
      actual=try validated(row,kind:effect.kind).effect
    }
    guard let actual,try actual.stableHash==effect.stableHash else {throw StorageError.corruptIdentity}
    return actual
  }
  func page(_ kind:DocumentEffectKind,after:Int64,limit:Int)throws->DocumentOutboxPage {
    guard after>=0,limit>0,limit<=16 else {throw StorageError.corruptIdentity}
    let name=try table(kind),reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    var invalid:Int64 = -1
    try reader.statement("SELECT count(*) FROM \(name) WHERE rowid<=0 OR document_id != ?",[.text(identity.documentID!)]) {invalid=sqlite3_column_int64($0,0)}
    guard invalid==0 else {throw StorageError.corruptIdentity}
    var entries:[DocumentOutboxEntry]=[],size=0,hasMore=false,last=after
    try reader.statement("SELECT rowid,event_id,document_id,operation_id,payload,payload_hash,projected FROM \(name) WHERE rowid>? ORDER BY rowid LIMIT ?",[.integer(after),.integer(Int64(limit+1))]) { [self] row in
      if hasMore {return}
      if entries.count==limit {hasMore=true;return}
      let count=Int(sqlite3_column_bytes(row,4))
      guard count>0,count<=RecoveryValidation.maximumPayload else {throw StorageError.corruptIdentity}
      if !entries.isEmpty && size+count>RecoveryValidation.maximumPayload {hasMore=true;return}
      let entry=try validated(row,kind:kind);guard entry.rowID>last else {throw StorageError.corruptIdentity}
      entries.append(entry);last=entry.rowID;size+=count
    }
    return .init(kind:kind,entries:entries,lastRowID:last,hasMore:hasMore)
  }
  func acknowledge(_ effect:DocumentEffect,receipt:LibraryProjectionReceipt,faults:StorageFaults)throws {
    try RecoveryValidation.effect(effect)
    guard effect.sourceStoreID==identity.storeID,effect.sourceGeneration==identity.generation,effect.documentID==identity.documentID,
      receipt.sourceStoreID==identity.storeID,receipt.sourceGeneration==identity.generation,receipt.documentID==identity.documentID,
      receipt.eventID==effect.eventID,receipt.kind==effect.kind,receipt.effectHash == (try effect.stableHash),
      UUID(uuidString:receipt.libraryStoreID) != nil,receipt.libraryGeneration>0,receipt.librarySequence>0 else {throw StorageError.corruptIdentity}
    let name=try table(effect.kind)
    try db.transaction {
      var matched=false
      try db.statement("SELECT operation_id,payload_hash FROM \(name) WHERE event_id=? AND document_id=?",[.text(effect.eventID),.text(effect.documentID)]) {row in
        guard try SQLColumn.text(row,0)==effect.operationID,try SQLColumn.text(row,1)==effect.rawReceiptHash else {throw StorageError.corruptIdentity};matched=true
      }
      guard matched else {throw StorageError.corruptIdentity}
      try db.statement("UPDATE \(name) SET projected=1 WHERE event_id=? AND document_id=?",[.text(effect.eventID),.text(effect.documentID)])
      try faults.reach("document_projection_before_commit")
    }
    do {
      try faults.reach("document_projection_committed")
      let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
      var matched=false
      try reader.statement("SELECT projected,payload_hash FROM \(name) WHERE event_id=? AND document_id=?",[.text(effect.eventID),.text(effect.documentID)]) {row in
        guard sqlite3_column_int64(row,0)==1,try SQLColumn.text(row,1)==effect.rawReceiptHash else {throw StorageError.corruptIdentity};matched=true
      }
      guard matched else {throw StorageError.corruptIdentity}
      try db.confirmCommittedBytes()
    } catch {throw StorageError.unknownCommit(EIO)}
  }
}
