import AppKit

private final class EditorUndoRouter:UndoManager {
  weak var owner:DraftTextView?
  override var canUndo:Bool {MainActor.assumeIsolated {owner?.canRouteUndo==true}}
  override var canRedo:Bool {MainActor.assumeIsolated {owner?.canRouteRedo==true}}
  override func undo() {MainActor.assumeIsolated {owner?.routeUndo(redo:false)}}
  override func redo() {MainActor.assumeIsolated {owner?.routeUndo(redo:true)}}
}
/// Marked-text changes can happen without textDidChange (for example composition ownership
/// changes while the bytes stay identical). Report them synchronously before a COMMIT barrier.
@MainActor private final class DraftTextView:NSTextView {
  var compositionChanged:(()->Void)?
  var dirty:()->Bool = {true}
  var inputBegan:(()->String)?
  var inputEnded:(()->Void)?
  var focusChanged:(()->Void)?
  var finishReading:(()->Void)?
  var focusLost:(()->Void)?
  var newline:()->String = {"\n"}
  override func becomeFirstResponder()->Bool {let accepted=super.becomeFirstResponder();if accepted {focusChanged?()};return accepted}
  override func resignFirstResponder()->Bool {
    let accepted=super.resignFirstResponder()
    if accepted,!hasMarkedText() {focusLost?()}
    return accepted
  }
  override func keyDown(with event:NSEvent) {
    let commandReturn=event.keyCode==36 && event.modifierFlags.intersection([.command,.option,.control,.shift]) == .command
    if finishReading != nil,!hasMarkedText(),event.keyCode==53 || commandReturn {
      finishReading?();return
    }
    super.keyDown(with:event)
  }
  var documentManager:UndoManager?
  let local=UndoManager()
  private let router=EditorUndoRouter()
  private var commandDepth=0
  private var openInputGroup:String?
  private var groupTimer:Task<Void,Never>?
  override var undoManager:UndoManager? {
    router.owner=self
    return commandDepth>0 ? local : router
  }
  var canRouteUndo:Bool {hasMarkedText() ? false : (dirty() ? local.canUndo : documentManager?.canUndo==true)}
  var canRouteRedo:Bool {hasMarkedText() ? false : (local.canRedo || (!dirty() && documentManager?.canRedo==true))}
  func routeUndo(redo:Bool) {
    guard !hasMarkedText() else {return}
    finishInputGroup()
    if (redo && local.canRedo) || (!redo && dirty() && local.canUndo) {
      commandDepth+=1;defer {commandDepth-=1}
      if redo {local.redo()} else {local.undo()}
      compositionChanged?()
    } else if !dirty() {
      if redo {documentManager?.redo()} else {documentManager?.undo()}
    }
  }
  private func input(_ body:()->Void) {
    let outer=commandDepth==0
    if outer {
      if !dirty() {finishInputGroup();local.removeAllActions()}
      let group=inputBegan?() ?? UUID().uuidString.lowercased()
      if openInputGroup != group || local.groupingLevel==0 {
        finishInputGroup();local.groupsByEvent=false;local.beginUndoGrouping();openInputGroup=group
      }
    }
    commandDepth+=1;body();commandDepth-=1
    if outer {
      inputEnded?();groupTimer?.cancel()
      groupTimer=Task { [weak self] in
        do {try await Task.sleep(for:.milliseconds(600))} catch {return}
        guard let self,!hasMarkedText() else {return};finishInputGroup()
      }
    }
  }
  /// Close only our native group, outside a text command. Composition keeps one group until
  /// the input engine actually ends it; a durable echo cannot create another text operation.
  func finishInputGroup() {
    groupTimer?.cancel();groupTimer=nil
    if openInputGroup != nil,local.groupingLevel>0 {local.endUndoGrouping()}
    openInputGroup=nil
  }
  override func insertText(_ string:Any,replacementRange:NSRange) {input {super.insertText(string,replacementRange:replacementRange)}}
  override func deleteBackward(_ sender:Any?) {input {super.deleteBackward(sender)}}
  override func deleteForward(_ sender:Any?) {input {super.deleteForward(sender)}}
  override func paste(_ sender:Any?) {
    input {super.pasteAsPlainText(sender)}
  }
  override func pasteAsPlainText(_ sender:Any?) {input {super.pasteAsPlainText(sender)}}
  override func insertNewline(_ sender:Any?) {
    guard !hasMarkedText() else {super.insertNewline(sender);return}
    insertText(newline(),replacementRange:NSRange(location:NSNotFound,length:0))
  }
  override func setMarkedText(_ string:Any,selectedRange:NSRange,replacementRange:NSRange) {
    input {super.setMarkedText(string,selectedRange:selectedRange,replacementRange:replacementRange)}
    compositionChanged?()
  }
  override func unmarkText() {super.unmarkText();compositionChanged?()}
  func acknowledged() {if !dirty(),!hasMarkedText() {finishInputGroup();local.removeAllActions()}}
}
/// Minimal real native marked-text/input adapter. Rich TextKit2 syntax/selection rendering is
/// completed by the editor tasks; this layer never substitutes a model/fixture IME flag.
@MainActor final class DraftTextViewAdapter:NSObject,NSTextViewDelegate {
  let textView:NSTextView
  private let store:DraftStore
  private let cell:String
  private let generation:UInt64
  private var group=UUID().uuidString.lowercased()
  private var lastInputNS:UInt64=0
  private var previousCaret:NSRange?
  private var inComposition=false
  var didEdit:(()->Void)?
  var didFocus:(()->Void)?
  var didFinishReading:(()->Void)? {didSet {(textView as? DraftTextView)?.finishReading=didFinishReading}}
  var didLoseFocus:(()->Void)? {didSet {(textView as? DraftTextView)?.focusLost=didLoseFocus}}
  var inputGroup:String {group}
  private(set) var indexMap:SourceIndexMap
  init(store:DraftStore,cell:String,generation:UInt64) throws {
    self.store=store;self.cell=cell;self.generation=generation
    let native=DraftTextView(usingTextLayoutManager:true)
    textView=native
    indexMap=try SourceIndexMap(source:"",draftSequence:0)
    super.init()
    try store.attachEditor(cell:cell,generation:generation)
    textView.isRichText=false;textView.allowsUndo=true;textView.string=store.overlay(cell)!.source
    indexMap=try SourceIndexMap(source:textView.string,draftSequence:store.overlay(cell)!.sequence)
    if store.confirmed.file.cells.first(where:{$0.id.utf8.elementsEqual(cell.utf8)})?.kind == .math {
      textView.smartInsertDeleteEnabled=false
      textView.isAutomaticQuoteSubstitutionEnabled=false;textView.isAutomaticDashSubstitutionEnabled=false
      textView.isAutomaticTextReplacementEnabled=false;textView.isAutomaticSpellingCorrectionEnabled=false
      textView.isAutomaticTextCompletionEnabled=false;textView.isAutomaticLinkDetectionEnabled=false
      textView.isAutomaticDataDetectionEnabled=false;textView.isContinuousSpellCheckingEnabled=false
    }
    textView.selectedRanges=store.overlay(cell)!.selections.map{NSValue(range:$0.native)};textView.delegate=self
    native.compositionChanged={ [weak self] in try? self?.reportCurrentInput() }
    native.dirty={ [weak store] in guard let value=store?.overlay(cell) else {return true};return value.composing || value.conflict || !value.source.utf8.elementsEqual(value.baseSource.utf8)}
    native.inputBegan={ [weak self] in self?.beginInputGroup() ?? UUID().uuidString.lowercased() }
    native.focusChanged={ [weak self] in self?.didFocus?() }
    native.newline={ [weak self] in self?.indexMap.preferredNewline ?? "\n" }
    native.inputEnded={ [weak self] in try? self?.reportCurrentInput();self?.previousCaret=self?.textView.selectedRange() }
  }
  func bindDocumentUndo(_ manager:UndoManager) {(textView as? DraftTextView)?.documentManager=manager}
  func breakInputGroup() {(textView as? DraftTextView)?.finishInputGroup();group=UUID().uuidString.lowercased();previousCaret=nil;lastInputNS=0}
  private func beginInputGroup()->String {
    let now=DispatchTime.now().uptimeNanoseconds,caret=textView.selectedRange()
    if !inComposition && (lastInputNS==0 || now-lastInputNS>600_000_000 || previousCaret != caret) {group=UUID().uuidString.lowercased()}
    lastInputNS=now
    return group
  }
  func textDidChange(_ notification:Notification) {try? reportCurrentInput()}
  func textViewDidChangeSelection(_ notification:Notification) {try? reportCurrentInput()}
  func reportCurrentInput() throws {
    if textView.hasMarkedText() {inComposition=true}
    else if inComposition {inComposition=false;lastInputNS=DispatchTime.now().uptimeNanoseconds}
    let before=store.overlay(cell)?.sequence
    let selections=textView.selectedRanges.map{DraftTextRange($0.rangeValue)}
    let marked=textView.hasMarkedText() ? DraftTextRange(textView.markedRange()) : nil
    var map=indexMap.source.utf8.elementsEqual(textView.string.utf8) ? indexMap : try SourceIndexMap(source:textView.string,draftSequence:before ?? 0)
    for range in selections {_ = try map.byteRange(for:range.native)}
    if let marked {_ = try map.byteRange(for:marked.native)}
    try store.edited(cell:cell,generation:generation,source:textView.string,hasMarkedText:textView.hasMarkedText(),selections:selections,markedRange:marked)
    map.bindSequence(store.overlay(cell)!.sequence);indexMap=map
    if store.overlay(cell)?.sequence != before {didEdit?()}
  }
  /// Owned acknowledgement never replaces a later overlay or force-ends composition.
  func reflectAcknowledgedSource() {
    guard !textView.hasMarkedText(),let overlay=store.overlay(cell),!overlay.conflict else {return}
    if overlay.source.utf8.elementsEqual(textView.string.utf8) {indexMap.bindSequence(overlay.sequence);(textView as? DraftTextView)?.acknowledged();return}
    let selected=textView.selectedRanges
    textView.string=overlay.source;textView.selectedRanges=selected
    if let actual=try? SourceIndexMap(source:overlay.source,draftSequence:overlay.sequence) {indexMap=actual}
    (textView as? DraftTextView)?.acknowledged()
  }
}
