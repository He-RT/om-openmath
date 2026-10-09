import AppKit
import CryptoKit
import Foundation
import SQLite3
private struct SourceFixture:Decodable {let initial:NativeSourceSnapshot}
private final class Pause:@unchecked Sendable {
  let reached=DispatchSemaphore(value:0),release=DispatchSemaphore(value:0)
  func stop() {reached.signal();release.wait()}
  func awaitReached() async ->Bool {await withCheckedContinuation { continuation in DispatchQueue.global(qos:.utility).async {continuation.resume(returning:self.reached.wait(timeout:.now()+5) == .success)} }}
}
@main struct CommitFixtures {
  @MainActor static func main() async throws {
    let initial=try JSONDecoder().decode(SourceFixture.self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[1]))).initial
    let root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let opened=try await storage.openDocument(initial.document_id)
    _ = try await storage.initializeSource(initial)
    let client=try await NativeHostClient.open(runtime:"commit-fixture")
    let attached=try await client.sourceCommand(.source_open(.init(type:.source_open,snapshot:initial,store_id:opened.identity.storeID,calculation:.init(nil),config_revision:try .init(0))))
    let clock=SourceOwnerClock();clock.calibrate(attached.owner_time_ms.value)
    let drafts=DraftStore(source:initial,runtime:"commit-fixture",generation:attached.document_generation.value,clock:{clock.now()})
    let editor=try DraftTextViewAdapter(store:drafts,cell:initial.file.cells[0].id,generation:1)
    let port=DocCommitPort(client:client,storage:storage,drafts:drafts,clock:clock)
    func input(_ source:String)->NativePreviewInput {
      let confirmed=drafts.confirmed
      let cell=confirmed.file.cells[0]
      let expected=SHA256.hash(data:Data(cell.source.utf8)).map{String(format:"%02x",$0)}.joined()
      return .preview_patch_input(.init(kind:.patch,operations:[.preview_update_cell(.init(type:.update_cell,target:.init(cell_id:cell.id),expected_source_hash:expected,source:source,kind:nil,dialect:nil))]))
    }
    let preview=try await port.preview(input("let a=2; a+1"))
    let result=try await port.apply(preview.preview_ref.value!)
    precondition(result.phase == .completed && drafts.confirmed.revision.value==1)
    editor.reflectAcknowledgedSource()
    let repeated=try await port.apply(preview.preview_ref.value!)
    precondition(repeated.operation_id==result.operation_id && drafts.confirmed.revision.value==1)
    let originalReceipt=try await storage.sourceReceipt(document:initial.document_id,operation:result.operation_id)!
    var forgedReceipt=originalReceipt;forgedReceipt.snapshot_hash=String(repeating:"0",count:64)
    do {
      _ = try await client.sourceCommand(.source_completed(.init(type:.source_completed,operation_id:result.operation_id,receipt:forgedReceipt)))
      fatalError("duplicate receipt with different immutable facts was accepted")
    } catch NativeHostClientError.rejected { }
    // Real marked text makes synchronization busy, never committed or auto-accepted.
    editor.textView.setSelectedRange(NSRange(location:0,length:0))
    editor.textView.setMarkedText("中文",selectedRange:NSRange(location:2,length:0),replacementRange:NSRange(location:0,length:0))
    precondition(editor.textView.hasMarkedText() && drafts.overlay(initial.file.cells[0].id)!.composing)
    do {try await port.synchronizeDrafts();fatalError("IME draft was submitted")}
    catch CommitPortError.editingBusy { }
    do {try drafts.attachEditor(cell:initial.file.cells[0].id,generation:2);fatalError("active IME was remounted")}
    catch CommitPortError.editingBusy { }
    editor.textView.unmarkText();editor.textView.string="let a=3; a+1";try editor.reportCurrentInput()
    try await port.synchronizeDrafts()
    precondition(drafts.confirmed.file.cells[0].source=="let a=3; a+1")
    // Incomplete manual source is still the user's durable draft, never an executable preview.
    editor.textView.string="let a =";try editor.reportCurrentInput();try await port.synchronizeDrafts()
    precondition(drafts.confirmed.file.cells[0].source=="let a =")
    editor.textView.string="let a=3; a+1";try editor.reportCurrentInput();try await port.synchronizeDrafts()
    editor.textView.string="let a=8; a+1";try editor.reportCurrentInput()
    let ownLostFault=StorageFaults { point in if point=="source_committed" {throw StorageError.system(EIO,"test-own-draft-lost-ack")} }
    do {try await port.synchronizeDrafts(faults:ownLostFault);fatalError("own draft lost ACK reported success")}
    catch CommitPortError.unknownOutcome { }
    guard let ownID=await port.unresolvedOperation() else {fatalError("unknown manual commit lost original ID")}
    editor.textView.string="let a=9; a+1";try editor.reportCurrentInput()
    let ownAck=try await port.reconcile(ownID)
    precondition(ownAck.phase == .completed && drafts.confirmed.file.cells[0].source=="let a=8; a+1")
    let ownOverlay=drafts.overlay(initial.file.cells[0].id)!
    precondition(ownOverlay.source=="let a=9; a+1" && ownOverlay.baseSource=="let a=8; a+1" && !ownOverlay.conflict)
    editor.textView.string="let a=3; a+1";try editor.reportCurrentInput();try await port.synchronizeDrafts()
    let admissionPreview=try await port.preview(input("let a=31; a+1"))
    let admissionFault=StorageFaults { point in if point=="source_admission_committed" {throw StorageError.system(EIO,"test-admission-lost-ack")} }
    do {_ = try await port.apply(admissionPreview.preview_ref.value!,faults:admissionFault);fatalError("lost admission ACK claimed success")}
    catch CommitPortError.unknownOutcome { }
    let admissionPending=try await client.sourceCommand(.source_begin(.init(type:.source_begin,preview_ref:admissionPreview.preview_ref.value!)))
    let admissionID=admissionPending.plan.value!.commit.operation_id
    let durableAdmission=try await storage.sourceAdmission(document:initial.document_id,operation:admissionID)
    precondition(durableAdmission?.phase == .accepted && drafts.confirmed.file.cells[0].source=="let a=3; a+1")
    let noEffect=try await port.reconcile(admissionID)
    precondition(noEffect.phase == .failed)
    let admissionAgain=try await port.apply(admissionPreview.preview_ref.value!)
    precondition(admissionAgain.phase == .failed && admissionAgain.operation_id==admissionID)
    let lost=try await port.preview(input("let a=30; a+1"))
    let lostFault=StorageFaults { point in if point=="source_committed" {throw StorageError.system(EIO,"test-native-lost-ack")} }
    do {_ = try await port.apply(lost.preview_ref.value!,faults:lostFault);fatalError("lost actual ACK claimed success")}
    catch CommitPortError.unknownOutcome { }
    let hidden=try await client.sourceCommand(.source_begin(.init(type:.source_begin,preview_ref:lost.preview_ref.value!)))
    let lostID=hidden.plan.value!.commit.operation_id
    precondition(hidden.operation.value?.phase == .unknown)
    precondition(drafts.confirmed.file.cells[0].source=="let a=3; a+1")
    let recovered=try await port.reconcile(lostID)
    precondition(recovered.phase == .completed && drafts.confirmed.file.cells[0].source=="let a=30; a+1")
    editor.reflectAcknowledgedSource()
    let newPreview=try await port.preview(input("let a=4; a+1"))
    let pause=Pause()
    let fault=StorageFaults { point in if point=="before_source_commit" {pause.stop()} }
    let application=Task {try await port.apply(newPreview.preview_ref.value!,faults:fault)}
    let firstReached=await pause.awaitReached();precondition(firstReached)
    do {_ = try await port.apply(newPreview.preview_ref.value!);fatalError("in-flight duplicate was enqueued behind SQLite")}
    catch CommitPortError.operationInProgress { }
    let currentBefore=drafts.confirmed.snapshot_hash
    editor.textView.string="user newer draft";try editor.reportCurrentInput()
    // Native current source can still be read while SQLite is intentionally held by the fixture.
    let current=try await client.sourceCommand(.source_read(.init(type:.source_read)))
    precondition(current.snapshot.value?.snapshot_hash==currentBefore)
    pause.release.signal()
    do {_ = try await application.value;fatalError("new input before barrier was overwritten")}
    catch CommitPortError.editingBusy { }
    precondition(drafts.confirmed.snapshot_hash==currentBefore && drafts.overlay(initial.file.cells[0].id)!.source=="user newer draft")
    // Persist the new manual draft, then change it again after actual COMMIT but before ACK.
    editor.textView.string="let a=5; a+1";try editor.reportCurrentInput();try await port.synchronizeDrafts()
    let expiredPreview=try await port.preview(input("let a=50; a+1"))
    let expiryPause=Pause(),expiryFault=StorageFaults { point in if point=="before_source_commit" {expiryPause.stop()} }
    let expiry=Task {try await port.apply(expiredPreview.preview_ref.value!,faults:expiryFault)}
    let expiryReached=await expiryPause.awaitReached();precondition(expiryReached)
    try await Task.sleep(for:.milliseconds(1100));expiryPause.release.signal()
    do {_ = try await expiry.value;fatalError("expired native fence committed")}
    catch CommitPortError.editingBusy { }
    precondition(drafts.confirmed.file.cells[0].source=="let a=5; a+1")
    let cancelPreview=try await port.preview(input("let a=51; a+1"))
    let cancelPause=Pause(),cancelFault=StorageFaults { point in if point=="before_source_commit" {cancelPause.stop()} }
    let cancelTask=Task {try await port.apply(cancelPreview.preview_ref.value!,faults:cancelFault)}
    let cancelReached=await cancelPause.awaitReached();precondition(cancelReached)
    let cancelIds=await port.activeOperations();precondition(cancelIds.count==1)
    await port.cancel(cancelIds[0]);cancelPause.release.signal()
    do {_ = try await cancelTask.value;fatalError("pre-barrier stop committed source")}
    catch CommitPortError.cancelled { }
    precondition(drafts.confirmed.file.cells[0].source=="let a=5; a+1")
    let cancelledState=try await port.apply(cancelPreview.preview_ref.value!)
    precondition(cancelledState.phase == .cancelled && cancelledState.operation_id==cancelIds[0])
    let earlyPreview=try await port.preview(input("let a=52; a+1"))
    let earlyPause=Pause(),earlyFault=StorageFaults { point in if point=="source_admission_committed" {earlyPause.stop()} }
    let earlyTask=Task {try await port.apply(earlyPreview.preview_ref.value!,faults:earlyFault)}
    let earlyReached=await earlyPause.awaitReached();precondition(earlyReached)
    let earlyIDs=await port.activeOperations();precondition(earlyIDs.count==1)
    await port.cancel(earlyIDs[0]);earlyPause.release.signal()
    let earlyResult=try await earlyTask.value
    precondition(earlyResult.phase == .cancelled && drafts.confirmed.file.cells[0].source=="let a=5; a+1")
    let afterPreview=try await port.preview(input("let a=6; a+1"))
    let afterPause=Pause(),afterFault=StorageFaults { point in if point=="source_committed" {afterPause.stop()} }
    let afterTask=Task {try await port.apply(afterPreview.preview_ref.value!,faults:afterFault)}
    let afterReached=await afterPause.awaitReached();precondition(afterReached)
    let ids=await port.activeOperations();precondition(ids.count==1)
    editor.textView.setMarkedText("更晚输入",selectedRange:NSRange(location:4,length:0),replacementRange:NSRange(location:0,length:0))
    let composed=editor.textView.string
    await port.cancel(ids[0])
    let closing=Task {await port.close()}
    await Task.yield();afterPause.release.signal()
    let committed=try await afterTask.value
    precondition(committed.phase == .completed && committed.cancel_requested)
    precondition(drafts.confirmed.file.cells[0].source=="let a=6; a+1")
    precondition(editor.textView.hasMarkedText() && drafts.overlay(initial.file.cells[0].id)!.source==composed && drafts.overlay(initial.file.cells[0].id)!.conflict)
    let finalRevision=drafts.confirmed.revision.value
    let database=root.appendingPathComponent("Documents").appendingPathComponent(initial.document_id).appendingPathComponent("Generations").appendingPathComponent(String(format:"%06lld",opened.identity.generation)).appendingPathComponent("authority.sqlite")
    let counts=try await Task.detached {
      let db=try SQLiteDatabase(url:database,readonly:true);defer {try? db.close()}
      return [try db.integer("SELECT COUNT(*) FROM transactions"),try db.integer("SELECT COUNT(*) FROM operations"),try db.integer("SELECT COUNT(*) FROM outbox"),try db.integer("SELECT COUNT(*) FROM source_admissions WHERE phase='completed'"),try db.integer("SELECT COUNT(*) FROM source_admissions")]
    }.value
    precondition(counts.prefix(4).allSatisfy{$0==Int64(finalRevision)} && counts[4]>Int64(finalRevision))
    editor.textView.unmarkText()
    await closing.value
    try await client.close();try await storage.close()
    if CommandLine.arguments.count>3 {
      let report:[String:Any] = ["task":"R4.1.06","development_only":true,"fixture_root":root.path,
        "document_id":initial.document_id,"store_id":opened.identity.storeID,"final_revision":finalRevision,
        "transactions":counts[0],"operations":counts[1],"outbox":counts[2],"completed_admissions":counts[3],"all_admissions":counts[4],
        "real_marked_text":true,"manual_invalid_source":true,"duplicate_receipt_tamper_rejected":true,
        "admission_lost_ack_no_replay":true,"commit_lost_ack_readback":true,"own_ack_preserves_newer_draft":true,
        "new_input_rollback":true,"expired_fence_rollback":true,"pre_barrier_cancel":true,"pre_fence_cancel":true,
        "post_commit_cancel_keeps_completed":true,"late_marked_overlay_conflict":true,"close_waited_for_io":true,
        "main_actor_sql":false,"final_candidate_gate":false]
      try JSONSerialization.data(withJSONObject:report,options:[.prettyPrinted,.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[3]),options:.atomic)
    }
    print("Actual C ABI + MainActor NSTextView + SQLite: IME/raw invalid source protection; original-ID duplicate/no replay; admission and COMMIT lost ACK readback; typing/expiry/pre-barrier stop rollback; read-during-IO; post-COMMIT stop success, newer marked overlay/conflict and ordered close passed")
  }
}
