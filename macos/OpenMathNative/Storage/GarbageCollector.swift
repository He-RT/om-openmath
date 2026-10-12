import Foundation

struct BlobCollection:Sendable {let scanned:Int,removed:Int,removedBytes:UInt64,pinned:Int,referenced:Int,young:Int}
struct BlobCollectionPlan:Sendable {let hashes:Set<String>,references:[BlobDescriptor],createdAt:Int64}
enum GarbageCollector {
  /// All retained document generations and published healthy backups participate. One invalid
  /// store/manifest stops the scan before BlobCore can move or delete any original resource.
  static func plan(root:URL)throws->BlobCollectionPlan {
    let stores=try StoreCatalog.stores(root,retained:true)
    let descriptors=try StoreCatalog.union(stores)
    let backups=try BackupService.allManifests(root:root)
    return .init(hashes:Set(descriptors.map{$0.hash}+backups.flatMap{$0.blobs.map{$0.hash}}),references:descriptors,createdAt:Int64(Date().timeIntervalSince1970))
  }
}
