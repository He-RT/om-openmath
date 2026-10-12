import Darwin
import Foundation

struct StorageSpace:Sendable {let available:UInt64,writeAllowed:Bool,largeWriteAllowed:Bool}
struct StorageUsage:Sendable {let totalBytes:UInt64,backupBytes:UInt64,derivedBytes:UInt64,space:StorageSpace;let overSoftBudget:Bool}
enum StorageQuota {
  static func space(_ url:URL)throws->StorageSpace {
    var target=url,info=statfs()
    while statfs(target.path,&info) != 0 {
      guard errno==ENOENT,target.path != "/" else {throw StorageError.system(errno,"storage_space")}
      target=target.deletingLastPathComponent()
    }
    let (available,overflow)=UInt64(info.f_bavail).multipliedReportingOverflow(by:UInt64(info.f_bsize))
    guard !overflow else {throw StorageError.corruptIdentity}
    return evaluate(available)
  }
  static func evaluate(_ available:UInt64)->StorageSpace {
    .init(available:available,writeAllowed:available>=256*1024*1024,largeWriteAllowed:available>=1024*1024*1024)
  }
  static func require(_ url:URL,bytes:UInt64=0,large:Bool=false)throws {
    let state=try space(url)
    let reserve:UInt64=large ? 1024*1024*1024 : 256*1024*1024
    guard state.available>=reserve,bytes<=state.available-reserve else {throw StorageError.lowDiskSpace}
  }
  static func usage(_ root:URL)throws->StorageUsage {
    var total:UInt64=0,backups:UInt64=0,derived:UInt64=0,count=0
    var failed=false
    guard let files=FileManager.default.enumerator(at:root,includingPropertiesForKeys:nil,errorHandler:{_,_ in failed=true;return false}) else {throw StorageError.recoveryRequired}
    while let url=files.nextObject() as? URL {
      count+=1;guard count<=1000000,try ManagedFiles.check(url) else {throw StorageError.recoveryRequired}
      var info=stat();guard lstat(url.path,&info)==0 else {throw StorageError.system(errno,"storage_usage")}
      if info.st_mode&S_IFMT==S_IFDIR {continue}
      guard info.st_mode&S_IFMT==S_IFREG,info.st_size>=0 else {throw StorageError.unsafePath}
      let bytes=UInt64(info.st_size);total+=bytes
      if url.path.hasPrefix(root.appendingPathComponent("Backups").path+"/") {backups+=bytes}
      if url.path.hasPrefix(root.appendingPathComponent("DerivedCache").path+"/") {derived+=bytes}
    }
    guard !failed else {throw StorageError.recoveryRequired}
    return .init(totalBytes:total,backupBytes:backups,derivedBytes:derived,space:try space(root),overSoftBudget:total>5*1024*1024*1024)
  }
}
