import AppKit
import CryptoKit
import Foundation
private struct Fixture:Decodable {let initial:NativeSourceSnapshot}
private actor FaultOnce {
  var first=true
  func consume()->Bool {let result=first;first=false;return result}
}
@main struct UndoFixtures {
  @MainActor static func main() async throws {
    let initial=try JSONDecoder().decode(Fixture.self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[1]))).initial
    let root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let opened=try await storage.openDocument(initial.document_id);_ = try await storage.initializeSource(initial)
    let client=try await NativeHostClient.open(runtime:"undo-fixture")
    let attached=try await client.sourceCommand(.source_open(.init(type:.source_open,snapshot:initial,store_id:opened.identity.storeID,calculation:.init(nil),config_revision:try .init(0))))
    let clock=SourceOwnerClock();clock.calibrate(attached.owner_time_ms.value)
    let drafts=DraftStore(source:initial,runtime:client.runtimeInstanceID,generation:attached.document_generation.value,clock:{clock.now()})
    let editor=try DraftTextViewAdapter(store:drafts,cell:initial.file.cells[0].id,generation:1)
    let port=DocCommitPort(client:client,storage:storage,drafts:drafts,clock:clock)
    func apply(_ source:String) async throws ->NativeCommitState {
      let cell=drafts.confirmed.file.cells[0]
      let hash=SHA256.hash(data:Data(cell.source.utf8)).map{String(format:"%02x",$0)}.joined()
      let preview=try await port.preview(.preview_patch_input(.init(kind:.patch,operations:[.preview_update_cell(.init(type:.update_cell,target:.init(cell_id:cell.id),expected_source_hash:hash,source:source,kind:nil,dialect:nil))])))
      let result=try await port.apply(preview.preview_ref.value!);editor.reflectAcknowledgedSource();return result
    }
    let first=try await apply("let undo_probe=2"),second=try await apply("let undo_probe=5")
    let tx1=first.receipt.value!.receipt.transaction_id.value!,tx2=second.receipt.value!.receipt.transaction_id.value!
    let originalGroup=[tx1,tx2],operation="undo-original",group="native-input-group"
    let undone=try await port.undo(transactions:originalGroup,group:group,operation:operation)
    precondition(undone.phase == .completed && drafts.confirmed.revision.value==3)
    precondition(drafts.confirmed.file.cells[0].source.utf8.elementsEqual(initial.file.cells[0].source.utf8))
    let duplicate=try await port.undo(transactions:originalGroup,group:group,operation:operation)
    precondition(duplicate.operation_id==operation && drafts.confirmed.revision.value==3)
    do {_ = try await port.undo(transactions:[tx1],group:group,operation:operation);fatalError("same undo ID changed its transaction group")}
    catch StorageError.idempotencyConflict { }
    let undoTx=undone.receipt.value!.receipt.transaction_id.value!
    let redo=try await port.undo(transactions:[undoTx],group:"redo-group",operation:"redo-original")
    precondition(redo.phase == .completed && drafts.confirmed.file.cells[0].source=="let undo_probe=5")
    editor.reflectAcknowledgedSource()
    editor.textView.string="later user input";try editor.reportCurrentInput();try await port.synchronizeDrafts()
    let revision=drafts.confirmed.revision.value
    do {_ = try await port.undo(transactions:[tx2],group:"conflicting-group",operation:"conflicting-undo");fatalError("undo overwrote later manual input")}
    catch NativeHostClientError.rejected(let error) {precondition(error.code == .editing_busy)}
    precondition(drafts.confirmed.revision.value==revision && drafts.confirmed.file.cells[0].source=="later user input")
    let changed=try await apply("let undo_probe=8")
    let changedTx=changed.receipt.value!.receipt.transaction_id.value!
    let lost=StorageFaults {point in if point=="source_committed" {throw StorageError.system(EIO,"undo-actual-ack-lost")} }
    do {_ = try await port.undo(transactions:[changedTx],group:"lost-group",operation:"lost-undo",faults:lost);fatalError("lost undo ACK reported success")}
    catch CommitPortError.unknownOutcome { }
    let actual=try await port.reconcile("lost-undo")
    precondition(actual.phase == .completed && drafts.confirmed.file.cells[0].source=="later user input")
    let confirmed=drafts.confirmed
    await port.close();try await client.close();try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let reopenInfo=try await reopened.openDocument(initial.document_id)
    let head=try await reopened.committedSource(initial.document_id)!
    precondition(head.snapshot_hash==confirmed.snapshot_hash)
    let fresh=try await NativeHostClient.open(runtime:"undo-restart")
    let read=try await fresh.sourceCommand(.source_open(.init(type:.source_open,snapshot:head,store_id:reopenInfo.identity.storeID,calculation:.init(nil),config_revision:try .init(0))))
    let freshClock=SourceOwnerClock();freshClock.calibrate(read.owner_time_ms.value)
    let freshDrafts=DraftStore(source:head,runtime:fresh.runtimeInstanceID,generation:read.document_generation.value,clock:{freshClock.now()})
    let freshPort=DocCommitPort(client:fresh,storage:reopened,drafts:freshDrafts,clock:freshClock)
    let originalAfterRestart=try await freshPort.undo(transactions:originalGroup,group:group,operation:operation)
    let lostAfterRestart=try await freshPort.undo(transactions:[changedTx],group:"lost-group",operation:"lost-undo")
    precondition(originalAfterRestart.phase == .completed && lostAfterRestart.phase == .completed)
    precondition(freshDrafts.confirmed.snapshot_hash==head.snapshot_hash)
    let freshEditor=try DraftTextViewAdapter(store:freshDrafts,cell:initial.file.cells[0].id,generation:1)
    func freshApply(_ source:String) async throws ->NativeCommitState {
      let cell=freshDrafts.confirmed.file.cells[0],hash=SHA256.hash(data:Data(freshDrafts.confirmed.file.cells[0].source.utf8)).map{String(format:"%02x",$0)}.joined()
      let preview=try await freshPort.preview(.preview_patch_input(.init(kind:.patch,operations:[.preview_update_cell(.init(type:.update_cell,target:.init(cell_id:cell.id),expected_source_hash:hash,source:source,kind:nil,dialect:nil))])))
      let result=try await freshPort.apply(preview.preview_ref.value!);freshEditor.reflectAcknowledgedSource();return result
    }
    func settle(_ coordinator:UndoCoordinator) async throws {
      let deadline=DispatchTime.now().uptimeNanoseconds+5_000_000_000
      while coordinator.phase == .pending {
        precondition(DispatchTime.now().uptimeNanoseconds<deadline);try await Task.sleep(for:.milliseconds(5))
      }
    }
    let native=UndoCoordinator {ids,group,operation in try await freshPort.undo(transactions:ids,group:group,operation:operation)}
    let n1=try await freshApply("let native_probe=2"),n2=try await freshApply("let native_probe=3")
    try native.registerCommitted(n1,group:"typing-group",name:"编辑数学单元格")
    try native.registerCommitted(n2,group:"typing-group",name:"编辑数学单元格")
    try native.registerCommitted(n2,group:"typing-group",name:"回显不得重复登记")
    var inconsistent=n2;inconsistent.request_hash=String(repeating:"0",count:64)
    inconsistent.receipt.value!.receipt.request_hash=inconsistent.request_hash
    do {try native.registerCommitted(inconsistent,group:"typing-group",name:"错误回显");fatalError("different-content echo was ignored")}
    catch StorageError.idempotencyConflict { }
    precondition(native.manager.canUndo && !native.manager.canRedo)
    native.manager.undo();precondition(native.phase == .pending && !native.manager.canUndo)
    try await settle(native)
    precondition(native.phase == .ready && freshDrafts.confirmed.file.cells[0].source=="later user input")
    precondition(native.manager.canRedo && !native.manager.canUndo)
    native.manager.redo();try await settle(native)
    precondition(native.phase == .ready && freshDrafts.confirmed.file.cells[0].source=="let native_probe=3" && native.manager.canUndo)
    freshEditor.reflectAcknowledgedSource()
    freshEditor.textView.string="later native user input";try freshEditor.reportCurrentInput();try await freshPort.synchronizeDrafts()
    let beforeConflict=freshDrafts.confirmed.snapshot_hash
    native.manager.undo();try await settle(native)
    precondition(native.phase == .failed && native.manager.canUndo && !native.manager.canRedo)
    precondition(freshDrafts.confirmed.snapshot_hash==beforeConflict)
    native.close()
    let unknownChange=try await freshApply("let unknown_probe=9"),once=FaultOnce()
    let unknownNative=UndoCoordinator {ids,group,operation in
      let lose=await once.consume()
      return try await freshPort.undo(transactions:ids,group:group,operation:operation,
        faults:lose ? StorageFaults {point in if point=="source_committed" {throw StorageError.system(EIO,"native-ui-undo-lost-ack")} } : .init())
    }
    try unknownNative.registerCommitted(unknownChange,group:"unknown-ui-group",name:"修改单元格")
    unknownNative.manager.undo();try await settle(unknownNative)
    precondition(unknownNative.phase == .unknown && !unknownNative.manager.canUndo && !unknownNative.manager.canRedo)
    let originalUnknown=unknownNative.lastOperation
    unknownNative.manager.undo();precondition(unknownNative.lastOperation==originalUnknown)
    unknownNative.reconcile();try await settle(unknownNative)
    precondition(unknownNative.phase == .ready && unknownNative.lastOperation==originalUnknown)
    precondition(freshDrafts.confirmed.file.cells[0].source=="later native user input" && unknownNative.manager.canRedo)
    let finalRevision=freshDrafts.confirmed.revision.value
    let database=root.appendingPathComponent("Documents").appendingPathComponent(initial.document_id).appendingPathComponent("Generations").appendingPathComponent(String(format:"%06lld",opened.identity.generation)).appendingPathComponent("authority.sqlite")
    let counts=try await Task.detached {
      let db=try SQLiteDatabase(url:database,readonly:true);defer {try? db.close()}
      return [try db.integer("SELECT COUNT(*) FROM transactions"),try db.integer("SELECT COUNT(*) FROM operations"),try db.integer("SELECT COUNT(*) FROM outbox"),try db.integer("SELECT COUNT(*) FROM source_admissions WHERE phase='completed'")]
    }.value
    precondition(counts.allSatisfy{$0==Int64(finalRevision)})
    unknownNative.close()
    await freshPort.close();try await fresh.close();try await reopened.close()
    if CommandLine.arguments.count>3 {
      let report:[String:Any]=["task":"R4.1.07","task_done":false,"development_only":true,"final_candidate_gate":false,
        "fixture_root":root.path,"document_id":initial.document_id,"store_id":opened.identity.storeID,"final_revision":finalRevision,
        "transactions":counts[0],"operations":counts[1],"outbox":counts[2],"completed_admissions":counts[3],
        "source_group_inverse_one_commit":true,"source_duplicate_id_one_effect":true,"wrong_group_same_id_rejected":true,
        "later_manual_conflict_no_effect":true,"actual_redo_inverse":true,"undo_ack_lost_reconciled":true,
        "fresh_runtime_original_id_readback":true,"real_undo_manager":true,"closed_group_name_fault_fixed":true,
        "same_group_coalesced":true,"echo_deduplicated":true,"different_hash_echo_rejected":true,
        "pending_command_not_consumed":true,"failed_command_retained":true,"unknown_command_fenced":true,
        "unknown_ui_reconciles_original_id":true,"unknown_ui_operation":originalUnknown ?? "",
        "persistent_200_retention_complete":false,"editor_auto_text_groups_complete":false]
      try JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[3]),options:.atomic)
    }
    print("Actual native undo: 2 stored transactions -> 1 inverse COMMIT; original-ID duplicate; redo inverse; later manual conflict; lost ACK recovery; fresh runtime readback; real UndoManager grouping/echo/pending/undo/redo/failed-command retention passed")
  }
}
