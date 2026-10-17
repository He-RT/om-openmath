import AppKit
import SwiftUI

/// A whole code block owns one non-wrapping horizontal native scroll surface.
@MainActor final class NativeCodeScrollView:NSScrollView {
  let code=NSTextView(usingTextLayoutManager:true)
  override init(frame:NSRect) {
    super.init(frame:frame)
    drawsBackground=false;hasHorizontalScroller=true;hasVerticalScroller=false;autohidesScrollers=true
    code.isEditable=false;code.isSelectable=true;code.isRichText=false;code.drawsBackground=false
    code.font = .monospacedSystemFont(ofSize:12.5,weight:.regular)
    code.textContainerInset=NSSize(width:14,height:6)
    code.textContainer?.lineFragmentPadding=0;code.textContainer?.widthTracksTextView=false
    code.textContainer?.containerSize=NSSize(width:CGFloat.greatestFiniteMagnitude,height:CGFloat.greatestFiniteMagnitude)
    code.isHorizontallyResizable=true;code.isVerticallyResizable=true
    code.setAccessibilityLabel("代码块");documentView=code
  }
  required init?(coder:NSCoder) {fatalError("not used")}
  func install(_ text:String) {if code.string != text {code.string=text};resizeCode()}
  override func layout() {super.layout();resizeCode()}
  private func resizeCode() {
    let font=code.font ?? NSFont.monospacedSystemFont(ofSize:12.5,weight:.regular)
    let widest=code.string.split(separator:"\n",omittingEmptySubsequences:false).map{(String($0) as NSString).size(withAttributes:[.font:font]).width}.max() ?? 0
    let lines=max(1,code.string.reduce(1){$1 == "\n" ? $0+1:$0})
    code.frame.size=NSSize(width:max(contentSize.width,ceil(widest)+28),height:CGFloat(lines)*19+12)
  }
}
private struct NativeCodeSurface:NSViewRepresentable {
  let code:String
  func makeNSView(context:Context)->NativeCodeScrollView {let view=NativeCodeScrollView();view.install(code);return view}
  func updateNSView(_ view:NativeCodeScrollView,context:Context) {view.install(code)}
  func sizeThatFits(_ proposal:ProposedViewSize,nsView:NativeCodeScrollView,context:Context)->CGSize? {
    let lines=max(1,code.reduce(1){$1 == "\n" ? $0+1:$0})
    return CGSize(width:proposal.width ?? 400,height:CGFloat(lines)*19+24)
  }
}
struct NativeCodeBlockView:View {
  let block:MarkdownCodeBlock
  var onInsert:((String,String)->Void)?=nil
  @State private var copied=false
  var body:some View {
    VStack(alignment:.leading,spacing:0) {
      HStack(spacing:10) {
        Text(block.language.isEmpty ? "代码":block.language).font(.system(size:11.5,weight:.semibold))
        Spacer(minLength:8)
        Button(copied ? "已复制":"复制") {
          NSPasteboard.general.clearContents();copied=NSPasteboard.general.setString(block.code,forType:.string)
        }
        if block.isOpenMath,let onInsert {
          Button("插入为单元格") {onInsert(block.code,block.language)}
        }
      }.font(.system(size:11.5)).buttonStyle(.borderless).foregroundStyle(Theme.Color.ink2)
        .padding(.horizontal,14).padding(.top,8).padding(.bottom,4)
      NativeCodeSurface(code:block.code)
    }.background(Theme.Color.codeBackground,in:RoundedRectangle(cornerRadius:Theme.Radius.code))
      .overlay(alignment:.leading) {
        UnevenRoundedRectangle(topLeadingRadius:Theme.Radius.code,bottomLeadingRadius:Theme.Radius.code)
          .fill(block.isOpenMath ? Theme.Color.verified:Theme.Color.ink3).frame(width:3)
      }
      .accessibilityElement(children:.contain).accessibilityLabel("代码块，"+(block.language.isEmpty ? "未指定语言":block.language))
  }
}
