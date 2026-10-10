import Foundation

/// v3 document stores make mathematical acceptance and Blob references one physical transaction.
enum KernelSchema {
  static let tables:Set<String>=["accepted_checkpoints","kernel_head","accepted_results","kernel_operations","kernel_transitions","kernel_outbox"]
  static func ensure(_ db:SQLiteDatabase) throws {
    var present=Set<String>()
    try db.statement("SELECT name FROM sqlite_schema WHERE type='table'") {present.insert(try SQLColumn.text($0,0))}
    let found=present.intersection(tables)
    if found==tables {return}
    guard found.isEmpty else {throw StorageError.recoveryRequired}
    try db.transaction {
      try db.statement("CREATE TABLE accepted_checkpoints(checkpoint_id TEXT PRIMARY KEY,document_id TEXT NOT NULL,operation_id TEXT NOT NULL,request_hash TEXT NOT NULL,blob_hash TEXT NOT NULL,byte_length INTEGER NOT NULL,source_revision INTEGER NOT NULL,source_snapshot_hash TEXT NOT NULL,accepted_source_revision INTEGER NOT NULL,accepted_snapshot_hash TEXT NOT NULL,execution_epoch INTEGER NOT NULL,kernel_state_revision INTEGER NOT NULL,plan BLOB NOT NULL,UNIQUE(document_id,operation_id),UNIQUE(document_id,checkpoint_id),FOREIGN KEY(document_id,source_revision,source_snapshot_hash) REFERENCES document_revisions(document_id,revision,snapshot_hash),FOREIGN KEY(document_id,accepted_source_revision,accepted_snapshot_hash) REFERENCES document_revisions(document_id,revision,snapshot_hash),FOREIGN KEY(blob_hash) REFERENCES blob_objects(blob_hash))")
      try db.statement("CREATE TABLE kernel_head(document_id TEXT PRIMARY KEY,checkpoint_id TEXT NOT NULL,kernel_state_revision INTEGER NOT NULL,FOREIGN KEY(document_id,checkpoint_id) REFERENCES accepted_checkpoints(document_id,checkpoint_id))")
      try db.statement("CREATE TABLE accepted_results(result_id TEXT PRIMARY KEY,document_id TEXT NOT NULL,checkpoint_id TEXT NOT NULL,cell_id TEXT NOT NULL,cell_source_hash TEXT NOT NULL,terminal_status TEXT NOT NULL,successful_statements INTEGER NOT NULL,out_index INTEGER,FOREIGN KEY(document_id,checkpoint_id) REFERENCES accepted_checkpoints(document_id,checkpoint_id))")
      try db.statement("CREATE TABLE kernel_operations(document_id TEXT NOT NULL,operation_id TEXT NOT NULL,request_hash TEXT NOT NULL,checkpoint_id TEXT NOT NULL,receipt BLOB NOT NULL,receipt_hash TEXT NOT NULL,PRIMARY KEY(document_id,operation_id),FOREIGN KEY(document_id,checkpoint_id) REFERENCES accepted_checkpoints(document_id,checkpoint_id))")
      try db.statement("CREATE TABLE kernel_transitions(document_id TEXT NOT NULL,operation_id TEXT NOT NULL,sequence INTEGER NOT NULL,phase TEXT NOT NULL CHECK(phase='accepted'),receipt_hash TEXT NOT NULL,PRIMARY KEY(document_id,operation_id,sequence),FOREIGN KEY(document_id,operation_id) REFERENCES kernel_operations(document_id,operation_id))")
      try db.statement("CREATE TABLE kernel_outbox(event_id TEXT PRIMARY KEY,document_id TEXT NOT NULL,operation_id TEXT NOT NULL,payload BLOB NOT NULL,payload_hash TEXT NOT NULL,projected INTEGER NOT NULL DEFAULT 0 CHECK(projected IN (0,1)),UNIQUE(document_id,operation_id),FOREIGN KEY(document_id,operation_id) REFERENCES kernel_operations(document_id,operation_id))")
    }
  }
}
