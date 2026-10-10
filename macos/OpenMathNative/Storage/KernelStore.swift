import Foundation
import SQLite3

/// Physical acceptance is one SQLite transaction; producer semantics and final cancel remain Rust.
struct KernelCommitControls:Sendable {
  let ownerBarrier:@Sendable () throws->Void
}
final class KernelStore {
  private let db:SQLiteDatabase
  private let url:URL
  private let identity:StoreIdentity
  init(db:SQLiteDatabase,url:URL,identity:StoreIdentity) throws {
    guard identity.kind=="document",identity.storeVersion==3,identity.documentID != nil else {throw StorageError.unsupportedVersion}
    self.db=db;self.url=url;self.identity=identity
    try KernelSchema.ensure(db)
  }
  private func encoded<T:Encodable>(_ value:T)throws->Data {
    let bytes=try JSONEncoder().encode(value);guard bytes.count<=4*1024*1024 else {throw StorageError.corruptIdentity};return bytes
  }
  private func receipt(_ operation:String,reader:SQLiteDatabase)throws->NativeKernelReceipt? {
    var result:NativeKernelReceipt?
    try reader.statement("SELECT o.receipt,o.receipt_hash,o.request_hash,o.checkpoint_id,c.plan,c.blob_hash,c.byte_length,e.payload,e.payload_hash,e.event_id,t.receipt_hash,c.source_revision,c.source_snapshot_hash,c.accepted_source_revision,c.accepted_snapshot_hash,c.execution_epoch,c.kernel_state_revision,c.operation_id,r.snapshot,a.snapshot FROM kernel_operations o JOIN accepted_checkpoints c ON o.document_id=c.document_id AND o.checkpoint_id=c.checkpoint_id JOIN kernel_outbox e ON o.document_id=e.document_id AND o.operation_id=e.operation_id JOIN kernel_transitions t ON o.document_id=t.document_id AND o.operation_id=t.operation_id AND t.sequence=1 JOIN document_revisions r ON c.document_id=r.document_id AND c.source_revision=r.revision AND c.source_snapshot_hash=r.snapshot_hash JOIN document_revisions a ON c.document_id=a.document_id AND c.accepted_source_revision=a.revision AND c.accepted_snapshot_hash=a.snapshot_hash WHERE o.document_id=? AND o.operation_id=?",[.text(identity.documentID!),.text(operation)]) { [self] row in
      guard result==nil else {throw StorageError.corruptIdentity}
      let bytes=try SQLColumn.data(row,0),hash=KernelHashes.data(bytes)
      let actual=try JSONDecoder().decode(NativeKernelReceipt.self,from:bytes)
      let plan=try JSONDecoder().decode(NativeKernelCommit.self,from:SQLColumn.data(row,4))
      try KernelValidation.receipt(actual,plan:plan)
      let producer=try JSONDecoder().decode(NativeSourceSnapshot.self,from:SQLColumn.data(row,18))
      let accepted=try JSONDecoder().decode(NativeSourceSnapshot.self,from:SQLColumn.data(row,19))
      try SourceValidation.snapshot(producer);try SourceValidation.snapshot(accepted)
      guard plan.store_id==identity.storeID,plan.producer.document_id==identity.documentID,plan.operation_id==operation,
        hash==(try SQLColumn.text(row,1)),actual.request_hash==(try SQLColumn.text(row,2)),actual.checkpoint_id==(try SQLColumn.text(row,3)),
        plan.checkpoint_blob_hash==(try SQLColumn.text(row,5)),Int64(plan.checkpoint_byte_length.value)==sqlite3_column_int64(row,6),
        bytes==(try SQLColumn.data(row,7)),hash==(try SQLColumn.text(row,8)),actual.outbox_event_id==(try SQLColumn.text(row,9)),hash==(try SQLColumn.text(row,10)),
        Int64(plan.source.revision.value)==sqlite3_column_int64(row,11),plan.source.snapshot_hash==(try SQLColumn.text(row,12)),
        Int64(plan.acceptance_source.revision.value)==sqlite3_column_int64(row,13),plan.acceptance_source.snapshot_hash==(try SQLColumn.text(row,14)),
        Int64(plan.producer.execution_epoch.value)==sqlite3_column_int64(row,15),Int64(plan.producer.kernel_state_revision.value)==sqlite3_column_int64(row,16),
        operation==(try SQLColumn.text(row,17)),producer.snapshot_hash==plan.source.snapshot_hash,accepted.snapshot_hash==plan.acceptance_source.snapshot_hash else {throw StorageError.corruptIdentity}
      let refOwner=BlobOwner(kind:.checkpoint,id:actual.checkpoint_id)
      var referenceFound=false
      try reader.statement("SELECT o.byte_length,r.media_type,r.codec_version FROM blob_refs r JOIN blob_objects o ON r.blob_hash=o.blob_hash WHERE r.owner_kind=? AND r.owner_id=? AND r.blob_hash=?",[.text(refOwner.kind.rawValue),.text(refOwner.id),.text(actual.checkpoint_blob_hash)]) { blob in
        guard Int64(actual.checkpoint_byte_length.value)==sqlite3_column_int64(blob,0),try SQLColumn.text(blob,1)=="application/x-openmath-kernel-checkpoint",try SQLColumn.text(blob,2)=="omks-v1" else {throw StorageError.corruptIdentity};referenceFound=true
      }
      guard referenceFound else {throw StorageError.corruptIdentity}
      var rows=0
      try reader.statement("SELECT result_id,cell_id,cell_source_hash,terminal_status,successful_statements,out_index FROM accepted_results WHERE document_id=? AND checkpoint_id=?",[.text(identity.documentID!),.text(actual.checkpoint_id)]) { r in
        rows+=1
        guard plan.result_id.value==(try SQLColumn.text(r,0)),plan.producer.cell_id.value==(try SQLColumn.text(r,1)),
          plan.producer.cell_source_hash.value==(try SQLColumn.text(r,2)),plan.producer.terminal_status.rawValue==(try SQLColumn.text(r,3)),
          Int64(plan.producer.successful_statements.value)==sqlite3_column_int64(r,4),
          plan.producer.out_index.value.map({Int64($0.value)})==(sqlite3_column_type(r,5)==SQLITE_NULL ? nil : sqlite3_column_int64(r,5)) else {throw StorageError.corruptIdentity}
      }
      guard rows==(plan.result_id.value==nil ? 0 : 1) else {throw StorageError.corruptIdentity}
      var checkpointIdentity:String?,transitionCount:Int64=0
      try reader.statement("SELECT request_hash FROM accepted_checkpoints WHERE checkpoint_id=? AND document_id=?",[.text(actual.checkpoint_id),.text(identity.documentID!)]) {checkpointIdentity=try SQLColumn.text($0,0)}
      try reader.statement("SELECT COUNT(*) FROM kernel_transitions WHERE document_id=? AND operation_id=? AND sequence=1 AND phase='accepted'",[.text(identity.documentID!),.text(operation)]) {transitionCount=sqlite3_column_int64($0,0)}
      guard checkpointIdentity==actual.request_hash,transitionCount==1 else {throw StorageError.corruptIdentity}
      result=actual
    }
    if result==nil {
      var exists=false
      try reader.statement("SELECT 1 FROM kernel_operations WHERE document_id=? AND operation_id=?",[.text(identity.documentID!),.text(operation)]) {_ in exists=true}
      if exists {throw StorageError.corruptIdentity}
    }
    return result
  }
  func query(_ operation:String)throws->NativeKernelReceipt? {
    guard SourceValidation.identity(operation) else {throw StorageError.corruptIdentity}
    let read=try SQLiteDatabase(url:url,readonly:true);defer {try? read.close()}
    let result=try receipt(operation,reader:read);if result != nil {try db.confirmCommittedBytes()};return result
  }
  private func head(_ reader:SQLiteDatabase)throws->NativeKernelReceipt? {
    var pointer:(String,Int64,String)?
    try reader.statement("SELECT h.checkpoint_id,h.kernel_state_revision,c.operation_id FROM kernel_head h JOIN accepted_checkpoints c ON h.document_id=c.document_id AND h.checkpoint_id=c.checkpoint_id WHERE h.document_id=?",[.text(identity.documentID!)]) {pointer=(try SQLColumn.text($0,0),sqlite3_column_int64($0,1),try SQLColumn.text($0,2))}
    guard let pointer else {
      var exists=false;try reader.statement("SELECT 1 FROM kernel_head WHERE document_id=?",[.text(identity.documentID!)]) {_ in exists=true}
      if exists {throw StorageError.corruptIdentity};return nil
    }
    guard let result=try receipt(pointer.2,reader:reader),result.checkpoint_id==pointer.0,Int64(result.kernel_state_revision.value)==pointer.1 else {throw StorageError.corruptIdentity}
    return result
  }
  func readHead()throws->NativeKernelReceipt? {
    let read=try SQLiteDatabase(url:url,readonly:true);defer {try? read.close()}
    let result=try head(read);if result != nil {try db.confirmCommittedBytes()};return result
  }
  /// Caller retains a verified BlobStore transfer pin until the same-DB references settle.
  func commit(_ plan:NativeKernelCommit,descriptor:BlobDescriptor,faults:StorageFaults,controls:KernelCommitControls)throws->NativeKernelReceipt {
    try KernelValidation.plan(plan)
    guard plan.store_id==identity.storeID,plan.producer.document_id==identity.documentID,
      descriptor.hash==plan.checkpoint_blob_hash,descriptor.byteLength==plan.checkpoint_byte_length.value else {throw StorageError.corruptIdentity}
    if let prior=try query(plan.operation_id) {
      guard prior.request_hash==plan.request_hash else {throw StorageError.idempotencyConflict};return prior
    }
    let date=ISO8601DateFormatter();date.formatOptions=[.withInternetDateTime,.withFractionalSeconds]
    let receipt=NativeKernelReceipt(protocol_version:1,store_id:identity.storeID,document_id:plan.producer.document_id,operation_id:plan.operation_id,request_hash:plan.request_hash,
      checkpoint_id:plan.checkpoint_id,checkpoint_blob_hash:plan.checkpoint_blob_hash,checkpoint_byte_length:plan.checkpoint_byte_length,codec_version:1,producer:plan.producer,
      accepted_source_revision:plan.acceptance_source.revision,accepted_snapshot_hash:plan.acceptance_source.snapshot_hash,kernel_state_revision:plan.producer.kernel_state_revision,
      result_id:plan.result_id,outbox_event_id:plan.outbox_event_id,committed_at:date.string(from:Date()))
    let bytes=try encoded(receipt),planBytes=try encoded(plan),digest=KernelHashes.data(bytes)
    try db.transaction {
      var current:(Int64,Int64,String)?
      try db.statement("SELECT revision,execution_epoch,snapshot_hash FROM document_head WHERE document_id=?",[.text(identity.documentID!)]) {current=(sqlite3_column_int64($0,0),sqlite3_column_int64($0,1),try SQLColumn.text($0,2))}
      guard let current,current.0==Int64(plan.acceptance_source.revision.value),current.1==Int64(plan.producer.execution_epoch.value),current.2==plan.acceptance_source.snapshot_hash else {throw StorageError.staleRevision}
      let parent=try head(db)
      guard parent?.checkpoint_id==plan.expected_parent_checkpoint_id.value,(parent?.kernel_state_revision.value ?? 0)==plan.expected_kernel_state_revision.value else {throw StorageError.staleRevision}
      let reference=BlobReference(owner:.init(kind:.checkpoint,id:plan.checkpoint_id),descriptor:descriptor,mediaType:"application/x-openmath-kernel-checkpoint",codecVersion:"omks-v1")
      try BlobReferences.insertWithinTransaction(reference,db:db);try faults.reach("kernel_blob_referenced")
      try db.statement("INSERT INTO accepted_checkpoints VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",[.text(plan.checkpoint_id),.text(identity.documentID!),.text(plan.operation_id),.text(plan.request_hash),.text(plan.checkpoint_blob_hash),.integer(Int64(plan.checkpoint_byte_length.value)),.integer(Int64(plan.source.revision.value)),.text(plan.source.snapshot_hash),.integer(Int64(plan.acceptance_source.revision.value)),.text(plan.acceptance_source.snapshot_hash),.integer(Int64(plan.producer.execution_epoch.value)),.integer(Int64(plan.producer.kernel_state_revision.value)),.blob(planBytes)])
      if let result=plan.result_id.value {
        let p=plan.producer
        try db.statement("INSERT INTO accepted_results VALUES(?,?,?,?,?,?,?,?)",[.text(result),.text(identity.documentID!),.text(plan.checkpoint_id),.text(p.cell_id.value!),.text(p.cell_source_hash.value!),.text(p.terminal_status.rawValue),.integer(Int64(p.successful_statements.value)),p.out_index.value.map{.integer(Int64($0.value))} ?? .null])
      }
      try faults.reach("kernel_checkpoint_inserted")
      try db.statement("INSERT INTO kernel_operations VALUES(?,?,?,?,?,?)",[.text(identity.documentID!),.text(plan.operation_id),.text(plan.request_hash),.text(plan.checkpoint_id),.blob(bytes),.text(digest)])
      try db.statement("INSERT INTO kernel_transitions VALUES(?,?,1,'accepted',?)",[.text(identity.documentID!),.text(plan.operation_id),.text(digest)])
      try db.statement("INSERT INTO kernel_outbox VALUES(?,?,?,?,?,0)",[.text(plan.outbox_event_id),.text(identity.documentID!),.text(plan.operation_id),.blob(bytes),.text(digest)])
      try db.statement("INSERT INTO kernel_head VALUES(?,?,?) ON CONFLICT(document_id) DO UPDATE SET checkpoint_id=excluded.checkpoint_id,kernel_state_revision=excluded.kernel_state_revision",[.text(identity.documentID!),.text(plan.checkpoint_id),.integer(Int64(plan.producer.kernel_state_revision.value))])
      try db.statement("UPDATE document_head SET active_checkpoint_ref=? WHERE document_id=?",[.text(plan.checkpoint_id),.text(identity.documentID!)])
      try faults.reach("before_kernel_commit")
      try controls.ownerBarrier()
    }
    do {
      try faults.reach("kernel_committed")
      guard let result=try query(plan.operation_id) else {throw StorageError.corruptIdentity}
      try KernelValidation.receipt(result,plan:plan);try faults.reach("kernel_receipt_readback");return result
    } catch {throw StorageError.unknownCommit(EIO)}
  }
}
