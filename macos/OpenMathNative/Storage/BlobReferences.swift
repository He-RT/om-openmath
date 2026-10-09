import Foundation
import SQLite3

/// Per-owner-store references, never a second cross-database source authority.
final class BlobReferences {
  private let db:SQLiteDatabase
  private let url:URL
  init(db:SQLiteDatabase,url:URL) throws {
    self.db=db;self.url=url
    var tables=Set<String>()
    try db.statement("SELECT name FROM sqlite_schema WHERE type='table'"){tables.insert(try SQLColumn.text($0,0))}
    let found=tables.intersection(["blob_objects","blob_refs"])
    if found==["blob_objects","blob_refs"] {return}
    guard found.isEmpty else {throw StorageError.recoveryRequired}
    try db.transaction {
      try db.statement("CREATE TABLE blob_objects(blob_hash TEXT PRIMARY KEY CHECK(length(blob_hash)=64),byte_length INTEGER NOT NULL CHECK(byte_length>=0 AND byte_length<=134217728))")
      try db.statement("CREATE TABLE blob_refs(owner_kind TEXT NOT NULL,owner_id TEXT NOT NULL,blob_hash TEXT NOT NULL,media_type TEXT NOT NULL,codec_version TEXT NOT NULL,PRIMARY KEY(owner_kind,owner_id,blob_hash),FOREIGN KEY(blob_hash) REFERENCES blob_objects(blob_hash))")
    }
  }
  private func validate(_ reference:BlobReference) throws {
    guard reference.descriptor.byteLength<=128*1024*1024,reference.descriptor.hash.utf8.count==64,
      reference.descriptor.hash.utf8.allSatisfy({($0>=48 && $0<=57)||($0>=97 && $0<=102)}),
      SourceValidation.identity(reference.owner.id),!reference.mediaType.isEmpty,reference.mediaType.utf8.count<=256,
      reference.mediaType.utf8.allSatisfy({$0>=32 && $0<127}),SourceValidation.identity(reference.codecVersion) else {throw StorageError.corruptIdentity}
  }
  /// Caller keeps an independent verified BlobStore transfer pin until this transaction settles.
  func attach(_ reference:BlobReference,faults:StorageFaults) throws ->BlobReference {
    try validate(reference)
    try db.transaction {
      var length:Int64?
      try db.statement("SELECT byte_length FROM blob_objects WHERE blob_hash=?",[.text(reference.descriptor.hash)]){length=sqlite3_column_int64($0,0)}
      if let length {guard length==Int64(reference.descriptor.byteLength) else {throw StorageError.corruptIdentity}}
      else {try db.statement("INSERT INTO blob_objects VALUES(?,?)",[.text(reference.descriptor.hash),.integer(Int64(reference.descriptor.byteLength))])}
      if let previous=try query(reference.owner,hash:reference.descriptor.hash,reader:db) {
        guard previous==reference else {throw StorageError.idempotencyConflict}
      } else {
        try db.statement("INSERT INTO blob_refs VALUES(?,?,?,?,?)",[.text(reference.owner.kind.rawValue),.text(reference.owner.id),.text(reference.descriptor.hash),.text(reference.mediaType),.text(reference.codecVersion)])
      }
      try faults.reach("blob_reference_inserted")
    }
    do {
      try faults.reach("blob_reference_committed")
      let read=try SQLiteDatabase(url:url,readonly:true);defer {try? read.close()}
      guard try query(reference.owner,hash:reference.descriptor.hash,reader:read)==reference else {throw StorageError.corruptIdentity}
      try db.confirmCommittedBytes()
      return reference
    } catch {throw StorageError.unknownCommit(EIO)}
  }
  private func query(_ owner:BlobOwner,hash:String,reader:SQLiteDatabase) throws ->BlobReference? {
    var reference:BlobReference?
    try reader.statement("SELECT o.byte_length,r.media_type,r.codec_version FROM blob_refs r JOIN blob_objects o ON r.blob_hash=o.blob_hash WHERE r.owner_kind=? AND r.owner_id=? AND r.blob_hash=?",[.text(owner.kind.rawValue),.text(owner.id),.text(hash)]) { row in
      let size=sqlite3_column_int64(row,0);guard size>=0 else {throw StorageError.corruptIdentity}
      let decoded=BlobReference(owner:owner,descriptor:.init(hash:hash,byteLength:UInt64(size)),mediaType:try SQLColumn.text(row,1),codecVersion:try SQLColumn.text(row,2))
      try self.validate(decoded);reference=decoded
    }
    if reference==nil {
      var exists=false
      try reader.statement("SELECT 1 FROM blob_refs WHERE owner_kind=? AND owner_id=? AND blob_hash=?",[.text(owner.kind.rawValue),.text(owner.id),.text(hash)]){_ in exists=true}
      if exists {throw StorageError.corruptIdentity}
    }
    return reference
  }
  func reference(_ owner:BlobOwner,hash:String) throws ->BlobReference? {
    guard SourceValidation.identity(owner.id) else {throw StorageError.corruptIdentity}
    let read=try SQLiteDatabase(url:url,readonly:true);defer {try? read.close()}
    let result=try query(owner,hash:hash,reader:read)
    if result != nil {try db.confirmCommittedBytes()}
    return result
  }
  func count(_ hash:String) throws ->Int {
    var count=0
    try db.statement("SELECT count(*) FROM blob_refs WHERE blob_hash=?",[.text(hash)]){count=Int(sqlite3_column_int64($0,0))}
    return count
  }
}
