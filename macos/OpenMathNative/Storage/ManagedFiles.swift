import Darwin
import Foundation

/// POSIX operations only on host-derived managed paths; never on paths chosen by a model.
enum ManagedFiles {
  static func check(_ url:URL,directory:Bool?=nil) throws ->Bool {
    var ancestor=""
    for component in url.standardizedFileURL.pathComponents.dropLast() {
      ancestor=component=="/" ? "/" : (ancestor as NSString).appendingPathComponent(component)
      var parent=stat()
      if lstat(ancestor,&parent)==0 { guard parent.st_mode & S_IFMT != S_IFLNK else { throw StorageError.unsafePath } }
      else if errno != ENOENT { throw StorageError.system(errno,"ancestor") }
    }
    var info=stat()
    if lstat(url.path,&info) != 0 {
      if errno==ENOENT { return false }
      throw StorageError.system(errno,"lstat")
    }
    guard info.st_uid==getuid(),info.st_mode & S_IFMT != S_IFLNK else { throw StorageError.unsafePath }
    if let directory { guard (info.st_mode & S_IFMT==S_IFDIR)==directory else { throw StorageError.unsafePath } }
    return true
  }
  static func directory(_ url:URL) throws {
    if try check(url,directory:true) {
      var info=stat();guard stat(url.path,&info)==0,info.st_mode & S_IWUSR != 0,
        info.st_mode & S_IXUSR != 0 else { throw StorageError.inaccessible }
      return
    }
    if !(try check(url.deletingLastPathComponent(),directory:true)) { try directory(url.deletingLastPathComponent()) }
    guard mkdir(url.path,0o700)==0 else { throw StorageError.system(errno,"mkdir") }
    try syncDirectory(url.deletingLastPathComponent())
  }
  static func validateLocal(_ root:URL) throws {
    guard root.isFileURL else { throw StorageError.unsafePath }
    var filesystem=statfs()
    guard statfs(root.path,&filesystem)==0 else { throw StorageError.system(errno,"statfs") }
    guard filesystem.f_flags & UInt32(MNT_LOCAL) != 0 else { throw StorageError.nonlocalVolume }
    if (try root.resourceValues(forKeys:[.isUbiquitousItemKey])).isUbiquitousItem==true { throw StorageError.nonlocalVolume }
  }
  static func readJSON<T:Decodable>(_ type:T.Type,_ url:URL,keys:Set<String>) throws ->T {
    guard try check(url,directory:false) else { throw StorageError.recoveryRequired }
    let fd=open(url.path,O_RDONLY|O_NOFOLLOW|O_CLOEXEC)
    guard fd>=0 else { throw StorageError.system(errno,"open_read") }
    defer { Darwin.close(fd) }
    var info=stat();guard fstat(fd,&info)==0,info.st_size>0,info.st_size<=65536 else { throw StorageError.corruptIdentity }
    var data=Data(count:Int(info.st_size))
    try data.withUnsafeMutableBytes { raw in
      var offset=0
      while offset<raw.count {
        let count=Darwin.read(fd,raw.baseAddress!.advanced(by:offset),raw.count-offset)
        if count<0 && errno==EINTR { continue }
        guard count>0 else { throw StorageError.corruptIdentity }
        offset+=count
      }
    }
    guard let object=try JSONSerialization.jsonObject(with:data) as? [String:Any],Set(object.keys)==keys else { throw StorageError.corruptIdentity }
    return try JSONDecoder().decode(type,from:data)
  }
  static func sync(_ fd:Int32) throws {
    guard fcntl(fd,F_FULLFSYNC)==0 else { throw StorageError.system(errno,"fullfsync") }
  }
  static func syncDirectory(_ url:URL) throws {
    let fd=open(url.path,O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC)
    guard fd>=0 else { throw StorageError.system(errno,"open_directory") }
    defer { Darwin.close(fd) }
    guard fsync(fd)==0 else { throw StorageError.system(errno,"directory_sync") }
  }
  static func publish<T:Encodable>(_ value:T,to url:URL,faults:StorageFaults) throws {
    let encoder=JSONEncoder();encoder.outputFormatting=[.sortedKeys,.withoutEscapingSlashes]
    let data=try encoder.encode(value)
    let temporary=url.deletingLastPathComponent().appendingPathComponent(".pending-"+UUID().uuidString)
    let fd=open(temporary.path,O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC,0o600)
    guard fd>=0 else { throw StorageError.system(errno,"open_temporary") }
    defer { Darwin.close(fd) }
    try faults.reach("before_file_write")
    var offset=0
    try data.withUnsafeBytes { raw in
      while offset<data.count {
        let count=Darwin.write(fd,raw.baseAddress!.advanced(by:offset),raw.count-offset)
        if count<0 && errno==EINTR { continue }
        guard count>0 else { throw StorageError.system(errno,"file_write") }
        offset+=count
      }
    }
    try sync(fd);try faults.reach("file_synced")
    guard rename(temporary.path,url.path)==0 else { throw StorageError.system(errno,"publish_rename") }
    try faults.reach("renamed");try syncDirectory(url.deletingLastPathComponent())
    try faults.reach("directory_synced")
  }
}
/// OS lease, not a PID-file heuristic. The empty lock file may persist after process exit.
final class RootLease:@unchecked Sendable {
  private let fd:Int32
  init(_ root:URL) throws {
    let path=root.appendingPathComponent("store.lock")
    _ = try ManagedFiles.check(path,directory:false)
    fd=open(path.path,O_RDWR|O_CREAT|O_NOFOLLOW|O_CLOEXEC,0o600)
    guard fd>=0 else { throw StorageError.system(errno,"open_lock") }
    guard flock(fd,LOCK_EX|LOCK_NB)==0 else {
      let error=errno;Darwin.close(fd)
      if error==EWOULDBLOCK { throw StorageError.inUse }
      throw StorageError.system(error,"lock")
    }
  }
  deinit { _ = flock(fd,LOCK_UN);Darwin.close(fd) }
}
