import CryptoKit
import Darwin
import Foundation

struct BackupStoreRecord:Codable,Sendable {
  let identity:StoreIdentity,databaseHash:String
  let byteLength:UInt64,tableCounts:[String:Int64],references:[BlobDescriptor]
}
struct BackupManifest:Codable,Sendable {
  let formatVersion:Int,backupID:String,installationID:String,channel:StorageChannel
  let createdAt:Int64,stores:[BackupStoreRecord],blobs:[BlobDescriptor]
  let purpose:BackupPurpose
}
enum BackupPurpose:String,Codable,Sendable {case manual,beforeUpgrade}
struct BackupRetention:Sendable {let removed:Int,retained:Int,retainedBytes:UInt64,overBudget:Bool}
struct PendingBackup:Sendable {let folder:URL,manifest:BackupManifest}
/// Self-contained snapshots: database copies use SQLite's API; immutable original bytes are
/// copied/verified before the manifest is published. No runtime tokens or credentials are added.
enum BackupService {
  static let keys:Set<String>=["formatVersion","backupID","installationID","channel","createdAt","stores","blobs","purpose"]
  static func databaseName(_ identity:StoreIdentity)->String {identity.kind=="library" ? "library.sqlite" : "authority.sqlite"}
  static func rootIdentity(_ root:URL)throws->RootIdentity {
    try ManagedFiles.readJSON(RootIdentity.self,root.appendingPathComponent("store.json"),keys:["format_version","installation_id","channel"])
  }
  static func prepare(root:URL,stores:[CatalogStore],purpose:BackupPurpose = .manual,faults:StorageFaults)throws->PendingBackup {
    let identity=try rootIdentity(root),id=UUID().uuidString.lowercased(),backups=root.appendingPathComponent("Backups")
    try ManagedFiles.directory(backups)
    let pending=backups.appendingPathComponent(".pending-"+id)
    try ManagedFiles.directory(pending);var records:[BackupStoreRecord]=[]
    for store in stores {
      let directory=pending.appendingPathComponent("Stores/"+store.identity.storeID)
      try ManagedFiles.directory(directory)
      let target=directory.appendingPathComponent(databaseName(store.identity)),db=try SQLiteDatabase(url:store.database,readonly:true)
      defer {try? db.close()}
      try db.backup(to:target,faults:faults)
      let read=try SQLiteDatabase(url:target,readonly:true);defer {try? read.close()}
      guard try StorageBootstrap.readHeader(read)==store.identity,try StoreCatalog.counts(read)==store.counts,
        try StoreCatalog.references(read)==store.references else {throw StorageError.corruptIdentity}
      try StoreCatalog.validate(read,identity:store.identity,url:target);try read.close()
      let file=try digest(target)
      records.append(.init(identity:store.identity,databaseHash:file.hash,byteLength:file.byteLength,tableCounts:store.counts,references:store.references))
      try ManagedFiles.syncDirectory(directory);try faults.reach("backup.store_verified")
    }
    let manifest=BackupManifest(formatVersion:1,backupID:id,installationID:identity.installationID,channel:identity.channel,createdAt:Int64(Date().timeIntervalSince1970),stores:records,blobs:try StoreCatalog.union(stores),purpose:purpose)
    return .init(folder:pending,manifest:manifest)
  }
  static func publish(_ pending:PendingBackup,root:URL,faults:StorageFaults)throws->BackupManifest {
    for blob in pending.manifest.blobs {
      guard try digest(pending.folder.appendingPathComponent("Blobs/"+blob.hash))==blob else {throw BlobError.corrupt}
    }
    // Large manifests are read with a bounded reader (the small root/selector reader stays 64K).
    let data=try JSONEncoder().encode(pending.manifest)
    guard data.count<=16*1024*1024 else {throw BlobError.budgetExceeded}
    let inventory=pending.folder.appendingPathComponent("inventory.json")
    try ManagedFiles.publish(pending.manifest,to:inventory,faults:.init())
    let inventoryHash=try digest(inventory).hash
    try ManagedFiles.publish(CanonicalBackupManifest(pending.manifest,hash:inventoryHash),to:pending.folder.appendingPathComponent("manifest.json"),faults:.init())
    try ManagedFiles.syncDirectory(pending.folder.appendingPathComponent("Stores"))
    if !pending.manifest.blobs.isEmpty {try ManagedFiles.syncDirectory(pending.folder.appendingPathComponent("Blobs"))}
    try ManagedFiles.syncDirectory(pending.folder);try faults.reach("backup.before_publish")
    let published=root.appendingPathComponent("Backups/"+pending.manifest.backupID)
    guard !(try ManagedFiles.check(published,directory:true)),rename(pending.folder.path,published.path)==0 else {throw StorageError.system(errno,"backup_publish")}
    try ManagedFiles.syncDirectory(published.deletingLastPathComponent());try faults.reach("backup.published")
    _ = try verify(root:root,id:pending.manifest.backupID)
    return pending.manifest
  }
  static func manifest(root:URL,id:String)throws->BackupManifest {
    guard UUID(uuidString:id) != nil,id==id.lowercased() else {throw StorageError.corruptIdentity}
    let folder=root.appendingPathComponent("Backups/"+id)
    let url=folder.appendingPathComponent("inventory.json")
    let bytes=try read(url,maximum:16*1024*1024),value=try JSONDecoder().decode(BackupManifest.self,from:bytes)
    guard let object=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],Set(object.keys)==keys else {throw StorageError.corruptIdentity}
    let identity=try rootIdentity(root)
    guard value.formatVersion==1,value.backupID==id,value.installationID==identity.installationID,value.channel==identity.channel,
      value.stores.count>0,value.stores.count<=10001,value.blobs.count<=100000,Set(value.stores.map{$0.identity.storeID}).count==value.stores.count,
      Set(value.blobs.map{$0.hash}).count==value.blobs.count else {throw StorageError.corruptIdentity}
    var descriptors:[String:BlobDescriptor]=[:]
    for store in value.stores {
      try store.identity.validate()
      guard KernelValidation.hash(store.databaseHash),store.byteLength>0,store.tableCounts.count<=128 else {throw StorageError.corruptIdentity}
      for blob in store.references {
        guard KernelValidation.hash(blob.hash),blob.byteLength<=128*1024*1024 else {throw StorageError.corruptIdentity}
        if let previous=descriptors[blob.hash],previous != blob {throw StorageError.corruptIdentity};descriptors[blob.hash]=blob
      }
    }
    guard value.stores.filter({$0.identity.kind=="library"}).count==1,value.blobs==descriptors.values.sorted(by:{$0.hash<$1.hash}) else {throw StorageError.corruptIdentity}
    _ = try CanonicalBackupManifest.read(folder.appendingPathComponent("manifest.json"),inventory:value,inventoryBytes:bytes)
    return value
  }
  static func verify(root:URL,id:String)throws->BackupManifest {
    let value=try manifest(root:root,id:id),folder=root.appendingPathComponent("Backups/"+id)
    for store in value.stores {
      let url=folder.appendingPathComponent("Stores/"+store.identity.storeID+"/"+databaseName(store.identity))
      guard try digest(url)==BlobDescriptor(hash:store.databaseHash,byteLength:store.byteLength) else {throw StorageError.corruptIdentity}
      let db=try SQLiteDatabase(url:url,readonly:true);defer {try? db.close()}
      guard try StorageBootstrap.readHeader(db)==store.identity,try StoreCatalog.counts(db)==store.tableCounts,
        try StoreCatalog.references(db)==store.references else {throw StorageError.corruptIdentity}
      try StoreCatalog.validate(db,identity:store.identity,url:url)
    }
    for blob in value.blobs {guard try digest(folder.appendingPathComponent("Blobs/"+blob.hash))==blob else {throw BlobError.corrupt}}
    return value
  }
  static func allManifests(root:URL)throws->[BackupManifest] {
    let folder=root.appendingPathComponent("Backups")
    guard try ManagedFiles.check(folder,directory:true) else {return []}
    let names=try FileManager.default.contentsOfDirectory(atPath:folder.path)
    guard names.count<=10000 else {throw BlobError.budgetExceeded}
    return try names.filter{!$0.hasPrefix(".pending-")}.sorted().map{try verify(root:root,id:$0)}
  }
  static func retainHealthy(root:URL)throws->BackupRetention {
    // Validate the entire set before deleting anything. A corrupt backup is preserved for
    // recovery and does not make another snapshot dispensable. Soft quota never wins over
    // the newest seven normal backups or the two newest migration backups.
    let manifests=try allManifests(root:root).sorted {a,b in a.createdAt==b.createdAt ? a.backupID>b.backupID : a.createdAt>b.createdAt}
    var keep=Set(manifests.filter{$0.purpose == .manual}.prefix(7).map{$0.backupID}+manifests.filter{$0.purpose == .beforeUpgrade}.prefix(2).map{$0.backupID})
    if let complete=manifests.first(where:{$0.stores.allSatisfy{$0.tableCounts["store_recovery_state"]==nil}}) {keep.insert(complete.backupID)}
    var removed=0,retainedBytes:UInt64=0
    for manifest in manifests {
      if keep.contains(manifest.backupID) {retainedBytes+=manifest.stores.reduce(0){$0+$1.byteLength}+manifest.blobs.reduce(0){$0+$1.byteLength}}
      else {
        let directory=root.appendingPathComponent("Backups/"+manifest.backupID)
        guard try ManagedFiles.check(directory,directory:true) else {throw StorageError.recoveryRequired}
        // Only our verified manifest's exact files; refuse unknown additions/symlinks.
        let children=try FileManager.default.contentsOfDirectory(atPath:directory.path)
        guard Set(children).isSubset(of:["Stores","Blobs","manifest.json","inventory.json"]) else {throw StorageError.recoveryRequired}
        try validateTree(directory,manifest:manifest)
        try FileManager.default.removeItem(at:directory);try ManagedFiles.syncDirectory(directory.deletingLastPathComponent());removed+=1
      }
    }
    return .init(removed:removed,retained:keep.count,retainedBytes:retainedBytes,overBudget:retainedBytes>1024*1024*1024)
  }
  private static func validateTree(_ directory:URL,manifest:BackupManifest)throws {
    var allowed:Set<String>=["Stores","Blobs","manifest.json","inventory.json"]
    for store in manifest.stores {
      allowed.insert("Stores/"+store.identity.storeID)
      allowed.insert("Stores/"+store.identity.storeID+"/"+databaseName(store.identity))
      // Apple's VFS can leave its 32KiB SHM cache after WAL->DELETE and final close. It is
      // never part of the snapshot authority; WAL/journal and every unknown file still stop.
      allowed.insert("Stores/"+store.identity.storeID+"/"+databaseName(store.identity)+"-shm")
    }
    for blob in manifest.blobs {allowed.insert("Blobs/"+blob.hash)}
    var count=0,failed=false
    guard let enumerator=FileManager.default.enumerator(at:directory,includingPropertiesForKeys:nil,errorHandler:{_,_ in failed=true;return false}) else {throw StorageError.recoveryRequired}
    while let url=enumerator.nextObject() as? URL {
      count+=1;let relative=String(url.path.dropFirst(directory.path.count+1))
      guard count<=100000,allowed.contains(relative),try ManagedFiles.check(url) else {throw StorageError.unsafePath}
    }
    guard !failed else {throw StorageError.recoveryRequired}
  }
  static func read(_ url:URL,maximum:Int)throws->Data {
    guard try ManagedFiles.check(url,directory:false) else {throw StorageError.recoveryRequired}
    let fd=open(url.path,O_RDONLY|O_NOFOLLOW|O_CLOEXEC);guard fd>=0 else {throw StorageError.system(errno,"backup_read")}
    defer {Darwin.close(fd)}
    var info=stat();guard fstat(fd,&info)==0,info.st_mode&S_IFMT==S_IFREG,info.st_size>=0,info.st_size<=maximum else {throw BlobError.budgetExceeded}
    var data=Data(count:Int(info.st_size));try data.withUnsafeMutableBytes {raw in
      var offset=0;while offset<raw.count {
        let count=Darwin.read(fd,raw.baseAddress!.advanced(by:offset),raw.count-offset)
        if count<0 && errno==EINTR {continue};guard count>0 else {throw StorageError.corruptIdentity};offset+=count
      }
    };return data
  }
  static func digest(_ url:URL)throws->BlobDescriptor {
    guard try ManagedFiles.check(url,directory:false) else {throw StorageError.recoveryRequired}
    let fd=open(url.path,O_RDONLY|O_NOFOLLOW|O_CLOEXEC);guard fd>=0 else {throw StorageError.system(errno,"backup_hash_open")}
    defer {Darwin.close(fd)}
    var before=stat();guard fstat(fd,&before)==0,before.st_mode&S_IFMT==S_IFREG,before.st_size>=0,before.st_size<=8*1024*1024*1024 else {throw BlobError.budgetExceeded}
    var hash=SHA256(),buffer=Data(count:64*1024),total:UInt64=0
    while true {
      let count=buffer.withUnsafeMutableBytes {Darwin.read(fd,$0.baseAddress,$0.count)}
      if count<0 && errno==EINTR {continue};guard count>=0 else {throw StorageError.system(errno,"backup_hash_read")}
      if count==0 {break};total+=UInt64(count);hash.update(data:buffer.prefix(count))
      guard total<=8*1024*1024*1024 else {throw BlobError.budgetExceeded}
    }
    var after=stat();guard fstat(fd,&after)==0,total==UInt64(before.st_size),before.st_size==after.st_size,
      before.st_mtimespec.tv_sec==after.st_mtimespec.tv_sec,before.st_mtimespec.tv_nsec==after.st_mtimespec.tv_nsec,
      before.st_ctimespec.tv_sec==after.st_ctimespec.tv_sec,before.st_ctimespec.tv_nsec==after.st_ctimespec.tv_nsec else {throw BlobError.sourceChanged}
    return .init(hash:hash.finalize().map{String(format:"%02x",$0)}.joined(),byteLength:total)
  }
}
