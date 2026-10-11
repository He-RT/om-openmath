import Foundation

struct DocumentProjectionCursor:Sendable {
  let documentID:String
  let storeID:String
  let generation:Int64
  let kindIndex:Int
  let rowID:Int64
}
struct DocumentProjectionRecovery:Sendable {
  let documentID:String
  let checkedEvents:Int
  let insertedEvents:Int
  let unsupportedKinds:[DocumentEffectKind]
  let complete:Bool
  let nextCursor:DocumentProjectionCursor?
}
/// Only original durable document effects are projected here. No FFI host, evaluator, HTTP
/// client, provider credential or tool executor is available to this recovery owner.
actor RecoveryService {
  let storage:StorageService
  init(storage:StorageService) {self.storage=storage}
  func projectDocument(_ document:String,startingAt:DocumentProjectionCursor?=nil,maximumEvents:Int=256,faults:StorageFaults = .init()) async throws->DocumentProjectionRecovery {
    guard maximumEvents>0,maximumEvents<=4096 else {throw StorageError.corruptIdentity}
    let identity=try await storage.documentIdentity(document)
    if let startingAt {
      guard startingAt.documentID==document,startingAt.storeID==identity.storeID,startingAt.generation==identity.generation,
        (0...2).contains(startingAt.kindIndex),startingAt.rowID>=0 else {throw StorageError.corruptIdentity}
    }
    var checked=0,inserted=0,unsupported:[DocumentEffectKind]=[]
    let kinds:[DocumentEffectKind]=[.source,.kernel,.save]
    for index in (startingAt?.kindIndex ?? 0)..<kinds.count {
      let kind=kinds[index]
      var cursor:Int64=startingAt?.kindIndex==index ? startingAt!.rowID : 0,more=true
      while more {
        try Task.checkCancellation()
        if checked==maximumEvents {
          let next=DocumentProjectionCursor(documentID:document,storeID:identity.storeID,generation:identity.generation,kindIndex:index,rowID:cursor)
          return .init(documentID:document,checkedEvents:checked,insertedEvents:inserted,unsupportedKinds:unsupported,complete:false,nextCursor:next)
        }
        let page:DocumentOutboxPage
        do {page=try await storage.documentOutbox(document:document,kind:kind,after:cursor,limit:min(16,maximumEvents-checked))}
        catch StorageError.unsupportedVersion {unsupported.append(kind);break}
        for entry in page.entries {
          let receipt=try await storage.projectDocumentEffect(entry.effect,faults:faults)
          checked+=1;if receipt.inserted {inserted+=1}
        }
        guard page.lastRowID>=cursor,(!page.hasMore || page.lastRowID>cursor) else {throw StorageError.corruptIdentity}
        cursor=page.lastRowID;more=page.hasMore
      }
    }
    return .init(documentID:document,checkedEvents:checked,insertedEvents:inserted,unsupportedKinds:unsupported,complete:true,nextCursor:nil)
  }
}
