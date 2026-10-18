import AppKit
import SwiftUI

private final class AssistantComposerUndoManager:UndoManager {
  weak var editor:AssistantComposerTextView?
  override var canUndo:Bool {MainActor.assumeIsolated {editor?.hasMarkedText() != true && super.canUndo}}
  override var canRedo:Bool {MainActor.assumeIsolated {editor?.hasMarkedText() != true && super.canRedo}}
  override func undo() {MainActor.assumeIsolated {if editor?.hasMarkedText() != true {super.undo()}}}
  override func redo() {MainActor.assumeIsolated {if editor?.hasMarkedText() != true {super.redo()}}}
}
@MainActor final class AssistantComposerTextView:NSTextView {
  private let localUndo=AssistantComposerUndoManager()
  private var editingDepth=0
  override var undoManager:UndoManager? {localUndo.editor=self;localUndo.groupsByEvent=false;return localUndo}
  private func edit(_ work:()->Void) {
    let outer=editingDepth==0
    if outer,localUndo.groupingLevel==0 {localUndo.beginUndoGrouping()}
    editingDepth+=1;work();editingDepth-=1
    if outer,!hasMarkedText(),localUndo.groupingLevel>0 {localUndo.endUndoGrouping()}
  }
  func replaceDraft(_ text:String) {
    guard !hasMarkedText() else {return}
    if localUndo.groupingLevel>0 {localUndo.endUndoGrouping()}
    string=text;localUndo.removeAllActions()
  }
  var submit:(()->Void)?,compositionChanged:((Bool)->Void)?,focusChanged:((Bool)->Void)?
  var pasteAttachments:((NSPasteboard)->Bool)?
  var shortcut:AssistantSendShortcut = .enter
  private var markedReturn=false
  override func becomeFirstResponder()->Bool {let value=super.becomeFirstResponder();if value {focusChanged?(true)};return value}
  override func resignFirstResponder()->Bool {let value=super.resignFirstResponder();if value {focusChanged?(false)};return value}
  override func insertText(_ string:Any,replacementRange:NSRange) {edit {super.insertText(string,replacementRange:replacementRange)}}
  override func deleteBackward(_ sender:Any?) {edit {super.deleteBackward(sender)}}
  override func deleteForward(_ sender:Any?) {edit {super.deleteForward(sender)}}
  override func paste(_ sender:Any?) {if pasteAttachments?(.general) != true {edit {super.pasteAsPlainText(sender)}}}
  override func draggingEntered(_ sender:any NSDraggingInfo)->NSDragOperation {
    AssistantAttachmentInput.accepts(sender.draggingPasteboard) ? .copy:super.draggingEntered(sender)
  }
  override func performDragOperation(_ sender:any NSDraggingInfo)->Bool {
    if pasteAttachments?(sender.draggingPasteboard)==true {return true};return super.performDragOperation(sender)
  }
  override func insertNewline(_ sender:Any?) {edit {super.insertNewline(sender)}}
  override func setMarkedText(_ string:Any,selectedRange:NSRange,replacementRange:NSRange) {edit {super.setMarkedText(string,selectedRange:selectedRange,replacementRange:replacementRange)};compositionChanged?(hasMarkedText())}
  override func unmarkText() {super.unmarkText();if editingDepth==0,localUndo.groupingLevel>0 {localUndo.endUndoGrouping()};compositionChanged?(hasMarkedText())}
  private func isReturn(_ event:NSEvent)->Bool {event.keyCode==36 || event.keyCode==76}
  override func keyDown(with event:NSEvent) {
    guard isReturn(event) else {super.keyDown(with:event);return}
    if hasMarkedText() {
      markedReturn=true;defer {markedReturn=false;compositionChanged?(hasMarkedText())}
      super.keyDown(with:event);return
    }
    let flags=event.modifierFlags.intersection([.command,.shift,.option,.control])
    if AssistantComposerRules.shouldSend(marked:markedReturn,command:flags.contains(.command),shift:flags.contains(.shift),otherModifiers:!flags.intersection([.option,.control]).isEmpty,shortcut:shortcut) {submit?()}
    else {insertNewline(nil)}
  }
  override func performKeyEquivalent(with event:NSEvent)->Bool {
    if window?.firstResponder === self,isReturn(event),event.modifierFlags.contains(.command) {keyDown(with:event);return true}
    return super.performKeyEquivalent(with:event)
  }
}

@MainActor final class AssistantComposerScrollView:NSScrollView {
  let editor=AssistantComposerTextView(usingTextLayoutManager:true)
  var heightChanged:((CGFloat)->Void)?
  var maximumHeight:CGFloat=180
  override init(frame:NSRect) {
    super.init(frame:frame);drawsBackground=false;hasVerticalScroller=true;autohidesScrollers=true
    editor.isRichText=false;editor.drawsBackground=false;editor.isEditable=true;editor.isSelectable=true;editor.allowsUndo=true
    editor.font=NSFont.systemFont(ofSize:13.5);editor.textColor = .labelColor
    editor.textContainerInset=NSSize(width:0,height:5);editor.textContainer?.lineFragmentPadding=0
    editor.textContainer?.widthTracksTextView=true;editor.isVerticallyResizable=true;editor.isHorizontallyResizable=false
    documentView=editor;editor.setAccessibilityLabel("给助手的问题");editor.setAccessibilityIdentifier("assistant-input")
    editor.registerForDraggedTypes([.fileURL,.png,.tiff])
  }
  required init?(coder:NSCoder) {fatalError("not used")}
  @discardableResult func measure(width:CGFloat)->CGFloat {
    let width=max(1,width)
    editor.frame.size.width=width;editor.textContainer?.containerSize=NSSize(width:width,height:.greatestFiniteMagnitude)
    var height:CGFloat=24
    if let layout=editor.textLayoutManager,let content=layout.textContentManager {
      layout.ensureLayout(for:content.documentRange)
      layout.enumerateTextLayoutFragments(from:content.documentRange.location,options:[.ensuresLayout,.ensuresExtraLineFragment]) {fragment in
        height=max(height,fragment.layoutFragmentFrame.maxY);return true
      }
    }
    height=ceil(height+10);let visible=min(maximumHeight,max(46,height))
    editor.frame.size.height=max(visible,height);hasVerticalScroller=height>maximumHeight
    return visible
  }
  override func layout() {super.layout();let height=measure(width:contentSize.width);heightChanged?(height)}
}

struct AssistantComposerEditor:NSViewRepresentable {
  @ObservedObject var assistant:NativeAssistant
  let maximumHeight:CGFloat
  @Binding var height:CGFloat
  @Binding var focused:Bool
  func makeCoordinator()->Coordinator {Coordinator(self)}
  func makeNSView(context:Context)->AssistantComposerScrollView {
    let scroll=AssistantComposerScrollView();scroll.editor.delegate=context.coordinator
    scroll.editor.string=assistant.draft
    scroll.editor.submit={ [weak assistant] in assistant?.send() }
    scroll.editor.pasteAttachments={ [weak assistant] board in guard let assistant else {return false};return AssistantAttachmentInput.read(board,assistant:assistant)}
    assistant.addFilesRequested={ [weak scroll,weak assistant] in guard let assistant else {return};AssistantAttachmentInput.choose(window:scroll?.window,assistant:assistant)}
    scroll.editor.compositionChanged={ [weak assistant] marked in assistant?.composerMarked=marked }
    scroll.editor.focusChanged={value in focused=value}
    scroll.heightChanged={value in DispatchQueue.main.async {if abs(height-value)>0.5 {height=value}}}
    assistant.focusComposer={ [weak scroll] in guard let editor=scroll?.editor else {return};editor.window?.makeFirstResponder(editor)}
    return scroll
  }
  func updateNSView(_ scroll:AssistantComposerScrollView,context:Context) {
    context.coordinator.parent=self;scroll.maximumHeight=maximumHeight;scroll.editor.shortcut=assistant.sendShortcut
    if !scroll.editor.hasMarkedText(),!scroll.editor.string.utf8.elementsEqual(assistant.draft.utf8) {
      let selection=scroll.editor.selectedRange();scroll.editor.replaceDraft(assistant.draft)
      scroll.editor.setSelectedRange(NSRange(location:min(selection.location,(assistant.draft as NSString).length),length:0))
    }
    scroll.needsLayout=true
  }
  func sizeThatFits(_ proposal:ProposedViewSize,nsView:AssistantComposerScrollView,context:Context)->CGSize? {
    CGSize(width:proposal.width ?? 340,height:nsView.measure(width:proposal.width ?? 340))
  }
  @MainActor final class Coordinator:NSObject,NSTextViewDelegate {
    var parent:AssistantComposerEditor
    init(_ parent:AssistantComposerEditor) {self.parent=parent}
    func textDidChange(_ notification:Notification) {
      guard let editor=notification.object as? AssistantComposerTextView else {return}
      parent.assistant.composerMarked=editor.hasMarkedText();parent.assistant.draftEdited(editor.string)
      editor.enclosingScrollView?.needsLayout=true
    }
  }
}
