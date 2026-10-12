import AppKit
import CryptoKit

@main struct CompositionFixtures {
  @MainActor static func until(_ check:@escaping @MainActor ()->Bool) async throws {
    let deadline=ContinuousClock.now.advanced(by:.seconds(10))
    while !check() {precondition(ContinuousClock.now<deadline);try await Task.sleep(for:.milliseconds(10))}
  }
  @MainActor static func main() async throws {
    let app=NSApplication.shared;app.setActivationPolicy(.accessory)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    let baseline="# 合成与撤销验收\n",file=NativeSourceFile(version:1,title:"真实原生输入事件",cells:[.init(id:"a",kind:.math,source:baseline,dialect:.modern)])
    let url=root.appendingPathComponent("composition.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let session=try await NativeDocumentSession.open(storage:services.open(),url:url)
    let adapter=try session.editor("a"),editor=adapter.textView
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:720,height:400),styleMask:[.titled,.resizable],backing:.buffered,defer:false)
    let scroll=NSScrollView();scroll.documentView=editor;window.contentView=scroll;window.makeKeyAndOrderFront(nil);window.makeFirstResponder(editor)
    editor.setSelectedRange(NSRange(location:baseline.utf16.count,length:0))
    editor.insertText("a",replacementRange:NSRange(location:NSNotFound,length:0))
    let typingGroup=adapter.inputGroup
    editor.insertText("b",replacementRange:NSRange(location:NSNotFound,length:0))
    precondition(editor.string==baseline+"ab" && adapter.inputGroup==typingGroup && editor.undoManager!.canUndo)
    editor.undoManager!.undo();precondition(editor.string==baseline,"continuous native typing did not undo as one group")
    editor.undoManager!.redo();precondition(editor.string==baseline+"ab")
    try await Task.sleep(for:.milliseconds(700))
    editor.insertText("c",replacementRange:NSRange(location:NSNotFound,length:0))
    precondition(adapter.inputGroup != typingGroup)
    editor.undoManager!.undo();precondition(editor.string==baseline+"ab")
    editor.undoManager!.undo();precondition(editor.string==baseline)

    var cases:[String]=[]
    for (pending,updated,committed) in [("中","中文","中文"),("に","にほん","日本語")] {
      editor.setSelectedRange(NSRange(location:editor.string.utf16.count,length:0));adapter.breakInputGroup()
      let hash=SHA256.hash(data:Data(baseline.utf8)).map {String(format:"%02x",$0)}.joined()
      let preview=try await session.source.preview(.preview_patch_input(.init(kind:.patch,operations:[.preview_update_cell(.init(type:.update_cell,target:.init(cell_id:"a"),expected_source_hash:hash,source:"# agent replacement",kind:nil,dialect:nil))])))
      editor.setMarkedText(pending,selectedRange:NSRange(location:pending.utf16.count,length:0),replacementRange:NSRange(location:NSNotFound,length:0))
      let compositionGroup=adapter.inputGroup
      editor.setMarkedText(updated,selectedRange:NSRange(location:updated.utf16.count,length:0),replacementRange:NSRange(location:NSNotFound,length:0))
      precondition(editor.hasMarkedText() && session.drafts.overlay("a")!.composing && adapter.inputGroup==compositionGroup)
      precondition(!editor.undoManager!.canUndo)
      do {_ = try session.drafts.pendingEdits();fatalError("marked source submitted")} catch CommitPortError.editingBusy {}
      do {_ = try await session.source.apply(preview.preview_ref.value!);fatalError("agent replaced actual marked source")} catch CommitPortError.editingBusy {}
      session.runtime.run("a");try await until {session.runtime.operation==nil}
      guard let projection=session.runtime.projection else {fatalError("actual kernel state not read")}
      precondition(session.runtime.results.isEmpty && projection.active_checkpoint_id.value==nil && projection.kernel_state_revision.value==0)
      precondition(session.drafts.confirmed.revision.value==0 && editor.hasMarkedText() && editor.string==baseline+updated)
      try await Task.sleep(for:.milliseconds(700))
      precondition(editor.hasMarkedText() && adapter.inputGroup==compositionGroup)
      // This is the real NSTextInputClient API/marked range, not a composition boolean fixture.
      // Actual OS candidate windows remain a separate live acceptance requirement.
      editor.insertText(committed,replacementRange:editor.markedRange());editor.unmarkText()
      try adapter.reportCurrentInput()
      precondition(!editor.hasMarkedText() && !session.drafts.overlay("a")!.composing && editor.string==baseline+committed)
      editor.undoManager!.undo();precondition(editor.string==baseline,"one actual composition did not undo atomically")
      cases.append(committed)
    }
    // Native plain-text paste is one operation, preserves Unicode/CRLF and never makes cells.
    adapter.breakInputGroup();let pasted="🙂e\u{301}\r\n# second line"
    NSPasteboard.general.clearContents();NSPasteboard.general.setString(pasted,forType:.string);editor.paste(nil)
    precondition(Data(editor.string.utf8)==Data((baseline+pasted).utf8))
    editor.undoManager!.undo();precondition(Data(editor.string.utf8)==Data(baseline.utf8))
    precondition(session.drafts.confirmed.file.cells.count==1 && session.drafts.confirmed.revision.value==0)
    let report:[String:Any]=["continuous_typing_one_undo":true,"idle_break_new_group":true,"real_marked_cases":cases,"marked_blocks_run_and_agent_apply":true,"composition_over_idle_atomic_undo":true,"plain_paste_one_undo_exact_bytes":true,"source_revision":0,"os_candidate_windows":"not_verified_by_API_fixture"]
    print(String(decoding:try JSONSerialization.data(withJSONObject:report,options:[.sortedKeys]),as:UTF8.self))
    window.orderOut(nil);try await session.close();try await services.close()
    print("Real NSTextInputClient Chinese/Japanese marked events, idle composition, no accepted calculation or source replacement, continuous native undo/redo and raw paste passed; OS candidate windows not claimed")
  }
}
