import CryptoKit
import Darwin
import Foundation
import SQLite3

private struct Plans:Decodable {let initial:NativeSourceSnapshot,first:NativeSourceCommit,second:NativeSourceCommit}
private final class Pause:@unchecked Sendable {
  let reached=DispatchSemaphore(value:0),release=DispatchSemaphore(value:0)
  func fault(_ point:String)throws {if point=="blob_stage_opened" {reached.signal();release.wait()}}
  func waitForArrival() {reached.wait()}
}
@main struct MaintenanceFixtures {
  static func worker<T:Sendable>(_ job:@escaping @Sendable ()throws->T) async throws->T {try await Task.detached(operation:job).value}
  private static func prepare(_ root:URL,_ plans:Plans) async throws->StorageService {
    let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    _ = try await store.openDocument(plans.initial.document_id);_ = try await store.initializeSource(plans.initial)
    _ = try await store.commitSource(plans.first)
    _ = try await store.createAgentSession("maintenance-session")
    let ledger=SessionLedgerPort(storage:store,session:"maintenance-session",sourceInstance:"maintenance-host")
    for index in 0..<24 {
      let call=SessionToolCall(callID:"call-\(index)",toolName:"notebook.apply_patch",arguments:Data("{}".utf8))
      _ = try await ledger.record(SessionAssistantEnd(codecVersion:1,messageID:"message-\(index)",textUTF8:Data("原会话🙂".utf8),calls:[call]),id:"event-\(index)",kind:.assistant_end,turn:"turn-\(index)",task:"task")
    }
    for entry in try await store.documentOutbox(document:plans.initial.document_id,kind:.source).entries {_ = try await store.projectDocumentEffect(entry.effect)}
    return store
  }
  static func age(_ root:URL,_ descriptor:BlobDescriptor)throws {
    try FileManager.default.setAttributes([.modificationDate:Date(timeIntervalSinceNow:-3*24*60*60)],ofItemAtPath:root.appendingPathComponent("Blobs/sha256/"+descriptor.hash.prefix(2)+"/"+descriptor.hash).path)
  }
  static func main() async throws {
    precondition(om_fixture_install_sync_probe()==SQLITE_OK)
    let root=URL(fileURLWithPath:CommandLine.arguments[1]),plans=try JSONDecoder().decode(Plans.self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[2])))
    if CommandLine.arguments.count>3 {
      let mode=CommandLine.arguments[3]
      if mode=="inspect" {
        let store=try await StorageService.openWorkspace(paths:.init(root:root,channel:.preview))
        let manifests=try await worker {try BackupService.allManifests(root:root)}
        let selected=try await worker {try StoreCatalog.stores(root)}
        let result:[String:Any]=["backups":manifests.count,"generations":selected.map{$0.identity.generation}]
        print(String(decoding:try JSONSerialization.data(withJSONObject:result,options:[.sortedKeys]),as:UTF8.self));try await store.close();return
      }
      if mode=="init" || mode=="gc-init" {
        let store=try await prepare(root,plans)
        if mode=="gc-init" {
          let orphan=try await store.publishBlob(data:Data("GC crash orphan".utf8));try await store.releaseBlob(orphan);try age(root,orphan.descriptor)
          try JSONEncoder().encode(orphan.descriptor).write(to:root.appendingPathComponent("fixture-orphan.json"))
        }
        try await store.close();print("initialized");return
      }
      if mode=="restore-init" {
        let store=try await prepare(root,plans);let backup=try await store.createBackup();try Data(backup.backupID.utf8).write(to:root.appendingPathComponent("fixture-backup-id"));try await store.close();print("initialized");return
      }
      if mode=="gc-resume" {
        let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));let result=try await store.collectBlobs();print(result.removed);try await store.close();return
      }
      let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
      let faults=StorageFaults {point in if point==mode {FileHandle.standardOutput.write(Data((point+"\n").utf8));raise(SIGSTOP)}}
      if mode.hasPrefix("generation.") {
        let id=String(decoding:try Data(contentsOf:root.appendingPathComponent("fixture-backup-id")),as:UTF8.self)
        _ = try await store.restoreBackup(id,document:plans.initial.document_id,faults:faults)
      } else if mode.hasPrefix("gc.") {_ = try await store.collectBlobs(faults:faults)}
      else {_ = try await store.createBackup(faults:faults)}
      fatalError("crash boundary not reached")
    }
    let store=try await prepare(root,plans),id=plans.initial.document_id
    let wal=root.appendingPathComponent("Documents/"+id+"/Generations/000001/authority.sqlite-wal")
    precondition(FileManager.default.fileExists(atPath:wal.path))
    let original=Data("original attachment with a durable owner🙂".utf8),owned=try await store.publishBlob(data:original)
    _ = try await store.attachBlob(owned,owner:.init(kind:.attachment,id:"owned-original"),mediaType:"text/plain",codec:"raw-v1",document:id)
    try await store.releaseBlob(owned);try await store.closeDocument(id)
    let pinned=try await store.publishBlob(data:Data("running producer".utf8))
    let readerPublication=try await store.publishBlob(data:Data("running reader".utf8)),reader=try await store.openBlob(readerPublication)
    try await store.releaseBlob(readerPublication)
    let orphan=try await store.publishBlob(data:Data("old complete unreferenced resource".utf8));try await store.releaseBlob(orphan)
    let young=try await store.publishBlob(data:Data("young unreferenced resource".utf8));try await store.releaseBlob(young)
    for descriptor in [owned.descriptor,pinned.descriptor,orphan.descriptor] {try age(root,descriptor)}
    let backup=try await store.createBackup()
    precondition(backup.stores.count==2 && backup.blobs==[owned.descriptor])
    let verified=try await store.verifyBackup(backup.backupID);precondition(verified.backupID==backup.backupID)
    let canonical=try Data(contentsOf:root.appendingPathComponent("Backups/"+backup.backupID+"/manifest.json"))
    let manifestURL=root.appendingPathComponent("Backups/"+backup.backupID+"/manifest.json")
    var invalid=try JSONSerialization.jsonObject(with:canonical) as! [String:Any];invalid["verified"]=false
    try JSONSerialization.data(withJSONObject:invalid).write(to:manifestURL)
    do {_ = try await store.verifyBackup(backup.backupID);fatalError("unverified canonical manifest accepted")}
    catch StorageError.corruptIdentity {}
    try canonical.write(to:manifestURL)
    let evidence=URL(fileURLWithPath:FileManager.default.currentDirectoryPath).appendingPathComponent("target/acceptance/pre-alpha.4/development/r41")
    try FileManager.default.createDirectory(at:evidence,withIntermediateDirectories:true)
    try (Data("[".utf8)+canonical+Data("]".utf8)).write(to:evidence.appendingPathComponent("backup-canonical.json"))
    _ = try await store.openDocument(id);_ = try await store.commitSource(plans.second);try await store.closeDocument(id)
    let staleStaging=root.appendingPathComponent("Staging/blob-"+UUID().uuidString.lowercased())
    try Data("unpublished staging prefix".utf8).write(to:staleStaging)
    try FileManager.default.setAttributes([.modificationDate:Date(timeIntervalSinceNow:-3*24*60*60)],ofItemAtPath:staleStaging.path)
    let collected=try await store.collectBlobs();precondition(collected.removed==1 && collected.removedBytes==orphan.descriptor.byteLength && collected.pinned>=1 && collected.referenced>=1 && collected.young>=1 && !FileManager.default.fileExists(atPath:staleStaging.path))
    let ownedState=try await store.inspectBlob(owned.descriptor);precondition(ownedState.state == .orphanCandidate && !ownedState.allStoresChecked)
    let readerBytes=try await store.readBlob(reader,offset:0,count:1024);precondition(readerBytes==Data("running reader".utf8))
    let cachePin=try await store.cachePin("producer-cache")
    _ = try await store.cachePut("producer-cache",bytes:Data("pinned derived bytes".utf8))
    _ = try await store.cachePut("cold-cache",bytes:Data("cold derived bytes".utf8))
    let prune=try await store.pruneCache(budget:0);precondition(prune.removed==1 && prune.overBudget)
    let retainedCache=try await store.cachedBytes("producer-cache");precondition(retainedCache==Data("pinned derived bytes".utf8))
    let absentCache=try await store.cachedBytes("cold-cache");precondition(absentCache==nil)
    try await store.releaseCache(cachePin)
    let cleared=try await store.pruneCache(budget:0);precondition(cleared.removed==1 && !cleared.overBudget)
    let originalInspection=try await store.inspectBlob(owned.descriptor);precondition(originalInspection.state != .missing)
    try await store.closeBlob(reader);try await store.releaseBlob(pinned)
    // A maintenance request drains an already copying publication. New admission is rejected
    // while it waits, and the old job still receives a real publication pin before sweeping.
    let pause=Pause()
    let pending=Task {try await store.publishBlob(data:Data("in-flight upload".utf8),faults:.init {try pause.fault($0)})}
    await Task.detached {pause.waitForArrival()}.value
    let sweep=Task {try await store.collectBlobs()}
    let deadline=ContinuousClock.now.advanced(by:.seconds(10))
    while !(await store.maintenanceInProgress()) {precondition(ContinuousClock.now<deadline);await Task.yield()}
    do {_ = try await store.publishBlob(data:Data("must wait".utf8));fatalError("new admission crossed maintenance barrier")}
    catch StorageError.transactionUnavailable {}
    pause.release.signal();let live=try await pending.value;_ = try await sweep.value
    let liveState=try await store.inspectBlob(live.descriptor);precondition(liveState.state == .pinned);try await store.releaseBlob(live)
    // A lost backup acknowledgement preserves the published healthy backup, never claims failure
    // removed it. A real destination SQLite ENOSPC leaves only unpublished staging resources.
    let noSpace=StorageFaults {point in if point=="backup.started" {om_fixture_fail_writes(1)}}
    do {_ = try await store.createBackup(faults:noSpace);fatalError("SQLite ENOSPC returned a backup")}
    catch StorageError.sqlite(let code,_) {precondition(code&255==SQLITE_FULL || code&255==SQLITE_IOERR)}
    om_fixture_fail_writes(0)
    let stillHealthy=try await store.verifyBackup(backup.backupID);precondition(stillHealthy.backupID==backup.backupID)
    let unknown=StorageFaults {point in if point=="backup.published" {throw StorageError.system(EIO,"lost-backup-ack")}}
    do {_ = try await store.createBackup(faults:unknown);fatalError("lost ack returned success")} catch StorageError.system {}
    let retained=try await worker {try BackupService.allManifests(root:root)};precondition(retained.count==2)
    // Downgrade only this synthetic library to the real preceding schema, preserving all event
    // bytes. Explicit upgrade builds its index in a new generation and preserves old receipts.
    try await store.close()
    let oldLibrary=root.appendingPathComponent("Library/Generations/000001/library.sqlite")
    try await worker {
      let db=try SQLiteDatabase(url:oldLibrary);defer {try? db.close()}
      try db.statement("DROP TABLE session_request_index");try db.statement("DROP TABLE session_tool_index");try db.statement("DROP TABLE session_event_links")
      try db.statement("UPDATE store_header SET store_version=3,minimum_reader_version=3,codec_version=3");try db.statement("PRAGMA user_version=3")
      let identity=try StorageBootstrap.readHeader(db);try db.close()
      try ManagedFiles.publish(StoreSelector(formatVersion:1,storeID:identity.storeID,generation:1,headerHash:identity.hash),to:root.appendingPathComponent("Library/active.json"),faults:.init())
    }
    let originalHash=try await worker {let db=try SQLiteDatabase(url:oldLibrary,readonly:true);defer {try? db.close()};return try StoreGenerationService.tableHash(db,"session_events")}
    let (legacy,_) = try await StorageService.open(paths:.init(root:root,channel:.preview))
    let upgraded=try await legacy.upgradeStore();precondition(upgraded.generation==2 && upgraded.storeVersion==4)
    let page=try await legacy.sessionRecovery("maintenance-session",limit:32);precondition(page.complete && page.tools.count==24)
    try await legacy.close()
    let afterHash=try await worker {let db=try SQLiteDatabase(url:oldLibrary,readonly:true);defer {try? db.close()};let identity=try StorageBootstrap.readHeader(db);precondition(identity.storeVersion==3 && identity.generation==1);return try StoreGenerationService.tableHash(db,"session_events")};precondition(originalHash==afterHash)
    let (restoring,_) = try await StorageService.open(paths:.init(root:root,channel:.preview))
    let restored=try await restoring.restoreBackup(backup.backupID,document:id);precondition(restored.generation==2)
    do {_ = try await restoring.openDocument(id);fatalError("rollback quarantine acquired a writer")}
    catch StorageError.rollbackQuarantine {}
    let readOnly=try await restoring.quarantinedSource(document:id);precondition(readOnly?.snapshot_hash==plans.first.after.snapshot_hash)
    let sourceFile=root.appendingPathComponent("Documents/"+id+"/Generations/000002/authority.sqlite")
    let before=try await worker {try BackupService.digest(sourceFile)}
    try await worker {let db=try SQLiteDatabase(url:sourceFile,readonly:true);defer {try? db.close()};var found=false;try db.statement("SELECT 1 FROM operations WHERE operation_id=?",[.text(plans.second.commit.operation_id)]) {_ in found=true};precondition(!found)}
    do {_ = try await restoring.commitSource(plans.second);fatalError("lost post-backup operation was replayed")}
    catch StorageError.closing {}
    do {_ = try await restoring.upgradeStore(document:id);fatalError("quarantine bypassed by upgrade")}
    catch StorageError.rollbackQuarantine {}
    let retention=try await restoring.retainBackups();precondition(retention.removed==0 && retention.retained>=3)
    for _ in 0..<7 {_ = try await restoring.createBackup()}
    let limited=try await restoring.retainBackups();precondition(limited.removed>=1 && limited.retained>=8 && limited.retained<=9 && limited.removed+limited.retained==10)
    let keptBackups=try await worker {try BackupService.allManifests(root:root)}
    precondition(keptBackups.contains{$0.stores.allSatisfy{$0.tableCounts["store_recovery_state"]==nil}})
    try await restoring.close()
    let after=try await worker {try BackupService.digest(sourceFile)};precondition(before==after)
    let currentLibrary=root.appendingPathComponent("Library/Generations/000002/library.sqlite")
    try await worker {let db=try SQLiteDatabase(url:currentLibrary);defer {try? db.close()};try db.statement("PRAGMA user_version=99")}
    let degraded=try await StorageService.openWorkspace(paths:.init(root:root,channel:.preview))
    let independent=try await degraded.quarantinedSource(document:id);precondition(independent?.snapshot_hash==plans.first.after.snapshot_hash)
    let usage=try await degraded.storageUsage();precondition(usage.backupBytes>0 && usage.totalBytes>=usage.backupBytes && usage.space.available>0)
    try await degraded.close()
    let legacyRoot=root.appendingPathComponent("fixture-native-legacy")
    let oldDocumentStore=try await prepare(legacyRoot,plans);try await oldDocumentStore.close()
    let legacyDatabase=legacyRoot.appendingPathComponent("Documents/"+id+"/Generations/000001/authority.sqlite")
    try await worker {
      let db=try SQLiteDatabase(url:legacyDatabase);defer {try? db.close()}
      for table in ["draft_state","draft_batches","draft_owners","file_head","save_outbox","save_receipts","save_intents","file_bindings","kernel_head","kernel_outbox","kernel_transitions","kernel_operations","accepted_results","accepted_checkpoints","source_history_pins","source_history"] {try db.statement("DROP TABLE \(table)")}
      try db.statement("UPDATE store_header SET store_version=1,minimum_reader_version=1,codec_version=1");try db.statement("PRAGMA user_version=1")
      let identity=try StorageBootstrap.readHeader(db);try db.close()
      try ManagedFiles.publish(StoreSelector(formatVersion:1,storeID:identity.storeID,generation:1,headerHash:identity.hash),to:legacyDatabase.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("active.json"),faults:.init())
    }
    let (nativeLegacy,_) = try await StorageService.open(paths:.init(root:legacyRoot,channel:.preview))
    let legacyInfo=try await nativeLegacy.openDocument(id);precondition(legacyInfo.identity.storeVersion==1 && legacyInfo.identity.generation==1)
    try await nativeLegacy.closeDocument(id)
    let migrated=try await nativeLegacy.upgradeStore(document:id);precondition(migrated.storeVersion==5 && migrated.generation==2)
    _ = try await nativeLegacy.openDocument(id)
    let oldReceipt=try await nativeLegacy.sourceReceipt(document:id,operation:plans.first.commit.operation_id)
    precondition(oldReceipt?.receipt.request_hash==plans.first.commit.request_hash)
    let oldHistory=try await nativeLegacy.sourceHistory(document:id);precondition(oldHistory.count==1 && oldHistory[0].operation==plans.first.commit.operation_id)
    let oldEffect=try await nativeLegacy.documentOutbox(document:id,kind:.source).entries[0].effect
    let repeatedProjection=try await nativeLegacy.projectDocumentEffect(oldEffect);precondition(!repeatedProjection.inserted)
    try await nativeLegacy.closeDocument(id)
    let blockedOrphan=try await nativeLegacy.publishBlob(data:Data("cross-store failure retains old orphan".utf8));try await nativeLegacy.releaseBlob(blockedOrphan);try age(legacyRoot,blockedOrphan.descriptor)
    let migratedDatabase=legacyRoot.appendingPathComponent("Documents/"+id+"/Generations/000002/authority.sqlite")
    try await worker {let db=try SQLiteDatabase(url:migratedDatabase);defer {try? db.close()};try db.statement("PRAGMA user_version=99")}
    do {_ = try await nativeLegacy.collectBlobs();fatalError("GC ignored unsupported unopened store")}
    catch StorageError.unsupportedVersion {}
    let protectedOrphan=try await nativeLegacy.inspectBlob(blockedOrphan.descriptor);precondition(protectedOrphan.state != .missing && !protectedOrphan.allStoresChecked)
    try await nativeLegacy.close()
    let report:[String:Any]=["backup_id":backup.backupID,"stores":backup.stores.count,"blobs":backup.blobs.count,"gc_removed":collected.removed,"library_generation":upgraded.generation,"restored_generation":restored.generation,"rollback_quarantine":true,"actual_fullsync_calls":om_fixture_fullsync_calls()]
    print(String(decoding:try JSONSerialization.data(withJSONObject:report,options:[.sortedKeys]),as:UTF8.self))
    print("Actual WAL backup/unopened refs/pins/reader/24h grace/maintenance drain/SQLite ENOSPC/lost ACK/Library3 indexed migration/readonly rollback quarantine passed")
  }
}
