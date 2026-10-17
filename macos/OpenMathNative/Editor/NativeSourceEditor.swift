import AppKit
import SwiftUI

private final class NativeEditorScrollView:NSScrollView {
  var focusOnAttach=false
  override func viewDidMoveToWindow() {
    super.viewDidMoveToWindow()
    if focusOnAttach,let window,let documentView {focusOnAttach=false;window.makeFirstResponder(documentView)}
  }
}
/// AppKit/TextKit2 presentation of the existing document-owned editor. No second text owner.
struct NativeSourceEditor:NSViewRepresentable {
  let session:NativeDocumentSession
  let cell:NativeSourceCell
  func makeNSView(context:Context)->NSScrollView {
    let scroll=NativeEditorScrollView();scroll.hasVerticalScroller=true;scroll.hasHorizontalScroller=false;scroll.drawsBackground=false
    do {
      let native=try session.editor(cell.id).textView
      native.font = cell.kind == .math ? Theme.Font.nsSource : .systemFont(ofSize:16)
      native.textContainerInset=NSSize(width:14,height:10)
      native.drawsBackground=false
      native.isVerticallyResizable=true;native.isHorizontallyResizable=false;native.autoresizingMask=[.width]
      native.textContainer?.widthTracksTextView=true
      native.setAccessibilityLabel(cell.kind == .math ? "数学源码" : "文字源码")
      native.setAccessibilityIdentifier("notebook-cell-"+cell.id);scroll.documentView=native
      scroll.focusOnAttach=session.claimEditorFocus(cell.id)
    } catch {
      let message=NSTextView();message.string="编辑器不可用："+error.localizedDescription;message.isEditable=false;scroll.documentView=message
    }
    return scroll
  }
  func updateNSView(_ nsView:NSScrollView,context:Context) {}
}
