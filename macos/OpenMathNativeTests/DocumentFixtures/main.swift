import Foundation
import SQLite3
private struct Plans:Decodable {
  let initial:NativeSourceSnapshot
  let first:NativeSourceCommit
  let second:NativeSourceCommit
  let preparation_only:Bool
}
@main struct DocumentFixtures {
  static func main() async throws {
    let fullsyncFault=CommandLine.arguments.count>4 && CommandLine.arguments[4]=="fullsync-fault"
    if fullsyncFault { precondition(om_fixture_install_sync_probe()==SQLITE_OK) }
    let plans=try JSONDecoder().decode(Plans.self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[1])))
    var nfc=plans.initial.file.cells[0],nfd=nfc
    nfc.source="é";nfd.source="e\u{301}"
    precondition(nfc.source==nfd.source && !SourceHashes.same(nfc,nfd))
    let root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    let receiptURL=URL(fileURLWithPath:CommandLine.arguments[3])
    let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let id=plans.initial.document_id
    _ = try await store.openDocument(id)
    let initial=try await store.initializeSource(plans.initial)
    precondition(initial.snapshot_hash==plans.initial.snapshot_hash)
    let first=try await store.commitSource(plans.first)
    precondition(first.receipt.committed_revision.value?.value==1)
    precondition(first.receipt.updated_at != plans.first.commit.committed_at)
    try JSONEncoder().encode(first).write(to:receiptURL)
    let duplicate=try await store.commitSource(plans.first)
    precondition(duplicate.receipt.transaction_id.value==first.receipt.transaction_id.value)
    var conflict=plans.first
    conflict.after.file.title+="different"
    conflict.after.snapshot_hash=SourceHashes.snapshot(conflict.after)
    conflict.commit.snapshot_hash=conflict.after.snapshot_hash
    conflict.commit.request_hash=SourceHashes.request(conflict)
    do { _ = try await store.commitSource(conflict);fatalError("idempotency conflict accepted") }
    catch StorageError.idempotencyConflict { }
    let current=try await store.committedSource(id)!
    precondition(current.revision.value==1 && current.snapshot_hash==plans.first.after.snapshot_hash)
    var malformed=plans.second;malformed.after.file.cells.append(malformed.after.file.cells[0])
    do { _ = try await store.commitSource(malformed);fatalError("partial malformed source saved") }
    catch StorageError.corruptIdentity { }
    var stale=plans.first
    stale.commit.operation_id="op-stale";stale.commit.transaction_id="tx-stale";stale.commit.outbox_event_id="event-stale"
    stale.commit.request_hash=SourceHashes.request(stale)
    do { _ = try await store.commitSource(stale);fatalError("stale original base accepted under a new ID") }
    catch StorageError.staleRevision { }
    let beforeSecond=try await store.committedSource(id)!
    precondition(beforeSecond.snapshot_hash==current.snapshot_hash)
    let rollback=StorageFaults { point in if point=="revision_inserted" { throw StorageError.system(ENOSPC,"test-midtransaction") } }
    do { _ = try await store.commitSource(plans.second,faults:rollback);fatalError("midtransaction failure acknowledged") }
    catch StorageError.system { }
    let absent=try await store.sourceReceipt(document:id,operation:"op-two")
    let rolledBack=try await store.committedSource(id)!
    precondition(absent==nil && rolledBack.revision.value==1)
    if fullsyncFault {
      let failing=StorageFaults { point in if point=="before_source_commit" { om_fixture_fail_fullsync(1) } }
      do { _ = try await store.commitSource(plans.second,faults:failing);fatalError("full-sync failure acknowledged") }
      catch StorageError.unknownCommit { }
      do { _ = try await store.sourceReceipt(document:id,operation:"op-two");fatalError("unstable bytes reported durable by query") }
      catch StorageError.unknownCommit { }
      om_fixture_fail_fullsync(0)
    } else {
    let disconnected=StorageFaults { point in if point=="source_committed" { throw StorageError.system(EIO,"test-lost-ack") } }
    do { _ = try await store.commitSource(plans.second,faults:disconnected);fatalError("lost ack returned definite success") }
    catch StorageError.unknownCommit { }
    }
    let second=try await store.sourceReceipt(document:id,operation:"op-two")!
    precondition(second.receipt.committed_revision.value?.value==2)
    // Same original source-edit ID is still a single effect after later successful commits.
    let original=try await store.commitSource(plans.first)
    precondition(original.receipt.committed_revision.value?.value==1)
    let head=try await store.committedSource(id)!
    precondition(head.revision.value==2 && head.file.cells[0].source=="let a=5; a+1")
    precondition(head.file.cells[1].source.utf8.elementsEqual(plans.initial.file.cells[1].source.utf8))
    precondition(head.file.title.utf8.elementsEqual(plans.initial.file.title.utf8))
    try await store.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    _ = try await reopened.openDocument(id)
    let restored=try await reopened.committedSource(id)!
    let replay=try await reopened.sourceReceipt(document:id,operation:"op-one")!
    precondition(restored.snapshot_hash==head.snapshot_hash && replay.receipt.transaction_id.value=="tx-one")
    try await reopened.close()
    // Count all physical facts with an independent read connection, off MainActor.
    let url=root.appendingPathComponent("Documents/"+id+"/Generations/000001/authority.sqlite")
    let counts=try await withCheckedThrowingContinuation { (continuation:CheckedContinuation<[Int64],any Error>) in
      DispatchQueue.global(qos:.utility).async {
        do {
          let db=try SQLiteDatabase(url:url,readonly:true)
          let counts=try ["document_revisions","transactions","operations","operation_transitions","outbox"].map{try db.integer("SELECT count(*) FROM "+$0)}
          try db.close();continuation.resume(returning:counts)
        } catch { continuation.resume(throwing:error) }
      }
    }
    precondition(counts==[3,2,2,2,2])
    if fullsyncFault { print("Actual source COMMIT then F_FULLFSYNC failure: unknown until original-ID readback reconfirms stable bytes") }
    print("Actual Rust plans -> Swift SQLite: original UTF8/order/title, 0->1->2 revisions, same-ID one effect, different content conflict, malformed whole-plan rejection, restart receipt/head and five same-DB fact counts passed")
  }
}
