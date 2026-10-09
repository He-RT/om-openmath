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
