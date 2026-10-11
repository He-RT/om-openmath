import Foundation
import SQLite3

/// Save intent/binding/completion are original same-document DB facts. User file bytes are a
/// separately coordinated resource and become "saved" only after actual readback plus this COMMIT.
final class SaveStore {
  private let db:SQLiteDatabase
  private let url:URL
  private let identity:StoreIdentity
  private static let tables:Set<String>=["file_bindings","file_head","save_intents","save_receipts","save_outbox"]
  init(db:SQLiteDatabase,url:URL,identity:StoreIdentity)throws {
    guard identity.kind=="document",identity.storeVersion==4,identity.documentID != nil else {throw StorageError.unsupportedVersion}
    self.db=db;self.url=url;self.identity=identity
    var present=Set<String>();try db.statement("SELECT name FROM sqlite_schema WHERE type='table'") {present.insert(try SQLColumn.text($0,0))}
    let found=present.intersection(Self.tables);if found==Self.tables {return};guard found.isEmpty else {throw StorageError.recoveryRequired}
    try db.transaction {
      try db.statement("CREATE TABLE file_bindings(document_id TEXT NOT NULL,binding_revision INTEGER NOT NULL,record BLOB NOT NULL,record_hash TEXT NOT NULL,PRIMARY KEY(document_id,binding_revision))")
      try db.statement("CREATE TABLE file_head(document_id TEXT PRIMARY KEY,binding_revision INTEGER NOT NULL,saved_revision INTEGER,saved_snapshot_hash TEXT,last_save_operation TEXT,FOREIGN KEY(document_id,binding_revision) REFERENCES file_bindings(document_id,binding_revision),FOREIGN KEY(document_id,saved_revision,saved_snapshot_hash) REFERENCES document_revisions(document_id,revision,snapshot_hash))")
      try db.statement("CREATE TABLE save_intents(document_id TEXT NOT NULL,operation_id TEXT NOT NULL,source_revision INTEGER NOT NULL,source_snapshot_hash TEXT NOT NULL,binding_revision INTEGER NOT NULL,file_hash TEXT NOT NULL,file_length INTEGER NOT NULL,record BLOB NOT NULL,record_hash TEXT NOT NULL,phase TEXT NOT NULL CHECK(phase IN ('prepared','completed','not_written','conflict')),PRIMARY KEY(document_id,operation_id),FOREIGN KEY(document_id,source_revision,source_snapshot_hash) REFERENCES document_revisions(document_id,revision,snapshot_hash),FOREIGN KEY(document_id,binding_revision) REFERENCES file_bindings(document_id,binding_revision),FOREIGN KEY(file_hash) REFERENCES blob_objects(blob_hash))")
      try db.statement("CREATE TABLE save_receipts(document_id TEXT NOT NULL,operation_id TEXT NOT NULL,record BLOB NOT NULL,record_hash TEXT NOT NULL,PRIMARY KEY(document_id,operation_id),FOREIGN KEY(document_id,operation_id) REFERENCES save_intents(document_id,operation_id))")
      try db.statement("CREATE TABLE save_outbox(event_id TEXT PRIMARY KEY,document_id TEXT NOT NULL,operation_id TEXT NOT NULL,payload BLOB NOT NULL,payload_hash TEXT NOT NULL,projected INTEGER NOT NULL DEFAULT 0 CHECK(projected IN (0,1)),FOREIGN KEY(document_id,operation_id) REFERENCES save_receipts(document_id,operation_id))")
    }
  }
  private func encoded<T:Encodable>(_ value:T)throws->Data {let e=JSONEncoder();e.outputFormatting=[.sortedKeys,.withoutEscapingSlashes];let bytes=try e.encode(value);guard bytes.count<=4*1024*1024 else {throw StorageError.corruptIdentity};return bytes}
  private func binding(_ revision:UInt64,reader:SQLiteDatabase)throws->FileBinding? {
    var value:FileBinding?
    try reader.statement("SELECT record,record_hash FROM file_bindings WHERE document_id=? AND binding_revision=?",[.text(identity.documentID!),.integer(Int64(revision))]) {row in
      let bytes=try SQLColumn.data(row,0);guard KernelHashes.data(bytes)==(try SQLColumn.text(row,1)) else {throw StorageError.corruptIdentity}
      let b=try JSONDecoder().decode(FileBinding.self,from:bytes);try SaveValidation.binding(b)
      guard b.documentID==self.identity.documentID,b.revision==revision else {throw StorageError.corruptIdentity};value=b
    };return value
  }
  private func head(_ reader:SQLiteDatabase)throws->SavedFileHead? {
    var value:SavedFileHead?
    try reader.statement("SELECT binding_revision,saved_revision,saved_snapshot_hash,last_save_operation FROM file_head WHERE document_id=?",[.text(identity.documentID!)]) {row in
      let revision=sqlite3_column_int64(row,0);guard revision>0,revision<=Int64(HostSerial.maximum),let binding=try self.binding(UInt64(revision),reader:reader) else {throw StorageError.corruptIdentity}
      let saved=sqlite3_column_type(row,1)==SQLITE_NULL ? nil : UInt64(sqlite3_column_int64(row,1)),hash=sqlite3_column_type(row,2)==SQLITE_NULL ? nil : try SQLColumn.text(row,2),op=sqlite3_column_type(row,3)==SQLITE_NULL ? nil : try SQLColumn.text(row,3)
      guard (saved==nil)==(hash==nil) else {throw StorageError.corruptIdentity}
      if let saved,let hash {
        var found=false;try reader.statement("SELECT snapshot_hash FROM document_revisions WHERE document_id=? AND revision=?",[.text(self.identity.documentID!),.integer(Int64(saved))]) {found=try SQLColumn.text($0,0)==hash}
        guard found else {throw StorageError.corruptIdentity}
      }
      value = .init(binding:binding,savedRevision:saved,savedSnapshotHash:hash,lastSaveOperation:op)
    };return value
  }
  func readHead()throws->SavedFileHead? {let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()};return try head(reader)}
  func choose(_ binding:FileBinding,opened:SaveSnapshot?=nil,faults:StorageFaults = .init())throws->SavedFileHead {
    try SaveValidation.binding(binding);guard binding.documentID==identity.documentID else {throw StorageError.corruptIdentity}
    let old=try readHead();guard binding.revision==(old?.binding.revision ?? 0)+1 else {throw SaveError.bindingChanged}
    let bytes=try encoded(binding)
    try db.transaction {
      if let opened {guard opened.source.document_id==identity.documentID,opened.source.revision.value==0,binding.expected != nil else {throw StorageError.corruptIdentity}}
      try db.statement("INSERT INTO file_bindings VALUES(?,?,?,?)",[.text(identity.documentID!),.integer(Int64(binding.revision)),.blob(bytes),.text(KernelHashes.data(bytes))])
      try db.statement("INSERT INTO file_head VALUES(?,?,?,?,NULL) ON CONFLICT(document_id) DO UPDATE SET binding_revision=excluded.binding_revision,saved_revision=excluded.saved_revision,saved_snapshot_hash=excluded.saved_snapshot_hash,last_save_operation=NULL",[.text(identity.documentID!),.integer(Int64(binding.revision)),opened.map{.integer(Int64($0.source.revision.value))} ?? .null,opened.map{.text($0.source.snapshot_hash)} ?? .null])
      try faults.reach("file_binding_before_commit")
    }
    do {try faults.reach("file_binding_committed");guard let confirmed=try readHead(),confirmed.binding.revision==binding.revision else {throw StorageError.corruptIdentity};try db.confirmCommittedBytes();return confirmed} catch {throw StorageError.unknownCommit(EIO)}
  }
  private func intent(_ operation:String,reader:SQLiteDatabase)throws->SaveIntent? {
    var value:SaveIntent?
    try reader.statement("SELECT record,record_hash,source_revision,source_snapshot_hash,binding_revision,file_hash,file_length FROM save_intents WHERE document_id=? AND operation_id=?",[.text(identity.documentID!),.text(operation)]) {row in
      let bytes=try SQLColumn.data(row,0);guard KernelHashes.data(bytes)==(try SQLColumn.text(row,1)) else {throw StorageError.corruptIdentity}
      let i=try JSONDecoder().decode(SaveIntent.self,from:bytes);try SaveValidation.intent(i)
      guard i.documentID==self.identity.documentID,i.storeID==self.identity.storeID,i.operationID==operation,
        Int64(i.sourceRevision)==sqlite3_column_int64(row,2),i.sourceSnapshotHash==(try SQLColumn.text(row,3)),Int64(i.binding.revision)==sqlite3_column_int64(row,4),i.fileHash==(try SQLColumn.text(row,5)),Int64(i.fileByteLength)==sqlite3_column_int64(row,6) else {throw StorageError.corruptIdentity};value=i
    };return value
  }
  func readIntent(_ operation:String)throws->SaveIntent? {let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()};return try intent(operation,reader:reader)}
  func prepare(snapshot:SaveSnapshot,binding:FileBinding,operation:String,descriptor:BlobDescriptor,faults:StorageFaults)throws->SaveIntent {
    let value=SaveIntent(codecVersion:1,operationID:operation,documentID:identity.documentID!,storeID:identity.storeID,sourceRevision:snapshot.source.revision.value,sourceSnapshotHash:snapshot.source.snapshot_hash,binding:binding,fileHash:snapshot.fileHash,fileByteLength:UInt64(snapshot.bytes.count));try SaveValidation.intent(value)
    guard descriptor.hash==value.fileHash,descriptor.byteLength==value.fileByteLength else {throw StorageError.corruptIdentity}
    if let previous=try readIntent(operation) {guard try encoded(previous)==encoded(value) else {throw StorageError.idempotencyConflict};try db.confirmCommittedBytes();return previous}
    let bytes=try encoded(value)
    try db.transaction {
      guard let current=try head(db),current.binding.revision==binding.revision,try encoded(current.binding)==encoded(binding) else {throw SaveError.bindingChanged}
      var actual:String?;try db.statement("SELECT snapshot_hash FROM document_revisions WHERE document_id=? AND revision=?",[.text(identity.documentID!),.integer(Int64(value.sourceRevision))]) {actual=try SQLColumn.text($0,0)}
      guard actual==value.sourceSnapshotHash else {throw SaveError.sourceChanged}
      let ref=BlobReference(owner:.init(kind:.export,id:operation),descriptor:descriptor,mediaType:"application/x-openmath-notebook",codecVersion:"omnb-v1")
      try BlobReferences.insertWithinTransaction(ref,db:db)
      try db.statement("INSERT INTO save_intents VALUES(?,?,?,?,?,?,?,?,?,'prepared')",[.text(identity.documentID!),.text(operation),.integer(Int64(value.sourceRevision)),.text(value.sourceSnapshotHash),.integer(Int64(binding.revision)),.text(value.fileHash),.integer(Int64(value.fileByteLength)),.blob(bytes),.text(KernelHashes.data(bytes))])
      try faults.reach("save_intent_before_commit")
    }
    do {try faults.reach("save_intent_committed");guard let actual=try readIntent(operation),try encoded(actual)==bytes else {throw StorageError.corruptIdentity};try db.confirmCommittedBytes();return actual} catch {throw StorageError.unknownCommit(EIO)}
  }
  private func receipt(_ operation:String,reader:SQLiteDatabase)throws->SaveReceipt? {
    var value:SaveReceipt?
    try reader.statement("SELECT r.record,r.record_hash,e.payload,e.payload_hash FROM save_receipts r JOIN save_outbox e ON r.document_id=e.document_id AND r.operation_id=e.operation_id WHERE r.document_id=? AND r.operation_id=?",[.text(identity.documentID!),.text(operation)]) {row in
      let bytes=try SQLColumn.data(row,0);guard bytes==(try SQLColumn.data(row,2)),KernelHashes.data(bytes)==(try SQLColumn.text(row,1)),KernelHashes.data(bytes)==(try SQLColumn.text(row,3)),let i=try self.intent(operation,reader:reader) else {throw StorageError.corruptIdentity}
      let r=try JSONDecoder().decode(SaveReceipt.self,from:bytes);try SaveValidation.receipt(r,intent:i);value=r
    }
    if value==nil {var exists=false;try reader.statement("SELECT 1 FROM save_receipts WHERE document_id=? AND operation_id=?",[.text(identity.documentID!),.text(operation)]) {_ in exists=true};if exists {throw StorageError.corruptIdentity}}
    return value
  }
  func query(_ operation:String)throws->SaveReceipt? {let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()};let value=try receipt(operation,reader:reader);if value != nil {try db.confirmCommittedBytes()};return value}
  func completed(_ actual:SaveReceipt,faults:StorageFaults)throws->SaveReceipt {
    guard let i=try readIntent(actual.operationID) else {throw StorageError.corruptIdentity};try SaveValidation.receipt(actual,intent:i)
    if let previous=try query(actual.operationID) {guard previous.fileHash==actual.fileHash else {throw StorageError.idempotencyConflict};return previous}
    let bytes=try encoded(actual)
    try db.transaction {
      try db.statement("INSERT INTO save_receipts VALUES(?,?,?,?)",[.text(identity.documentID!),.text(actual.operationID),.blob(bytes),.text(KernelHashes.data(bytes))])
      try db.statement("INSERT INTO save_outbox VALUES(?,?,?,?,?,0)",[.text("save-event-"+actual.operationID),.text(identity.documentID!),.text(actual.operationID),.blob(bytes),.text(KernelHashes.data(bytes))])
      try db.statement("UPDATE save_intents SET phase='completed' WHERE document_id=? AND operation_id=?",[.text(identity.documentID!),.text(actual.operationID)])
      guard let old=try binding(actual.bindingRevision,reader:db) else {throw StorageError.corruptIdentity}
      let updated=FileBinding(codecVersion:old.codecVersion,documentID:old.documentID,revision:old.revision,bookmark:actual.updatedBookmark,fileName:old.fileName,displayPath:old.displayPath,expected:actual.actual)
      let bindingBytes=try encoded(updated)
      try db.statement("UPDATE file_bindings SET record=?,record_hash=? WHERE document_id=? AND binding_revision=?",[.blob(bindingBytes),.text(KernelHashes.data(bindingBytes)),.text(identity.documentID!),.integer(Int64(actual.bindingRevision))])
      try db.statement("UPDATE file_head SET saved_revision=?,saved_snapshot_hash=?,last_save_operation=? WHERE document_id=? AND binding_revision=?",[.integer(Int64(actual.sourceRevision)),.text(actual.sourceSnapshotHash),.text(actual.operationID),.text(identity.documentID!),.integer(Int64(actual.bindingRevision))])
      try faults.reach("save_receipt_before_commit")
    }
    do {try faults.reach("save_receipt_committed");guard let confirmed=try query(actual.operationID) else {throw StorageError.corruptIdentity};return confirmed} catch {throw StorageError.unknownCommit(EIO)}
  }
}
