import AppKit

/// Marked-text changes can happen without textDidChange (for example composition ownership
/// changes while the bytes stay identical). Report them synchronously before a COMMIT barrier.
@MainActor private final class DraftTextView:NSTextView {
  var compositionChanged:(()->Void)?
  override func setMarkedText(_ string:Any,selectedRange:NSRange,replacementRange:NSRange) {
    super.setMarkedText(string,selectedRange:selectedRange,replacementRange:replacementRange)
    compositionChanged?()
  }
  override func unmarkText() {super.unmarkText();compositionChanged?()}
}
/// Minimal real native marked-text/input adapter. Rich TextKit2 syntax/selection rendering is
/// completed by the editor tasks; this layer never substitutes a model/fixture IME flag.
@MainActor final class DraftTextViewAdapter:NSObject,NSTextViewDelegate {
  let textView:NSTextView
  private let store:DraftStore
  private let cell:String
  private let generation:UInt64
  init(store:DraftStore,cell:String,generation:UInt64) throws {
    self.store=store;self.cell=cell;self.generation=generation
    let native=DraftTextView(usingTextLayoutManager:true)
    textView=native
    super.init()
    try store.attachEditor(cell:cell,generation:generation)
    textView.isRichText=false;textView.string=store.overlay(cell)!.source;textView.delegate=self
    native.compositionChanged={ [weak self] in try? self?.reportCurrentInput() }
  }
  func textDidChange(_ notification:Notification) {try? reportCurrentInput()}
  func reportCurrentInput() throws {
    try store.edited(cell:cell,generation:generation,source:textView.string,hasMarkedText:textView.hasMarkedText())
  }
  /// Owned acknowledgement never replaces a later overlay or force-ends composition.
  func reflectAcknowledgedSource() {
    guard !textView.hasMarkedText(),let overlay=store.overlay(cell),!overlay.conflict,
      !overlay.source.utf8.elementsEqual(textView.string.utf8) else {return}
    let selected=textView.selectedRanges
    textView.string=overlay.source;textView.selectedRanges=selected
  }
}
