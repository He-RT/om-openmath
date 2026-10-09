import Foundation
import SQLite3
import Darwin

enum SQLValue:Sendable { case integer(Int64),text(String),blob(Data),null }
/// Confined to its StoreWriter queue. Statements never escape and every buffer uses SQLite copy.
final class SQLiteDatabase {
  private var db:OpaquePointer?
  private let url:URL
  init(url:URL,readonly:Bool=false,create:Bool=false) throws {
    self.url=url
    precondition(!Thread.isMainThread,"SQLite must remain on its storage queue")
    if !create { guard try ManagedFiles.check(url,directory:false) else { throw StorageError.recoveryRequired } }
    else { _ = try ManagedFiles.check(url,directory:false) }
    let flags=(readonly ? SQLITE_OPEN_READONLY : SQLITE_OPEN_READWRITE)|(create ? SQLITE_OPEN_CREATE : 0)|SQLITE_OPEN_NOMUTEX|SQLITE_OPEN_NOFOLLOW
    let status=sqlite3_open_v2(url.path,&db,flags,nil)
    guard status==SQLITE_OK else { if let db { sqlite3_close_v2(db) };db=nil;throw StorageError.sqlite(status,"open") }
    if create { guard chmod(url.path,0o600)==0 else { throw StorageError.system(errno,"database_permissions") } }
    sqlite3_extended_result_codes(db,1);sqlite3_busy_timeout(db,1000)
  }
  deinit { if let db { sqlite3_close_v2(db) } }
  func close() throws {
    if let handle=db { let result=sqlite3_close(handle);guard result==SQLITE_OK else { throw StorageError.sqlite(result,"close") };db=nil }
  }
  func statement(_ sql:String,_ values:[SQLValue]=[],rows:((OpaquePointer)throws->Void)?=nil) throws {
    precondition(!Thread.isMainThread,"SQLite must remain on its storage queue")
    guard !sql.utf8.contains(0),sql.utf8.count<=65536 else { throw StorageError.corruptIdentity }
    guard let db else { throw StorageError.closing }
    var raw:OpaquePointer?
    let prepared=sql.withCString { bytes in
      var tail:UnsafePointer<CChar>?
      let result=sqlite3_prepare_v2(db,bytes,-1,&raw,&tail)
      if result==SQLITE_OK,let tail,!String(cString:tail).trimmingCharacters(in:.whitespacesAndNewlines).isEmpty {
        if let raw { sqlite3_finalize(raw) };raw=nil;return SQLITE_MISUSE
      }
      return result
    }
    guard prepared==SQLITE_OK,let raw else { throw StorageError.sqlite(prepared,"prepare") }
    defer { sqlite3_finalize(raw) }
    guard sqlite3_bind_parameter_count(raw)==Int32(values.count) else { throw StorageError.corruptIdentity }
    let transient=unsafeBitCast(-1,to:sqlite3_destructor_type.self)
    for (position,value) in values.enumerated() {
      let index=Int32(position+1),result:Int32
      switch value {
      case .integer(let n):result=sqlite3_bind_int64(raw,index,n)
      case .text(let text):
        guard text.utf8.count<=64*1024*1024 else { throw StorageError.corruptIdentity };result=text.withCString{sqlite3_bind_text(raw,index,$0,Int32(text.utf8.count),transient)}
      case .blob(let data):
        guard data.count<=64*1024*1024 else { throw StorageError.corruptIdentity }
        if data.isEmpty { result=sqlite3_bind_zeroblob(raw,index,0) }
        else { result=data.withUnsafeBytes{sqlite3_bind_blob(raw,index,$0.baseAddress,Int32($0.count),transient)} }
      case .null:result=sqlite3_bind_null(raw,index)
      }
      guard result==SQLITE_OK else { throw StorageError.sqlite(result,"bind") }
    }
    while true {
      let result=sqlite3_step(raw)
      if result==SQLITE_DONE { break }
      guard result==SQLITE_ROW else { throw StorageError.sqlite(result,"step") }
      try rows?(raw)
    }
  }
  func integer(_ sql:String) throws ->Int64 { var output:Int64?;try statement(sql){output=sqlite3_column_int64($0,0)};guard let output else { throw StorageError.corruptIdentity };return output
  }
  func text(_ sql:String) throws ->String { var output:String?;try statement(sql){if let bytes=sqlite3_column_text($0,0){output=String(cString:bytes)}};guard let output else { throw StorageError.corruptIdentity };return output
  }
  func configure() throws ->SQLiteRuntimeInfo {
    guard sqlite3_libversion_number()>=3051003 else { throw StorageError.unsupportedVersion }
    try statement("PRAGMA trusted_schema=OFF");try statement("PRAGMA foreign_keys=ON")
    guard try text("PRAGMA journal_mode=WAL").lowercased()=="wal" else { throw StorageError.unsupportedSync }
    for command in ["PRAGMA synchronous=FULL","PRAGMA fullfsync=ON","PRAGMA checkpoint_fullfsync=ON","PRAGMA wal_autocheckpoint=0"] { try statement(command) }
    guard let vfs=sqlite3_vfs_find(nil),vfs.pointee.iVersion>=3 else { throw StorageError.unsupportedSync }
    let info=SQLiteRuntimeInfo(version:String(cString:sqlite3_libversion()),sourceID:String(cString:sqlite3_sourceid()),vfs:String(cString:vfs.pointee.zName),vfsVersion:vfs.pointee.iVersion,journalMode:try text("PRAGMA journal_mode"),synchronous:try integer("PRAGMA synchronous"),foreignKeys:try integer("PRAGMA foreign_keys"),fullfsync:try integer("PRAGMA fullfsync"),checkpointFullfsync:try integer("PRAGMA checkpoint_fullfsync"))
    guard info.synchronous==2,info.foreignKeys==1,info.fullfsync==1,info.checkpointFullfsync==1 else { throw StorageError.unsupportedSync }
    return info
  }
  func transaction<T>(_ body:()throws->T) throws ->T {
    try statement("BEGIN IMMEDIATE")
    let value:T
    do { value=try body() }
    catch { try? statement("ROLLBACK");throw error }
    do { try statement("COMMIT") }
    catch {
      // Autocommit=true after a commit error means the transaction already ended; do not
      // certify rollback. Higher owners must inspect the original operation receipt.
      if let db,sqlite3_get_autocommit(db) != 0 { throw StorageError.unknownCommit(EIO) }
      try? statement("ROLLBACK");throw error
    }
    // Apple's system VFS currently uses F_BARRIERFSYNC, even with fullfsync=ON.
    // Explicitly wait for F_FULLFSYNC on the actual WAL before returning durable success.
    // A failure after COMMIT is an unknown acknowledgement, never a claimed rollback.
    do { try synchronizeCommittedBytes() }
    catch StorageError.system(let code,_) { throw StorageError.unknownCommit(code) }
    catch { throw StorageError.unknownCommit(EIO) }
    return value
  }
  private func synchronizeCommittedBytes() throws {
    let wal=URL(fileURLWithPath:url.path+"-wal")
    let target=try ManagedFiles.check(wal,directory:false) ? wal : url
    let fd=Darwin.open(target.path,O_RDWR|O_NOFOLLOW|O_CLOEXEC)
    guard fd>=0 else { throw StorageError.system(errno,"committed_bytes_open") }
    defer { Darwin.close(fd) }
    try ManagedFiles.sync(fd)
  }
  func checkpoint() throws {
    try statement("PRAGMA wal_checkpoint(PASSIVE)")
    let fd=Darwin.open(url.path,O_RDWR|O_NOFOLLOW|O_CLOEXEC)
    guard fd>=0 else { throw StorageError.system(errno,"checkpoint_open") }
    defer { Darwin.close(fd) }
    try ManagedFiles.sync(fd)
  }
}
