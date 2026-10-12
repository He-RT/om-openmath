import Foundation
import Darwin

struct CacheRecord:Codable,Sendable {let id:String,descriptor:BlobDescriptor;var accessedAt:UInt64}
struct CachePin:Sendable {let id:String,token:String}
struct CachePrune:Sendable {let removed:Int,bytes:UInt64,retainedBytes:UInt64,overBudget:Bool}
private struct CacheManifest:Codable {let formatVersion:Int;let records:[CacheRecord]}
/// Disposable derived bytes have their own directory/index. Original attachment/checkpoint
/// Blobs never enter this owner and cannot be evicted through a cache budget or clear action.
final class DerivedCache:@unchecked Sendable {
  private let queue=DispatchQueue(label:"org.openmath.storage.derived-cache",qos:.utility)
  private let root:URL,lease:RootLease
  private var records:[String:CacheRecord]=[:],pins:[String:String]=[:]
  private var serial:UInt64=0,loaded=false
  init(root:URL,lease:RootLease) {self.root=root.appendingPathComponent("DerivedCache");self.lease=lease}
  private nonisolated(nonsending) func perform<T:Sendable>(_ work:@escaping @Sendable (DerivedCache)throws->T) async throws->T {
    try await withCheckedThrowingContinuation {continuation in queue.async { [self] in
      do {try load();continuation.resume(returning:try work(self))} catch {continuation.resume(throwing:error)}
    }}
  }
  private func load()throws {
    if loaded {return};try ManagedFiles.directory(root)
    let manifest=root.appendingPathComponent("index.json")
    if try ManagedFiles.check(manifest,directory:false) {
      let decoded=try JSONDecoder().decode(CacheManifest.self,from:BackupService.read(manifest,maximum:4*1024*1024))
      guard decoded.formatVersion==1,decoded.records.count<=10000,Set(decoded.records.map{$0.id}).count==decoded.records.count else {throw StorageError.corruptIdentity}
      for record in decoded.records {
        guard SourceValidation.identity(record.id),KernelValidation.hash(record.descriptor.hash),record.descriptor.byteLength<=128*1024*1024 else {throw StorageError.corruptIdentity}
        records[record.id]=record;serial=max(serial,record.accessedAt)
      }
    };loaded=true
  }
  private func save()throws {
    try ManagedFiles.publish(CacheManifest(formatVersion:1,records:records.values.sorted{$0.id<$1.id}),to:root.appendingPathComponent("index.json"),faults:.init())
  }
  private func path(_ hash:String)->URL {root.appendingPathComponent("Bytes/"+hash)}
  private func touch(_ id:String)throws {
    guard serial<UInt64.max else {throw BlobError.budgetExceeded};serial+=1;records[id]?.accessedAt=serial
  }
  nonisolated(nonsending) func pin(_ id:String) async throws->CachePin {
    try await perform {cache in
      guard SourceValidation.identity(id),cache.pins.count<128 else {throw BlobError.budgetExceeded}
      let token=UUID().uuidString.lowercased();cache.pins[token]=id;return .init(id:id,token:token)
    }
  }
  nonisolated(nonsending) func release(_ pin:CachePin) async throws {
    try await perform {cache in guard cache.pins[pin.token]==pin.id else {throw BlobError.invalidPin};cache.pins.removeValue(forKey:pin.token)}
  }
  nonisolated(nonsending) func put(_ id:String,bytes:Data) async throws->BlobDescriptor {
    try await perform {cache in
      guard SourceValidation.identity(id),bytes.count<=128*1024*1024,cache.records[id] != nil || cache.records.count<10000 else {throw BlobError.budgetExceeded}
      try StorageQuota.require(cache.root,bytes:UInt64(bytes.count),large:bytes.count>=8*1024*1024)
      let descriptor=BlobDescriptor(hash:KernelHashes.data(bytes),byteLength:UInt64(bytes.count)),target=cache.path(descriptor.hash)
      if let previous=cache.records[id],previous.descriptor != descriptor {throw StorageError.idempotencyConflict}
      try ManagedFiles.directory(target.deletingLastPathComponent())
      if try ManagedFiles.check(target,directory:false) {guard try BackupService.digest(target)==descriptor else {throw BlobError.corrupt}}
      else {
        // Cache payload is not an authoritative receipt. A failed index update leaves only
        // unindexed disposable bytes; it never converts a source resource into a cache entry.
        let temporary=target.deletingLastPathComponent().appendingPathComponent(".pending-"+UUID().uuidString.lowercased())
        try bytes.write(to:temporary,options:.withoutOverwriting)
        try ManagedFiles.checkPermissionsAndSync(temporary)
        guard rename(temporary.path,target.path)==0 else {throw StorageError.system(errno,"cache_publish")}
        try ManagedFiles.syncDirectory(target.deletingLastPathComponent())
      }
      let old=cache.records[id]
      cache.records[id] = .init(id:id,descriptor:descriptor,accessedAt:cache.serial);try cache.touch(id)
      do {try cache.save()} catch {cache.records[id]=old;throw error}
      return descriptor
    }
  }
  nonisolated(nonsending) func read(_ id:String) async throws->Data? {
    try await perform {cache in
      guard let record=cache.records[id] else {return nil}
      let bytes=try BackupService.read(cache.path(record.descriptor.hash),maximum:128*1024*1024)
      guard UInt64(bytes.count)==record.descriptor.byteLength,KernelHashes.data(bytes)==record.descriptor.hash else {throw BlobError.corrupt}
      try cache.touch(id);try cache.save();return bytes
    }
  }
  nonisolated(nonsending) func prune(budget:UInt64=512*1024*1024) async throws->CachePrune {
    try await perform {cache in
      guard budget<=512*1024*1024 else {throw BlobError.budgetExceeded}
      func total()->UInt64 {Dictionary(cache.records.values.map{($0.descriptor.hash,$0.descriptor.byteLength)},uniquingKeysWith:{a,_ in a}).values.reduce(0,+)}
      var remaining=total(),removed=0,removedBytes:UInt64=0
      let pinned=Set(cache.pins.values),old=cache.records
      var hashes=Set<String>()
      for record in cache.records.values.sorted(by:{$0.accessedAt==$1.accessedAt ? $0.id<$1.id : $0.accessedAt<$1.accessedAt}) where remaining>budget && !pinned.contains(record.id) {
        cache.records.removeValue(forKey:record.id);removed+=1
        if !cache.records.values.contains(where:{$0.descriptor.hash==record.descriptor.hash}) {remaining-=record.descriptor.byteLength;removedBytes+=record.descriptor.byteLength;hashes.insert(record.descriptor.hash)}
      }
      do {try cache.save()} catch {cache.records=old;throw error}
      for hash in hashes {let path=cache.path(hash);if try ManagedFiles.check(path,directory:false) {guard unlink(path.path)==0 else {throw StorageError.system(errno,"cache_delete")}}}
      if !hashes.isEmpty {try ManagedFiles.syncDirectory(cache.root.appendingPathComponent("Bytes"))}
      return .init(removed:removed,bytes:removedBytes,retainedBytes:remaining,overBudget:remaining>budget)
    }
  }
}
