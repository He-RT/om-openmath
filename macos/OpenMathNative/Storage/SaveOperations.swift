import Foundation
import Darwin
import CryptoKit

struct OpenedNotebook:Sendable {let file:NativeSourceFile;let binding:FileBinding;let bytes:Data}
private final class ScopedFile {
  let url:URL
  private let active:Bool
  private let grant:URL
  init(binding:FileBinding)throws {
    var stale=false
    let parent:URL
    do {parent=try URL(resolvingBookmarkData:binding.bookmark,options:[.withSecurityScope,.withoutUI],relativeTo:nil,bookmarkDataIsStale:&stale)}
    catch {throw SaveError.permissionDenied}
    guard parent.isFileURL else {throw SaveError.unsafeTarget}
    grant=parent;active=parent.startAccessingSecurityScopedResource();url=parent.appendingPathComponent(binding.fileName)
  }
  deinit {if active {grant.stopAccessingSecurityScopedResource()}}
}
/// User-granted file IO, never a model-provided path. All callbacks run off MainActor.
enum SaveOperations {
  static func bookmark(_ url:URL)throws->Data {
    do {return try url.bookmarkData(options:.withSecurityScope,includingResourceValuesForKeys:[.fileResourceIdentifierKey,.volumeIdentifierKey],relativeTo:nil)}
    catch {throw SaveError.permissionDenied}
  }
  private static func statStamp(_ fd:Int32,_ data:Data)throws->FileStamp {
    var s=stat();guard fstat(fd,&s)==0,s.st_mode&S_IFMT==S_IFREG,s.st_size>=0 else {throw SaveError.unsafeTarget}
    return .init(device:UInt64(UInt32(bitPattern:s.st_dev)),inode:UInt64(s.st_ino),byteLength:UInt64(s.st_size),modifiedSeconds:Int64(s.st_mtimespec.tv_sec),modifiedNanoseconds:Int64(s.st_mtimespec.tv_nsec),changedSeconds:Int64(s.st_ctimespec.tv_sec),changedNanoseconds:Int64(s.st_ctimespec.tv_nsec),byteHash:KernelHashes.data(data))
  }
  private static func read(_ url:URL)throws->(Data,FileStamp) {
    let fd=Darwin.open(url.path,O_RDONLY|O_NONBLOCK|O_NOFOLLOW|O_CLOEXEC)
    guard fd>=0 else {if errno==ENOENT {throw SaveError.fileUnavailable};if errno==EACCES {throw SaveError.permissionDenied};throw SaveError.system(errno,"read_open")}
    defer {Darwin.close(fd)}
    var before=stat();guard fstat(fd,&before)==0,before.st_mode&S_IFMT==S_IFREG,before.st_size>=0,before.st_size<=NotebookFileCodec.maximumBytes else {throw SaveError.unsafeTarget}
    var bytes=Data(count:Int(before.st_size));try bytes.withUnsafeMutableBytes {buffer in
      var offset=0
      while offset<buffer.count {let n=Darwin.read(fd,buffer.baseAddress!.advanced(by:offset),buffer.count-offset);if n<0&&errno==EINTR {continue};guard n>0 else {throw SaveError.fileUnavailable};offset+=n}
    }
    let after=try statStamp(fd,bytes)
    guard after.device==UInt64(UInt32(bitPattern:before.st_dev)),after.inode==UInt64(before.st_ino),after.byteLength==UInt64(before.st_size),after.modifiedSeconds==Int64(before.st_mtimespec.tv_sec),after.modifiedNanoseconds==Int64(before.st_mtimespec.tv_nsec),after.changedSeconds==Int64(before.st_ctimespec.tv_sec),after.changedNanoseconds==Int64(before.st_ctimespec.tv_nsec) else {throw SaveError.externalConflict}
    return (bytes,after)
  }
  private static func existing(_ url:URL)throws->(Data,FileStamp)? {
    var s=stat();if lstat(url.path,&s)<0 {if errno==ENOENT {return nil};throw SaveError.system(errno,"target_lstat")}
    guard s.st_mode&S_IFMT==S_IFREG else {throw SaveError.unsafeTarget};return try read(url)
  }
  private static func coordinatedRead<T>(_ url:URL,_ body:(URL)throws->T)throws->T {
    precondition(!Thread.isMainThread)
    let coordinator=NSFileCoordinator(filePresenter:nil);var error:NSError?,result:Result<T,any Error>?
    coordinator.coordinate(readingItemAt:url,options:[],error:&error) {url in result=Result{try body(url)}}
    if let error {throw error};guard let result else {throw SaveError.fileUnavailable};return try result.get()
  }
  static func authorize(_ url:URL,document:String,revision:UInt64)throws->FileBinding {
    precondition(!Thread.isMainThread)
    guard url.isFileURL,url.pathExtension.lowercased()=="omnb" else {throw SaveError.unsafeTarget}
    let active=url.startAccessingSecurityScopedResource();defer {if active {url.stopAccessingSecurityScopedResource()}}
    // Authorization is a read-only stable stamp. The write phase later coordinates/rechecks the
    // exact target; selecting a different target must not wait behind another file replacement.
    let expected=try existing(url)?.1
    return .init(codecVersion:1,documentID:document,revision:revision,bookmark:try bookmark(url.deletingLastPathComponent()),fileName:url.lastPathComponent,displayPath:url.path,expected:expected)
  }
  private static func coordinatedConfirmation<T>(_ url:URL,_ body:(URL)throws->T)throws->T {
    precondition(!Thread.isMainThread)
    let coordinator=NSFileCoordinator(filePresenter:nil);var error:NSError?,result:Result<T,any Error>?
    coordinator.coordinate(writingItemAt:url,options:[],error:&error) {url in result=Result{try body(url)}}
    if let error {throw error};guard let result else {throw SaveError.fileUnavailable};return try result.get()
  }
  static func open(_ url:URL,document:String)throws->OpenedNotebook {
    precondition(!Thread.isMainThread)
    let active=url.startAccessingSecurityScopedResource();defer {if active {url.stopAccessingSecurityScopedResource()}}
    let (bytes,stamp)=try coordinatedRead(url,read)
    let file=try NotebookFileCodec.decode(bytes)
    let binding=FileBinding(codecVersion:1,documentID:document,revision:1,bookmark:try bookmark(url.deletingLastPathComponent()),fileName:url.lastPathComponent,displayPath:url.path,expected:stamp)
    return .init(file:file,binding:binding,bytes:bytes)
  }
  private static func requireExpected(_ url:URL,_ expected:FileStamp?)throws {
    let actual=try existing(url)?.1
    guard actual==expected else {throw SaveError.externalConflict}
    if actual != nil {var s=stat();guard lstat(url.path,&s)==0,s.st_mode&S_IWUSR != 0 else {throw SaveError.permissionDenied}}
  }
  private static func writeAll(_ data:Data,_ fd:Int32)throws {
    try data.withUnsafeBytes {buffer in var offset=0;while offset<buffer.count {let n=Darwin.write(fd,buffer.baseAddress!.advanced(by:offset),buffer.count-offset);if n<0&&errno==EINTR {continue};guard n>0 else {throw SaveError.system(errno,"write")};offset+=n}}
  }
  /// Full same-directory write and coordination; displaced original bytes remain available until
  /// they match the expected original. No provider with missing atomic/sync support is called saved.
  static func write(_ snapshot:SaveSnapshot,intent:SaveIntent,faults:StorageFaults = .init())throws->SaveReceipt {
    precondition(!Thread.isMainThread);try SaveValidation.intent(intent)
    guard snapshot.source.document_id==intent.documentID,snapshot.source.revision.value==intent.sourceRevision,snapshot.source.snapshot_hash==intent.sourceSnapshotHash,snapshot.fileHash==intent.fileHash,UInt64(snapshot.bytes.count)==intent.fileByteLength else {throw SaveError.sourceChanged}
    let scoped=try ScopedFile(binding:intent.binding),coordinator=NSFileCoordinator(filePresenter:nil)
    var error:NSError?,result:Result<SaveReceipt,any Error>?
    coordinator.coordinate(writingItemAt:scoped.url,options:.forReplacing,error:&error) {url in result=Result {
      try requireExpected(url,intent.binding.expected)
      let folder=url.deletingLastPathComponent(),temporary=folder.appendingPathComponent(".openmath-save-"+intent.operationID)
      let fd=Darwin.open(temporary.path,O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC,0o600)
      guard fd>=0 else {throw SaveError.system(errno,"temporary_open")};defer {Darwin.close(fd)}
      var preserve=false
      defer {if !preserve { _ = unlink(temporary.path) }}
      try faults.reach("save_temporary_opened");try writeAll(snapshot.bytes,fd);try ManagedFiles.sync(fd);try faults.reach("save_file_synced")
      try faults.reach("before_save_replace");try requireExpected(url,intent.binding.expected)
      let flags=UInt32(intent.binding.expected==nil ? RENAME_EXCL : RENAME_SWAP)
      guard renameatx_np(AT_FDCWD,temporary.path,AT_FDCWD,url.path,flags)==0 else {if errno==EEXIST {throw SaveError.externalConflict};throw SaveError.atomicReplaceUnavailable}
      preserve=true
      do {
        try faults.reach("save_replaced")
        if let old=intent.binding.expected {let displaced=try read(temporary);guard displaced.1.byteHash==old.byteHash,displaced.1.device==old.device,displaced.1.inode==old.inode else {throw SaveError.externalConflict}}
        try ManagedFiles.syncDirectory(folder);try faults.reach("save_directory_synced")
        let (bytes,stamp)=try read(url);guard bytes==snapshot.bytes,stamp.byteHash==intent.fileHash else {throw SaveError.externalConflict}
        let target=Darwin.open(url.path,O_RDWR|O_NOFOLLOW|O_CLOEXEC);guard target>=0 else {throw SaveError.permissionDenied};defer {Darwin.close(target)}
        try ManagedFiles.sync(target);try ManagedFiles.syncDirectory(folder);try faults.reach("save_file_readback")
        let bookmark=try Self.bookmark(url.deletingLastPathComponent())
        preserve=false
        let date=ISO8601DateFormatter();date.formatOptions=[.withInternetDateTime,.withFractionalSeconds]
        return .init(codecVersion:1,operationID:intent.operationID,documentID:intent.documentID,sourceRevision:intent.sourceRevision,sourceSnapshotHash:intent.sourceSnapshotHash,bindingRevision:intent.binding.revision,fileHash:intent.fileHash,actual:stamp,updatedBookmark:bookmark,recovered:false,completedAt:date.string(from:Date()))
      } catch SaveError.externalConflict {throw SaveError.externalConflict}
      catch {throw SaveError.unknownOutcome}
    }}
    if let error {throw error};guard let result else {throw SaveError.fileUnavailable};return try result.get()
  }
  /// Recover only from original intent and actual authorized target bytes. No rewrite or old-file
  /// fallback occurs; absence/offline/third-party bytes remain failure/unknown/conflict.
  static func reconcile(_ intent:SaveIntent)throws->SaveReceipt? {
    precondition(!Thread.isMainThread);try SaveValidation.intent(intent)
    let scoped=try ScopedFile(binding:intent.binding)
    return try coordinatedConfirmation(scoped.url) {url in
      guard let (bytes,stamp)=try existing(url) else {if intent.binding.expected==nil {return nil};throw SaveError.fileUnavailable}
      if KernelHashes.data(bytes)==intent.fileHash {
        let displaced=url.deletingLastPathComponent().appendingPathComponent(".openmath-save-"+intent.operationID)
        if let old=intent.binding.expected,(stamp.device != old.device || stamp.inode != old.inode),let backup=try existing(displaced) {
          guard backup.1.byteHash==old.byteHash,backup.1.device==old.device,backup.1.inode==old.inode else {throw SaveError.externalConflict}
        }
        let fd=Darwin.open(url.path,O_RDWR|O_NOFOLLOW|O_CLOEXEC);guard fd>=0 else {throw SaveError.permissionDenied};defer {Darwin.close(fd)}
        try ManagedFiles.sync(fd);try ManagedFiles.syncDirectory(url.deletingLastPathComponent())
        let (confirmed,current)=try read(url);guard confirmed==bytes,current==stamp else {throw SaveError.externalConflict}
        let date=ISO8601DateFormatter();date.formatOptions=[.withInternetDateTime,.withFractionalSeconds]
        return .init(codecVersion:1,operationID:intent.operationID,documentID:intent.documentID,sourceRevision:intent.sourceRevision,sourceSnapshotHash:intent.sourceSnapshotHash,bindingRevision:intent.binding.revision,fileHash:intent.fileHash,actual:stamp,updatedBookmark:try bookmark(url.deletingLastPathComponent()),recovered:true,completedAt:date.string(from:Date()))
      }
      if stamp==intent.binding.expected {return nil}
      throw SaveError.externalConflict
    }
  }
}
