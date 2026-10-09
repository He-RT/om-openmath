import Foundation

/// Physical source tables. Each source revision, reverse plan, receipt and outbox share one DB.
enum DocumentSchema {
  static let tables:[String]=["document_head","document_revisions","transactions","operations","operation_transitions","outbox"]
  static func ensure(_ db:SQLiteDatabase) throws {
    var present=Set<String>()
    try db.statement("SELECT name FROM sqlite_schema WHERE type='table'") { row in present.insert(try SQLColumn.text(row,0)) }
    let existing=present.intersection(tables)
    if existing==Set(tables) { return }
    guard existing.isEmpty,present==["store_header","store_state"] else { throw StorageError.recoveryRequired }
    try db.transaction {
      try db.statement("CREATE TABLE document_revisions(document_id TEXT NOT NULL,revision INTEGER NOT NULL CHECK(revision>=0 AND revision<=9007199254740991),execution_epoch INTEGER NOT NULL,snapshot_hash TEXT NOT NULL,snapshot BLOB NOT NULL,PRIMARY KEY(document_id,revision),UNIQUE(document_id,revision,snapshot_hash))")
      try db.statement("CREATE TABLE document_head(document_id TEXT PRIMARY KEY,revision INTEGER NOT NULL,execution_epoch INTEGER NOT NULL,snapshot_hash TEXT NOT NULL,active_checkpoint_ref TEXT,FOREIGN KEY(document_id,revision,snapshot_hash) REFERENCES document_revisions(document_id,revision,snapshot_hash))")
      try db.statement("CREATE TABLE transactions(transaction_id TEXT PRIMARY KEY,document_id TEXT NOT NULL,operation_id TEXT NOT NULL,base_revision INTEGER NOT NULL,committed_revision INTEGER NOT NULL,request_hash TEXT NOT NULL,forward_plan BLOB NOT NULL,inverse_snapshot BLOB NOT NULL,inverse_hash TEXT NOT NULL,UNIQUE(document_id,operation_id),FOREIGN KEY(document_id,base_revision) REFERENCES document_revisions(document_id,revision),FOREIGN KEY(document_id,committed_revision) REFERENCES document_revisions(document_id,revision))")
      try db.statement("CREATE TABLE operations(document_id TEXT NOT NULL,operation_id TEXT NOT NULL,request_hash TEXT NOT NULL,phase TEXT NOT NULL CHECK(phase IN ('completed')),transaction_id TEXT NOT NULL,committed_revision INTEGER NOT NULL,receipt BLOB NOT NULL,PRIMARY KEY(document_id,operation_id),FOREIGN KEY(transaction_id) REFERENCES transactions(transaction_id),FOREIGN KEY(document_id,committed_revision) REFERENCES document_revisions(document_id,revision))")
      try db.statement("CREATE TABLE operation_transitions(document_id TEXT NOT NULL,operation_id TEXT NOT NULL,sequence INTEGER NOT NULL,phase TEXT NOT NULL,receipt_hash TEXT NOT NULL,PRIMARY KEY(document_id,operation_id,sequence),FOREIGN KEY(document_id,operation_id) REFERENCES operations(document_id,operation_id))")
      try db.statement("CREATE TABLE outbox(event_id TEXT PRIMARY KEY,document_id TEXT NOT NULL,operation_id TEXT NOT NULL,payload BLOB NOT NULL,payload_hash TEXT NOT NULL,projected INTEGER NOT NULL DEFAULT 0 CHECK(projected IN (0,1)),UNIQUE(document_id,operation_id),FOREIGN KEY(document_id,operation_id) REFERENCES operations(document_id,operation_id))")
    }
  }
}
