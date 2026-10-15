import AppKit
import SwiftUI
import SwiftMath

struct NativeMathMetrics:Sendable {
  let size:CGSize,ascent:CGFloat,descent:CGFloat,baselineFromTop:CGFloat
}
/// Each surface owns a different native label. Its immutable input/key stays attached to the
/// original result, and SwiftMath/NSView/font operations are confined to MainActor.
@MainActor final class NativeMathContainer:NSView {
  private(set) var plan:NativeMathPlan?
  private(set) var math:MTMathUILabel?
  private(set) var metrics=NativeMathMetrics(size:.zero,ascent:0,descent:0,baselineFromTop:0)
  override var isFlipped:Bool {false}
  override var intrinsicContentSize:NSSize {metrics.size}
  override var fittingSize:NSSize {metrics.size}
  override var firstBaselineOffsetFromTop:CGFloat {metrics.baselineFromTop}
  override var lastBaselineOffsetFromBottom:CGFloat {metrics.size.height-metrics.baselineFromTop}
  override var acceptsFirstResponder:Bool {plan?.state == .ready && plan?.request.inline == false}
  override func becomeFirstResponder()->Bool {needsDisplay=true;return super.becomeFirstResponder()}
  override func resignFirstResponder()->Bool {needsDisplay=true;return super.resignFirstResponder()}
  override func mouseDown(with event:NSEvent) {
    if acceptsFirstResponder {window?.makeFirstResponder(self);needsDisplay=true}
    else {super.mouseDown(with:event)}
  }
  override func draw(_ dirtyRect:NSRect) {
    super.draw(dirtyRect)
    if window?.firstResponder === self {NSColor.selectedTextBackgroundColor.withAlphaComponent(0.25).setFill();bounds.fill()}
  }
  @objc func copy(_ sender:Any?) {
    guard let original=plan?.request.originalLatex else {return}
    NSPasteboard.general.clearContents();NSPasteboard.general.setString(original,forType:.string)
  }
  override func performKeyEquivalent(with event:NSEvent)->Bool {
    if window?.firstResponder === self,event.modifierFlags.contains(.command),event.charactersIgnoringModifiers?.lowercased()=="c" {copy(nil);return true}
    return super.performKeyEquivalent(with:event)
  }
  func configure(_ request:NativeMathRequest) {
    if plan?.request==request {return}
    configure(MathRenderAdapter.prepare(request))
  }
  func configure(_ actual:NativeMathPlan) {
    if plan?.request==actual.request,plan?.key==actual.key,plan?.state==actual.state {return}
    math?.removeFromSuperview();math=nil;plan=actual
    guard actual.state == .ready,let list=actual.list,let font=actual.font else {
      metrics = .init(size:NSSize(width:1,height:1),ascent:0,descent:0,baselineFromTop:0);invalidateIntrinsicContentSize();return
    }
    let label=MTMathUILabel();math=label
    label.font=font;label.fontSize=actual.request.fontSize;label.mathList=list
    label.labelMode=actual.request.inline ? .text : .display
    label.textAlignment = .left;label.displayErrorInline=false;label.textColor = .labelColor
    appearance=NSAppearance(named:actual.request.dark ? .darkAqua : .aqua)
    let size=label.fittingSize
    guard size.width.isFinite,size.height.isFinite,size.width>=0,size.height>=0 else {
      math=nil;metrics = .init(size:NSSize(width:1,height:1),ascent:0,descent:0,baselineFromTop:0);invalidateIntrinsicContentSize();return
    }
    label.frame=NSRect(origin:.zero,size:NSSize(width:max(1,size.width),height:max(actual.request.fontSize/2,size.height)))
    addSubview(label);label.needsLayout=true;label.layoutSubtreeIfNeeded()
    if let display=label.displayList {
      // The fixed label places its baseline at actual descent when its frame is at least
      // max(ascent+descent,fontSize/2). No private position API or guessed half-height baseline.
      metrics = .init(size:label.frame.size,ascent:display.ascent,descent:display.descent,baselineFromTop:label.frame.height-display.descent)
    } else {metrics = .init(size:label.frame.size,ascent:0,descent:0,baselineFromTop:label.frame.height)}
    setAccessibilityElement(true);setAccessibilityRole(.staticText)
    setAccessibilityLabel(actual.request.wolfram.isEmpty ? actual.request.originalLatex : actual.request.wolfram)
    setAccessibilityIdentifier("native-math-"+actual.request.identity)
    invalidateIntrinsicContentSize();needsLayout=true
  }
  override func layout() {super.layout();if let math {math.frame=NSRect(origin:.zero,size:metrics.size)}}
}

private struct NativeMathSurface:NSViewRepresentable {
  let plan:NativeMathPlan
  func makeNSView(context:Context)->NativeMathContainer {let view=NativeMathContainer();view.configure(plan);return view}
  func updateNSView(_ view:NativeMathContainer,context:Context) {view.configure(plan)}
  func sizeThatFits(_ proposal:ProposedViewSize,nsView:NativeMathContainer,context:Context)->CGSize? {nsView.metrics.size}
}
/// Fallback pages contain original characters, not a repaired/reformatted equation. The complete
/// original is always available to copy, including inputs larger than the rendering budget.
private struct NativeMathOriginalSource:View {
  let source:String
  @State private var page=0
  private let chunk=4096
  var body:some View {
    let start=min(source.count,page*chunk),end=min(source.count,start+chunk)
    let first=source.index(source.startIndex,offsetBy:start),last=source.index(first,offsetBy:end-start)
    VStack(alignment:.leading,spacing:6) {
      ScrollView(.horizontal) {Text(String(source[first..<last])).font(.system(.callout,design:.monospaced)).textSelection(.enabled).fixedSize(horizontal:true,vertical:false)}
      if source.count>chunk {
        HStack {
          Button("上一段") {page=max(0,page-1)}.disabled(page==0)
          Text("原式 \(start+1)–\(end) / \(source.count)").font(.caption).foregroundStyle(.secondary)
          Button("下一段") {page+=1}.disabled(end==source.count)
        }
      }
    }
  }
}

struct NativeMathView:View {
  let request:NativeMathRequest
  @State private var size:CGFloat=22
  @State private var showOriginal=false
  @Environment(\.colorScheme) private var scheme
  private var current:NativeMathRequest {var value=request;value.fontSize=size;value.dark=scheme == .dark;return value}
  var body:some View {
    let actual=current
    HStack(alignment:.top,spacing:12) {
      NativeMathDisplay(request:actual).frame(maxWidth:.infinity,alignment:.leading)
      Menu("公式操作",systemImage:"ellipsis") {
        Button("复制 LaTeX") {copy(request.originalLatex)}
        Button("复制 Wolfram 源式") {copy(request.wolfram)}.disabled(request.wolfram.isEmpty)
        Button("复制 Modern 源式") {if let modern=request.modern {copy(modern)}}.disabled(request.modern==nil)
        Divider()
        Button("查看原始 LaTeX…") {showOriginal=true}
        Divider()
        Text("字号 \(Int(size)) pt")
        Button("放大字号") {size=min(96,size+2)}.disabled(size>=96)
        Button("缩小字号") {size=max(8,size-2)}.disabled(size<=8)
        Button("恢复默认字号") {size=request.fontSize}
      }.labelStyle(.iconOnly).menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize().foregroundStyle(.secondary)
        .popover(isPresented:$showOriginal) {
          VStack(alignment:.leading,spacing:10) {Text("原始 LaTeX").font(.headline);NativeMathOriginalSource(source:request.originalLatex);Button("复制原始 LaTeX") {copy(request.originalLatex)}}.padding(16).frame(width:480)
        }
    }.onAppear {size=request.fontSize}
      .accessibilityElement(children:.contain).accessibilityIdentifier("notebook-formula-"+request.identity)
      .contextMenu {
        Button("复制原始 LaTeX") {copy(request.originalLatex)}
        Button("复制 Wolfram 源式") {copy(request.wolfram)}.disabled(request.wolfram.isEmpty)
        if let modern=request.modern {Button("复制 Modern 源式") {copy(modern)}}
      }
  }
  private func copy(_ value:String) {NSPasteboard.general.clearContents();NSPasteboard.general.setString(value,forType:.string)}
}
private struct NativeMathDisplay:View {
  let request:NativeMathRequest
  @State private var plan:NativeMathPlan?
  var body:some View {
    Group {
      if let plan,plan.request==request {
        if plan.state == .ready {
          ScrollView(.horizontal) {NativeMathSurface(plan:plan).fixedSize(horizontal:true,vertical:true)}
        } else {
          Text(plan.state.reason).font(.caption).foregroundStyle(.secondary)
          NativeMathOriginalSource(source:request.originalLatex).id(request.identity)
        }
      } else {ProgressView("正在准备公式").controlSize(.small)}
    }.task(id:request) {if !Task.isCancelled {plan=MathRenderAdapter.prepare(request)}}
  }
}

/// TextKit2 gets the true mathematical ascent/descent via a native view provider. Markdown
/// composition/copy policy is layered above this attachment in its dedicated task.
@MainActor final class NativeMathAttachment:NSTextAttachment {
  nonisolated let request:NativeMathRequest
  let plan:NativeMathPlan
  let metrics:NativeMathMetrics
  override nonisolated var fileType:String? {get {"org.openmath.native-math"} set {}}
  override nonisolated var contents:Data? {get {Data(request.originalLatex.utf8)} set {}}
  private static let registration:Void = {NSTextAttachment.registerViewProviderClass(NativeMathAttachmentProvider.self,forFileType:"org.openmath.native-math")}()
  init(request:NativeMathRequest) {
    var inline=request;inline.inline=true;self.request=inline
    plan=MathRenderAdapter.prepare(inline)
    if plan.state == .ready {let measure=NativeMathContainer();measure.configure(plan);metrics=measure.metrics}
    else {let fallback=NativeMathFallbackLabel(request:inline,state:plan.state);metrics=fallback.metrics}
    Self.registration
    super.init(data:Data(inline.originalLatex.utf8),ofType:"org.openmath.native-math")
    // Original bytes and the registered type are immutable subclass properties; macOS's
    // legacy data/file-wrapper inference must not erase the custom view-provider identity.
    allowsTextAttachmentView=true
    bounds=NSRect(x:0,y:-metrics.descent,width:metrics.size.width,height:metrics.size.height)
  }
  required init?(coder:NSCoder) {fatalError("native formula attachments require an original request")}
  override func attachmentBounds(for attributes:[NSAttributedString.Key:Any],location:any NSTextLocation,textContainer:NSTextContainer?,proposedLineFragment:CGRect,position:CGPoint)->CGRect {bounds}
  override func image(for imageBounds:CGRect,attributes:[NSAttributedString.Key:Any],location:any NSTextLocation,textContainer:NSTextContainer?)->NSImage? {
    // Keep AppKit's view-provider discovery, then suppress its legacy generic-file icon.
    _ = super.image(for:imageBounds,attributes:attributes,location:location,textContainer:textContainer)
    return NSImage(size:metrics.size)
  }
  override nonisolated func viewProvider(for parentView:NSView?,location:any NSTextLocation,textContainer:NSTextContainer?)->NSTextAttachmentViewProvider? {
    let provider=super.viewProvider(for:parentView,location:location,textContainer:textContainer)
    precondition(Thread.isMainThread)
    MainActor.assumeIsolated {parentView?.needsLayout=true}
    return provider
  }
}
@MainActor final class NativeMathAttachmentProvider:NSTextAttachmentViewProvider {
  override nonisolated func loadView() {
    // AppKit's ObjC callback predates actor annotations. This is an in-place assertion of its
    // UI-thread contract, never a task/dispatch or transfer of a native view to another thread.
    precondition(Thread.isMainThread)
    nonisolated(unsafe) let owner=self
    MainActor.assumeIsolated {
      guard let attachment=owner.textAttachment as? NativeMathAttachment else {return}
      if attachment.plan.state == .ready {let content=NativeMathContainer();content.configure(attachment.plan);owner.view=content}
      else {owner.view=NativeMathFallbackLabel(request:attachment.request,state:attachment.plan.state)}
    }
  }
}
@MainActor final class NativeMathFallbackLabel:NSTextField {
  let original:String
  let metrics:NativeMathMetrics
  init(request:NativeMathRequest,state:NativeMathState) {
    original=request.originalLatex
    let size=request.fontSize.isFinite && request.fontSize>=8 && request.fontSize<=96 ? request.fontSize : 16
    let face=NSFont.monospacedSystemFont(ofSize:size,weight:.regular)
    let text=String(original.prefix(4096))+(original.count>4096 ? " …（可复制完整原式）" : "")
    let measured=(text as NSString).size(withAttributes:[.font:face])
    metrics = .init(size:NSSize(width:max(1,measured.width)+8,height:face.ascender-face.descender),ascent:face.ascender,descent:-face.descender,baselineFromTop:face.ascender)
    super.init(frame:NSRect(origin:.zero,size:metrics.size))
    isEditable=false;isSelectable=true;isBezeled=false;isBordered=false;drawsBackground=false
    font=face;stringValue=text;textColor = .labelColor;maximumNumberOfLines=1
    lineBreakMode = .byClipping;cell?.wraps=false;cell?.isScrollable=true
    toolTip=state.reason;setAccessibilityLabel(state.reason+"："+text)
    let context=NSMenu();let item=NSMenuItem(title:"复制原始 LaTeX",action:#selector(copyOriginal),keyEquivalent:"");item.target=self;context.addItem(item);menu=context
  }
  required init?(coder:NSCoder) {fatalError("original formula is required")}
  @objc func copyOriginal() {NSPasteboard.general.clearContents();NSPasteboard.general.setString(original,forType:.string)}
}

/// AppKit supplies the real provider and attachment frame on each laid-out fragment, but the
/// stock macOS text view does not insert our custom provider's view into its rendering surface.
/// This host installs only the current viewport's native views at those exact TextKit2 frames.
@MainActor final class NativeMathTextView:NSTextView {
  private var mathViews:[ObjectIdentifier:NSView]=[:]
  private var mounting=false
  override func layout() {super.layout();mountMathAttachments()}
  override func textViewportLayoutControllerDidLayout(_ controller:NSTextViewportLayoutController) {
    super.textViewportLayoutControllerDidLayout(controller)
    mountMathAttachments()
  }
  private func mountMathAttachments() {
    guard !mounting,let manager=textLayoutManager,let content=manager.textContentManager else {return}
    mounting=true;defer {mounting=false}
    let visible=visibleRect.insetBy(dx:0,dy:-256),origin=textContainerOrigin
    let start=manager.textViewportLayoutController.viewportRange?.location ?? content.documentRange.location
    var current=Set<ObjectIdentifier>()
    manager.enumerateTextLayoutFragments(from:start,options:.ensuresLayout) {fragment in
      let area=fragment.layoutFragmentFrame.offsetBy(dx:origin.x,dy:origin.y)
      if area.minY>visible.maxY {return false}
      for provider in fragment.textAttachmentViewProviders {
        guard provider.textAttachment is NativeMathAttachment,let view=provider.view else {continue}
        let local=fragment.frameForTextAttachment(at:provider.location)
        guard local.width>0,local.height>0 else {continue}
        let id=ObjectIdentifier(provider);current.insert(id)
        if view.superview !== self {view.removeFromSuperview();addSubview(view)}
        view.frame=local.offsetBy(dx:area.minX,dy:area.minY);mathViews[id]=view
      }
      return true
    }
    for id in Array(mathViews.keys) where !current.contains(id) {mathViews.removeValue(forKey:id)?.removeFromSuperview()}
  }
}
