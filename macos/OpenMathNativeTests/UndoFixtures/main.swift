import AppKit
import CryptoKit
import Foundation
private struct Fixture:Decodable {let initial:NativeSourceSnapshot}
private actor FaultOnce {
  var first=true
  func consume()->Bool {let result=first;first=false;return result}
}
private final class ReadPause:@unchecked Sendable {
  let reached=DispatchSemaphore(value:0),release=DispatchSemaphore(value:0)
  func hold() {reached.signal();release.wait()}
  func awaitReached() async ->Bool {await withCheckedContinuation {continuation in DispatchQueue.global(qos:.utility).async {continuation.resume(returning:self.reached.wait(timeout:.now()+5) == .success)}}}
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
    unknownNative.close();freshEditor.reflectAcknowledgedSource()
    let textUndo=UndoCoordinator {ids,group,operation in try await freshPort.undo(transactions:ids,group:group,operation:operation)}
    let binding=EditorCommitBinding(editor:freshEditor,port:freshPort,undo:textUndo)
    let textBefore=freshEditor.textView.string
    freshEditor.textView.setSelectedRange(NSRange(location:(textBefore as NSString).length,length:0))
    freshEditor.textView.insertText("🙂",replacementRange:NSRange(location:NSNotFound,length:0))
    precondition(freshDrafts.overlay(initial.file.cells[0].id)!.source==textBefore+"🙂")
    precondition(freshEditor.textView.undoManager!.canUndo)
    freshEditor.textView.undoManager!.undo()
    precondition(freshDrafts.overlay(initial.file.cells[0].id)!.source==textBefore && freshEditor.textView.undoManager!.canRedo)
    freshEditor.textView.undoManager!.redo()
    precondition(freshDrafts.overlay(initial.file.cells[0].id)!.source==textBefore+"🙂")
    let nativeGroup=freshEditor.inputGroup
    let textCommit=try await binding.flush()!
    let textTransaction=textCommit.receipt.value!.receipt.transaction_id.value!
    let textRecord=try await reopened.sourceTransactions(document:initial.document_id,ids:[textTransaction])
    precondition(textRecord.first?.plan.input_group_id==nativeGroup)
    freshEditor.textView.undoManager!.undo();try await settle(textUndo);freshEditor.reflectAcknowledgedSource()
    precondition(textUndo.phase == .ready && freshEditor.textView.string==textBefore)
    freshEditor.textView.undoManager!.redo();try await settle(textUndo);freshEditor.reflectAcknowledgedSource()
    precondition(freshEditor.textView.string==textBefore+"🙂")
    freshEditor.textView.setSelectedRange(NSRange(location:(freshEditor.textView.string as NSString).length,length:0))
    freshEditor.textView.setMarkedText("中",selectedRange:NSRange(location:1,length:0),replacementRange:NSRange(location:NSNotFound,length:0))
    let imeGroup=freshEditor.inputGroup
    freshEditor.textView.setMarkedText("中文",selectedRange:NSRange(location:2,length:0),replacementRange:NSRange(location:NSNotFound,length:0))
    precondition(freshEditor.inputGroup==imeGroup && !freshEditor.textView.undoManager!.canUndo)
    do {_ = try await binding.flush();fatalError("binding committed marked input")}
    catch CommitPortError.editingBusy { }
    freshEditor.textView.unmarkText();_ = try await binding.flush()
    precondition(freshEditor.inputGroup==imeGroup)
    let history=try await reopened.sourceHistory(document:initial.document_id)
    precondition(history.first?.inputGroup==imeGroup)
    let pause=ReadPause(),paused=StorageFaults {point in if point=="source_transactions_readback" {pause.hold()} }
    let beforeStop=freshDrafts.confirmed.snapshot_hash
    let preparing=Task {try await freshPort.undo(transactions:[history.first!.transaction],group:"early-stop",operation:"preparing-stop",faults:paused)}
    let reached=await pause.awaitReached();precondition(reached)
    let currentOps=await freshPort.activeOperations();precondition(currentOps==["preparing-stop"])
    await freshPort.cancel("preparing-stop");pause.release.signal()
    let early=try await preparing.value
    precondition(early.phase == .cancelled && freshDrafts.confirmed.snapshot_hash==beforeStop)
    let closePause=ReadPause(),closingFault=StorageFaults {point in if point=="source_transactions_readback" {closePause.hold()} }
    let duringClose=Task {try await freshPort.undo(transactions:[history.first!.transaction],group:"early-close",operation:"preparing-close",faults:closingFault)}
    let closeReached=await closePause.awaitReached();precondition(closeReached)
    let closeTask=Task {await freshPort.close()};await Task.yield();closePause.release.signal()
    do {_ = try await duringClose.value;fatalError("closed port admitted prepared source IO")}
    catch StorageError.closing { }
    await closeTask.value
    textUndo.close()
    let finalRevision=freshDrafts.confirmed.revision.value
    let database=root.appendingPathComponent("Documents").appendingPathComponent(initial.document_id).appendingPathComponent("Generations").appendingPathComponent(String(format:"%06lld",opened.identity.generation)).appendingPathComponent("authority.sqlite")
    let counts=try await Task.detached {
      let db=try SQLiteDatabase(url:database,readonly:true);defer {try? db.close()}
      return [try db.integer("SELECT COUNT(*) FROM transactions"),try db.integer("SELECT COUNT(*) FROM operations"),try db.integer("SELECT COUNT(*) FROM outbox"),try db.integer("SELECT COUNT(*) FROM source_admissions WHERE phase='completed'")]
    }.value
    precondition(counts.allSatisfy{$0==Int64(finalRevision)})
    await freshPort.close();try await fresh.close();try await reopened.close()
    let (agedStore,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let agedInfo=try await agedStore.openDocument(initial.document_id)
    let agedHead=try await agedStore.committedSource(initial.document_id)!
    let agedClient=try await NativeHostClient.open(runtime:"undo-aged")
    let agedOpen=try await agedClient.sourceCommand(.source_open(.init(type:.source_open,snapshot:agedHead,store_id:agedInfo.identity.storeID,calculation:.init(nil),config_revision:try .init(0))))
    let agedClock=SourceOwnerClock();agedClock.calibrate(agedOpen.owner_time_ms.value)
    let agedDrafts=DraftStore(source:agedHead,runtime:agedClient.runtimeInstanceID,generation:agedOpen.document_generation.value,clock:{agedClock.now()})
    let agedEditor=try DraftTextViewAdapter(store:agedDrafts,cell:initial.file.cells[0].id,generation:1)
    let agedPort=DocCommitPort(client:agedClient,storage:agedStore,drafts:agedDrafts,clock:agedClock)
    for n in 1...205 {agedEditor.textView.string="let rollover=\(n)";try agedEditor.reportCurrentInput();try await agedPort.synchronizeDrafts()}
    let agedHash=agedDrafts.confirmed.snapshot_hash
    let compact=try await agedStore.compactSourceHistory(document:initial.document_id)
    precondition(compact.retained==200 && compact.transactions>0)
    do {_ = try await agedStore.sourceTransactions(document:initial.document_id,ids:[undoTx]);fatalError("old full inverse still present after physical retention")}
    catch StorageError.transactionUnavailable { }
    let compactDuplicate=try await agedPort.undo(transactions:originalGroup,group:group,operation:operation)
    precondition(compactDuplicate.phase == .completed && agedDrafts.confirmed.snapshot_hash==agedHash)
    await agedPort.close();try await agedClient.close();try await agedStore.close()
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
        "persistent_200_retention_complete":false,"editor_auto_text_groups_complete":true,
        "native_draft_undo_redo":true,"unicode_native_input":true,"ime_one_group":true,
        "input_group_bound_to_stored_request":true,"stop_before_preparation_no_source_effect":true,
        "close_waited_for_preparation":true,"old_undo_duplicate_after_payload_prune":true,
        "retained_after_rollover":compact.retained,"pruned_after_rollover":compact.transactions]
      try JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[3]),options:.atomic)
    }
    print("Actual native undo: 2 stored transactions -> 1 inverse COMMIT; original-ID duplicate; redo inverse; later manual conflict; lost ACK recovery; fresh runtime readback; real UndoManager grouping/echo/pending/undo/redo/failed-command retention passed")
  }
}
