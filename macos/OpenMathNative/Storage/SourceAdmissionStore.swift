import Foundation
import SQLite3

/// Original-ID durable admission in the same physical document DB. A crash here never means
/// the source operation ran; an old runtime owner must be reconciled and explicitly resumed.
final class SourceAdmissionStore {
  private let db:SQLiteDatabase
  private let identity:StoreIdentity
  init(db:SQLiteDatabase,identity:StoreIdentity) throws {
    self.db=db;self.identity=identity
    try db.transaction {
      try db.statement("CREATE TABLE IF NOT EXISTS source_admissions(document_id TEXT NOT NULL,operation_id TEXT NOT NULL,request_hash TEXT NOT NULL,created_by_runtime TEXT NOT NULL,phase TEXT NOT NULL CHECK(phase IN ('accepted','completed','cancelled','failed','unknown')),plan BLOB NOT NULL,PRIMARY KEY(document_id,operation_id))")
      try db.statement("CREATE TABLE IF NOT EXISTS source_admission_transitions(document_id TEXT NOT NULL,operation_id TEXT NOT NULL,sequence INTEGER NOT NULL,phase TEXT NOT NULL,PRIMARY KEY(document_id,operation_id,sequence),FOREIGN KEY(document_id,operation_id) REFERENCES source_admissions(document_id,operation_id))")
    }
  }
  func query(_ operation:String) throws ->NativeSourceAdmission? {
    guard SourceValidation.identity(operation) else {throw StorageError.corruptIdentity}
    var result:NativeSourceAdmission?
    try db.statement("SELECT request_hash,created_by_runtime,phase,plan FROM source_admissions WHERE document_id=? AND operation_id=?",[.text(identity.documentID!),.text(operation)]) { [self] row in
      let plan=try JSONDecoder().decode(NativeSourceCommit.self,from:SQLColumn.data(row,3));try SourceValidation.commit(plan)
      let hash=try SQLColumn.text(row,0),runtime=try SQLColumn.text(row,1)
      guard plan.commit.document_id==identity.documentID,plan.commit.operation_id==operation,plan.commit.request_hash==hash,
        SourceValidation.identity(runtime),let phase=NativeSourceAdmissionPhase(rawValue:try SQLColumn.text(row,2)) else {throw StorageError.corruptIdentity}
      result=NativeSourceAdmission(protocol_version:1,document_id:identity.documentID!,operation_id:operation,request_hash:hash,phase:phase,created_by_runtime:runtime,receipt:.init(nil))
    }
    if let result {
      var count:Int64=0,maximum:Int64=0,lastPhase:String?
      try db.statement("SELECT COUNT(*),MAX(sequence) FROM source_admission_transitions WHERE document_id=? AND operation_id=?",[.text(identity.documentID!),.text(operation)]) {
        count=sqlite3_column_int64($0,0);maximum=sqlite3_column_int64($0,1)
      }
      try db.statement("SELECT phase FROM source_admission_transitions WHERE document_id=? AND operation_id=? AND sequence=?",[.text(identity.documentID!),.text(operation),.integer(maximum)]) {lastPhase=try SQLColumn.text($0,0)}
      guard count>0,count==maximum,lastPhase==result.phase.rawValue else {throw StorageError.corruptIdentity}
    }
    return result
  }
  func admit(_ plan:NativeSourceCommit,runtime:String,completed:NativeDurableSourceReceipt?,faults:StorageFaults = .init()) throws ->NativeSourceAdmission {
    try SourceValidation.commit(plan)
    guard plan.commit.document_id==identity.documentID,SourceValidation.identity(runtime) else {throw StorageError.corruptIdentity}
    if let completed {
      guard completed.receipt.request_hash==plan.commit.request_hash else {throw StorageError.idempotencyConflict}
      return NativeSourceAdmission(protocol_version:1,document_id:plan.commit.document_id,operation_id:plan.commit.operation_id,request_hash:plan.commit.request_hash,phase:.completed,created_by_runtime:runtime,receipt:.init(completed))
    }
    if let old=try query(plan.commit.operation_id) {
      guard old.request_hash==plan.commit.request_hash else {throw StorageError.idempotencyConflict}
      try db.confirmCommittedBytes();return old
    }
    let bytes=try JSONEncoder().encode(plan)
    try db.transaction {
      try db.statement("INSERT INTO source_admissions VALUES(?,?,?,?,'accepted',?)",[.text(plan.commit.document_id),.text(plan.commit.operation_id),.text(plan.commit.request_hash),.text(runtime),.blob(bytes)])
      try db.statement("INSERT INTO source_admission_transitions VALUES(?,?,1,'accepted')",[.text(plan.commit.document_id),.text(plan.commit.operation_id)])
      try faults.reach("source_admission_before_commit")
    }
    do {try faults.reach("source_admission_committed")}
    catch {throw StorageError.unknownCommit(EIO)}
    guard let actual=try query(plan.commit.operation_id),actual.request_hash==plan.commit.request_hash else {throw StorageError.unknownCommit(EIO)}
    return actual
  }
  /// Transaction caller records completion beside source/head/outbox, never afterward.
  func transition(_ operation:String,_ phase:NativeSourceAdmissionPhase) throws {
    guard let previous=try query(operation) else {throw StorageError.corruptIdentity}
    if previous.phase == .completed {
      guard phase == .completed else {throw StorageError.idempotencyConflict};return
    }
    if previous.phase == phase {return}
    guard previous.phase == .accepted || previous.phase == .unknown else {throw StorageError.idempotencyConflict}
    try db.statement("UPDATE source_admissions SET phase=? WHERE document_id=? AND operation_id=?",[.text(phase.rawValue),.text(identity.documentID!),.text(operation)])
    var sequence:Int64=0
    try db.statement("SELECT COALESCE(MAX(sequence),0)+1 FROM source_admission_transitions WHERE document_id=? AND operation_id=?",[.text(identity.documentID!),.text(operation)]){sequence=sqlite3_column_int64($0,0)}
    guard sequence>0,sequence<Int64(HostSerial.maximum) else {throw StorageError.corruptIdentity}
    try db.statement("INSERT INTO source_admission_transitions VALUES(?,?,?,?)",[.text(identity.documentID!),.text(operation),.integer(sequence),.text(phase.rawValue)])
  }
  func settle(_ operation:String,cancelled:Bool) throws ->NativeSourceAdmission {
    try db.transaction {try transition(operation,cancelled ? .cancelled : .failed)}
    guard let result=try query(operation) else {throw StorageError.corruptIdentity};return result
  }
}
