import CryptoKit
import Foundation
import SQLite3

struct SourceHistoryEntry:Sendable {
  let transaction:String
  let operation:String
  let revision:UInt64
  let retained:Bool
  let pinned:Bool
  let inputGroup:String?
}
struct SourceHistoryPrune:Sendable {let transactions:Int;let payloadBytes:Int;let retained:Int}
/// Version-2 document retention. Immutable facts stay after full source payloads are pruned.
/// Only the queue-confined DocumentStore can perform mutations; this is not an Agent tool.
final class SourceHistoryStore {
  let db:SQLiteDatabase
  let document:String
  init(db:SQLiteDatabase,document:String) throws {
    self.db=db;self.document=document
    try db.transaction {
      try db.statement("CREATE TABLE IF NOT EXISTS source_history(transaction_id TEXT PRIMARY KEY,document_id TEXT NOT NULL,operation_id TEXT NOT NULL,revision INTEGER NOT NULL,summary BLOB NOT NULL,summary_hash TEXT NOT NULL,retained INTEGER NOT NULL CHECK(retained IN (0,1)),UNIQUE(document_id,operation_id),FOREIGN KEY(transaction_id) REFERENCES transactions(transaction_id))")
      try db.statement("CREATE TABLE IF NOT EXISTS source_history_pins(transaction_id TEXT NOT NULL,owner_kind TEXT NOT NULL CHECK(owner_kind IN ('user','task','native_undo','system')),owner_id TEXT NOT NULL,PRIMARY KEY(transaction_id,owner_kind,owner_id),FOREIGN KEY(transaction_id) REFERENCES source_history(transaction_id))")
    }
  }
  static func summary(_ plan:NativeSourceCommit,forward:Data,inverse:Data)->NativeSourceTransactionTombstone {
    .init(record_type:.source_transaction_tombstone,codec_version:1,commit:plan.commit,
      calculation_change:plan.calculation_change,undo_group:plan.undo_group,
      forward_plan_hash:DocumentStore.digest(forward),inverse_payload_hash:DocumentStore.digest(inverse),input_group_id:plan.input_group_id)
  }
  static func validate(_ value:NativeSourceTransactionTombstone) throws {
    let c=value.commit
    guard value.codec_version==1,SourceValidation.identity(c.document_id),SourceValidation.identity(c.operation_id),
      SourceValidation.identity(c.transaction_id),SourceValidation.identity(c.outbox_event_id),c.base_revision.value<HostSerial.maximum,
      c.committed_revision.value==c.base_revision.value+1,c.snapshot_blob_hash.value==nil,
      c.request_hash==SourceHashes.request(commit:c,change:value.calculation_change.value,group:value.undo_group,inputGroup:value.input_group_id),
      [c.snapshot_hash,c.inverse_plan_hash,value.forward_plan_hash,value.inverse_payload_hash].allSatisfy({$0.utf8.count==64 && $0.utf8.allSatisfy{($0>=48 && $0<=57)||($0>=97 && $0<=102)}})
      else {throw StorageError.corruptIdentity}
    if c.actor == .agent {guard let task=c.task_id.value,SourceValidation.identity(task) else {throw StorageError.corruptIdentity}}
    if c.actor == .undo {guard let undo=c.undo_of.value,SourceValidation.identity(undo) else {throw StorageError.corruptIdentity}}
    if let input=value.input_group_id {guard c.actor == .manual,SourceValidation.identity(input) else {throw StorageError.corruptIdentity}}
    if let group=value.undo_group {
      guard c.actor == .undo,SourceValidation.identity(group.group_id),!group.transaction_ids.isEmpty,group.transaction_ids.count<=32,
        Set(group.transaction_ids).count==group.transaction_ids.count,group.transaction_ids.allSatisfy(SourceValidation.identity),
        c.undo_of.value==group.transaction_ids.first else {throw StorageError.corruptIdentity}
    }
  }
  /// Called inside the source COMMIT, beside its exact immutable forward/inverse bytes.
  func insert(_ plan:NativeSourceCommit,forward:Data,inverse:Data) throws {
    let summary=Self.summary(plan,forward:forward,inverse:inverse);try Self.validate(summary)
    let bytes=try JSONEncoder().encode(summary)
    try db.statement("INSERT INTO source_history VALUES(?,?,?,?,?,?,1)",[.text(plan.commit.transaction_id),.text(document),.text(plan.commit.operation_id),.integer(Int64(plan.after.revision.value)),.blob(bytes),.text(DocumentStore.digest(bytes))])
    if plan.calculation_change.value != nil {
      try db.statement("DELETE FROM source_history_pins WHERE owner_kind='system' AND owner_id='calculation-settings'")
      try db.statement("INSERT INTO source_history_pins VALUES(?,'system','calculation-settings')",[.text(plan.commit.transaction_id)])
    }
  }
  static func read(_ db:SQLiteDatabase,document:String,transaction:String) throws ->(NativeSourceTransactionTombstone,Bool)? {
      var output:(NativeSourceTransactionTombstone,Bool)?
      try db.statement("SELECT summary,summary_hash,retained,operation_id,revision FROM source_history WHERE document_id=? AND transaction_id=?",[.text(document),.text(transaction)]) {row in
        let bytes=try SQLColumn.data(row,0),hash=try SQLColumn.text(row,1)
        guard bytes.count<=32768,DocumentStore.digest(bytes)==hash else {throw StorageError.corruptIdentity}
        let value=try JSONDecoder().decode(NativeSourceTransactionTombstone.self,from:bytes);try Self.validate(value)
        guard value.commit.transaction_id==transaction,value.commit.document_id==document,
          value.commit.operation_id==(try SQLColumn.text(row,3)),Int64(value.commit.committed_revision.value)==sqlite3_column_int64(row,4),
          [0,1].contains(sqlite3_column_int64(row,2)) else {throw StorageError.corruptIdentity}
        output=(value,sqlite3_column_int64(row,2)==1)
      }
      return output
  }
  func pin(transaction:String,kind:String,owner:String,enabled:Bool) throws {
    guard ["user","task","native_undo"].contains(kind),SourceValidation.identity(owner),
      let (_,retained)=try Self.read(db,document:document,transaction:transaction),retained else {throw StorageError.transactionUnavailable}
    try db.transaction {
      if enabled {
        var exists=false
        try db.statement("SELECT 1 FROM source_history_pins WHERE transaction_id=? AND owner_kind=? AND owner_id=?",[.text(transaction),.text(kind),.text(owner)]) {_ in exists=true}
        if !exists {let count=try db.integer("SELECT COUNT(*) FROM source_history_pins");guard count<4096 else {throw StorageError.inaccessible}}
        try db.statement("INSERT OR IGNORE INTO source_history_pins VALUES(?,?,?)",[.text(transaction),.text(kind),.text(owner)])
      } else {try db.statement("DELETE FROM source_history_pins WHERE transaction_id=? AND owner_kind=? AND owner_id=?",[.text(transaction),.text(kind),.text(owner)])}
    }
  }
  func entries(limit:Int=200) throws ->[SourceHistoryEntry] {
      guard limit>0,limit<=200 else {throw StorageError.corruptIdentity}
      var output:[SourceHistoryEntry]=[]
      try db.statement("SELECT h.transaction_id,h.operation_id,h.revision,h.retained,EXISTS(SELECT 1 FROM source_history_pins p WHERE p.transaction_id=h.transaction_id),h.summary,h.summary_hash FROM source_history h WHERE h.document_id=? ORDER BY h.revision DESC LIMIT ?",[.text(document),.integer(Int64(limit))]) {row in
        let data=try SQLColumn.data(row,5);guard DocumentStore.digest(data)==(try SQLColumn.text(row,6)) else {throw StorageError.corruptIdentity}
        let summary=try JSONDecoder().decode(NativeSourceTransactionTombstone.self,from:data);try Self.validate(summary)
        output.append(.init(transaction:try SQLColumn.text(row,0),operation:try SQLColumn.text(row,1),revision:UInt64(sqlite3_column_int64(row,2)),retained:sqlite3_column_int64(row,3)==1,pinned:sqlite3_column_int64(row,4)==1,inputGroup:summary.input_group_id))
      }
      return output
  }
  /// Bounded physical compaction, original receipt bytes and all stable identities survive.
  /// Caller verifies each original healthy receipt before any payload is erased.
  func compact(faults:StorageFaults,verify:(String)throws->Void) throws ->SourceHistoryPrune {
    var candidates:[(String,String)]=[]
    try db.statement("SELECT h.transaction_id,h.operation_id FROM source_history h WHERE h.document_id=? AND h.retained=1 AND h.transaction_id NOT IN (SELECT transaction_id FROM source_history WHERE document_id=? ORDER BY revision DESC LIMIT 200) AND NOT EXISTS(SELECT 1 FROM source_history_pins p WHERE p.transaction_id=h.transaction_id) ORDER BY h.revision LIMIT 256",[.text(document),.text(document)]) {row in candidates.append((try SQLColumn.text(row,0),try SQLColumn.text(row,1)))}
    var payloadBytes=0
    for (transaction,operation) in candidates {
      try verify(operation)
      guard let (summary,retained)=try Self.read(db,document:document,transaction:transaction),retained else {throw StorageError.corruptIdentity}
      try db.statement("SELECT forward_plan,inverse_snapshot FROM transactions WHERE transaction_id=? AND document_id=?",[.text(transaction),.text(document)]) {row in
        let forward=try SQLColumn.data(row,0),inverse=try SQLColumn.data(row,1)
        guard DocumentStore.digest(forward)==summary.forward_plan_hash,DocumentStore.digest(inverse)==summary.inverse_payload_hash else {throw StorageError.corruptIdentity}
        payloadBytes+=forward.count+inverse.count
      }
    }
    if !candidates.isEmpty {
      try db.transaction {
        for (transaction,operation) in candidates {
          try db.statement("UPDATE transactions SET forward_plan=?,inverse_snapshot=? WHERE document_id=? AND transaction_id=?",[.blob(Data()),.blob(Data()),.text(document),.text(transaction)])
          // The completed admission can use immutable compact facts, not another full source copy.
          var summary:Data?
          try db.statement("SELECT summary FROM source_history WHERE transaction_id=? AND document_id=?",[.text(transaction),.text(document)]) {summary=try SQLColumn.data($0,0)}
          guard let summary else {throw StorageError.corruptIdentity}
          try db.statement("UPDATE source_admissions SET plan=? WHERE document_id=? AND operation_id=? AND phase='completed'",[.blob(summary),.text(document),.text(operation)])
          try db.statement("UPDATE source_history SET retained=0 WHERE transaction_id=? AND document_id=?",[.text(transaction),.text(document)])
          try faults.reach("history_payloads_pruned")
        }
        // Historical revision identities/hashes remain for FKs and tombstones; full snapshots
        // stay for active head and every retained transaction's before/after reference.
        var kernelTables=false
        try db.statement("SELECT 1 FROM sqlite_schema WHERE type='table' AND name='accepted_checkpoints'") {_ in kernelTables=true}
        let preserveKernel=kernelTables ? " AND revision NOT IN (SELECT source_revision FROM accepted_checkpoints WHERE document_id=?) AND revision NOT IN (SELECT accepted_source_revision FROM accepted_checkpoints WHERE document_id=?)" : ""
        var values:[SQLValue]=[.blob(Data()),.text(document),.text(document),.text(document),.text(document)]
        if kernelTables {values += [.text(document),.text(document)]}
        try db.statement("UPDATE document_revisions SET snapshot=? WHERE document_id=? AND revision NOT IN (SELECT revision FROM document_head WHERE document_id=?) AND revision NOT IN (SELECT t.base_revision FROM transactions t JOIN source_history h ON t.transaction_id=h.transaction_id WHERE h.retained=1 AND h.document_id=?) AND revision NOT IN (SELECT t.committed_revision FROM transactions t JOIN source_history h ON t.transaction_id=h.transaction_id WHERE h.retained=1 AND h.document_id=?)"+preserveKernel,values)
        try faults.reach("history_revisions_pruned")
      }
      do {try faults.reach("history_prune_committed")} catch {throw StorageError.unknownCommit(EIO)}
      try db.confirmCommittedBytes()
    }
    let retained=Int(try db.integer("SELECT COUNT(*) FROM source_history WHERE retained=1"))
    return .init(transactions:candidates.count,payloadBytes:payloadBytes,retained:retained)
  }
}
