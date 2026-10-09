import Foundation
import SQLite3

/// Bootstrap selects only a validated active generation. Unselected databases are recovery data.
enum StorageBootstrap {
  static let selectorKeys:Set<String>=["format_version","store_id","generation","header_hash"]
  static func root(_ paths:NativeStoragePaths,faults:StorageFaults) throws ->RootLease {
    try ManagedFiles.directory(paths.root);try ManagedFiles.validateLocal(paths.root)
    let metadata=paths.root.appendingPathComponent("store.json")
    // A future root is inspected before even creating a lock/temporary file.
    if try ManagedFiles.check(metadata,directory:false) { try validateRoot(metadata,paths.channel) }
    let lease=try RootLease(paths.root)
    if try ManagedFiles.check(metadata,directory:false) { try validateRoot(metadata,paths.channel) }
    else {
      let names=try FileManager.default.contentsOfDirectory(atPath:paths.root.path)
      guard names.allSatisfy({$0=="store.lock" || $0==".DS_Store"}) else { throw StorageError.recoveryRequired }
      let identity=RootIdentity(formatVersion:1,installationID:UUID().uuidString.lowercased(),channel:paths.channel)
      try ManagedFiles.publish(identity,to:metadata,faults:.init { try faults.reach("root."+$0) })
      try faults.reach("root_published")
    }
    return lease
  }
  private static func validateRoot(_ metadata:URL,_ channel:StorageChannel) throws {
    let identity=try ManagedFiles.readJSON(RootIdentity.self,metadata,keys:["format_version","installation_id","channel"])
    guard identity.formatVersion==1 else { throw StorageError.unsupportedVersion }
    guard identity.channel==channel,UUID(uuidString:identity.installationID) != nil else { throw StorageError.corruptIdentity }
  }
  static func openStore(folder:URL,kind:String,document:String?,faults:StorageFaults) throws ->(SQLiteDatabase,StoreIdentity,SQLiteRuntimeInfo) {
    try ManagedFiles.directory(folder)
    let selectorURL=folder.appendingPathComponent("active.json")
    if try ManagedFiles.check(selectorURL,directory:false) {
      let selector=try ManagedFiles.readJSON(StoreSelector.self,selectorURL,keys:selectorKeys)
      guard selector.formatVersion==1 else { throw StorageError.unsupportedVersion }
      guard selector.generation>0,selector.generation<=999999,UUID(uuidString:selector.storeID) != nil,
        selector.headerHash.count==64,selector.headerHash.allSatisfy({$0.isHexDigit && !$0.isUppercase}) else { throw StorageError.corruptIdentity }
      let generation=folder.appendingPathComponent("Generations",isDirectory:true).appendingPathComponent(String(format:"%06lld",selector.generation),isDirectory:true)
      guard try ManagedFiles.check(generation,directory:true) else { throw StorageError.recoveryRequired }
      let url=generation.appendingPathComponent(kind=="library" ? "library.sqlite" : "authority.sqlite")
      // Inspect every version/header/hash in a read-only connection before enabling any writing.
      let read=try SQLiteDatabase(url:url,readonly:true)
      let header=try readHeader(read)
      try read.close()
      guard header.storeID==selector.storeID,header.generation==selector.generation,header.kind==kind,
        header.documentID==document,try header.hash==selector.headerHash else { throw StorageError.corruptIdentity }
      let writer=try SQLiteDatabase(url:url)
      let runtime=try writer.configure()
      try writer.transaction { try writer.statement("UPDATE store_state SET last_clean_shutdown=0 WHERE singleton=1") }
      return (writer,header,runtime)
    }
    let generations=folder.appendingPathComponent("Generations",isDirectory:true)
    let names=try FileManager.default.contentsOfDirectory(atPath:folder.path)
    guard names.isEmpty else { throw StorageError.recoveryRequired }
    try ManagedFiles.directory(generations)
    let generation=generations.appendingPathComponent("000001",isDirectory:true)
    try ManagedFiles.directory(generation)
    let url=generation.appendingPathComponent(kind=="library" ? "library.sqlite" : "authority.sqlite")
    try faults.reach("before_database_open")
    let db=try SQLiteDatabase(url:url,create:true)
    let runtime=try db.configure(),header=StoreIdentity.fresh(kind:kind,document:document)
    try db.transaction {
      try db.statement("CREATE TABLE store_header(singleton INTEGER PRIMARY KEY CHECK(singleton=1),store_id TEXT NOT NULL,kind TEXT NOT NULL,generation INTEGER NOT NULL,store_version INTEGER NOT NULL,minimum_reader_version INTEGER NOT NULL,codec_version INTEGER NOT NULL,document_id TEXT)")
      try db.statement("CREATE TABLE store_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),last_clean_shutdown INTEGER NOT NULL CHECK(last_clean_shutdown IN (0,1)))")
      try db.statement("INSERT INTO store_header VALUES(1,?,?,?,?,?,?,?)",[.text(header.storeID),.text(kind),.integer(1),.integer(header.storeVersion),.integer(header.minimumReaderVersion),.integer(header.codecVersion),document.map(SQLValue.text) ?? .null])
      try db.statement("INSERT INTO store_state VALUES(1,0)")
      try db.statement(kind=="document" ? "PRAGMA user_version=2" : "PRAGMA user_version=1")
      try faults.reach("before_bootstrap_commit")
    }
    try faults.reach("bootstrap_committed")
    let read=try SQLiteDatabase(url:url,readonly:true)
    guard try readHeader(read)==header else { throw StorageError.corruptIdentity }
    try read.close();try faults.reach("bootstrap_readback")
    try ManagedFiles.syncDirectory(generation);try ManagedFiles.syncDirectory(generations)
    let selector=StoreSelector(formatVersion:1,storeID:header.storeID,generation:1,headerHash:try header.hash)
    try faults.reach("before_selector_publish")
    try ManagedFiles.publish(selector,to:selectorURL,faults:.init { try faults.reach("selector."+$0) })
    try faults.reach("selector_published")
    return (db,header,runtime)
  }
  static func readHeader(_ db:SQLiteDatabase) throws ->StoreIdentity {
    let version=try db.integer("PRAGMA user_version")
    guard version==1 || version==2 else { throw StorageError.unsupportedVersion }
    var header:StoreIdentity?
    try db.statement("SELECT store_id,kind,generation,store_version,minimum_reader_version,codec_version,document_id FROM store_header WHERE singleton=1") { row in
      func text(_ column:Int32) throws ->String {
        let count=Int(sqlite3_column_bytes(row,column))
        guard let bytes=sqlite3_column_text(row,column),count>0,count<1024,
          let value=String(bytes:UnsafeBufferPointer(start:bytes,count:count),encoding:.utf8),!value.contains("\0") else { throw StorageError.corruptIdentity }
        return value
      }
      header=StoreIdentity(storeID:try text(0),kind:try text(1),generation:sqlite3_column_int64(row,2),storeVersion:sqlite3_column_int64(row,3),minimumReaderVersion:sqlite3_column_int64(row,4),codecVersion:sqlite3_column_int64(row,5),documentID:sqlite3_column_type(row,6)==SQLITE_NULL ? nil : try text(6))
    }
    guard let header else { throw StorageError.corruptIdentity }
    try header.validate()
    guard version==header.storeVersion else {throw StorageError.unsupportedVersion}
    guard try db.integer("SELECT count(*) FROM store_header")==1,
      try db.integer("SELECT count(*) FROM store_state WHERE singleton=1 AND last_clean_shutdown IN (0,1)")==1 else { throw StorageError.corruptIdentity }
    guard try db.text("PRAGMA quick_check(1)")=="ok" else { throw StorageError.corruptIdentity }
    return header
  }
}
