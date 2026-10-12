import Darwin
import CryptoKit
import Foundation
import SQLite3

/// Explicit native-only upgrades/restores. Never imports another application's storage.
/// Target connections close and the complete generation is synced before active.json changes.
enum StoreGenerationService {
  static func nextGeneration(_ folder:URL)throws->Int64 {
    let generations=folder.appendingPathComponent("Generations")
    guard try ManagedFiles.check(generations,directory:true) else {throw StorageError.recoveryRequired}
    let names=try FileManager.default.contentsOfDirectory(atPath:generations.path)
    guard names.count<=10000 else {throw StorageError.recoveryRequired}
    var maximum:Int64=0
    for name in names {
      if name.hasPrefix(".pending-") {continue}
      guard name.count==6,let value=Int64(name),value>0,value<=999999 else {throw StorageError.recoveryRequired}
      maximum=max(maximum,value)
    }
    guard maximum<999999 else {throw BlobError.budgetExceeded};return maximum+1
  }
  static func upgrade(_ store:CatalogStore,faults:StorageFaults)throws->StoreIdentity {
    let source=try SQLiteDatabase(url:store.database,readonly:true)
    defer {try? source.close()}
    try StorageBootstrap.rejectQuarantine(source)
    let version:Int64=store.identity.kind=="document" ? 5 : 4
    guard store.identity.storeVersion<version else {return store.identity}
    let old=store.identity,new=StoreIdentity(storeID:old.storeID,kind:old.kind,generation:try nextGeneration(store.folder),storeVersion:version,minimumReaderVersion:version,codecVersion:version,documentID:old.documentID)
    let pending=store.folder.appendingPathComponent("Generations/.pending-"+UUID().uuidString.lowercased())
    try ManagedFiles.directory(pending)
    let target=pending.appendingPathComponent(BackupService.databaseName(old))
    try source.backup(to:target,faults:faults)
    let db=try SQLiteDatabase(url:target);defer {try? db.close()}
    _ = try db.configure()
    try db.transaction {
      try db.statement("UPDATE store_header SET generation=?,store_version=?,minimum_reader_version=?,codec_version=? WHERE singleton=1",[.integer(new.generation),.integer(version),.integer(version),.integer(version)])
      try db.statement("PRAGMA user_version=\(version)")
    }
    if old.kind=="library" {
      _ = try LibraryStore(db:db,url:target,identity:new,create:true)
      _ = try AgentSessionStore(db:db,url:target,identity:new,create:true)
    } else {
      _ = try BlobReferences(db:db,url:target)
      _ = try DocumentStore(db:db,url:target,identity:new)
      if old.storeVersion==1 {
        let history=try SourceHistoryStore(db:db,document:new.documentID!)
        try db.transaction {
          try db.statement("SELECT forward_plan,inverse_snapshot FROM transactions ORDER BY committed_revision") {row in
            let forward=try SQLColumn.data(row,0),inverse=try SQLColumn.data(row,1)
            let plan=try JSONDecoder().decode(NativeSourceCommit.self,from:forward);try SourceValidation.commit(plan)
            try history.insert(plan,forward:forward,inverse:inverse)
          }
        }
      }
      _ = try KernelStore(db:db,url:target,identity:new)
      _ = try SaveStore(db:db,url:target,identity:new)
      _ = try DraftBackupStore(db:db,url:target,identity:new,create:old.storeVersion<5)
    }
    try verifyOriginalTables(source:source,target:db,counts:store.counts)
    guard try StoreCatalog.references(db)==store.references else {throw StorageError.corruptIdentity}
    try StoreCatalog.validate(db,identity:new,url:target)
    try db.checkpoint();try db.close();try source.close()
    try validateClosed(target,identity:new);try faults.reach("generation.verified")
    try publish(pending:pending,folder:store.folder,identity:new,faults:faults)
    return new
  }
  static func restore(_ record:BackupStoreRecord,backupID:String,root:URL,folder:URL,faults:StorageFaults)throws->StoreIdentity {
    let old=record.identity,new=StoreIdentity(storeID:old.storeID,kind:old.kind,generation:try nextGeneration(folder),storeVersion:old.storeVersion,minimumReaderVersion:old.minimumReaderVersion,codecVersion:old.codecVersion,documentID:old.documentID)
    let pending=folder.appendingPathComponent("Generations/.pending-"+UUID().uuidString.lowercased())
    try ManagedFiles.directory(pending)
    let original=root.appendingPathComponent("Backups/"+backupID+"/Stores/"+old.storeID+"/"+BackupService.databaseName(old))
    let source=try SQLiteDatabase(url:original,readonly:true);defer {try? source.close()}
    let target=pending.appendingPathComponent(BackupService.databaseName(old));try source.backup(to:target,faults:faults)
    let db=try SQLiteDatabase(url:target);defer {try? db.close()}
    _ = try db.configure()
    try db.transaction {
      try db.statement("UPDATE store_header SET generation=? WHERE singleton=1",[.integer(new.generation)])
      try db.statement("CREATE TABLE IF NOT EXISTS store_recovery_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),rollback_quarantine INTEGER NOT NULL CHECK(rollback_quarantine=1),backup_id TEXT NOT NULL)")
      try db.statement("INSERT OR REPLACE INTO store_recovery_state VALUES(1,1,?)",[.text(backupID)])
    }
    try verifyOriginalTables(source:source,target:db,counts:record.tableCounts)
    try StoreCatalog.validate(db,identity:new,url:target)
    try db.checkpoint();try db.close();try source.close()
    try validateClosed(target,identity:new);try faults.reach("generation.verified")
    try publish(pending:pending,folder:folder,identity:new,faults:faults);return new
  }
  static func validateClosed(_ url:URL,identity:StoreIdentity)throws {
    let db=try SQLiteDatabase(url:url,readonly:true);defer {try? db.close()}
    guard try StorageBootstrap.readHeader(db)==identity,try db.text("PRAGMA integrity_check")=="ok" else {throw StorageError.corruptIdentity}
    try StoreCatalog.validate(db,identity:identity,url:url)
  }
  static func verifyOriginalTables(source:SQLiteDatabase,target:SQLiteDatabase,counts:[String:Int64])throws {
    let actual=try StoreCatalog.counts(target)
    for (table,count) in counts where table != "store_header" && table != "store_recovery_state" {
      guard actual[table]==count else {throw StorageError.corruptIdentity}
      // Compare complete typed rows, including raw receipt/plan BLOBs, never only counts.
      guard try tableHash(source,table)==tableHash(target,table) else {throw StorageError.corruptIdentity}
    }
  }
  static func tableHash(_ db:SQLiteDatabase,_ table:String)throws->String {
    var columns=0;try db.statement("PRAGMA table_info(\(table))") {_ in columns+=1}
    guard columns>0,columns<=128 else {throw StorageError.corruptIdentity}
    var hash=SHA256()
    let order=(1...columns).map(String.init).joined(separator:",")
    try db.statement("SELECT * FROM \(table) ORDER BY \(order)") {row in
      for column in 0..<columns {
        let index=Int32(column),type=sqlite3_column_type(row,index)
        var tag=type.bigEndian;withUnsafeBytes(of:&tag) {hash.update(data:Data($0))}
        if type==SQLITE_INTEGER {
          var value=sqlite3_column_int64(row,index).bigEndian;withUnsafeBytes(of:&value) {hash.update(data:Data($0))}
        } else if type==SQLITE_FLOAT {
          var bits=sqlite3_column_double(row,index).bitPattern.bigEndian;withUnsafeBytes(of:&bits) {hash.update(data:Data($0))}
        } else {
          let count=sqlite3_column_bytes(row,index)
          var length=Int64(count).bigEndian;withUnsafeBytes(of:&length) {hash.update(data:Data($0))}
          if count>0,let bytes=sqlite3_column_blob(row,index) {hash.update(data:Data(bytes:bytes,count:Int(count)))}
        }
      }
    };return hash.finalize().map{String(format:"%02x",$0)}.joined()
  }
  static func publish(pending:URL,folder:URL,identity:StoreIdentity,faults:StorageFaults)throws {
    try ManagedFiles.syncDirectory(pending)
    let generations=folder.appendingPathComponent("Generations"),final=generations.appendingPathComponent(String(format:"%06lld",identity.generation))
    guard !(try ManagedFiles.check(final,directory:true)),rename(pending.path,final.path)==0 else {throw StorageError.system(errno,"generation_publish")}
    try ManagedFiles.syncDirectory(generations);try faults.reach("generation.published")
    let selector=StoreSelector(formatVersion:1,storeID:identity.storeID,generation:identity.generation,headerHash:try identity.hash)
    try faults.reach("generation.before_selector")
    try ManagedFiles.publish(selector,to:folder.appendingPathComponent("active.json"),faults:.init {try faults.reach("generation.selector."+$0)})
    try faults.reach("generation.selected")
  }
}
