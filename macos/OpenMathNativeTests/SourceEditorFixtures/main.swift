import AppKit
import CryptoKit

@main struct SourceEditorFixtures {
  static func rejects(_ body:()throws->Void) {do {try body();fatalError("invalid scalar/range accepted")} catch {}}
  @MainActor static func main() async throws {
    let corpus=["","A🙂B","中文","e\u{301}","é","👨‍👩‍👧‍👦","🇨🇳","\tA\r\nB\rC\n","\r\n","line\n"]
    var scalarCases=0
    for source in corpus {
      let original=Data(source.utf8),map=try SourceIndexMap(utf8:original,draftSequence:17)
      precondition(Data(map.source.utf8)==original)
      var byte=0,unit=0,legalBytes=Set([0]),legalUnits=Set([0])
      for scalar in source.unicodeScalars {
        byte+=scalar.utf8.count;unit+=scalar.value>0xffff ? 2 : 1;legalBytes.insert(byte);legalUnits.insert(unit)
      }
      for offset in 0...byte {
        if legalBytes.contains(offset) {
          let native=try map.nativeRange(start:offset,end:offset),range=try map.byteRange(for:native)
          precondition(range==offset..<offset);scalarCases+=1
        } else {rejects {_ = try map.utf16Offset(forUTF8:offset)}}
      }
      for offset in 0...unit {
        if legalUnits.contains(offset) {let raw=try map.utf8Offset(forUTF16:offset),actual=try map.utf16Offset(forUTF8:raw);precondition(actual==offset)}
        else {rejects {_ = try map.byteRange(for:NSRange(location:offset,length:0))}}
      }
      for first in legalBytes {for last in legalBytes where last>=first {
        let native=try map.nativeRange(start:first,end:last)
        let actual=try map.byteRange(for:native);precondition(actual==first..<last)
      }}
      rejects {_ = try map.byteRange(for:NSRange(location:NSNotFound,length:0))}
      rejects {_ = try map.byteRange(for:NSRange(location:0,length:Int.max))}
      rejects {_ = try map.nativeRange(start:-1,end:0)}
      rejects {try map.validate(sourceHash:map.sourceHash,draftSequence:18)}
      rejects {try map.validate(sourceHash:String(repeating:"0",count:64),draftSequence:17)}
      try map.validate(sourceHash:map.sourceHash,draftSequence:17)
    }
    rejects {_ = try SourceIndexMap(utf8:Data([0xf0,0x9f,0x99]),draftSequence:0)}
    rejects {_ = try SourceIndexMap(utf8:Data([0xc0,0xaf]),draftSequence:0)}
    let nfd=try SourceIndexMap(source:"e\u{301}",draftSequence:0),nfc=try SourceIndexMap(source:"é",draftSequence:0)
    precondition(nfd.sourceHash != nfc.sourceHash)
    let combiningByte=try nfd.utf8Offset(forUTF16:1),composed=try nfd.isComposedRange(NSRange(location:1,length:0))
    precondition(combiningByte==1 && !composed)
    let line=try SourceIndexMap(source:"\tA\r\nB\rC\n",draftSequence:0)
    precondition(line.lineStarts.map(\.utf8)==[0,4,6,8] && line.preferredNewline=="\r\n")
    let position=try line.position(forUTF8:4);precondition(position.line==2 && position.column==1)

    let app=NSApplication.shared;app.setActivationPolicy(.accessory)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    let raw="let a=1 # e\u{301} 👨‍👩‍👧‍👦\r\n# 中文\t🇨🇳"
    let file=NativeSourceFile(version:1,title:"原生文本索引",cells:[.init(id:"a",kind:.math,source:raw,dialect:.modern)])
    let url=root.appendingPathComponent("source.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let session=try await NativeDocumentSession.open(storage:services.open(),url:url)
    let adapter=try session.editor("a"),editor=adapter.textView
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:700,height:360),styleMask:[.titled,.resizable],backing:.buffered,defer:false)
    let scroll=NSScrollView();scroll.documentView=editor;window.contentView=scroll;window.makeKeyAndOrderFront(nil);window.makeFirstResponder(editor)
    precondition(editor.textLayoutManager != nil && Data(editor.string.utf8)==Data(raw.utf8))
    precondition(!editor.isAutomaticQuoteSubstitutionEnabled && !editor.isAutomaticDashSubstitutionEnabled && !editor.isAutomaticTextReplacementEnabled && !editor.isAutomaticTextCompletionEnabled)
    editor.setSelectedRange(NSRange(location:editor.string.utf16.count,length:0));editor.insertNewline(nil)
    precondition(Data(editor.string.utf8)==Data((raw+"\r\n").utf8))
    let paste="\"quotes\" -- 中文🙂\r\nlet b=2"
    NSPasteboard.general.clearContents();NSPasteboard.general.setString(paste,forType:.string);editor.paste(nil)
    precondition(Data(editor.string.utf8)==Data((raw+"\r\n"+paste).utf8))
    precondition(session.drafts.confirmed.file.cells.count==1 && session.drafts.confirmed.revision.value==0)
    precondition(adapter.indexMap.draftSequence==session.drafts.overlay("a")!.sequence)
    try adapter.indexMap.validate(sourceHash:SHA256.hash(data:Data(editor.string.utf8)).map {String(format:"%02x",$0)}.joined(),draftSequence:session.drafts.overlay("a")!.sequence)
    let unchangedSequence=session.drafts.overlay("a")!.sequence
    try adapter.reportCurrentInput();try adapter.reportCurrentInput()
    precondition(session.drafts.overlay("a")!.sequence==unchangedSequence)
    editor.setMarkedText("中文",selectedRange:.init(location:2,length:0),replacementRange:.init(location:NSNotFound,length:0))
    let marked=session.drafts.overlay("a")!,markedBytes=Data(editor.string.utf8)
    precondition(marked.composing)
    try adapter.reportCurrentInput()
    precondition(session.drafts.overlay("a")!.sequence==marked.sequence)
    editor.unmarkText()
    precondition(Data(editor.string.utf8)==markedBytes && !session.drafts.overlay("a")!.composing && session.drafts.overlay("a")!.sequence>marked.sequence)
    precondition(adapter.indexMap.draftSequence==session.drafts.overlay("a")!.sequence)
    // A clean editor can be replaced by an agent while its old caret lies beyond the
    // new source, or inside the new source's surrogate pair. Backup must stay valid
    // even before AppKit has reflected the acknowledged text.
    for caret in [raw.utf16.count,1] {
      let replacement=DraftStore(source:session.drafts.confirmed,runtime:"selection-replacement",generation:1)
      let replacementAdapter=try DraftTextViewAdapter(store:replacement,cell:"a",generation:1)
      replacementAdapter.textView.setSelectedRange(.init(location:caret,length:0))
      try replacementAdapter.reportCurrentInput()
      var changed=replacement.confirmed
      changed.file.cells[0].source="🙂x";changed.revision=try HostSerial(1)
      changed.cell_revisions[0].revision=try HostSerial(1);changed.snapshot_hash=SourceHashes.snapshot(changed)
      try replacement.acknowledge(changed)
      let owner=DraftBackupOwner(ownerID:UUID().uuidString.lowercased(),storeID:UUID().uuidString.lowercased(),documentID:changed.document_id,leaseSerial:1,runtime:"selection-replacement",documentGeneration:1)
      _ = try replacement.backupSnapshot(owner:owner,sequence:1)
      replacementAdapter.reflectAcknowledgedSource()
      precondition(replacementAdapter.textView.string=="🙂x")
      _ = try replacement.backupSnapshot(owner:owner,sequence:2)
    }
    print("Exact scalar roundtrips \(scalarCases); Chinese/emoji/NFD/ZWJ/flags/Tab/CRLF/EOF, invalid bytes/surrogates/ranges rejected; actual TextKit2 newline/plain paste and original bytes passed")
    window.orderOut(nil);try await session.close();try await services.close()
  }
}
