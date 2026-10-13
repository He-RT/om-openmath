import AppKit
import SwiftUI

private struct GalleryRecord:Decodable {struct Output:Decodable {let original_latex:String,wolfram:String};let cell:String;let outputs:[Output]}
@main struct MathRenderUIFixture:App {
  private let records:[GalleryRecord]
  @State private var dark=false
  @State private var typography=false
  init() {
    guard let data=Bundle.main.url(forResource:"RecordedLatex",withExtension:"json"),let bytes=try? Data(contentsOf:data),let records=try? JSONDecoder().decode([GalleryRecord].self,from:bytes) else {fatalError("actual recorded LaTeX missing")}
    let names=Set(["original-2","original-4","original-26","science-fraction","science-matrix","science-root","science-precision"])
    self.records=records.filter{names.contains($0.cell)}
  }
  var body:some Scene {
    Window("OpenMath Native Formula Fixture",id:"formula") {
      VStack(alignment:.leading,spacing:12) {
        HStack {Text("原生公式与原始源式").font(.title2);Spacer();Toggle("深色",isOn:$dark).toggleStyle(.switch)}
        Text("以下数据来自已接纳的原53与科研用例。此窗口只排版，不重新执行。").font(.caption).foregroundStyle(.secondary)
        Toggle("检查分式、矩阵、条件、区间与内联基线",isOn:$typography)
        ScrollView {
          VStack(alignment:.leading,spacing:20) {
            if typography {
              Text("以下为明确的 LaTeX 排版样例，字号分别为 12、22、42 pt。").font(.caption)
              NativeInlineMathFixture(dark:dark).id(dark).frame(height:150)
              ForEach(Array([#"\frac{1}{3}"#,#"\sqrt{x^2+1}"#,#"\begin{pmatrix}1&2\\3&4\end{pmatrix}"#,#"\left[-\frac{3}{2},\infty\right)"#,#"\begin{cases}x&x>0\\-x&x\leq0\end{cases}"#].enumerated()),id:\.offset) {index,latex in
                ForEach([CGFloat(12),22,42],id:\.self) {size in
                  NativeMathView(request:.init(identity:"layout-\(index)-\(Int(size))",originalLatex:latex,wolfram:latex,modern:nil,fontSize:size))
                }
              }
            } else {
            ForEach(records,id:\.cell) {record in
              ForEach(Array(record.outputs.enumerated()),id:\.offset) {index,output in
                VStack(alignment:.leading,spacing:8) {
                  Text(record.cell).font(.headline)
                  NativeMathView(request:.init(identity:record.cell+"-\(index)",originalLatex:output.original_latex,wolfram:output.wolfram,modern:nil))
                }
              }
            }
            VStack(alignment:.leading,spacing:8) {
              Text("未知命令保留原式").font(.headline)
              NativeMathView(request:.init(identity:"unknown",originalLatex:#"\unknownOpenMathCommand{\frac{1}{3}}"#,wolfram:"1/3",modern:"1/3"))
            }
            }
          }.padding(12)
        }
      }.padding(18).frame(minWidth:640,minHeight:600).preferredColorScheme(dark ? .dark : .light)
    }.defaultSize(width:1042,height:800)
  }
}

private struct NativeInlineMathFixture:NSViewRepresentable {
  let dark:Bool
  func makeNSView(context:Context)->NativeMathTextView {
    let view=NativeMathTextView(usingTextLayoutManager:true);view.isEditable=false;view.isSelectable=true
    view.drawsBackground=false;view.textContainerInset=NSSize(width:8,height:16)
    view.appearance=NSAppearance(named:dark ? .darkAqua : .aqua)
    let attributes:[NSAttributedString.Key:Any]=[.font:NSFont.systemFont(ofSize:18),.foregroundColor:NSColor.labelColor]
    let text=NSMutableAttributedString(string:"文字 before ",attributes:attributes)
    text.append(NSAttributedString(attachment:NativeMathAttachment(request:.init(identity:"inline-fraction",originalLatex:#"\frac{1}{3}"#,wolfram:"1/3",modern:"1/3",fontSize:18,dark:dark))))
    text.append(NSAttributedString(string:" after 文字；根式 ",attributes:attributes))
    text.append(NSAttributedString(attachment:NativeMathAttachment(request:.init(identity:"inline-root",originalLatex:#"\sqrt{x^2+1}"#,wolfram:"Sqrt[1+x^2]",modern:nil,fontSize:18,dark:dark))))
    text.append(NSAttributedString(string:"。\n未知命令保留原式：",attributes:attributes))
    text.append(NSAttributedString(attachment:NativeMathAttachment(request:.init(identity:"inline-unknown",originalLatex:#"\unknownOpenMathCommand{x}"#,wolfram:"x",modern:nil,fontSize:18,dark:dark))))
    view.textStorage?.setAttributedString(text);return view
  }
  func updateNSView(_ view:NativeMathTextView,context:Context) {view.needsLayout=true}
}
