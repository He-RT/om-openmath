import Foundation
import SQLite3

/// Draft text is separate from source authority. Full snapshots are bounded to the latest two;
/// original receipts/tombstones retain idempotency without keeping every typing frame forever.
final class DraftBackupStore {
  static let tables:Set<String>=["draft_owners","draft_state","draft_batches"]
  private let db:SQLiteDatabase,url:URL,identity:StoreIdentity
  init(db:SQLiteDatabase,url:URL,identity:StoreIdentity,create:Bool=false)throws {
    guard identity.kind=="document",identity.storeVersion==5,identity.documentID != nil else {throw StorageError.unsupportedVersion}
    self.db=db;self.url=url;self.identity=identity
    var present=Set<String>();try db.statement("SELECT name FROM sqlite_schema WHERE type='table'"){present.insert(try SQLColumn.text($0,0))}
    let found=present.intersection(Self.tables)
    if found != Self.tables {
      guard create,found.isEmpty else {throw StorageError.recoveryRequired}
      try db.transaction {
        try db.statement("CREATE TABLE draft_owners(owner_id TEXT PRIMARY KEY,lease_serial INTEGER NOT NULL UNIQUE CHECK(lease_serial>0),record BLOB NOT NULL,record_hash TEXT NOT NULL)")
        try db.statement("CREATE TABLE draft_batches(owner_id TEXT NOT NULL,sequence INTEGER NOT NULL CHECK(sequence>0),document_id TEXT NOT NULL,source_revision INTEGER NOT NULL,source_snapshot_hash TEXT NOT NULL,snapshot BLOB NOT NULL,snapshot_hash TEXT NOT NULL,receipt BLOB NOT NULL,receipt_hash TEXT NOT NULL,PRIMARY KEY(owner_id,sequence),FOREIGN KEY(owner_id) REFERENCES draft_owners(owner_id),FOREIGN KEY(document_id,source_revision,source_snapshot_hash) REFERENCES document_revisions(document_id,revision,snapshot_hash))")
        try db.statement("CREATE TABLE draft_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),active_owner_id TEXT,current_owner_id TEXT,current_sequence INTEGER,FOREIGN KEY(active_owner_id) REFERENCES draft_owners(owner_id),FOREIGN KEY(current_owner_id,current_sequence) REFERENCES draft_batches(owner_id,sequence))")
        try db.statement("INSERT INTO draft_state VALUES(1,NULL,NULL,NULL)")
      }
    }
    _ = try readHead(db)
  }
  private func readOwner(_ id:String,_ reader:SQLiteDatabase)throws->DraftBackupOwner? {
    var result:DraftBackupOwner?
    try reader.statement("SELECT lease_serial,record,record_hash FROM draft_owners WHERE owner_id=?",[.text(id)]) { [self] row in
      let bytes=try SQLColumn.data(row,1);guard KernelHashes.data(bytes)==(try SQLColumn.text(row,2)) else {throw StorageError.corruptIdentity}
      let value=try JSONDecoder().decode(DraftBackupOwner.self,from:bytes);try DraftBackupValidation.owner(value)
      guard value.ownerID==id,value.storeID==identity.storeID,value.documentID==identity.documentID,Int64(value.leaseSerial)==sqlite3_column_int64(row,0) else {throw StorageError.corruptIdentity};result=value
    };return result
  }
  func beginOwner(_ id:String,runtime:String,generation:UInt64,faults:StorageFaults)throws->DraftBackupOwner {
    if let previous=try readOwner(id,db) {
      guard previous.runtime==runtime,previous.documentGeneration==generation else {throw StorageError.idempotencyConflict}
      var active:String?;try db.statement("SELECT active_owner_id FROM draft_state WHERE singleton=1") {if sqlite3_column_type($0,0) != SQLITE_NULL {active=try SQLColumn.text($0,0)}}
      guard active==id else {throw StorageError.staleRevision}
      try db.confirmCommittedBytes();return previous
    }
    var owner:DraftBackupOwner?
    try db.transaction {
      let serial=try db.integer("SELECT coalesce(max(lease_serial),0) FROM draft_owners")
      guard serial>=0,serial<Int64(HostSerial.maximum) else {throw StorageError.corruptIdentity}
      let value=DraftBackupOwner(ownerID:id,storeID:identity.storeID,documentID:identity.documentID!,leaseSerial:UInt64(serial+1),runtime:runtime,documentGeneration:generation);try DraftBackupValidation.owner(value)
      let bytes=try DraftBackupValidation.encode(value)
      try db.statement("INSERT INTO draft_owners VALUES(?,?,?,?)",[.text(id),.integer(serial+1),.blob(bytes),.text(KernelHashes.data(bytes))])
      try db.statement("UPDATE draft_state SET active_owner_id=? WHERE singleton=1",[.text(id)])
      try faults.reach("draft_owner_before_commit");owner=value
    }
    do {
      try faults.reach("draft_owner_committed")
      let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
      guard let actual=try readOwner(id,reader),actual==owner else {throw StorageError.corruptIdentity};try db.confirmCommittedBytes();return actual
    } catch {throw StorageError.unknownCommit(EIO)}
  }
  private func receipt(_ owner:String,sequence:UInt64,reader:SQLiteDatabase)throws->DraftBackupReceipt? {
    guard sequence>0,sequence<=HostSerial.maximum else {throw StorageError.corruptIdentity}
    var result:DraftBackupReceipt?
    try reader.statement("SELECT snapshot_hash,receipt,receipt_hash FROM draft_batches WHERE owner_id=? AND sequence=?",[.text(owner),.integer(Int64(sequence))]) { [self] row in
      let bytes=try SQLColumn.data(row,1);guard KernelHashes.data(bytes)==(try SQLColumn.text(row,2)) else {throw StorageError.corruptIdentity}
      let value=try JSONDecoder().decode(DraftBackupReceipt.self,from:bytes);try DraftBackupValidation.owner(value.owner)
      guard value.codecVersion==1,value.owner.ownerID==owner,value.owner.storeID==identity.storeID,value.owner.documentID==identity.documentID,
        value.sequence==sequence,value.snapshotHash==(try SQLColumn.text(row,0)),KernelValidation.hash(value.snapshotHash),
        try readOwner(owner,reader)==value.owner else {throw StorageError.corruptIdentity};result=value
    };return result
  }
  func query(owner:String,sequence:UInt64)throws->DraftBackupReceipt? {
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    let value=try receipt(owner,sequence:sequence,reader:reader);try db.confirmCommittedBytes();return value
  }
  func write(_ snapshot:DraftBackupSnapshot,faults:StorageFaults)throws->DraftBackupReceipt {
    try DraftBackupValidation.snapshot(snapshot)
    guard snapshot.owner.storeID==identity.storeID,snapshot.owner.documentID==identity.documentID else {throw StorageError.corruptIdentity}
    let bytes=try DraftBackupValidation.encode(snapshot),hash=KernelHashes.data(bytes)
    if let previous=try query(owner:snapshot.owner.ownerID,sequence:snapshot.sequence) {
      guard previous.snapshotHash==hash else {throw StorageError.idempotencyConflict};return previous
    }
    let value=DraftBackupReceipt(codecVersion:1,owner:snapshot.owner,sequence:snapshot.sequence,snapshotHash:hash),receiptBytes=try DraftBackupValidation.encode(value)
    try db.transaction {
      var active:String?;try db.statement("SELECT active_owner_id FROM draft_state WHERE singleton=1"){if sqlite3_column_type($0,0) != SQLITE_NULL {active=try SQLColumn.text($0,0)}}
      guard active==snapshot.owner.ownerID,try readOwner(snapshot.owner.ownerID,db)==snapshot.owner else {throw StorageError.staleRevision}
      var latest:Int64=0;try db.statement("SELECT coalesce(max(sequence),0) FROM draft_batches WHERE owner_id=?",[.text(snapshot.owner.ownerID)]) {latest=sqlite3_column_int64($0,0)}
      guard Int64(snapshot.sequence)>latest else {throw StorageError.staleRevision}
      var sourceHash:String?;try db.statement("SELECT snapshot_hash FROM document_revisions WHERE document_id=? AND revision=?",[.text(identity.documentID!),.integer(Int64(snapshot.sourceRevision))]) {sourceHash=try SQLColumn.text($0,0)}
      guard sourceHash==snapshot.sourceSnapshotHash else {throw StorageError.corruptIdentity}
      try db.statement("INSERT INTO draft_batches VALUES(?,?,?,?,?,?,?,?,?)",[.text(snapshot.owner.ownerID),.integer(Int64(snapshot.sequence)),.text(identity.documentID!),.integer(Int64(snapshot.sourceRevision)),.text(snapshot.sourceSnapshotHash),.blob(bytes),.text(hash),.blob(receiptBytes),.text(KernelHashes.data(receiptBytes))])
      try db.statement("UPDATE draft_state SET current_owner_id=?,current_sequence=? WHERE singleton=1",[.text(snapshot.owner.ownerID),.integer(Int64(snapshot.sequence))])
      try db.statement("UPDATE draft_batches SET snapshot=? WHERE rowid NOT IN (SELECT rowid FROM draft_batches WHERE length(snapshot)>0 ORDER BY rowid DESC LIMIT 2)",[.blob(Data())])
      try faults.reach("draft_backup_before_commit")
    }
    do {try faults.reach("draft_backup_committed");guard let actual=try query(owner:snapshot.owner.ownerID,sequence:snapshot.sequence),actual.snapshotHash==hash else {throw StorageError.corruptIdentity};try faults.reach("draft_backup_readback");return actual}
    catch {throw StorageError.unknownCommit(EIO)}
  }
  private func readHead(_ reader:SQLiteDatabase)throws->StoredDraftBackup? {
    guard try reader.integer("SELECT count(*) FROM draft_state")==1 else {throw StorageError.corruptIdentity}
    var result:StoredDraftBackup?,heads=0
    try reader.statement("SELECT b.owner_id,b.sequence,b.snapshot,b.snapshot_hash,b.source_revision,b.source_snapshot_hash FROM draft_state s JOIN draft_batches b ON s.current_owner_id=b.owner_id AND s.current_sequence=b.sequence WHERE s.singleton=1") { [self] row in
      heads+=1;let owner=try SQLColumn.text(row,0),sequence=sqlite3_column_int64(row,1),bytes=try SQLColumn.data(row,2,maximumBytes:DraftBackupValidation.maximumRecordBytes)
      guard sequence>0,sequence<=Int64(HostSerial.maximum),!bytes.isEmpty,bytes.count<=DraftBackupValidation.maximumRecordBytes,
        KernelHashes.data(bytes)==(try SQLColumn.text(row,3)),let actual=try receipt(owner,sequence:UInt64(sequence),reader:reader) else {throw StorageError.corruptIdentity}
      let value=try JSONDecoder().decode(DraftBackupSnapshot.self,from:bytes);try DraftBackupValidation.snapshot(value)
      guard value.owner==actual.owner,value.sequence==actual.sequence,actual.snapshotHash==KernelHashes.data(bytes),
        value.owner.documentID==identity.documentID,Int64(value.sourceRevision)==sqlite3_column_int64(row,4),value.sourceSnapshotHash==(try SQLColumn.text(row,5)) else {throw StorageError.corruptIdentity}
      result = .init(receipt:actual,snapshot:value)
    }
    guard heads<=1 else {throw StorageError.corruptIdentity}
    if result==nil,try reader.integer("SELECT count(*) FROM draft_state WHERE singleton=1 AND (current_owner_id IS NOT NULL OR current_sequence IS NOT NULL)") != 0 {throw StorageError.corruptIdentity}
    return result
  }
  func head()throws->StoredDraftBackup? {let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()};return try readHead(reader)}
}
