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
  var inputBegan:(()->Void)?
  var inputEnded:(()->Void)?
  var documentManager:UndoManager?
  let local=UndoManager()
  private let router=EditorUndoRouter()
  private var commandDepth=0
  override var undoManager:UndoManager? {
    router.owner=self
    return commandDepth>0 ? local : router
  }
  var canRouteUndo:Bool {hasMarkedText() ? false : (dirty() ? local.canUndo : documentManager?.canUndo==true)}
  var canRouteRedo:Bool {hasMarkedText() ? false : (local.canRedo || (!dirty() && documentManager?.canRedo==true))}
  func routeUndo(redo:Bool) {
    guard !hasMarkedText() else {return}
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
      if !dirty() {local.removeAllActions()}
      local.groupsByEvent=false;local.beginUndoGrouping();inputBegan?()
    }
    commandDepth+=1;body();commandDepth-=1
    if outer {local.endUndoGrouping();inputEnded?()}
  }
  override func insertText(_ string:Any,replacementRange:NSRange) {input {super.insertText(string,replacementRange:replacementRange)}}
  override func deleteBackward(_ sender:Any?) {input {super.deleteBackward(sender)}}
  override func deleteForward(_ sender:Any?) {input {super.deleteForward(sender)}}
  override func paste(_ sender:Any?) {input {super.paste(sender)}}
  override func setMarkedText(_ string:Any,selectedRange:NSRange,replacementRange:NSRange) {
    input {super.setMarkedText(string,selectedRange:selectedRange,replacementRange:replacementRange)}
    compositionChanged?()
  }
  override func unmarkText() {super.unmarkText();compositionChanged?()}
  func acknowledged() {if !dirty(),!hasMarkedText() {local.removeAllActions()}}
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
  var inputGroup:String {group}
  init(store:DraftStore,cell:String,generation:UInt64) throws {
    self.store=store;self.cell=cell;self.generation=generation
    let native=DraftTextView(usingTextLayoutManager:true)
    textView=native
    super.init()
    try store.attachEditor(cell:cell,generation:generation)
    textView.isRichText=false;textView.allowsUndo=true;textView.string=store.overlay(cell)!.source;textView.delegate=self
    native.compositionChanged={ [weak self] in try? self?.reportCurrentInput() }
    native.dirty={ [weak store] in guard let value=store?.overlay(cell) else {return true};return value.composing || value.conflict || !value.source.utf8.elementsEqual(value.baseSource.utf8)}
    native.inputBegan={ [weak self] in self?.beginInputGroup() }
    native.inputEnded={ [weak self] in try? self?.reportCurrentInput();self?.previousCaret=self?.textView.selectedRange() }
  }
  func bindDocumentUndo(_ manager:UndoManager) {(textView as? DraftTextView)?.documentManager=manager}
  func breakInputGroup() {group=UUID().uuidString.lowercased();previousCaret=nil;lastInputNS=0}
  private func beginInputGroup() {
    let now=DispatchTime.now().uptimeNanoseconds,caret=textView.selectedRange()
    if !inComposition && (lastInputNS==0 || now-lastInputNS>600_000_000 || previousCaret != caret) {group=UUID().uuidString.lowercased()}
    lastInputNS=now
  }
  func textDidChange(_ notification:Notification) {try? reportCurrentInput()}
  func reportCurrentInput() throws {
    if textView.hasMarkedText() {inComposition=true}
    else if inComposition {inComposition=false;lastInputNS=DispatchTime.now().uptimeNanoseconds}
    try store.edited(cell:cell,generation:generation,source:textView.string,hasMarkedText:textView.hasMarkedText())
  }
  /// Owned acknowledgement never replaces a later overlay or force-ends composition.
  func reflectAcknowledgedSource() {
    guard !textView.hasMarkedText(),let overlay=store.overlay(cell),!overlay.conflict else {return}
    if overlay.source.utf8.elementsEqual(textView.string.utf8) {(textView as? DraftTextView)?.acknowledged();return}
    let selected=textView.selectedRanges
    textView.string=overlay.source;textView.selectedRanges=selected
    (textView as? DraftTextView)?.acknowledged()
  }
}
