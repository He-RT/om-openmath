import AppKit
import CryptoKit
import SwiftUI

private extension NSAttributedString.Key {
  static let originalMarkdown=NSAttributedString.Key("org.openmath.markdown.original")
}
@MainActor final class MarkdownTextView:NativeMathTextView,NSTextViewDelegate {
  private(set) var mappings:[MarkdownSourceMapping]=[]
  private(set) var original=""
  private var renderedSource:String?
  private var renderedDark=false
  private var renderedSize:CGFloat=16
  private var renderedProjection=""
  private var renderedDigest=""
  private var renderedPaper=false
  var onDoubleClick:(()->Void)?
  private var installing=false
  private var pending:(MarkdownRenderDocument,Bool,CGFloat,Bool)?
  override func mouseDown(with event:NSEvent) {
    if event.clickCount==2,let onDoubleClick {onDoubleClick();return}
    super.mouseDown(with:event)
  }
  func configure() {
    isEditable=false;isSelectable=true;isRichText=true;drawsBackground=false
    textContainerInset=NSSize(width:2,height:4);textContainer?.lineFragmentPadding=0
    isVerticallyResizable=true;isHorizontallyResizable=false
    textContainer?.widthTracksTextView=true;delegate=self
    setAccessibilityLabel("Markdown 内容")
  }
  func install(_ document:MarkdownRenderDocument,dark:Bool,fontSize:CGFloat=16,paper:Bool=false) {
    let digest=document.sourceDigest.isEmpty ? SHA256.hash(data:Data(document.source.utf8)).map{String(format:"%02x",$0)}.joined():document.sourceDigest
    guard renderedDigest != digest || renderedDark != dark || renderedSize != fontSize || renderedProjection != document.projectionID || renderedPaper != paper else {return}
    // Pin a selected projection; install the pending update after selection ends.
    // Original source and formula delimiters remain available throughout.
    if selectedRange().length>0,renderedSource != nil {pending=(document,dark,fontSize,paper);return}
    installing=true;defer {installing=false};pending=nil;renderedProjection=document.projectionID;renderedDigest=digest
    original=document.source;renderedSource=document.source;renderedDark=dark;renderedSize=fontSize;renderedPaper=paper
    let output=NSMutableAttributedString(string:""),sourceBytes=Array(document.source.utf8),sourceHash=digest
    mappings=[]
    for run in document.runs {
      let style=run.style
      let size=paper && style.heading>0 ? (style.heading==1 ? 30:style.heading==2 ? 21:18) : style.heading>0 ? max(fontSize,fontSize+CGFloat(7-min(style.heading,6))*2):fontSize
      var font=style.code ? NSFont.monospacedSystemFont(ofSize:size,weight:.regular):paper ? Theme.Font.paper(size:size,weight:style.bold ? .semibold:.regular):NSFont.systemFont(ofSize:size,weight:style.bold ? .semibold:.regular)
      if style.bold,style.code {font=NSFontManager.shared.convert(font,toHaveTrait:.boldFontMask)}
      if style.italic {font=NSFontManager.shared.convert(font,toHaveTrait:.italicFontMask)}
      let paragraph=NSMutableParagraphStyle();paragraph.lineSpacing=paper ? 7:4;paragraph.paragraphSpacing=4
      paragraph.headIndent=CGFloat(style.indent)*18;paragraph.firstLineHeadIndent=paragraph.headIndent
      paragraph.tabStops=[NSTextTab(textAlignment:.left,location:paragraph.headIndent+20)]
      var attributes:[NSAttributedString.Key:Any]=[.font:font,.foregroundColor:style.quote>0 ? NSColor.secondaryLabelColor:NSColor.labelColor,.paragraphStyle:paragraph]
      if style.code {attributes[.backgroundColor]=NSColor.quaternaryLabelColor.withAlphaComponent(0.12)}
      if style.strike {attributes[.strikethroughStyle]=NSUnderlineStyle.single.rawValue}
      if let target=style.link,let url=Self.allowedURL(target) {attributes[.link]=url;attributes[.foregroundColor]=NSColor.linkColor}
      let start=output.length
      let kind:MarkdownSpanMapping
      switch run.content {
      case .text(let text):
        output.append(NSAttributedString(string:text,attributes:attributes))
        if let span=run.source,span.upperBound<=sourceBytes.count,sourceBytes[span].elementsEqual(text.utf8) {kind = .exact}
        else {kind=run.source==nil ? .unavailable:.atomic}
      case .math(let latex,let raw,let display):
        if display {paragraph.alignment = .center;paragraph.paragraphSpacingBefore=10;paragraph.paragraphSpacing=10}
        let identity="markdown-"+sourceHash+"-"+String(run.source?.lowerBound ?? start)
        let attachment=NativeMathAttachment(request:.init(identity:identity,originalLatex:latex,wolfram:"",modern:nil,inline:!display,fontSize:display ? max(22,size):size,dark:dark),display:display)
        let value=NSMutableAttributedString(attachment:attachment);value.addAttributes(attributes,range:NSRange(location:0,length:value.length));value.addAttribute(.originalMarkdown,value:raw,range:NSRange(location:0,length:value.length))
        output.append(value);kind = .atomic
      }
      mappings.append(.init(source:run.source,rendered:NSRange(location:start,length:output.length-start),mapping:kind))
    }
    textStorage?.setAttributedString(output);setSelectedRange(NSRange(location:0,length:0));needsLayout=true
  }
  func textViewDidChangeSelection(_ notification:Notification) {
    guard !installing,selectedRange().length==0,let (document,dark,size,paper)=pending else {return}
    pending=nil
    Task { [weak self] in self?.install(document,dark:dark,fontSize:size,paper:paper) }
  }
  func measuredHeight(width:CGFloat)->CGFloat {
    guard width.isFinite,width>0 else {return 1}
    frame.size.width=width
    textContainer?.containerSize=NSSize(width:max(1,width-textContainerInset.width*2),height:.greatestFiniteMagnitude)
    guard let layout=textLayoutManager,let content=layout.textContentManager else {return 24}
    layout.ensureLayout(for:content.documentRange)
    var end:CGFloat=0
    layout.enumerateTextLayoutFragments(from:content.documentRange.location,options:[.ensuresLayout,.ensuresExtraLineFragment]) {fragment in end=max(end,fragment.layoutFragmentFrame.maxY);return true}
    return max(24,ceil(end+textContainerInset.height*2))
  }
  override func copy(_ sender:Any?) {
    guard let textStorage else {return};let selected=selectedRange()
    guard selected.length>0,selected.location != NSNotFound,NSMaxRange(selected)<=textStorage.length else {return}
    var output=""
    textStorage.enumerateAttributes(in:selected) {attributes,range,_ in
      output+=(attributes[.originalMarkdown] as? String) ?? textStorage.attributedSubstring(from:range).string
    }
    NSPasteboard.general.clearContents();NSPasteboard.general.setString(output,forType:.string)
  }
  static func allowedURL(_ source:String)->URL? {
    guard let url=URL(string:source),let scheme=url.scheme?.lowercased(),["https","http","mailto"].contains(scheme) else {return nil};return url
  }
  func textView(_ textView:NSTextView,clickedOnLink link:Any,at charIndex:Int)->Bool {
    guard let url=Self.allowedURL((link as? URL)?.absoluteString ?? link as? String ?? "") else {return true}
    NSWorkspace.shared.open(url);return true
  }
}
private struct MarkdownNativeSurface:NSViewRepresentable {
  let document:MarkdownRenderDocument,dark:Bool,fontSize:CGFloat,paper:Bool,onDoubleClick:(()->Void)?
  func makeNSView(context:Context)->MarkdownTextView {let view=MarkdownTextView(usingTextLayoutManager:true);view.configure();view.onDoubleClick=onDoubleClick;view.install(document,dark:dark,fontSize:fontSize,paper:paper);return view}
  func updateNSView(_ view:MarkdownTextView,context:Context) {view.onDoubleClick=onDoubleClick;view.install(document,dark:dark,fontSize:fontSize,paper:paper)}
  func sizeThatFits(_ proposal:ProposedViewSize,nsView:MarkdownTextView,context:Context)->CGSize? {
    let width=proposal.width ?? 600;return CGSize(width:width,height:nsView.measuredHeight(width:width))
  }
}
struct NativeMarkdownView:View {
  let source:String
  var fontSize:CGFloat=16
  var paper=false
  var onDoubleClick:(()->Void)?=nil
  @State private var parsed:MarkdownRenderDocument?
  @Environment(\.colorScheme) private var scheme
  var body:some View {
    Group {
      if let parsed,parsed.source.utf8.elementsEqual(source.utf8) {
        VStack(alignment:.leading,spacing:8) {
          if parsed.fallback {Text("内容较长或嵌套过深，显示原文。").font(.caption).foregroundStyle(.secondary)}
          ForEach(parsed.sections) {section in
            if let text=section.text {MarkdownNativeSurface(document:text,dark:scheme == .dark,fontSize:fontSize,paper:paper,onDoubleClick:onDoubleClick)}
            if let table=section.table {NativeMarkdownTableView(table:table,dark:scheme == .dark,fontSize:fontSize)}
          }
        }
      } else {Text(source).font(paper ? Theme.Font.paperBody:.system(size:fontSize)).textSelection(.enabled).frame(maxWidth:.infinity,alignment:.leading)}
    }.task(id:Data(source.utf8)) {
      let value=await Task.detached(priority:.userInitiated) {MarkdownRenderDocument.parse(source)}.value
      if !Task.isCancelled {parsed=value}
    }.frame(maxWidth:.infinity,alignment:.leading)
      .contextMenu {Button("复制完整 Markdown") {NSPasteboard.general.clearContents();NSPasteboard.general.setString(source,forType:.string)}}
  }
}
