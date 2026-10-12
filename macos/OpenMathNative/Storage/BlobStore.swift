import CryptoKit
import Darwin
import Foundation

private struct BlobFileStamp:Equatable {
  let device:dev_t
  let inode:ino_t
  let size:off_t
  let modifiedSeconds:Int
  let modifiedNanos:Int
  let changedSeconds:Int
  let changedNanos:Int
  init(_ info:stat) { device=info.st_dev;inode=info.st_ino;size=info.st_size;modifiedSeconds=info.st_mtimespec.tv_sec;modifiedNanos=info.st_mtimespec.tv_nsec;changedSeconds=info.st_ctimespec.tv_sec;changedNanos=info.st_ctimespec.tv_nsec }
}
private final class BlobReader {
  let fd:Int32
  let descriptor:BlobDescriptor
  let stamp:BlobFileStamp
  init(fd:Int32,descriptor:BlobDescriptor,stamp:BlobFileStamp) {self.fd=fd;self.descriptor=descriptor;self.stamp=stamp}
  deinit { Darwin.close(fd) }
}
/// All mutation/read handles/pins stay on one utility queue. The root lease outlives queued work.
private final class BlobCore:@unchecked Sendable {
  static let maximum:UInt64=128*1024*1024
  let root:URL
  private let lease:RootLease
  private var pins:[String:BlobDescriptor]=[:]
  private var readers:[String:BlobReader]=[:]
  init(root:URL,lease:RootLease) {self.root=root;self.lease=lease}
  static func validHash(_ hash:String)->Bool {
    hash.utf8.count==64 && hash.utf8.allSatisfy{($0>=48 && $0<=57) || ($0>=97 && $0<=102)}
  }
  private func path(_ hash:String) throws ->URL {
    guard Self.validHash(hash) else { throw BlobError.invalidHash }
    return root.appendingPathComponent("Blobs/sha256",isDirectory:true).appendingPathComponent(String(hash.prefix(2)),isDirectory:true).appendingPathComponent(hash)
  }
  private func regular(_ fd:Int32) throws ->stat {
    var info=stat()
    guard fstat(fd,&info)==0 else { throw StorageError.system(errno,"blob_fstat") }
    guard info.st_mode & S_IFMT==S_IFREG,info.st_size>=0 else { throw StorageError.unsafePath }
    guard UInt64(info.st_size)<=Self.maximum else { throw BlobError.budgetExceeded }
    return info
  }
  private func readAll(_ fd:Int32,cancel:BlobCancellation,_ consume:(Data)throws->Void) throws ->UInt64 {
    guard lseek(fd,0,SEEK_SET)>=0 else { throw StorageError.system(errno,"blob_seek") }
    var total:UInt64=0,buffer=Data(count:64*1024)
    while true {
      try cancel.check()
      let count=buffer.withUnsafeMutableBytes { Darwin.read(fd,$0.baseAddress,$0.count) }
      if count<0 && errno==EINTR { continue }
      guard count>=0 else { throw StorageError.system(errno,"blob_read") }
      if count==0 { break }
      total+=UInt64(count);guard total<=Self.maximum else { throw BlobError.budgetExceeded }
      try consume(buffer.prefix(count))
    }
    return total
  }
  private func writeAll(_ fd:Int32,_ data:Data) throws {
    try data.withUnsafeBytes { bytes in
      var offset=0
      while offset<bytes.count {
        let n=Darwin.write(fd,bytes.baseAddress!.advanced(by:offset),bytes.count-offset)
        if n<0 && errno==EINTR { continue }
        guard n>0 else { throw StorageError.system(errno,"blob_write") }
        offset+=n
      }
    }
  }
  func publish(data:Data,expectedHash:String?,cancel:BlobCancellation,faults:StorageFaults) throws ->BlobPublication {
    guard data.count<=Self.maximum else { throw BlobError.budgetExceeded }
    try StorageQuota.require(root,bytes:UInt64(data.count),large:data.count>=8*1024*1024)
    return try staged(expectedHash:expectedHash,cancel:cancel,faults:faults) { fd,digest in
      for offset in stride(from:0,to:data.count,by:64*1024) {
        try cancel.check();let chunk=data.subdata(in:offset..<min(offset+64*1024,data.count))
        try writeAll(fd,chunk);digest.update(data:chunk)
      }
      return UInt64(data.count)
    }
  }
  func publish(file:URL,expectedHash:String?,cancel:BlobCancellation,faults:StorageFaults) throws ->BlobPublication {
    guard file.isFileURL else { throw StorageError.unsafePath }
    // The host owns file-picker/security-scope authorization; a model never supplies this URL.
    let input=Darwin.open(file.path,O_RDONLY|O_NONBLOCK|O_NOFOLLOW|O_CLOEXEC)
    guard input>=0 else { throw StorageError.system(errno,"blob_input_open") }
    defer { Darwin.close(input) }
    let before=BlobFileStamp(try regular(input))
    try StorageQuota.require(root,bytes:UInt64(before.size),large:before.size>=8*1024*1024)
    return try staged(expectedHash:expectedHash,cancel:cancel,faults:faults) { output,digest in
      let count=try readAll(input,cancel:cancel) { chunk in try writeAll(output,chunk);digest.update(data:chunk) }
      guard before==BlobFileStamp(try regular(input)),count==UInt64(before.size) else { throw BlobError.sourceChanged }
      return count
    }
  }
  private func staged(expectedHash:String?,cancel:BlobCancellation,faults:StorageFaults,_ copy:(Int32,inout SHA256)throws->UInt64) throws ->BlobPublication {
    if let expectedHash { guard Self.validHash(expectedHash) else { throw BlobError.invalidHash } }
    guard pins.count<128 else { throw BlobError.budgetExceeded }
    let staging=root.appendingPathComponent("Staging",isDirectory:true)
    try ManagedFiles.directory(staging)
    let temporary=staging.appendingPathComponent("blob-"+UUID().uuidString.lowercased())
    let fd=Darwin.open(temporary.path,O_RDWR|O_CREAT|O_EXCL|O_NOFOLLOW_ANY|O_CLOEXEC,0o600)
    guard fd>=0 else { throw StorageError.system(errno,"blob_stage_open") }
    defer { Darwin.close(fd);_ = unlink(temporary.path) }
    try faults.reach("blob_stage_opened")
    var digest=SHA256()
    let length=try copy(fd,&digest),hash=digest.finalize().map{String(format:"%02x",$0)}.joined()
    try faults.reach("blob_copied")
    if let expectedHash,expectedHash != hash { throw BlobError.corrupt }
    try cancel.check();try ManagedFiles.sync(fd);try faults.reach("blob_file_synced")
    let target=try path(hash)
    try ManagedFiles.directory(target.deletingLastPathComponent())
    try faults.reach("before_blob_publish");try cancel.check()
    // link is an atomic no-overwrite publication on the same managed local filesystem.
    // A crash may leave a staging hard link; it never leaves a target with partial bytes.
    if link(temporary.path,target.path) != 0 {
      guard errno==EEXIST else { throw StorageError.system(errno,"blob_publish_link") }
      let verified=try openVerified(.init(hash:hash,byteLength:length),cancel:cancel)
      Darwin.close(verified.fd)
    }
    try faults.reach("blob_linked")
    try ManagedFiles.syncDirectory(target.deletingLastPathComponent())
    try faults.reach("blob_directory_synced");try cancel.check()
    let descriptor=BlobDescriptor(hash:hash,byteLength:length)
    // Actual bytes and successful publication are required before issuing a pin/ready descriptor.
    let verified=try openVerified(descriptor,cancel:cancel)
    Darwin.close(verified.fd)
    let pin=UUID().uuidString.lowercased();pins[pin]=descriptor
    return .init(descriptor:descriptor,pinID:pin)
  }
  private func openVerified(_ descriptor:BlobDescriptor,cancel:BlobCancellation) throws ->(fd:Int32,stamp:BlobFileStamp) {
    guard descriptor.byteLength<=Self.maximum else { throw BlobError.budgetExceeded }
    let target=try path(descriptor.hash)
    guard try ManagedFiles.check(target,directory:false) else { throw BlobError.missing }
    let fd=Darwin.open(target.path,O_RDONLY|O_NONBLOCK|O_NOFOLLOW_ANY|O_CLOEXEC)
    guard fd>=0 else { throw StorageError.system(errno,"blob_open") }
    do {
      let stamp=BlobFileStamp(try regular(fd))
      guard UInt64(stamp.size)==descriptor.byteLength else { throw BlobError.corrupt }
      var digest=SHA256()
      let bytes=try readAll(fd,cancel:cancel) {digest.update(data:$0)}
      guard bytes==descriptor.byteLength,descriptor.hash==digest.finalize().map({String(format:"%02x",$0)}).joined(),stamp==BlobFileStamp(try regular(fd)) else { throw BlobError.corrupt }
      // Reconfirm bytes/parent publication after crash or a lost acknowledgement.
      try ManagedFiles.sync(fd);try ManagedFiles.syncDirectory(target.deletingLastPathComponent())
      return (fd,stamp)
    } catch { Darwin.close(fd);throw error }
  }
  func retain(_ publication:BlobPublication,cancel:BlobCancellation) throws ->BlobPublication {
    guard pins[publication.pinID]==publication.descriptor else { throw BlobError.invalidPin }
    guard pins.count<128 else { throw BlobError.budgetExceeded }
    let file=try openVerified(publication.descriptor,cancel:cancel);Darwin.close(file.fd)
    let id=UUID().uuidString.lowercased();pins[id]=publication.descriptor
    return .init(descriptor:publication.descriptor,pinID:id)
  }
  func release(_ publication:BlobPublication) throws {
    guard pins[publication.pinID]==publication.descriptor else { throw BlobError.invalidPin }
    pins.removeValue(forKey:publication.pinID)
  }
  func reader(_ publication:BlobPublication,cancel:BlobCancellation) throws ->BlobReadLease {
    guard pins[publication.pinID]==publication.descriptor else { throw BlobError.invalidPin }
    guard readers.count<64 else { throw BlobError.budgetExceeded }
    let file=try openVerified(publication.descriptor,cancel:cancel),id=UUID().uuidString.lowercased()
    readers[id]=BlobReader(fd:file.fd,descriptor:publication.descriptor,stamp:file.stamp)
    return .init(descriptor:publication.descriptor,readerID:id)
  }
  func referencedReader(_ descriptor:BlobDescriptor,cancel:BlobCancellation) throws ->BlobReadLease {
    guard readers.count<64 else {throw BlobError.budgetExceeded}
    let file=try openVerified(descriptor,cancel:cancel),id=UUID().uuidString.lowercased()
    readers[id]=BlobReader(fd:file.fd,descriptor:descriptor,stamp:file.stamp)
    return .init(descriptor:descriptor,readerID:id)
  }
  func chunk(_ lease:BlobReadLease,offset:UInt64,count:Int,cancel:BlobCancellation) throws ->Data {
    guard let reader=readers[lease.readerID],reader.descriptor==lease.descriptor else { throw BlobError.invalidReader }
    guard count>=0,count<=1024*1024,offset<=reader.descriptor.byteLength else { throw BlobError.budgetExceeded }
    try cancel.check()
    let size=min(count,Int(reader.descriptor.byteLength-offset))
    guard reader.stamp==BlobFileStamp(try regular(reader.fd)) else { throw BlobError.corrupt }
    var bytes=Data(count:size),position=0
    try bytes.withUnsafeMutableBytes { buffer in
      while position<size {
        let n=pread(reader.fd,buffer.baseAddress!.advanced(by:position),size-position,off_t(offset)+off_t(position))
        if n<0 && errno==EINTR { continue }
        guard n>0 else { throw BlobError.corrupt }
        position+=n;try cancel.check()
      }
    }
    guard reader.stamp==BlobFileStamp(try regular(reader.fd)) else { throw BlobError.corrupt }
    return bytes
  }
  func closeReader(_ lease:BlobReadLease) throws {
    guard readers[lease.readerID]?.descriptor==lease.descriptor else { throw BlobError.invalidReader }
    readers.removeValue(forKey:lease.readerID)
  }
  func inspect(_ descriptor:BlobDescriptor,references:Int,allStoresChecked:Bool,cancel:BlobCancellation) throws ->BlobInspection {
    let pinCount=pins.values.filter{$0==descriptor}.count+readers.values.filter{$0.descriptor==descriptor}.count
    do {
      let file=try openVerified(descriptor,cancel:cancel);Darwin.close(file.fd)
      return .init(state:pinCount>0 ? .pinned : (references>0 ? .referenced : .orphanCandidate),pinCount:pinCount,referenceCount:references,allStoresChecked:allStoresChecked)
    } catch BlobError.missing {return .init(state:.missing,pinCount:pinCount,referenceCount:references,allStoresChecked:allStoresChecked)}
    catch BlobError.corrupt {return .init(state:.corrupt,pinCount:pinCount,referenceCount:references,allStoresChecked:allStoresChecked)}
  }
  func close() {pins.removeAll();readers.removeAll()}
  func copyBackup(_ descriptors:[BlobDescriptor],folder:URL,cancel:BlobCancellation,faults:StorageFaults)throws {
    guard descriptors.count<=100000 else {throw BlobError.budgetExceeded}
    if descriptors.isEmpty {return}
    try ManagedFiles.directory(folder)
    for descriptor in descriptors {
      try cancel.check();try StorageQuota.require(folder,bytes:descriptor.byteLength,large:descriptor.byteLength>=8*1024*1024)
      let file=try openVerified(descriptor,cancel:cancel),pin=UUID().uuidString.lowercased()
      pins[pin]=descriptor
      defer {Darwin.close(file.fd);pins.removeValue(forKey:pin)}
      let target=folder.appendingPathComponent(descriptor.hash)
      _ = try ManagedFiles.check(target,directory:false)
      let fd=open(target.path,O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC,0o600)
      guard fd>=0 else {throw StorageError.system(errno,"backup_blob_open")}
      defer {Darwin.close(fd)}
      let count=try readAll(file.fd,cancel:cancel) {try self.writeAll(fd,$0)}
      guard count==descriptor.byteLength,file.stamp==BlobFileStamp(try regular(file.fd)) else {throw BlobError.sourceChanged}
      try ManagedFiles.sync(fd);try ManagedFiles.syncDirectory(folder);try faults.reach("backup.blob_synced")
    }
  }
  func collect(_ plan:BlobCollectionPlan,cancel:BlobCancellation,faults:StorageFaults)throws->BlobCollection {
    let base=root.appendingPathComponent("Blobs/sha256"),quarantine=root.appendingPathComponent("Blobs/Quarantine")
    // Verify all original-store references before any deletion. Missing/corrupt referenced
    // bytes indicate incomplete recovery, not permission to collect other original resources.
    for descriptor in plan.references {
      let canonical=try path(descriptor.hash)
      if !(try ManagedFiles.check(canonical,directory:false)) {
        let staged=quarantine.appendingPathComponent(descriptor.hash)
        guard try BackupService.digest(staged)==descriptor else {throw BlobError.corrupt}
        try ManagedFiles.directory(canonical.deletingLastPathComponent())
        guard rename(staged.path,canonical.path)==0 else {throw StorageError.system(errno,"gc_recover_reference")}
        try ManagedFiles.syncDirectory(canonical.deletingLastPathComponent());try ManagedFiles.syncDirectory(quarantine)
      }
      let verified=try openVerified(descriptor,cancel:cancel);Darwin.close(verified.fd)
    }
    let protected=plan.hashes.union(pins.values.map{$0.hash}).union(readers.values.map{$0.descriptor.hash})
    let pinned=Set(pins.values.map{$0.hash}+readers.values.map{$0.descriptor.hash})
    var candidates:[(URL,String,stat)]=[]
    if try ManagedFiles.check(base,directory:true) {
      let prefixes=try FileManager.default.contentsOfDirectory(atPath:base.path)
      guard prefixes.count<=256 else {throw StorageError.corruptIdentity}
      for prefix in prefixes.sorted() {
        guard prefix.count==2,Self.validHash(prefix+String(repeating:"0",count:62)) else {throw StorageError.corruptIdentity}
        let folder=base.appendingPathComponent(prefix)
        guard try ManagedFiles.check(folder,directory:true) else {throw StorageError.recoveryRequired}
        for hash in try FileManager.default.contentsOfDirectory(atPath:folder.path).sorted() {
          guard Self.validHash(hash),hash.hasPrefix(prefix) else {throw StorageError.corruptIdentity}
          let url=folder.appendingPathComponent(hash)
          guard try ManagedFiles.check(url,directory:false) else {throw StorageError.recoveryRequired}
          var info=stat();guard lstat(url.path,&info)==0,info.st_mode&S_IFMT==S_IFREG,info.st_size>=0,UInt64(info.st_size)<=Self.maximum else {throw StorageError.unsafePath}
          candidates.append((url,hash,info));guard candidates.count<=100000 else {throw BlobError.budgetExceeded}
        }
      }
    }
    // An interrupted sweep is resumed conservatively. A newly referenced hash is moved back
    // if its canonical file is absent; a quarantine path is never treated as a ready upload.
    if try ManagedFiles.check(quarantine,directory:true) {
      for hash in try FileManager.default.contentsOfDirectory(atPath:quarantine.path).sorted() {
        guard Self.validHash(hash) else {throw StorageError.recoveryRequired}
        let url=quarantine.appendingPathComponent(hash)
        guard try ManagedFiles.check(url,directory:false) else {throw StorageError.recoveryRequired}
        var info=stat();guard lstat(url.path,&info)==0,info.st_mode&S_IFMT==S_IFREG,info.st_size>=0,UInt64(info.st_size)<=Self.maximum else {throw StorageError.unsafePath}
        candidates.append((url,hash,info));guard candidates.count<=100000 else {throw BlobError.budgetExceeded}
      }
    }
    var removed=0,bytes:UInt64=0,kept=0,young=0,pinCount=0
    for (url,hash,info) in candidates {
      try cancel.check()
      if protected.contains(hash) {
        if pinned.contains(hash) {pinCount+=1} else {kept+=1}
        if url.deletingLastPathComponent()==quarantine {
          let canonical=try path(hash)
          if !(try ManagedFiles.check(canonical,directory:false)) {
            guard try BackupService.digest(url)==BlobDescriptor(hash:hash,byteLength:UInt64(info.st_size)) else {throw BlobError.corrupt}
            try ManagedFiles.directory(canonical.deletingLastPathComponent())
            guard rename(url.path,canonical.path)==0 else {throw StorageError.system(errno,"gc_restore")}
            try ManagedFiles.syncDirectory(canonical.deletingLastPathComponent());try ManagedFiles.syncDirectory(quarantine)
          }
        };continue
      }
      guard plan.createdAt>=Int64(info.st_mtimespec.tv_sec),plan.createdAt-Int64(info.st_mtimespec.tv_sec)>=24*60*60 else {young+=1;continue}
      let descriptor=BlobDescriptor(hash:hash,byteLength:UInt64(info.st_size))
      // Corrupt originals stay for explicit recovery, even if no reference was found.
      guard try BackupService.digest(url)==descriptor else {throw BlobError.corrupt}
      try ManagedFiles.directory(quarantine)
      let target=quarantine.appendingPathComponent(hash)
      if url != target {
        guard !(try ManagedFiles.check(target,directory:false)) else {throw StorageError.recoveryRequired}
        guard rename(url.path,target.path)==0 else {throw StorageError.system(errno,"gc_quarantine")}
        try ManagedFiles.syncDirectory(url.deletingLastPathComponent());try ManagedFiles.syncDirectory(quarantine)
      }
      try faults.reach("gc.quarantined");try cancel.check()
      guard !plan.hashes.contains(hash),!pins.values.contains(where:{$0.hash==hash}),!readers.values.contains(where:{$0.descriptor.hash==hash}) else {throw StorageError.transactionUnavailable}
      try faults.reach("gc.before_delete")
      guard unlink(target.path)==0 else {throw StorageError.system(errno,"gc_delete")}
      try ManagedFiles.syncDirectory(quarantine);removed+=1;bytes+=descriptor.byteLength
      try faults.reach("gc.deleted")
    }
    let staging=root.appendingPathComponent("Staging")
    if try ManagedFiles.check(staging,directory:true) {
      let names=try FileManager.default.contentsOfDirectory(atPath:staging.path)
      guard names.count<=100000 else {throw BlobError.budgetExceeded}
      var stale:[URL]=[]
      for name in names {
        guard name.hasPrefix("blob-"),UUID(uuidString:String(name.dropFirst(5))) != nil else {throw StorageError.recoveryRequired}
        let url=staging.appendingPathComponent(name);guard try ManagedFiles.check(url,directory:false) else {throw StorageError.recoveryRequired}
        var info=stat();guard lstat(url.path,&info)==0,info.st_mode&S_IFMT==S_IFREG,info.st_size>=0,UInt64(info.st_size)<=Self.maximum else {throw StorageError.unsafePath}
        if plan.createdAt>=Int64(info.st_mtimespec.tv_sec),plan.createdAt-Int64(info.st_mtimespec.tv_sec)>=24*60*60 {stale.append(url)}
      }
      // The maintenance barrier has drained every upload before this serial queue was entered.
      // Publication pins/readers point at canonical files, not these unselected staging paths.
      for url in stale {try cancel.check();guard unlink(url.path)==0 else {throw StorageError.system(errno,"gc_staging_delete")}}
      if !stale.isEmpty {try ManagedFiles.syncDirectory(staging)}
    }
    return .init(scanned:candidates.count,removed:removed,removedBytes:bytes,pinned:pinCount,referenced:kept,young:young)
  }
}

/// Bounded serial IO separate from SQLite and CAS. Direct cancellation only changes a per-job flag.
final class BlobStore:@unchecked Sendable {
  private let queue=DispatchQueue(label:"org.openmath.storage.blobs",qos:.utility)
  private let core:BlobCore
  private let admission=NSLock()
  private var jobs:[UUID:BlobCancellation]=[:]
  private var closing=false
  init(root:URL,lease:RootLease) {core=BlobCore(root:root,lease:lease)}
  private nonisolated(nonsending) func perform<T:Sendable>(_ work:@escaping @Sendable (BlobCore,BlobCancellation)throws->T) async throws ->T {
    let token=BlobCancellation(),id=UUID()
    return try await withTaskCancellationHandler {
      try Task.checkCancellation()
      return try await withCheckedThrowingContinuation { continuation in
        admission.lock()
        guard !closing else {admission.unlock();continuation.resume(throwing:BlobError.closing);return}
        guard jobs.count<32 else {admission.unlock();continuation.resume(throwing:BlobError.budgetExceeded);return}
        jobs[id]=token
        queue.async { [self] in
          precondition(!Thread.isMainThread)
          do { try token.check();continuation.resume(returning:try work(core,token)) }
          catch {continuation.resume(throwing:error)}
          _ = admission.withLock {jobs.removeValue(forKey:id)}
        }
        admission.unlock()
      }
    } onCancel: {token.cancel()}
  }
  nonisolated(nonsending) func publish(data:Data,expectedHash:String?=nil,faults:StorageFaults = .init()) async throws ->BlobPublication {try await perform{try $0.publish(data:data,expectedHash:expectedHash,cancel:$1,faults:faults)}}
  nonisolated(nonsending) func publish(file:URL,expectedHash:String?=nil,faults:StorageFaults = .init()) async throws ->BlobPublication {try await perform{try $0.publish(file:file,expectedHash:expectedHash,cancel:$1,faults:faults)}}
  nonisolated(nonsending) func retain(_ publication:BlobPublication) async throws ->BlobPublication {try await perform{try $0.retain(publication,cancel:$1)}}
  nonisolated(nonsending) func release(_ publication:BlobPublication) async throws {try await perform{core,_ in try core.release(publication)}}
  nonisolated(nonsending) func openReader(_ publication:BlobPublication) async throws ->BlobReadLease {try await perform{try $0.reader(publication,cancel:$1)}}
  nonisolated(nonsending) func openReference(_ descriptor:BlobDescriptor) async throws ->BlobReadLease {try await perform{try $0.referencedReader(descriptor,cancel:$1)}}
  nonisolated(nonsending) func read(_ lease:BlobReadLease,offset:UInt64,count:Int) async throws ->Data {try await perform{try $0.chunk(lease,offset:offset,count:count,cancel:$1)}}
  nonisolated(nonsending) func closeReader(_ lease:BlobReadLease) async throws {try await perform{core,_ in try core.closeReader(lease)}}
  nonisolated(nonsending) func inspect(_ descriptor:BlobDescriptor,references:Int,allStoresChecked:Bool) async throws ->BlobInspection {try await perform{try $0.inspect(descriptor,references:references,allStoresChecked:allStoresChecked,cancel:$1)}}
  nonisolated(nonsending) func copyBackup(_ descriptors:[BlobDescriptor],folder:URL,faults:StorageFaults) async throws {try await perform{try $0.copyBackup(descriptors,folder:folder,cancel:$1,faults:faults)}}
  nonisolated(nonsending) func collect(_ plan:BlobCollectionPlan,faults:StorageFaults) async throws->BlobCollection {try await perform{try $0.collect(plan,cancel:$1,faults:faults)}}
  func beginClose() {
    admission.withLock {closing=true;for token in jobs.values {token.cancel()}}
  }
  nonisolated(nonsending) func close() async {
    beginClose()
    await withCheckedContinuation { continuation in
      admission.lock();closing=true
      for token in jobs.values {token.cancel()}
      queue.async { [self] in core.close();continuation.resume() }
      admission.unlock()
    }
  }
  deinit { let final=core;queue.async {final.close()} }
}
