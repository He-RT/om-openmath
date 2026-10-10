import Foundation
import SQLite3
private struct HistoryFixture:Decodable {let initial:NativeSourceSnapshot;let plans:[NativeSourceCommit];let preparation_only:Bool}
@main struct HistoryFixtures {
  static func main() async throws {
    let fixture=try JSONDecoder().decode(HistoryFixture.self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[1])))
    precondition(fixture.preparation_only && fixture.plans.count==206)
    let root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true),document=fixture.initial.document_id
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let info=try await storage.openDocument(document)
    precondition(info.identity.storeVersion==3 && info.identity.minimumReaderVersion==3)
    _ = try await storage.initializeSource(fixture.initial)
    for plan in fixture.plans {_ = try await storage.commitSource(plan)}
    try await storage.pinSourceTransaction(document:document,transaction:"tx-1",kind:"user",owner:"user-pin",enabled:true)
    try await storage.pinSourceTransaction(document:document,transaction:"tx-2",kind:"task",owner:"continuation-task",enabled:true)
    let original=try await storage.sourceReceipt(document:document,operation:"op-3")!
    let encoder=JSONEncoder();encoder.outputFormatting=[.sortedKeys]
    let originalBytes=try encoder.encode(original)
    let head=try await storage.committedSource(document)!
    let rolledBack=StorageFaults {point in if point=="history_payloads_pruned" {throw StorageError.system(ENOSPC,"history-rollback")} }
    do {_ = try await storage.compactSourceHistory(document:document,faults:rolledBack);fatalError("partial pruning was acknowledged")}
    catch StorageError.system { }
    let before=try await storage.sourceTransactions(document:document,ids:["tx-3"])
    precondition(before.count==1)
    let lostAck=StorageFaults {point in if point=="history_prune_committed" {throw StorageError.system(EIO,"history-lost-ack")} }
    do {_ = try await storage.compactSourceHistory(document:document,faults:lostAck);fatalError("lost prune ACK reported success")}
    catch StorageError.unknownCommit { }
    do {_ = try await storage.sourceTransactions(document:document,ids:["tx-3"]);fatalError("pruned inverse was fabricated")}
    catch StorageError.transactionUnavailable { }
    let old=try await storage.sourceReceipt(document:document,operation:"op-3")!
    let oldBytes=try encoder.encode(old);precondition(oldBytes==originalBytes)
    let admission=try await storage.sourceAdmission(document:document,operation:"op-3")
    precondition(admission?.phase == .completed)
    let preserved=try await storage.sourceTransactions(document:document,ids:["tx-1","tx-2"])
    precondition(preserved.count==2)
    let noReplay=try await storage.commitSource(fixture.plans[2])
    let noReplayBytes=try encoder.encode(noReplay);precondition(noReplayBytes==originalBytes)
    let after=try await storage.committedSource(document)!
    precondition(after.snapshot_hash==head.snapshot_hash && after.revision.value==206)
    let recent=try await storage.sourceHistory(document:document)
    precondition(recent.count==200 && recent.first?.revision==206 && recent.last?.revision==7 && recent.allSatisfy(\.retained))
    do {try await storage.pinSourceTransaction(document:document,transaction:"tx-3",kind:"user",owner:"resurrect",enabled:true);fatalError("pin resurrected pruned inverse")}
    catch StorageError.transactionUnavailable { }
    let again=try await storage.compactSourceHistory(document:document)
    precondition(again.transactions==0 && again.retained==202)
    try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    _ = try await reopened.openDocument(document)
    let restored=try await reopened.sourceReceipt(document:document,operation:"op-3")!
    let restoredBytes=try encoder.encode(restored);precondition(restoredBytes==originalBytes)
    let restoredPins=try await reopened.sourceTransactions(document:document,ids:["tx-1","tx-2"])
    precondition(restoredPins.count==2)
    try await reopened.pinSourceTransaction(document:document,transaction:"tx-1",kind:"user",owner:"user-pin",enabled:false)
    let unpinned=try await reopened.compactSourceHistory(document:document)
    precondition(unpinned.transactions==1 && unpinned.retained==201)
    do {_ = try await reopened.sourceTransactions(document:document,ids:["tx-1"]);fatalError("unpinned old inverse remained available")}
    catch StorageError.transactionUnavailable { }
    let one=try await reopened.sourceReceipt(document:document,operation:"op-1")!
    precondition(one.receipt.committed_revision.value?.value==1)
    try await reopened.close()
    let database=root.appendingPathComponent("Documents/"+document+"/Generations/000001/authority.sqlite")
    let verified=try await Task.detached {
      let db=try SQLiteDatabase(url:database,readonly:true);defer {try? db.close()}
      let counts=[try db.integer("SELECT COUNT(*) FROM transactions"),try db.integer("SELECT COUNT(*) FROM operations"),try db.integer("SELECT COUNT(*) FROM outbox"),try db.integer("SELECT COUNT(*) FROM source_history WHERE retained=1"),try db.integer("SELECT COUNT(*) FROM transactions WHERE length(forward_plan)=0 AND length(inverse_snapshot)=0")]
      let compactAdmissions=try db.integer("SELECT COUNT(*) FROM source_admissions WHERE operation_id IN ('op-1','op-3','op-4','op-5','op-6') AND length(plan)<4096")
      let compactRevisions=try db.integer("SELECT COUNT(*) FROM document_revisions WHERE revision IN (0,3,4,5) AND length(snapshot)=0")
      return (counts,compactAdmissions,compactRevisions)
    }.value
    precondition(verified.0==[206,206,206,201,5] && verified.1==5 && verified.2==4)
    // The compact metadata is still integrity checked; absence from the old full payload graph
    // must not make a damaged tombstone look like "operation never ran".
    try await Task.detached {
      let writer=try SQLiteDatabase(url:database);defer {try? writer.close()}
      _ = try writer.configure()
      try writer.transaction {try writer.statement("UPDATE source_history SET summary_hash=? WHERE transaction_id='tx-3'",[.text(String(repeating:"0",count:64))])}
    }.value
    let (damaged,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    _ = try await damaged.openDocument(document)
    do {_ = try await damaged.sourceReceipt(document:document,operation:"op-3");fatalError("damaged tombstone reported successful or absent")}
    catch StorageError.corruptIdentity { }
    try await damaged.close()
    let legacyRoot=root.appendingPathComponent("legacy-native",isDirectory:true)
    let (legacySeed,_)=try await StorageService.open(paths:.init(root:legacyRoot,channel:.preview))
    let seeded=try await legacySeed.openDocument(document)
    _ = try await legacySeed.initializeSource(fixture.initial);try await legacySeed.close()
    let legacyHeader=StoreIdentity(storeID:seeded.identity.storeID,kind:"document",generation:1,storeVersion:1,minimumReaderVersion:1,codecVersion:1,documentID:document)
    let legacyFolder=legacyRoot.appendingPathComponent("Documents").appendingPathComponent(document)
    let legacyDatabase=legacyFolder.appendingPathComponent("Generations/000001/authority.sqlite")
    try await Task.detached {
      let writer=try SQLiteDatabase(url:legacyDatabase);defer {try? writer.close()};_ = try writer.configure()
      try writer.transaction {try writer.statement("UPDATE store_header SET store_version=1,minimum_reader_version=1,codec_version=1");try writer.statement("PRAGMA user_version=1")}
      try ManagedFiles.publish(StoreSelector(formatVersion:1,storeID:legacyHeader.storeID,generation:1,headerHash:try legacyHeader.hash),to:legacyFolder.appendingPathComponent("active.json"),faults:.init())
    }.value
    let (legacy,_)=try await StorageService.open(paths:.init(root:legacyRoot,channel:.preview))
    let legacyOpened=try await legacy.openDocument(document);precondition(legacyOpened.identity.storeVersion==1)
    _ = try await legacy.commitSource(fixture.plans[0])
    do {_ = try await legacy.compactSourceHistory(document:document);fatalError("legacy document was silently compacted")}
    catch StorageError.unsupportedVersion { }
    let legacyRead=try await legacy.sourceTransactions(document:document,ids:["tx-1"]);precondition(legacyRead.count==1)
    try await legacy.close()
    try await Task.detached {
      let writer=try SQLiteDatabase(url:legacyDatabase);defer {try? writer.close()};_ = try writer.configure()
      try writer.transaction {try writer.statement("PRAGMA user_version=4")}
    }.value
    let futureBytes=try Data(contentsOf:legacyDatabase)
    let (future,_)=try await StorageService.open(paths:.init(root:legacyRoot,channel:.preview))
    do {_ = try await future.openDocument(document);fatalError("future document format was accepted")}
    catch StorageError.unsupportedVersion { }
    let afterFuture=try Data(contentsOf:legacyDatabase);precondition(futureBytes==afterFuture);try await future.close()
    if CommandLine.arguments.count>3 {
      let report:[String:Any]=["task":"R4.1.07","development_only":true,"fixture_root":root.path,
        "source_commits":206,"retained":201,"pruned":5,"recent_window":200,"version_3_document":true,
        "atomic_prune_rollback":true,"postcommit_prune_unknown":true,"payload_removal_verified":true,
        "original_receipt_preserved":true,"original_id_no_replay":true,"persisted_user_task_pins":true,
        "unpin_allows_compaction":true,"damaged_tombstone_rejected":true,"legacy_native_v1_full_only":true,
        "future_document_readonly_rejected":true,"final_candidate_gate":false]
      try JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[3]),options:.atomic)
    }
    print("Actual 206 SQLite source commits: 200 + persistent pins, atomic prune rollback/lost ACK, physical removal, original-ID facts/no replay, reopen/unpin/damage refusal, legacy v1 full-only and future version readonly refusal; counts 206/206/206/201/5 passed")
  }
}
