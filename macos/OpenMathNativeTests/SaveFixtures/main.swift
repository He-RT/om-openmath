import AppKit
import Foundation
import SQLite3

private final class Pause:@unchecked Sendable {
  private let lock=NSCondition()
  private var reached=false,released=false
  func enter() {lock.lock();reached=true;lock.broadcast();while !released {lock.wait()};lock.unlock()}
  func release() {lock.lock();released=true;lock.broadcast();lock.unlock()}
  private func ready()->Bool {lock.lock();defer {lock.unlock()};return reached}
  func wait() async throws {for _ in 0..<2000 {if ready(){return};try await Task.sleep(for:.milliseconds(5))};fatalError("actual save boundary not reached")}
}
private func file(_ url:URL)throws->NativeSourceFile {try NotebookFileCodec.decode(Data(contentsOf:url))}
@main struct SaveFixtures {
  static func main() async throws {
    precondition(om_fixture_install_sync_probe()==SQLITE_OK)
    if CommandLine.arguments.count>2 {try await SaveInterruption.run();return}
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true),user=root.appendingPathComponent("user-files",isDirectory:true)
    try FileManager.default.createDirectory(at:user,withIntermediateDirectories:true)
    let target=user.appendingPathComponent("数学🙂.omnb"),second=user.appendingPathComponent("另存为.omnb"),documentID="00000000-0000-0000-0000-000000000411"
    let notebook=NativeSourceFile(version:1,title:"中文 e\u{301}",cells:[.init(id:"a",kind:.math,source:"let a=2",dialect:.modern),.init(id:"text",kind:.text,source:"原文🙂 α",dialect:.auto)])
    try NotebookFileCodec.encode(notebook).write(to:target)
    let service=NativeFileService(),opened=try await service.open(target,document:documentID)
    precondition(opened.file.cells[0].source=="let a=2")
    var initial=NativeSourceSnapshot(codec_version:1,document_id:documentID,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:opened.file,cell_revisions:try opened.file.cells.map{.init(cell_id:$0.id,revision:try HostSerial(0))},snapshot_hash:"")
    initial.snapshot_hash=SourceHashes.snapshot(initial)
    let (storage,_)=try await StorageService.open(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview)),info=try await storage.openDocument(documentID)
    precondition(info.identity.storeVersion==5 && info.identity.minimumReaderVersion==5)
    _ = try await storage.initializeSource(initial)
    let host=try await NativeHostClient.open()
    let openedHost=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:initial,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(openedHost.owner_time_ms.value)
    let drafts=try await MainActor.run {let store=DraftStore(source:initial,runtime:host.runtimeInstanceID,generation:openedHost.document_generation.value,clock:{clock.now()});try store.attachEditor(cell:"a",generation:1);return store}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),port=NativeFilePort(client:host,storage:storage,source:source,drafts:drafts,files:service,document:documentID)
    _ = try await port.adoptOpened(opened)
    let initialState=try await port.state();precondition(!initialState.dirty)
    let kernel=try await host.kernelCommand(.kernel_state(.init(type:.kernel_state)));precondition(kernel.active_checkpoint_id.value==nil && kernel.kernel_state_revision.value==0)
    let native=await MainActor.run {let d=NativeDocument();d.attach(port,opened:opened);return d}
    try await MainActor.run {try drafts.edited(cell:"a",generation:1,source:"let a=",hasMarkedText:false);native.noteEdit()}
    let invalid=try await native.saveNative(operation:"invalid-math-source")
    let invalidFile=try file(target);precondition(invalidFile.cells[0].source=="let a=" && invalid.sourceRevision==1)
    let saved=try await port.state();precondition(!saved.dirty)
    let wrongDestination=user.appendingPathComponent("未授权重绑定.omnb")
    for operationType:NSDocument.SaveOperationType in [.saveOperation,.autosaveInPlaceOperation] {
      let rejected=await withCheckedContinuation {continuation in
        native.save(to:wrongDestination,ofType:"org.openmath.notebook",for:operationType) {error in
          continuation.resume(returning:(error as? SaveError) == .bindingChanged)
        }
      }
      precondition(rejected && !FileManager.default.fileExists(atPath:wrongDestination.path))
    }
    let unchangedAfterRejectedDestination=try file(target)
    precondition(unchangedAfterRejectedDestination.cells[0].source=="let a=")
    let pause=Pause()
    try await MainActor.run {try drafts.edited(cell:"a",generation:1,source:"let a=5",hasMarkedText:false);native.noteEdit()}
    let paused=Task {try await native.saveNative(operation:"save-N",faults:.init {point in if point=="before_save_replace" {pause.enter()} })}
    try await pause.wait()
    try await MainActor.run {try drafts.edited(cell:"a",generation:1,source:"let a=9",hasMarkedText:false);native.noteEdit()}
    _ = try await source.synchronizeDrafts()
    pause.release();let n=try await paused.value
    let actualN=try file(target);let newer=try await port.state()
    precondition(n.sourceRevision==2 && actualN.cells[0].source=="let a=5" && newer.sourceRevision==3 && newer.dirty)
    precondition(native.isDocumentEdited)
    let acknowledged=try await native.saveNative(operation:"save-N-plus1");precondition(acknowledged.sourceRevision==3)
    let newBytes=try Data(contentsOf:target);let raw=try JSONSerialization.jsonObject(with:newBytes) as! [String:Any]
    precondition(Set(raw.keys)==["version","title","cells"] && !String(decoding:newBytes,as:UTF8.self).contains("kernel_state"))
    let oldBindingPause=Pause()
    let oldSave=Task {try await port.save(operation:"old-binding-ack",faults:.init {point in if point=="before_save_replace" {oldBindingPause.enter()} })}
    try await oldBindingPause.wait();let chosen=try await port.chooseTarget(second);precondition(chosen.binding.revision==2)
    oldBindingPause.release();let oldAck=try await oldSave.value;let currentBinding=try await port.state()
    precondition(oldAck.bindingRevision==1 && currentBinding.head?.binding.revision==2 && currentBinding.head?.savedRevision==nil && currentBinding.dirty)
    let savedAs=try await port.save(operation:"new-binding-save");precondition(savedAs.bindingRevision==2)
    let newTarget=try file(second);precondition(newTarget.cells[0].source=="let a=9")
    var outside=newTarget;outside.title="外部应用改动";try NotebookFileCodec.encode(outside).write(to:second)
    do {_ = try await port.save(operation:"external-conflict");fatalError("external bytes were overwritten")}
    catch SaveError.externalConflict { }
    let untouched=try file(second);precondition(untouched.title=="外部应用改动")
    _ = try await port.chooseTarget(second)
    let lost=StorageFaults {point in if point=="save_replaced" {throw SaveError.system(EIO,"lost-file-ack")} }
    let recovered=try await port.save(operation:"replace-lost-ack",faults:lost);precondition(recovered.recovered)
    let original=try await port.save(operation:"replace-lost-ack");precondition(original.operationID==recovered.operationID && original.completedAt==recovered.completedAt)
    let readOnly=user.appendingPathComponent("只读.omnb");try newBytes.write(to:readOnly);precondition(chmod(readOnly.path,0o400)==0)
    _ = try await port.chooseTarget(readOnly)
    do {_ = try await port.save(operation:"permission-denied");fatalError("read-only user file was replaced")}
    catch SaveError.permissionDenied { }
    precondition(chmod(readOnly.path,0o600)==0)
    _ = try await port.chooseTarget(second)
    try await MainActor.run {try drafts.edited(cell:"a",generation:1,source:"未完成拼音🙂",hasMarkedText:true);native.noteEdit()}
    let composed=try await port.save(operation:"save-with-marked-input");let composedFile=try file(second)
    let remaining=try await port.state();precondition(composedFile.cells[0].source=="let a=9" && remaining.hasDrafts && remaining.dirty && composed.sourceRevision==3)
    let overlay=drafts.overlay("a");precondition(overlay?.composing==true && overlay?.source=="未完成拼音🙂")
    try await MainActor.run {try drafts.edited(cell:"a",generation:1,source:"let a=12",hasMarkedText:false);native.noteEdit()}
    let actualFsync=StorageFaults {point in if point=="save_receipt_before_commit" {om_fixture_fail_fullsync(1)} }
    do {_ = try await port.save(operation:"actual-save-sync-unknown",faults:actualFsync);fatalError("unstable save receipt acknowledged")}
    catch SaveError.unknownOutcome { }
    let unknown=try await port.state();precondition(unknown.unknownOperation=="actual-save-sync-unknown")
    do {_ = try await port.save(operation:"wrong-new-id");fatalError("unknown target replaced under new ID")} catch SaveError.busy { }
    om_fixture_fail_fullsync(0);let actual=try await port.reconcile("actual-save-sync-unknown")!;precondition(actual.sourceRevision==4)
    await port.close();await source.close();try await host.close();try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview));_ = try await reopened.openDocument(documentID)
    let durable=try await reopened.saveReceipt(document:documentID,operation:"actual-save-sync-unknown")!;let head=try await reopened.fileHead(document:documentID)!
    precondition(durable.fileHash==actual.fileHash && head.savedRevision==4)
    try await reopened.close()
    // The app's actual factory opens the source-only file without replaying definitions and
    // shares one root lease. Its real TextKit input is saved through the native document port.
    let app=NativeAppStorage(paths:.init(root:root.appendingPathComponent("application"),channel:.preview))
    async let first=app.open();async let another=app.open()
    let (one,two)=try await (first,another);precondition(one === two)
    let unbound=try await NativeDocumentSession.open(storage:one)
    let unboundRejected=await withCheckedContinuation {continuation in
      unbound.document.save(to:wrongDestination,ofType:"org.openmath.notebook",for:.autosaveInPlaceOperation) {error in
        continuation.resume(returning:(error as? SaveError) == .bindingChanged)
      }
    }
    let unboundState=try await unbound.files.state()
    precondition(unboundRejected && unboundState.head==nil && !FileManager.default.fileExists(atPath:wrongDestination.path))
    try await unbound.close()
    let live=try await NativeDocumentSession.open(storage:one,url:second)
    let restored=try await live.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
    precondition(restored.active_checkpoint_id.value==nil && restored.kernel_state_revision.value==0)
    let editor=try live.editor("a")
    editor.textView.setSelectedRange(NSRange(location:0,length:editor.textView.string.utf16.count))
    editor.textView.insertText("let a=15 # 中文🙂",replacementRange:editor.textView.selectedRange())
    for _ in 0..<100 {
      if (try file(second)).cells[0].source=="let a=15 # 中文🙂",!(try await live.files.state()).dirty {break}
      try await Task.sleep(for:.milliseconds(50))
    }
    let auto=try file(second),autoState=try await live.files.state()
    precondition(auto.cells[0].source=="let a=15 # 中文🙂" && !autoState.dirty)
    precondition(live.undo.manager.canUndo)
    let firstClose=await live.document.prepareToClose();precondition(firstClose)
    // A real later input revokes the close decision instead of losing new draft bytes.
    editor.textView.insertText("\n# 关闭之前的新输入",replacementRange:editor.textView.selectedRange())
    do {try await live.close(approvalRequired:true);fatalError("later input lost at close")}
    catch SaveError.sourceChanged { }
    precondition(editor.textView.isEditable)
    _ = try await live.document.saveNative(operation:"last-source-before-close")
    let lastClose=await live.document.prepareToClose();precondition(lastClose);try await live.close(approvalRequired:true)
    let unicodeTarget=user.appendingPathComponent("unicode-id.omnb")
    let unicode=NativeSourceFile(version:1,title:"byte identity",cells:[.init(id:"é",kind:.math,source:"1",dialect:.modern),.init(id:"e\u{301}",kind:.math,source:"2",dialect:.modern)])
    try NotebookFileCodec.encode(unicode).write(to:unicodeTarget)
    let distinct=try await NativeDocumentSession.open(storage:one,url:unicodeTarget)
    let composedID=try distinct.editor("é"),decomposedID=try distinct.editor("e\u{301}")
    precondition(composedID !== decomposedID && composedID.textView.string=="1" && decomposedID.textView.string=="2")
    try await distinct.close()
    let originalURL=user.appendingPathComponent("rename-original.omnb"),renamedURL=user.appendingPathComponent("已改名🙂.omnb")
    try NotebookFileCodec.encode(notebook).write(to:originalURL)
    let renameSession=try await NativeDocumentSession.open(storage:one,url:originalURL)
    let renameEditor=try renameSession.editor("a")
    try FileManager.default.moveItem(at:originalURL,to:renamedURL)
    // An unrelated file can appear at the old path. The bookmark follows the original inode,
    // and the writer must leave this other source untouched.
    var other=notebook;other.title="旧路径的新文件";try NotebookFileCodec.encode(other).write(to:originalURL)
    renameEditor.textView.setSelectedRange(NSRange(location:0,length:renameEditor.textView.string.utf16.count))
    renameEditor.textView.insertText("let a=18",replacementRange:renameEditor.textView.selectedRange())
    _ = try await renameSession.document.saveNative(operation:"same-directory-rename")
    let renamedFile=try file(renamedURL),unrelated=try file(originalURL),renameHead=try await renameSession.files.state()
    precondition(renamedFile.cells[0].source=="let a=18" && unrelated.title=="旧路径的新文件" && renameHead.head?.binding.fileName==renamedURL.lastPathComponent)
    precondition(renameSession.document.fileURL?.path==renamedURL.path)
    // Moving beyond the existing directory grant asks for a deliberate new target binding.
    let movedFolder=user.appendingPathComponent("moved",isDirectory:true);try FileManager.default.createDirectory(at:movedFolder,withIntermediateDirectories:true)
    let movedURL=movedFolder.appendingPathComponent(renamedURL.lastPathComponent);try FileManager.default.moveItem(at:renamedURL,to:movedURL)
    do {_ = try await renameSession.files.save(operation:"move-outside-old-grant");fatalError("old directory grant widened")}
    catch SaveError.bindingChanged { }
    _ = try await renameSession.document.saveNative(to:movedURL,operation:"explicit-moved-target")
    precondition(renameSession.document.fileURL?.path==movedURL.path)
    // Real save producer resumes after a canceled close; a prior failure pause stays paused.
    let resumed=NativeAutosave(idleDelay:.milliseconds(30),maximumDelay:.milliseconds(80)) {
      _ = try await renameSession.files.save(operation:"close-cancel-autosave");return true
    }
    resumed.edited();let prior=resumed.isSuspended;resumed.suspend();resumed.restoreAfterCancelledClose(wasSuspended:prior)
    var resumedReceipt:SaveReceipt?
    for _ in 0..<100 {resumedReceipt=try await one.saveReceipt(document:renameSession.drafts.confirmed.document_id,operation:"close-cancel-autosave");if resumedReceipt != nil {break};try await Task.sleep(for:.milliseconds(10))}
    precondition(resumedReceipt != nil);await resumed.finish()
    let pausedAutosave=NativeAutosave(idleDelay:.milliseconds(20),maximumDelay:.milliseconds(50)) {
      _ = try await renameSession.files.save(operation:"must-not-resume-paused");return true
    }
    pausedAutosave.suspend();pausedAutosave.edited();pausedAutosave.restoreAfterCancelledClose(wasSuspended:true)
    try await Task.sleep(for:.milliseconds(70))
    let skipped=try await one.saveIntent(document:renameSession.drafts.confirmed.document_id,operation:"must-not-resume-paused");precondition(skipped==nil);await pausedAutosave.finish()
    try FileManager.default.removeItem(at:movedURL)
    do {_ = try await renameSession.files.save(operation:"missing-original-target");fatalError("missing existing file recreated")}
    catch SaveError.bindingChanged { }
    precondition(!FileManager.default.fileExists(atPath:movedURL.path))
    try await renameSession.close()
    try await app.close()
    // Corruption is an explicit read failure, never an integer-conversion trap or empty state.
    let corruptRoot=root.appendingPathComponent("application")
    let folders=try FileManager.default.contentsOfDirectory(at:corruptRoot.appendingPathComponent("Documents"),includingPropertiesForKeys:nil)
    let corruptDocument=folders.first{$0.lastPathComponent==renameSession.drafts.confirmed.document_id}!
    let corruptDatabase=corruptDocument.appendingPathComponent("Generations/000001/authority.sqlite")
    try await Task.detached {
      let corruptWriter=try SQLiteDatabase(url:corruptDatabase,readonly:false)
      try corruptWriter.statement("PRAGMA foreign_keys=OFF")
      try corruptWriter.transaction {try corruptWriter.statement("UPDATE file_head SET saved_revision=-1")}
      try corruptWriter.close()
    }.value
    let (corruptStore,_)=try await StorageService.open(paths:.init(root:corruptRoot,channel:.preview));_ = try await corruptStore.openDocument(corruptDocument.lastPathComponent)
    do {_ = try await corruptStore.fileHead(document:corruptDocument.lastPathComponent);fatalError("negative file revision accepted")}
    catch StorageError.corruptIdentity { }
    try await corruptStore.close()
    print("Actual user file/NSDocument/SQLite saves: source-only invalid syntax, N+1 dirty, old-binding ACK historical, Save As, external/read-only refusal, replace lost-ACK original readback, marked input kept, actual F_FULLFSYNC unknown and restart receipt passed")
    print("Application factory: shared root, source-only open, real TextKit input, 2s actual file autosave, native undo and later-input close rejection passed")
    print("Original file identity: same-directory rename follows inode, unrelated old path untouched, moved directory requires explicit new binding, missing target not recreated")
  }
}
