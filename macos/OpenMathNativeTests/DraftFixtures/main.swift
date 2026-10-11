import AppKit
import Foundation
import SQLite3

private final class Pause:@unchecked Sendable {
  private let lock=NSCondition();private var ready=false,released=false
  func hold() {lock.lock();ready=true;lock.broadcast();while !released {lock.wait()};lock.unlock()}
  func release() {lock.lock();released=true;lock.broadcast();lock.unlock()}
  func wait() async throws {for _ in 0..<1000 {let value=lock.withLock{ready};if value {return};try await Task.sleep(for:.milliseconds(5))};fatalError("actual draft IO boundary missing")}
}
private final class Once:@unchecked Sendable {
  private let lock=NSLock();private var used=false
  func claim()->Bool {lock.withLock {guard !used else {return false};used=true;return true}}
}
@MainActor @main struct DraftFixtures {
  static let documentID="00000000-0000-0000-0000-000000000512"
  private static func initial()throws->NativeSourceSnapshot {
    let file=NativeSourceFile(version:1,title:"草稿恢复",cells:[.init(id:"a",kind:.math,source:"let a=2",dialect:.modern),.init(id:"é",kind:.text,source:"NFC",dialect:.auto),.init(id:"e\u{301}",kind:.text,source:"NFD",dialect:.auto)])
    var value=NativeSourceSnapshot(codec_version:1,document_id:documentID,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:file,cell_revisions:try file.cells.map{.init(cell_id:$0.id,revision:try HostSerial(0))},snapshot_hash:"")
    value.snapshot_hash=SourceHashes.snapshot(value);return value
  }
  static func main() async throws {
    precondition(om_fixture_install_sync_probe()==SQLITE_OK)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    if CommandLine.arguments.count>2,CommandLine.arguments[2]=="recover" {
      let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
      let session=try await NativeDocumentSession.recover(storage:storage,documentID:documentID)
      let source=try await storage.committedSource(documentID)!,stored=try await storage.draftBackup(document:documentID)
      let expected=try initial();precondition(source.snapshot_hash==expected.snapshot_hash)
      let restoredEditor=try session.editor("a")
      if stored?.snapshot.cells.isEmpty==false {
        precondition(restoredEditor.textView.string=="未完成🙂" && !restoredEditor.textView.hasMarkedText() && session.drafts.overlay("a")!.requiresReview)
      }
      let hasDraft=session.drafts.overlay("a")?.requiresReview==true
      try await session.close();try await storage.close();print(!hasDraft ? "No draft COMMIT: original source preserved" : "Original draft COMMIT recovered: marked raw text preserved, source untouched");return
    }
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview)),info=try await storage.openDocument(documentID)
    precondition(info.identity.storeVersion==5)
    let source=try initial();_ = try await storage.initializeSource(source)
    let client=try await NativeHostClient.open(),opened=try await client.sourceCommand(.source_open(.init(type:.source_open,snapshot:source,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let drafts=DraftStore(source:source,runtime:client.runtimeInstanceID,generation:opened.document_generation.value)
    let editor=try DraftTextViewAdapter(store:drafts,cell:"a",generation:1)
    let owner=try await storage.beginDraftOwner(document:documentID,owner:UUID().uuidString.lowercased(),runtime:client.runtimeInstanceID,generation:opened.document_generation.value)
    editor.textView.setSelectedRange(NSRange(location:0,length:editor.textView.string.utf16.count))
    editor.textView.setMarkedText("未完成🙂",selectedRange:NSRange(location:3,length:2),replacementRange:NSRange(location:0,length:editor.textView.string.utf16.count))
    try editor.reportCurrentInput();precondition(editor.textView.hasMarkedText())
    if CommandLine.arguments.count>3,CommandLine.arguments[2]=="crash" {
      let stage=CommandLine.arguments[3],snapshot=try drafts.backupSnapshot(owner:owner,sequence:1)
      _ = try await storage.backupDrafts(snapshot,faults:.init {point in if point==stage {FileHandle.standardOutput.write(Data((point+"\n").utf8));raise(SIGSTOP)}})
      fatalError("draft boundary not reached")
    }
    let backup=NativeDraftBackup(storage:storage,store:drafts,owner:owner)
    try await backup.flush()
    let actual=try await storage.draftBackup(document:documentID)!
    precondition(actual.snapshot.cells[0].wasComposing && actual.snapshot.cells[0].markedRange != nil && editor.textView.hasMarkedText() && backup.isCurrent)
    precondition(String(data:actual.snapshot.cells[0].sourceUTF8,encoding:.utf8)=="未完成🙂")
    do {try DraftTextRange(NSRange(location:1,length:1)).validate(in:"🙂");fatalError("UTF16 surrogate split accepted")} catch StorageError.corruptIdentity { }
    // A new native runtime has fresh editor generations and ordinary pending text, not IME.
    let restored=DraftStore(source:source,runtime:"restored-runtime",generation:2);try restored.restoreBackup(actual)
    let restoredEditor=try DraftTextViewAdapter(store:restored,cell:"a",generation:1)
    precondition(restoredEditor.textView.string=="未完成🙂" && !restoredEditor.textView.hasMarkedText() && restored.overlay("a")!.requiresReview)
    do {_ = try restored.pendingEdits();fatalError("restored draft automatically submitted")} catch CommitPortError.editingBusy { }
    try restored.reviewRecovered(cell:"a",keep:true);precondition(!restored.overlay("a")!.requiresReview)
    let originalSource=try await storage.committedSource(documentID)!;precondition(originalSource.snapshot_hash==source.snapshot_hash)
    // Timers use real native input. A burst must reach an actual receipt within max 2 s;
    // continuous edits cannot postpone the maximum clock indefinitely.
    editor.textView.unmarkText()
    for _ in 0..<6 {editor.textView.insertText("x",replacementRange:NSRange(location:NSNotFound,length:0));try await Task.sleep(for:.milliseconds(350))}
    let timed=try await storage.draftBackup(document:documentID)!;precondition(timed.receipt.sequence>actual.receipt.sequence)
    try await backup.flush();backup.stop()
    let pause=Pause(),once=Once()
    let nextOwner=try await storage.beginDraftOwner(document:documentID,owner:UUID().uuidString.lowercased(),runtime:client.runtimeInstanceID,generation:opened.document_generation.value)
    let current=NativeDraftBackup(storage:storage,store:drafts,owner:nextOwner,faults:.init {point in if point=="draft_backup_before_commit" && once.claim() {pause.hold()}})
    editor.textView.insertText("N",replacementRange:NSRange(location:NSNotFound,length:0))
    let flushing=Task {try await current.flush()};try await pause.wait()
    editor.textView.insertText("🧪",replacementRange:NSRange(location:NSNotFound,length:0))
    precondition(!current.isCurrent);pause.release();try await flushing.value
    let latest=try await storage.draftBackup(document:documentID)!
    precondition(String(data:latest.snapshot.cells[0].sourceUTF8,encoding:.utf8)==editor.textView.string && current.isCurrent && latest.receipt.sequence==2)
    let old=try drafts.backupSnapshot(owner:owner,sequence:999)
    do {_ = try await storage.backupDrafts(old);fatalError("old runtime overwrote new owner")} catch StorageError.staleRevision { }
    current.stop()
    let syncOnce=Once(),unknownOwner=try await storage.beginDraftOwner(document:documentID,owner:UUID().uuidString.lowercased(),runtime:client.runtimeInstanceID,generation:opened.document_generation.value)
    let unknown=NativeDraftBackup(storage:storage,store:drafts,owner:unknownOwner,faults:.init {point in if point=="draft_backup_before_commit" && syncOnce.claim() {om_fixture_fail_fullsync(1)}})
    do {try await unknown.flush();fatalError("actual failed fullsync acknowledged")} catch StorageError.unknownCommit { }
    precondition(!unknown.isCurrent && unknown.status.contains("待核对"))
    om_fixture_fail_fullsync(0);try await unknown.flush()
    let confirmed=try await storage.draftBackup(document:documentID)!;precondition(confirmed.receipt.sequence==1 && unknown.isCurrent)
    let nfc=try DraftTextViewAdapter(store:drafts,cell:"é",generation:1),nfd=try DraftTextViewAdapter(store:drafts,cell:"e\u{301}",generation:1)
    nfc.textView.insertText("🙂",replacementRange:NSRange(location:NSNotFound,length:0));nfd.textView.insertText("λ",replacementRange:NSRange(location:NSNotFound,length:0))
    try await unknown.flush()
    let distinct=try await storage.draftBackup(document:documentID)!
    precondition(distinct.snapshot.cells.count==3 && Set(distinct.snapshot.cells.map{Data($0.cellID.utf8)}).count==3)
    try await unknown.finish();try await client.close();try await storage.close()
    let (reopen,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));_ = try await reopen.openDocument(documentID)
    let persisted=try await reopen.draftBackup(document:documentID)!,untouched=try await reopen.committedSource(documentID)!
    precondition(untouched.snapshot_hash==source.snapshot_hash && persisted.snapshot.cells.count==3)
    try await reopen.close()
    // A legitimate large source/base pair encodes above the old 4 MiB SQL reader ceiling.
    // Read the exact original bytes through the bounded 8 MiB draft codec, without parsing.
    let largeRoot=root.appendingPathComponent("large-backup",isDirectory:true)
    let (largeStore,_)=try await StorageService.open(paths:.init(root:largeRoot,channel:.preview))
    var large=try initial();large.document_id=UUID().uuidString.lowercased();large.file.cells=[large.file.cells[0]];large.cell_revisions=[large.cell_revisions[0]]
    large.file.cells[0].source=String(repeating:"a",count:1_600_000);large.snapshot_hash=SourceHashes.snapshot(large)
    _ = try await largeStore.openDocument(large.document_id);_ = try await largeStore.initializeSource(large)
    let largeOwner=try await largeStore.beginDraftOwner(document:large.document_id,owner:UUID().uuidString.lowercased(),runtime:"large-draft-runtime",generation:1)
    let largeCell=DraftBackupCell(cellID:"a",editorGeneration:1,draftSequence:1,baseCellRevision:0,baseUTF8:Data(large.file.cells[0].source.utf8),sourceUTF8:Data(String(repeating:"b",count:1_600_000).utf8),wasComposing:false,conflict:false,requiresReview:false,selections:[DraftTextRange(NSRange(location:0,length:0))],markedRange:nil)
    let largeBackup=DraftBackupSnapshot(codecVersion:1,owner:largeOwner,sequence:1,sourceRevision:0,sourceSnapshotHash:large.snapshot_hash,cells:[largeCell])
    let encodedLarge=try DraftBackupValidation.encode(largeBackup);precondition(encodedLarge.count>4*1024*1024 && encodedLarge.count<=8*1024*1024)
    _ = try await largeStore.backupDrafts(largeBackup);let largeReadback=try await largeStore.draftBackup(document:large.document_id)!
    precondition(largeReadback.snapshot.cells[0].sourceUTF8==largeCell.sourceUTF8);try await largeStore.close()
    print("Real TextKit/IME draft backups: exact UTF8/NFC-NFD/UTF16, no source submission, 500ms/max2s timers, newer input/owner guards, actual fullsync unknown original-ID readback, native restoration without IME and restart passed")
  }
}
