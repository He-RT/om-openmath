import Foundation
import SQLite3

/// Library stores immutable projections, never a second document authority. New format 2;
/// existing format 1 is not silently upgraded by an ordinary open.
final class LibraryStore {
  private let db:SQLiteDatabase,url:URL,identity:StoreIdentity
  init(db:SQLiteDatabase,url:URL,identity:StoreIdentity,create:Bool=false)throws {
    guard identity.kind=="library",(2...4).contains(identity.storeVersion) else {throw StorageError.unsupportedVersion}
    self.db=db;self.url=url;self.identity=identity
    var present=false
    try db.statement("SELECT 1 FROM sqlite_schema WHERE type='table' AND name='document_effect_events'") {_ in present=true}
    if !present {
      guard create else {throw StorageError.recoveryRequired}
      try db.transaction {
        try db.statement("CREATE TABLE document_effect_events(sequence INTEGER PRIMARY KEY CHECK(sequence>0),source_store_id TEXT NOT NULL,source_generation INTEGER NOT NULL,document_id TEXT NOT NULL,event_id TEXT NOT NULL,operation_id TEXT NOT NULL,kind TEXT NOT NULL,record BLOB NOT NULL,record_hash TEXT NOT NULL,effect_hash TEXT NOT NULL,UNIQUE(source_store_id,event_id))")
      }
    }
    // Bounded startup prefix validation; later history pages validate their own complete
    // records. A bad full record is never treated as a discardable stream tail.
    _ = try readPage(db,after:0,limit:16)
  }
  private func read(_ reader:SQLiteDatabase,store:String,event:String)throws->StoredDocumentEffect? {
    var result:StoredDocumentEffect?
    try reader.statement("SELECT sequence,source_store_id,source_generation,document_id,event_id,operation_id,kind,record,record_hash,effect_hash FROM document_effect_events WHERE source_store_id=? AND event_id=?",[.text(store),.text(event)]) {row in
      result=try Self.decode(row)
    };return result
  }
  private static func decode(_ row:OpaquePointer)throws->StoredDocumentEffect {
    let bytes=try SQLColumn.data(row,7);guard KernelHashes.data(bytes)==(try SQLColumn.text(row,8)) else {throw StorageError.corruptIdentity}
    let effect=try JSONDecoder().decode(DocumentEffect.self,from:bytes);try RecoveryValidation.effect(effect)
    let sequence=sqlite3_column_int64(row,0)
    guard sequence>0,effect.sourceStoreID==(try SQLColumn.text(row,1)),effect.sourceGeneration==sqlite3_column_int64(row,2),
      effect.documentID==(try SQLColumn.text(row,3)),effect.eventID==(try SQLColumn.text(row,4)),
      effect.operationID==(try SQLColumn.text(row,5)),effect.kind.rawValue==(try SQLColumn.text(row,6)),
      try effect.stableHash==SQLColumn.text(row,9) else {throw StorageError.corruptIdentity}
    return .init(sequence:sequence,effect:effect)
  }
  func append(_ effect:DocumentEffect,faults:StorageFaults)throws->LibraryProjectionReceipt {
    try RecoveryValidation.effect(effect)
    let bytes=try RecoveryValidation.encode(effect),hash=try effect.stableHash
    if let previous=try read(db,store:effect.sourceStoreID,event:effect.eventID) {
      guard try previous.effect.stableHash==hash else {throw StorageError.idempotencyConflict}
      return try confirmed(effect,inserted:false)
    }
    try db.transaction {
      let previous=try db.integer("SELECT coalesce(max(sequence),0) FROM document_effect_events")
      guard previous>=0,previous<Int64.max else {throw StorageError.corruptIdentity}
      try db.statement("INSERT INTO document_effect_events VALUES(?,?,?,?,?,?,?,?,?,?)",[.integer(previous+1),.text(effect.sourceStoreID),.integer(effect.sourceGeneration),.text(effect.documentID),.text(effect.eventID),.text(effect.operationID),.text(effect.kind.rawValue),.blob(bytes),.text(KernelHashes.data(bytes)),.text(hash)])
      try faults.reach("library_projection_before_commit")
    }
    do {try faults.reach("library_projection_committed");return try confirmed(effect,inserted:true)}
    catch {throw StorageError.unknownCommit(EIO)}
  }
  private func confirmed(_ effect:DocumentEffect,inserted:Bool)throws->LibraryProjectionReceipt {
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    guard let actual=try read(reader,store:effect.sourceStoreID,event:effect.eventID),try actual.effect.stableHash==effect.stableHash else {throw StorageError.corruptIdentity}
    try db.confirmCommittedBytes()
    return .init(libraryStoreID:identity.storeID,libraryGeneration:identity.generation,librarySequence:actual.sequence,sourceStoreID:effect.sourceStoreID,sourceGeneration:effect.sourceGeneration,documentID:effect.documentID,eventID:effect.eventID,kind:effect.kind,effectHash:try effect.stableHash,inserted:inserted)
  }
  func page(after:Int64,limit:Int)throws->[StoredDocumentEffect] {
    guard after>=0,limit>0,limit<=16 else {throw StorageError.corruptIdentity}
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    return try readPage(reader,after:after,limit:limit)
  }
  func event(store:String,id:String)throws->StoredDocumentEffect? {
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()};return try read(reader,store:store,event:id)
  }
  private func readPage(_ reader:SQLiteDatabase,after:Int64,limit:Int)throws->[StoredDocumentEffect] {
    var records:[StoredDocumentEffect]=[],bytes=0,stopped=false
    try reader.statement("SELECT sequence,source_store_id,source_generation,document_id,event_id,operation_id,kind,record,record_hash,effect_hash FROM document_effect_events WHERE sequence>? ORDER BY sequence LIMIT ?",[.integer(after),.integer(Int64(limit))]) {row in
      guard !stopped else {return}
      let length=sqlite3_column_bytes(row,7);guard length>=0,length<=2*RecoveryValidation.maximumPayload else {throw StorageError.corruptIdentity}
      guard bytes+Int(length)<=8*1024*1024 else {stopped=true;return}
      let actual=try Self.decode(row);records.append(actual);bytes+=Int(length)
    };return records
  }
}
