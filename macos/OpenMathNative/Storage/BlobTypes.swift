import Foundation

struct BlobDescriptor:Codable,Sendable,Equatable {
  let hash:String
  let byteLength:UInt64
}
/// Process-local retention, distinct from the scoped media/result capabilities signed by the host.
struct BlobPublication:Sendable,Equatable {
  let descriptor:BlobDescriptor
  let pinID:String
}
struct BlobReadLease:Sendable {
  let descriptor:BlobDescriptor
  let readerID:String
}
enum BlobOwnerKind:String,Codable,Sendable {
  case attachment, result, sourceSnapshot="source_snapshot", checkpoint, context, export, temporary
}
struct BlobOwner:Codable,Sendable,Equatable {
  let kind:BlobOwnerKind
  let id:String
}
struct BlobReference:Codable,Sendable,Equatable {
  let owner:BlobOwner
  let descriptor:BlobDescriptor
  let mediaType:String
  let codecVersion:String
}
struct BlobInspection:Sendable {
  enum State:Sendable { case missing,corrupt,pinned,referenced,orphanCandidate }
  let state:State
  let pinCount:Int
  let referenceCount:Int
  /// False means unopened document stores exist. An orphan candidate must never be deleted then.
  let allStoresChecked:Bool
}
enum BlobError:Error,Sendable,Equatable {
  case invalidHash,missing,corrupt,sourceChanged,budgetExceeded,cancelled,invalidPin,invalidReader,closing
}
final class BlobCancellation:@unchecked Sendable {
  private let lock=NSLock()
  private var stopped=false
  func cancel() { lock.withLock { stopped=true } }
  func check() throws { if lock.withLock({stopped}) { throw BlobError.cancelled } }
}
