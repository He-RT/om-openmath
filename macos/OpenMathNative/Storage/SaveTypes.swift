import Foundation
import CryptoKit

enum SaveError:Error,Sendable,Equatable {
  case invalidNotebook,unsupportedNotebookVersion,permissionDenied,fileUnavailable,externalConflict
  case unsafeTarget,atomicReplaceUnavailable,unknownOutcome,bindingChanged,busy,sourceChanged,closing
  case system(Int32,String)
}
extension SaveError:LocalizedError {
  var errorDescription:String? {
    switch self {
    case .invalidNotebook:return "文件不是有效的 OpenMath 笔记本。"
    case .unsupportedNotebookVersion:return "此笔记本格式版本尚不支持，请保留原文件。"
    case .permissionDenied:return "无法访问或写入文件，请检查权限或另存为。"
    case .fileUnavailable:return "文件已移动、离线或不可用，请重新选择原文件。"
    case .externalConflict:return "文件已被其他应用修改，未覆盖外部修改；请核对或另存为。"
    case .unsafeTarget:return "文件目标不是可保存的普通 .omnb 文件。"
    case .atomicReplaceUnavailable:return "此文件位置不支持可靠替换，请另存到本地磁盘。"
    case .unknownOutcome:return "保存结果待核对，请先核对原操作，当前修改已保留。"
    case .bindingChanged:return "保存位置已改变，旧操作不能确认当前文件已保存。"
    case .busy:return "已有文件操作正在进行，请等待完成。"
    case .sourceChanged:return "源码快照已改变，当前修改已保留。"
    case .closing:return "文档正在关闭。"
    case .system(let code,let stage):return "文件操作失败（\(stage)，系统错误 \(code)），当前修改已保留。"
    }
  }
}
struct FileStamp:Codable,Sendable,Equatable {
  let device:UInt64
  let inode:UInt64
  let byteLength:UInt64
  let modifiedSeconds:Int64
  let modifiedNanoseconds:Int64
  let changedSeconds:Int64
  let changedNanoseconds:Int64
  let byteHash:String
}
/// Native-only security scope. Paths/bookmarks never enter .omnb or model tool arguments.
struct FileBinding:Codable,Sendable {
  let codecVersion:Int
  let documentID:String
  let revision:UInt64
  let bookmark:Data
  let fileName:String
  let displayPath:String
  let expected:FileStamp?
}
/// Actual source-only bytes already frozen; writer never reads a mutable ViewModel during saving.
struct SaveSnapshot:Sendable {
  let source:NativeSourceSnapshot
  let bytes:Data
  let fileHash:String
  init(source:NativeSourceSnapshot) throws {
    try SourceValidation.snapshot(source)
    self.source=source;self.bytes=try NotebookFileCodec.encode(source.file);self.fileHash=KernelHashes.data(bytes)
  }
}
struct SaveIntent:Codable,Sendable {
  let codecVersion:Int
  let operationID:String
  let documentID:String
  let storeID:String
  let sourceRevision:UInt64
  let sourceSnapshotHash:String
  let binding:FileBinding
  let fileHash:String
  let fileByteLength:UInt64
}
struct SaveReceipt:Codable,Sendable {
  let codecVersion:Int
  let operationID:String
  let documentID:String
  let sourceRevision:UInt64
  let sourceSnapshotHash:String
  let bindingRevision:UInt64
  let fileHash:String
  let actual:FileStamp
  let updatedBookmark:Data
  let recovered:Bool
  let completedAt:String
}
struct SavedFileHead:Codable,Sendable {
  let binding:FileBinding
  let savedRevision:UInt64?
  let savedSnapshotHash:String?
  let lastSaveOperation:String?
}
enum NotebookFileCodec {
  static let maximumBytes=16*1024*1024
  static func decode(_ data:Data) throws->NativeSourceFile {
    guard !data.isEmpty,data.count<=maximumBytes,
      let raw=try JSONSerialization.jsonObject(with:data) as? [String:Any],let version=raw["version"] as? Int else {throw SaveError.invalidNotebook}
    guard version==1 else {throw SaveError.unsupportedNotebookVersion}
    let file:NativeSourceFile
    do {file=try JSONDecoder().decode(NativeSourceFile.self,from:data)} catch {throw SaveError.invalidNotebook}
    try validate(file);return file
  }
  static func validate(_ file:NativeSourceFile)throws {
    guard file.version==1,file.title.utf8.count<=16384,file.cells.count<=10000,
      Set(file.cells.map{Data($0.id.utf8)}).count==file.cells.count,
      file.cells.allSatisfy({!$0.id.isEmpty&&$0.id.utf8.count<=256}),
      file.title.utf8.count+file.cells.reduce(0,{$0+$1.id.utf8.count+$1.source.utf8.count})<=2*1024*1024 else {throw SaveError.invalidNotebook}
    // No parser/evaluator runs here. Invalid/incomplete mathematical source is valid notebook data.
  }
  static func encode(_ file:NativeSourceFile)throws->Data {
    try validate(file);let encoder=JSONEncoder();encoder.outputFormatting=[.prettyPrinted,.sortedKeys,.withoutEscapingSlashes]
    let bytes=try encoder.encode(file);guard bytes.count<=maximumBytes else {throw SaveError.invalidNotebook};return bytes
  }
}
enum SaveValidation {
  static func binding(_ b:FileBinding)throws {
    guard b.codecVersion==1,UUID(uuidString:b.documentID) != nil,b.revision>0,b.revision<=HostSerial.maximum,
      !b.bookmark.isEmpty,b.bookmark.count<=1024*1024,!b.fileName.isEmpty,b.fileName.utf8.count<=1024,!b.fileName.contains("/"),!b.fileName.utf8.contains(0),b.fileName != ".",b.fileName != "..",!b.displayPath.isEmpty,b.displayPath.utf8.count<=8192 else {throw StorageError.corruptIdentity}
    if let stamp=b.expected {guard stamp.byteLength<=UInt64(NotebookFileCodec.maximumBytes),KernelValidation.hash(stamp.byteHash),stamp.inode>0 else {throw StorageError.corruptIdentity}}
  }
  static func intent(_ i:SaveIntent)throws {
    try binding(i.binding)
    guard i.codecVersion==1,i.documentID==i.binding.documentID,SourceValidation.identity(i.operationID),UUID(uuidString:i.storeID) != nil,
      i.sourceRevision<=HostSerial.maximum,KernelValidation.hash(i.sourceSnapshotHash),KernelValidation.hash(i.fileHash),
      i.fileByteLength>0,i.fileByteLength<=UInt64(NotebookFileCodec.maximumBytes) else {throw StorageError.corruptIdentity}
  }
  static func receipt(_ r:SaveReceipt,intent i:SaveIntent)throws {
    try intent(i)
    guard r.codecVersion==1,r.operationID==i.operationID,r.documentID==i.documentID,r.sourceRevision==i.sourceRevision,
      r.sourceSnapshotHash==i.sourceSnapshotHash,r.bindingRevision==i.binding.revision,r.fileHash==i.fileHash,
      r.actual.byteHash==i.fileHash,r.actual.byteLength==i.fileByteLength,!r.updatedBookmark.isEmpty,r.updatedBookmark.count<=1024*1024,
      r.actual.inode>0,!r.completedAt.isEmpty else {throw StorageError.corruptIdentity}
  }
}
