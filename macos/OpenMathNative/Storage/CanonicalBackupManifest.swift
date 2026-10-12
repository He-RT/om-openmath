import Foundation

struct BackupLedgerStore:Codable,Sendable,Equatable {
  let store_id:String,store_generation:Int64,sqlite_snapshot_hash:String
}
/// Exact frozen macos-storage.schema.json BackupManifest envelope. The manifest hash binds
/// the separate sorted physical inventory; verified is published only after byte verification.
struct CanonicalBackupManifest:Codable,Sendable {
  let record_type:String,backup_id:String,format_version:Int,created_at:String
  let stores:[BackupLedgerStore],blob_hashes:[String],manifest_hash:String,verified:Bool
  static let keys:Set<String>=["record_type","backup_id","format_version","created_at","stores","blob_hashes","manifest_hash","verified"]
  init(_ inventory:BackupManifest,hash:String) {
    record_type="backup_manifest";backup_id=inventory.backupID;format_version=1
    created_at=ISO8601DateFormatter().string(from:Date(timeIntervalSince1970:TimeInterval(inventory.createdAt)))
    stores=inventory.stores.map{.init(store_id:$0.identity.storeID,store_generation:$0.identity.generation,sqlite_snapshot_hash:$0.databaseHash)}
    blob_hashes=inventory.blobs.map{$0.hash};manifest_hash=hash;verified=true
  }
  static func read(_ url:URL,inventory:BackupManifest,inventoryBytes:Data)throws->Self {
    let bytes=try BackupService.read(url,maximum:16*1024*1024)
    guard let object=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],Set(object.keys)==keys,
      let records=object["stores"] as? [[String:Any]],records.allSatisfy({Set($0.keys)==["store_id","store_generation","sqlite_snapshot_hash"]}) else {throw StorageError.corruptIdentity}
    let value=try JSONDecoder().decode(Self.self,from:bytes)
    let expected=Self(inventory,hash:KernelHashes.data(inventoryBytes))
    guard value.record_type==expected.record_type,value.backup_id==expected.backup_id,value.format_version==1,
      value.created_at==expected.created_at,value.stores==expected.stores,value.blob_hashes==expected.blob_hashes,
      value.manifest_hash==expected.manifest_hash,value.verified else {throw StorageError.corruptIdentity}
    return value
  }
}
