import Foundation
import CryptoKit

enum StorageError:Error,Sendable,Equatable {
  case closing, inUse, unsupportedVersion, corruptIdentity, recoveryRequired, nonlocalVolume
  case unsafePath, inaccessible, system(Int32,String), sqlite(Int32,String), unsupportedSync
  case unknownCommit(Int32),idempotencyConflict,staleRevision
  case transactionUnavailable
}
enum StorageChannel:String,Codable,Sendable { case preview, production }
struct NativeStoragePaths:Sendable {
  let root:URL
  let channel:StorageChannel
  static func system(_ channel:StorageChannel) throws ->Self {
    let base=try FileManager.default.url(for:.applicationSupportDirectory,in:.userDomainMask,appropriateFor:nil,create:false)
    return .init(root:base.appendingPathComponent("OpenMath",isDirectory:true).appendingPathComponent(channel == .preview ? "NativeMacPreview" : "NativeMac",isDirectory:true),channel:channel)
  }
}
struct StoreIdentity:Codable,Sendable,Equatable {
  let storeID:String
  let kind:String
  let generation:Int64
  let storeVersion:Int64
  let minimumReaderVersion:Int64
  let codecVersion:Int64
  let documentID:String?
  private enum CodingKeys:String,CodingKey {
    case storeID="store_id",kind,generation,storeVersion="store_version",minimumReaderVersion="minimum_reader_version",codecVersion="codec_version",documentID="document_id"
  }
  static func fresh(kind:String,document:String?=nil)->Self {
    let version:Int64=kind=="document" ? 3 : 1
    return .init(storeID:UUID().uuidString.lowercased(),kind:kind,generation:1,storeVersion:version,minimumReaderVersion:version,codecVersion:version,documentID:document)
  }
  func validate() throws {
    guard (storeVersion==1 || kind=="document" && (storeVersion==2 || storeVersion==3)),minimumReaderVersion==storeVersion,codecVersion==storeVersion else { throw StorageError.unsupportedVersion }
    guard generation>0,generation<=999999,UUID(uuidString:storeID) != nil,
      (kind=="library" && documentID==nil || kind=="document" && documentID.flatMap(UUID.init(uuidString:)) != nil)
    else { throw StorageError.corruptIdentity }
  }
  var hash:String {
    get throws { let encoder=JSONEncoder();encoder.outputFormatting=[.sortedKeys,.withoutEscapingSlashes]
      return SHA256.hash(data:try encoder.encode(self)).map{String(format:"%02x",$0)}.joined() }
  }
}
struct StoreSelector:Codable,Sendable {
  let formatVersion:Int
  let storeID:String
  let generation:Int64
  let headerHash:String
  private enum CodingKeys:String,CodingKey { case formatVersion="format_version",storeID="store_id",generation,headerHash="header_hash" }
}
struct RootIdentity:Codable,Sendable {
  let formatVersion:Int
  let installationID:String
  let channel:StorageChannel
  private enum CodingKeys:String,CodingKey { case formatVersion="format_version",installationID="installation_id",channel }
}
struct SQLiteRuntimeInfo:Sendable,Codable {
  let version:String
  let sourceID:String
  let vfs:String
  let vfsVersion:Int32
  let journalMode:String
  let synchronous:Int64
  let foreignKeys:Int64
  let fullfsync:Int64
  let checkpointFullfsync:Int64
}
/// Trusted host fault boundaries. A test can throw or stop a disposable child at these points.
/// No fault hook may turn an unsuccessful write/commit into a success receipt.
struct StorageFaults:Sendable {
  var reach:@Sendable (String) throws ->Void = { _ in }
}
