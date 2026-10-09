import CryptoKit
import Foundation
import SQLite3

/// Physical storage consumes only trusted frozen plans. Permissions and editor/CAS fences remain
/// Rust owner duties. No model-supplied SQL/path, no optimistic document projection on write.
final class DocumentStore {
  private let db:SQLiteDatabase
  private let url:URL
  private let identity:StoreIdentity
  init(db:SQLiteDatabase,url:URL,identity:StoreIdentity) throws {
    guard identity.kind=="document",identity.documentID != nil else { throw StorageError.corruptIdentity }
    self.db=db;self.url=url;self.identity=identity
    try DocumentSchema.ensure(db)
  }
  private func encoded<T:Encodable>(_ value:T) throws ->Data {
    let bytes=try JSONEncoder().encode(value)
    guard bytes.count<=4*1024*1024 else { throw StorageError.corruptIdentity }
    return bytes
  }
  static func digest(_ data:Data)->String { SHA256.hash(data:data).map{String(format:"%02x",$0)}.joined() }
  func initialize(_ source:NativeSourceSnapshot) throws ->NativeSourceSnapshot {
    try SourceValidation.snapshot(source)
    guard source.document_id==identity.documentID,source.revision.value==0,source.execution_epoch.value==0 else { throw StorageError.corruptIdentity }
    if let current=try head(db) { return current }
    try db.transaction {
      try insert(source)
      try db.statement("INSERT INTO document_head(document_id,revision,execution_epoch,snapshot_hash) VALUES(?,?,?,?)",[.text(source.document_id),.integer(0),.integer(0),.text(source.snapshot_hash)])
    }
    let read=try SQLiteDatabase(url:url,readonly:true);defer { try? read.close() }
    guard let confirmed=try head(read),confirmed.snapshot_hash==source.snapshot_hash else { throw StorageError.unknownCommit(EIO) }
    return confirmed
  }
  private func insert(_ source:NativeSourceSnapshot) throws {
    try db.statement("INSERT INTO document_revisions VALUES(?,?,?,?,?)",[.text(source.document_id),.integer(Int64(source.revision.value)),.integer(Int64(source.execution_epoch.value)),.text(source.snapshot_hash),.blob(try encoded(source))])
  }
  private func head(_ reader:SQLiteDatabase) throws ->NativeSourceSnapshot? {
    var source:NativeSourceSnapshot?
    try reader.statement("SELECT r.snapshot,r.snapshot_hash,h.execution_epoch,h.revision FROM document_head h JOIN document_revisions r ON h.document_id=r.document_id AND h.revision=r.revision AND h.snapshot_hash=r.snapshot_hash WHERE h.document_id=?",[.text(identity.documentID!)]) { [self] row in
      let decoded=try JSONDecoder().decode(NativeSourceSnapshot.self,from:SQLColumn.data(row,0));try SourceValidation.snapshot(decoded)
      guard decoded.document_id==identity.documentID,decoded.snapshot_hash==(try SQLColumn.text(row,1)),Int64(decoded.execution_epoch.value)==sqlite3_column_int64(row,2),Int64(decoded.revision.value)==sqlite3_column_int64(row,3) else { throw StorageError.corruptIdentity }
      source=decoded
    }
    if source==nil,try reader.integer("SELECT count(*) FROM document_head") != 0 { throw StorageError.corruptIdentity }
    return source
  }
  func readHead() throws ->NativeSourceSnapshot? {
    let read=try SQLiteDatabase(url:url,readonly:true);defer { try? read.close() }
    return try head(read)
  }
  private func receipt(_ operation:String,reader:SQLiteDatabase) throws ->NativeDurableSourceReceipt? {
    var receipt:NativeDurableSourceReceipt?
    try reader.statement("SELECT o.request_hash,o.transaction_id,o.committed_revision,o.receipt,t.forward_plan,t.inverse_snapshot,t.inverse_hash,r.snapshot,r.snapshot_hash,e.payload,e.payload_hash,x.receipt_hash FROM operations o JOIN transactions t ON o.transaction_id=t.transaction_id AND o.document_id=t.document_id AND o.operation_id=t.operation_id JOIN document_revisions r ON o.document_id=r.document_id AND o.committed_revision=r.revision JOIN outbox e ON o.document_id=e.document_id AND o.operation_id=e.operation_id JOIN operation_transitions x ON o.document_id=x.document_id AND o.operation_id=x.operation_id AND x.sequence=1 WHERE o.document_id=? AND o.operation_id=?",[.text(identity.documentID!),.text(operation)]) { [self] row in
      let bytes=try SQLColumn.data(row,3)
      let value=try JSONDecoder().decode(NativeDurableSourceReceipt.self,from:bytes)
      let plan=try JSONDecoder().decode(NativeSourceCommit.self,from:SQLColumn.data(row,4))
      let inverse=try JSONDecoder().decode(NativeSourceSnapshot.self,from:SQLColumn.data(row,5))
      let actual=try JSONDecoder().decode(NativeSourceSnapshot.self,from:SQLColumn.data(row,7))
      try SourceValidation.commit(plan);try SourceValidation.snapshot(inverse);try SourceValidation.snapshot(actual)
      guard receipt==nil,value.protocol_version==1,value.receipt.store_id==identity.storeID,value.receipt.document_id.value==identity.documentID,
        value.receipt.operation_id==operation,value.receipt.phase == .completed,value.receipt.error_code.value==nil,
        value.receipt.operation_kind == (plan.commit.actor == .undo ? .undo : .source_edit),
        value.receipt.request_hash==(try SQLColumn.text(row,0)),value.receipt.transaction_id.value==(try SQLColumn.text(row,1)),
        value.receipt.committed_revision.value.map({Int64($0.value)})==sqlite3_column_int64(row,2),
        value.receipt.request_hash==plan.commit.request_hash,value.receipt.transaction_id.value==plan.commit.transaction_id,
        value.receipt.operation_id==plan.commit.operation_id,value.receipt.document_id.value==plan.commit.document_id,
        value.receipt.committed_revision.value?.value==actual.revision.value,
        value.snapshot_hash==actual.snapshot_hash,value.snapshot_hash==plan.after.snapshot_hash,
        value.inverse_plan_hash==inverse.snapshot_hash,value.inverse_plan_hash==plan.before.snapshot_hash,
        value.inverse_plan_hash==(try SQLColumn.text(row,6)),value.snapshot_hash==(try SQLColumn.text(row,8)),
        value.outbox_event_id==plan.commit.outbox_event_id,value.execution_epoch.value==actual.execution_epoch.value,
        bytes==(try SQLColumn.data(row,9)),Self.digest(bytes)==(try SQLColumn.text(row,10)),Self.digest(bytes)==(try SQLColumn.text(row,11))
        else { throw StorageError.corruptIdentity }
      receipt=value
    }
    if receipt==nil {
      var exists=false
      try reader.statement("SELECT 1 FROM operations WHERE document_id=? AND operation_id=?",[.text(identity.documentID!),.text(operation)]){_ in exists=true}
      if exists { throw StorageError.corruptIdentity } // a broken join is not absence
    }
    return receipt
  }
  func query(_ operation:String) throws ->NativeDurableSourceReceipt? {
    guard SourceValidation.identity(operation) else { throw StorageError.corruptIdentity }
    let read=try SQLiteDatabase(url:url,readonly:true);defer { try? read.close() }
    let result=try receipt(operation,reader:read)
    if result != nil { try db.confirmCommittedBytes() }
    return result
  }
  func commit(_ plan:NativeSourceCommit,faults:StorageFaults) throws ->NativeDurableSourceReceipt {
    try SourceValidation.commit(plan)
    guard plan.commit.document_id==identity.documentID else { throw StorageError.corruptIdentity }
    if let previous=try query(plan.commit.operation_id) {
      guard previous.receipt.request_hash==plan.commit.request_hash else { throw StorageError.idempotencyConflict }
      return previous
    }
    var storedPlan=plan
    let clock=ISO8601DateFormatter();clock.formatOptions=[.withInternetDateTime,.withFractionalSeconds]
    storedPlan.commit.committed_at=clock.string(from:Date()) // trusted physical commit time, never model/fixture time
    let record=storedPlan.commit
    let result=NativeDurableSourceReceipt(protocol_version:1,receipt:.init(record_type:.operation_receipt,store_id:identity.storeID,
      document_id:.init(record.document_id),operation_id:record.operation_id,operation_kind:record.actor == .undo ? .undo : .source_edit,
      request_hash:record.request_hash,phase:.completed,transaction_id:.init(record.transaction_id),committed_revision:.init(record.committed_revision),
      accepted_result_ids:[],details_blob_hash:.init(nil),details_retention:.full,error_code:.init(nil),updated_at:record.committed_at),
      snapshot_hash:plan.after.snapshot_hash,inverse_plan_hash:plan.before.snapshot_hash,execution_epoch:plan.after.execution_epoch,outbox_event_id:record.outbox_event_id)
    let receiptBytes=try encoded(result),forward=try encoded(storedPlan),inverse=try encoded(plan.before)
    try db.transaction {
      guard let current=try head(db),current.snapshot_hash==plan.before.snapshot_hash,current.revision.value==record.base_revision.value else { throw StorageError.staleRevision }
      try insert(plan.after);try faults.reach("revision_inserted")
      try db.statement("INSERT INTO transactions VALUES(?,?,?,?,?,?,?,?,?)",[.text(record.transaction_id),.text(record.document_id),.text(record.operation_id),.integer(Int64(record.base_revision.value)),.integer(Int64(record.committed_revision.value)),.text(record.request_hash),.blob(forward),.blob(inverse),.text(record.inverse_plan_hash)])
      try db.statement("INSERT INTO operations VALUES(?,?,?,'completed',?,?,?)",[.text(record.document_id),.text(record.operation_id),.text(record.request_hash),.text(record.transaction_id),.integer(Int64(record.committed_revision.value)),.blob(receiptBytes)])
      try db.statement("INSERT INTO operation_transitions VALUES(?,?,1,'completed',?)",[.text(record.document_id),.text(record.operation_id),.text(Self.digest(receiptBytes))])
      try db.statement("INSERT INTO outbox VALUES(?,?,?,?,?,0)",[.text(record.outbox_event_id),.text(record.document_id),.text(record.operation_id),.blob(receiptBytes),.text(Self.digest(receiptBytes))])
      try db.statement("UPDATE document_head SET revision=?,execution_epoch=?,snapshot_hash=?,active_checkpoint_ref=NULL WHERE document_id=? AND revision=? AND snapshot_hash=?",[.integer(Int64(record.committed_revision.value)),.integer(Int64(record.execution_epoch.value)),.text(record.snapshot_hash),.text(record.document_id),.integer(Int64(record.base_revision.value)),.text(plan.before.snapshot_hash)])
      try faults.reach("before_source_commit")
    }
    do {
      try faults.reach("source_committed")
      // Reconcile original ID and the immutable graph, then reconfirm actual stable bytes.
      guard let confirmed=try query(record.operation_id),confirmed.snapshot_hash==result.snapshot_hash,
        confirmed.receipt.request_hash==record.request_hash else { throw StorageError.corruptIdentity }
      try faults.reach("source_receipt_readback")
      return confirmed
    } catch { throw StorageError.unknownCommit(EIO) }
  }
}
