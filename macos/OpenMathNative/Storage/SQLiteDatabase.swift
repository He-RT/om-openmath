import Foundation
import SQLite3
import Darwin

enum SQLValue:Sendable { case integer(Int64),text(String),blob(Data),null }
/// Used only for a sidecar-free readonly view under the root lease / owning serial queue.
/// SQLite must never ignore a real WAL or hot rollback journal. The main file and absence
/// of both sidecars are checked again before/after every query while this view is alive.
private struct ReadOnlyFileGuard {
  let url:URL,device:dev_t,inode:ino_t,size:off_t,modified:timespec,changed:timespec
  static func capture(_ url:URL)throws->Self? {
    for suffix in ["-wal","-journal"] {
      if try ManagedFiles.check(URL(fileURLWithPath:url.path+suffix),directory:false) {return nil}
    }
    var info=stat();guard lstat(url.path,&info)==0,info.st_mode & S_IFMT==S_IFREG else {throw StorageError.recoveryRequired}
    return .init(url:url,device:info.st_dev,inode:info.st_ino,size:info.st_size,modified:info.st_mtimespec,changed:info.st_ctimespec)
  }
  func validate()throws {
    guard let actual=try Self.capture(url),actual.device==device,actual.inode==inode,actual.size==size,
      actual.modified.tv_sec==modified.tv_sec,actual.modified.tv_nsec==modified.tv_nsec,
      actual.changed.tv_sec==changed.tv_sec,actual.changed.tv_nsec==changed.tv_nsec else {throw StorageError.recoveryRequired}
  }
}
/// Confined to its StoreWriter queue. Statements never escape and every buffer uses SQLite copy.
final class SQLiteDatabase {
  private var db:OpaquePointer?
  private let url:URL
  private let readonlyGuard:ReadOnlyFileGuard?
  init(url:URL,readonly:Bool=false,create:Bool=false) throws {
    self.url=url
    precondition(!Thread.isMainThread,"SQLite must remain on its storage queue")
    if !create { guard try ManagedFiles.check(url,directory:false) else { throw StorageError.recoveryRequired } }
    else { _ = try ManagedFiles.check(url,directory:false) }
    readonlyGuard=readonly ? try ReadOnlyFileGuard.capture(url) : nil
    var flags=(readonly ? SQLITE_OPEN_READONLY : SQLITE_OPEN_READWRITE)|(create ? SQLITE_OPEN_CREATE : 0)|SQLITE_OPEN_NOMUTEX|SQLITE_OPEN_NOFOLLOW
    var filename=url.path
    if readonlyGuard != nil {
      // SDK 27's readonly VFS opens a missing WAL without O_CREAT (actual ENOENT).
      // A serialized, sidecar-free main file is immutable for this short reader lifetime;
      // use SQLite's documented readonly URI mode without creating/modifying the source DB.
      guard var components=URLComponents(url:url.absoluteURL,resolvingAgainstBaseURL:false) else {throw StorageError.unsafePath}
      components.queryItems=[.init(name:"mode",value:"ro"),.init(name:"immutable",value:"1")]
      guard let encoded=components.url?.absoluteString else {throw StorageError.unsafePath}
      filename=encoded;flags |= SQLITE_OPEN_URI
    }
    let status=sqlite3_open_v2(filename,&db,flags,nil)
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
    try readonlyGuard?.validate()
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
      guard result==SQLITE_ROW else {
        #if OPENMATH_STORAGE_FIXTURE
        // Only explicitly compiled disposable fixture programs print SQL structure/OS errno;
        // production diagnostics never print paths or bound source/model/credential values.
        FileHandle.standardError.write(Data("SQLite fixture step failure code=\(result) errno=\(sqlite3_system_errno(db)) message=\(String(cString:sqlite3_errmsg(db))) sql=\(sql)\n".utf8))
        #endif
        throw StorageError.sqlite(result,"step")
      }
      try rows?(raw)
    }
    try readonlyGuard?.validate()
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
    try StorageQuota.require(url)
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
  func confirmCommittedBytes() throws {
    do { try synchronizeCommittedBytes() }
    catch { throw StorageError.unknownCommit(EIO) }
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
  /// Copy SQLite's committed view, including WAL, into a new standalone database. Both
  /// handles stay on this queue; errors never publish a partially copied snapshot.
  func backup(to destination:URL,faults:StorageFaults = .init()) throws {
    precondition(!Thread.isMainThread)
    guard let db else {throw StorageError.closing}
    guard !(try ManagedFiles.check(destination,directory:false)) else {throw StorageError.inUse}
    try readonlyGuard?.validate()
    let target=try SQLiteDatabase(url:destination,create:true)
    defer {try? target.close()}
    try target.statement("PRAGMA synchronous=FULL")
    try target.statement("PRAGMA fullfsync=ON")
    guard let handle=target.db,let copy=sqlite3_backup_init(handle,"main",db,"main") else {throw StorageError.sqlite(target.db.map(sqlite3_errcode) ?? SQLITE_ERROR,"backup_init")}
    var finished=false
    defer {if !finished {_ = sqlite3_backup_finish(copy)}}
    try faults.reach("backup.started")
    while true {
      let result=sqlite3_backup_step(copy,128)
      guard result==SQLITE_OK || result==SQLITE_DONE else {throw StorageError.sqlite(result,"backup_step")}
      try faults.reach("backup.pages")
      if result==SQLITE_DONE {break}
    }
    let result=sqlite3_backup_finish(copy);finished=true
    guard result==SQLITE_OK else {throw StorageError.sqlite(result,"backup_finish")}
    try faults.reach("backup.finished")
    guard try target.text("PRAGMA journal_mode=DELETE").lowercased()=="delete",
      try target.text("PRAGMA integrity_check")=="ok" else {throw StorageError.corruptIdentity}
    var broken=false;try target.statement("PRAGMA foreign_key_check") {_ in broken=true}
    guard !broken else {throw StorageError.corruptIdentity}
    try target.synchronizeCommittedBytes();try target.close()
    try ManagedFiles.syncDirectory(destination.deletingLastPathComponent())
    try readonlyGuard?.validate();try faults.reach("backup.synced")
  }
}
