import AppKit
import SwiftUI

/// AppKit/TextKit2 presentation of the existing document-owned editor. No second text owner.
struct NativeSourceEditor:NSViewRepresentable {
  let session:NativeDocumentSession
  let cell:NativeSourceCell
  func makeNSView(context:Context)->NSScrollView {
    let scroll=NSScrollView();scroll.hasVerticalScroller=true;scroll.hasHorizontalScroller=true
    do {
      let native=try session.editor(cell.id).textView
      native.font = .monospacedSystemFont(ofSize:14,weight:.regular);native.textContainerInset=NSSize(width:8,height:8)
      native.isVerticallyResizable=true;native.isHorizontallyResizable=false;native.autoresizingMask=[.width]
      native.textContainer?.widthTracksTextView=true
      native.setAccessibilityLabel(cell.kind == .math ? "数学源码" : "文字源码")
      native.setAccessibilityIdentifier("notebook-cell-"+cell.id);scroll.documentView=native
    } catch {
      let message=NSTextView();message.string="编辑器不可用："+error.localizedDescription;message.isEditable=false;scroll.documentView=message
    }
    return scroll
  }
  func updateNSView(_ nsView:NSScrollView,context:Context) {}
}
