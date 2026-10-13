import AppKit
import SwiftUI
import SwiftMath
import CryptoKit

private struct CorpusCase:Decodable {let id:String,input:String}
@main struct MathRenderFixtures {
  @MainActor static func check(_ value:@autoclosure ()->Bool,_ message:String="native math assertion",line:UInt=#line) {
    guard value() else {FileHandle.standardError.write(Data(("Native math check failed at line \(line): \(message)\n").utf8));fatalError(message)}
  }
  @MainActor static func main() async throws {
    let app=NSApplication.shared;app.setActivationPolicy(.accessory)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    let fonts=NativeMathFonts.directory();check(fonts != nil && NativeMathFonts.digest(in:fonts) != nil,"installed font bundle not used")
    let original=#"\operatorname{Root}(x)+\operatorname{foo 2}(x)"#
    let changed=MathRenderAdapter.displayTransform(original)
    check(changed.source == #"\mathrm{Root}(x)+\mathrm{foo 2}(x)"# && changed.count==2)
    for unchanged in [#"\\operatorname{foo}"#,#"\operatorname*{foo}"#,#"\operatorname{中文}"#,#"\operatorname{a_{1}}"#] {
      let actual=MathRenderAdapter.displayTransform(unchanged);check(actual.source.utf8.elementsEqual(unchanged.utf8) && actual.count==0,"uncontrolled command adaptation")
    }
    func plan(_ latex:String,inline:Bool=false,size:CGFloat=22)->NativeMathPlan {MathRenderAdapter.prepare(.init(identity:latex,originalLatex:latex,wolfram:latex,modern:nil,inline:inline,fontSize:size))}
    check(plan(String(repeating:"{",count:129)+"x"+String(repeating:"}",count:129)).state == .depthBudget)
    check(plan(String(repeating:#"\sqrt "#,count:129)+"x").state == .depthBudget,"unbraced command recursion not bounded")
    check(plan(String(repeating:#"\mathrm "#,count:129)+"x").state == .depthBudget)
    check(plan(String(repeating:"x",count:20001)).state == .atomBudget)
    check(plan(String(repeating:"x",count:65537)).state == .byteBudget)
    check(plan(#"\unknownOpenMathCommand{x}"#).state == .parseFailure)
    check(plan(#"\frac{1}"#).state == .parseFailure)
    check(MathRenderAdapter.prepare(.init(identity:"missing-font",originalLatex:"x",wolfram:"x",modern:nil),fontDirectory:nil).state == .fontUnavailable)
    let corruptFonts=root.appendingPathComponent("corrupt-fonts",isDirectory:true);try FileManager.default.createDirectory(at:corruptFonts,withIntermediateDirectories:true)
    try Data("invalid font bytes".utf8).write(to:corruptFonts.appendingPathComponent("latinmodern-math.otf"))
    try Data("invalid metrics".utf8).write(to:corruptFonts.appendingPathComponent("latinmodern-math.plist"))
    check(MathRenderAdapter.prepare(.init(identity:"corrupt-font",originalLatex:"x",wolfram:"x",modern:nil),fontDirectory:corruptFonts).state == .fontUnavailable)
    let invalidSize=plan("x",size:.nan)
    check(invalidSize.state == .invalidSize && invalidSize.request==invalidSize.request,"invalid-size fallback cannot retain its request")
    let nfc=plan("é"),nfd=plan("e\u{301}");check(nfc.key.latexHash != nfd.key.latexHash)
    check(nfc.request != nfd.request,"canonical equivalence reused different original bytes")

    let formulas=[#"\frac{1}{3}"#,#"\sqrt{x^2+1}"#,#"\begin{pmatrix}1&2\\3&4\end{pmatrix}"#,#"\left[-\frac{3}{2},\infty\right)"#,#"\begin{cases}x&x>0\\-x&x\leq0\end{cases}"#]
    let gallery=NSStackView();gallery.orientation = .vertical;gallery.alignment = .leading;gallery.spacing=12
    var rendered=0
    for (index,latex) in formulas.enumerated() {
      for size in [CGFloat(12),22,42] {
        let actual=plan(latex,size:size);check(actual.state == .ready,"valid fraction/matrix/conditions/interval not rendered: "+latex)
        let view=NativeMathContainer();view.configure(actual)
        check(view.math != nil && view.metrics.size.width>0 && view.metrics.ascent+view.metrics.descent>0)
        check(abs(view.firstBaselineOffsetFromTop-(view.metrics.size.height-view.metrics.descent))<0.001)
        if size==22 {gallery.addArrangedSubview(view)}
        let data:[String:Any]=["case":index,"size":size,"atoms":actual.atoms,"width":view.metrics.size.width,"ascent":view.metrics.ascent,"descent":view.metrics.descent,"baseline":view.firstBaselineOffsetFromTop]
        FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject:data,options:[.sortedKeys]));FileHandle.standardOutput.write(Data("\n".utf8));rendered+=1
      }
    }
    let longLatex=(0..<300).map {"x_{\($0)}"}.joined(separator:"+")
    let long=NativeMathContainer();long.configure(plan(longLatex))
    check(long.metrics.size.width>640 && long.math?.fontSize==22,"long formula shrank")
    let scroll=NSScrollView(frame:NSRect(x:0,y:0,width:640,height:60));scroll.hasHorizontalScroller=true;scroll.documentView=long;long.frame.size=long.metrics.size
    check(long.frame.width>scroll.contentSize.width && long.math?.fontSize==22)
    gallery.addArrangedSubview(scroll)
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:900,height:780),styleMask:[.titled,.resizable],backing:.buffered,defer:false)
    window.title="Native Math Render Fixture";window.contentView=gallery;window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    check(window.makeFirstResponder(long),"whole display formula cannot receive keyboard selection")
    let copyKey=NSEvent.keyEvent(with:.keyDown,location:.zero,modifierFlags:.command,timestamp:0,windowNumber:window.windowNumber,context:nil,characters:"c",charactersIgnoringModifiers:"c",isARepeat:false,keyCode:8)!
    check(long.performKeyEquivalent(with:copyKey),"whole-formula Command-C not handled")
    check(NSPasteboard.general.string(forType:.string)?.utf8.elementsEqual(longLatex.utf8)==true,"whole formula copy truncated or adapted the original")

    let inline=NativeMathAttachment(request:.init(identity:"inline-fraction",originalLatex:#"\frac{1}{3}"#,wolfram:"1/3",modern:"1/3",fontSize:18))
    check(inline.request.inline && inline.plan.state == .ready && inline.bounds.origin.y == -inline.metrics.descent)
    let text=NativeMathTextView(usingTextLayoutManager:true);text.isEditable=false
    let attributed=NSMutableAttributedString(string:"文字 before ",attributes:[.font:NSFont.systemFont(ofSize:18)])
    attributed.append(NSAttributedString(attachment:inline));attributed.append(NSAttributedString(string:" after 文字"))
    text.textStorage?.setAttributedString(attributed);text.frame=NSRect(x:0,y:0,width:850,height:160)
    let inlineWindow=NSWindow(contentRect:NSRect(x:0,y:0,width:850,height:160),styleMask:[.titled,.resizable],backing:.buffered,defer:false)
    inlineWindow.title="Native TextKit2 Inline Math Fixture";inlineWindow.contentView=text;inlineWindow.makeKeyAndOrderFront(nil)
    text.textLayoutManager?.ensureLayout(for:text.textContentStorage!.documentRange)
    check(text.textLayoutManager != nil && inline.allowsTextAttachmentView)
    func descendants(_ parent:NSView)->[NSView] {[parent]+parent.subviews.flatMap{descendants($0)}}
    let inlineDeadline=ContinuousClock.now.advanced(by:.seconds(5))
    while !descendants(text).contains(where:{$0 is NativeMathContainer}) {
      check(ContinuousClock.now<inlineDeadline,"TextKit2 did not instantiate the real native attachment view")
      inlineWindow.displayIfNeeded();try await Task.sleep(for:.milliseconds(20))
    }
    let mounted=descendants(text).compactMap{$0 as? NativeMathContainer}.first!
    check(mounted.math != nil && mounted.plan?.request.inline==true && mounted.metrics.descent>0)
    check(abs(mounted.frame.width-inline.metrics.size.width)<0.01 && abs(mounted.frame.height-inline.metrics.size.height)<0.01,"TextKit2 attachment frame changed actual mathematical metrics")
    let fallback=NativeMathAttachment(request:.init(identity:"inline-unsupported",originalLatex:#"\unknownOpenMathCommand{x}"#,wolfram:"x",modern:nil))
    check(fallback.plan.state == .parseFailure && fallback.metrics.size.width>10,"inline fallback blank")
    let literal=NativeMathFallbackLabel(request:fallback.request,state:fallback.plan.state);literal.copyOriginal()
    check(NSPasteboard.general.string(forType:.string)?.utf8.elementsEqual(fallback.request.originalLatex.utf8)==true)
    let unsupported=NSMutableAttributedString(string:"fallback ",attributes:[.font:NSFont.systemFont(ofSize:18)])
    unsupported.append(NSAttributedString(attachment:fallback));text.textStorage?.setAttributedString(unsupported)
    text.textLayoutManager?.ensureLayout(for:text.textContentStorage!.documentRange)
    let fallbackDeadline=ContinuousClock.now.advanced(by:.seconds(5))
    while !descendants(text).contains(where:{$0 is NativeMathFallbackLabel}) {
      check(ContinuousClock.now<fallbackDeadline,"TextKit2 original fallback view missing")
      inlineWindow.displayIfNeeded();try await Task.sleep(for:.milliseconds(20))
    }
    check(!descendants(text).contains(where:{$0 === mounted}),"old attachment view survived source replacement")

    // Actual original corpus bytes feed the shared owner/physical receipt/readonly LaTeX path.
    let corpus=try JSONDecoder().decode([CorpusCase].self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[2])))
    check(corpus.count==53)
    let scientific:[NativeSourceCell]=[
      .init(id:"science-fraction",kind:.math,source:"1/3",dialect:.modern),
      .init(id:"science-matrix",kind:.math,source:"[[1,2],[3,4]]",dialect:.modern),
      .init(id:"science-root",kind:.math,source:"solve(x^5-x-1==0,x)",dialect:.modern),
      .init(id:"science-precision",kind:.math,source:"decimal(\"0.1\",precision:80)",dialect:.modern)]
    let cells=corpus.map{NativeSourceCell(id:"original-"+$0.id,kind:.math,source:$0.input,dialect:.wolfram)}+scientific
    let source=NativeSourceFile(version:1,title:"原53/科研公式",cells:cells),file=root.appendingPathComponent("original-corpus.omnb")
    try JSONEncoder().encode(source).write(to:file)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let session=try await NativeDocumentSession.open(storage:services.open(),url:file)
    var records:[[String:Any]]=[]
    for cell in cells {
      session.runtime.run(cell.id)
      let deadline=ContinuousClock.now.advanced(by:.seconds(30))
      while session.runtime.operation != nil {check(ContinuousClock.now<deadline,"actual corpus run timed out: "+cell.id);try await Task.sleep(for:.milliseconds(10))}
      guard let result=session.runtime.results[cell.id] else {fatalError("no real accepted record for "+cell.id+": "+session.runtime.message)}
      check(result.receipt.producer.terminal_status == .done && !result.values.isEmpty,"actual corpus did not complete with readable output: "+cell.id)
      var outputs:[[String:Any]]=[]
      for value in result.values {
        guard let latex=value.latex else {fatalError("actual LaTeX not read: "+cell.id)}
        let request=NativeMathRequest(identity:value.id,originalLatex:latex,wolfram:value.source,modern:value.modern)
        let actual=MathRenderAdapter.prepare(request)
        check(actual.request.originalLatex.utf8.elementsEqual(latex.utf8))
        if actual.state == .ready {let native=NativeMathContainer();native.configure(actual);check(native.metrics.size.width>0)}
        else {let raw=NativeMathFallbackLabel(request:request,state:actual.state);raw.copyOriginal();check(NSPasteboard.general.string(forType:.string)?.utf8.elementsEqual(latex.utf8)==true)}
        outputs.append(["original_latex":latex,"display_latex":actual.displayLatex,"wolfram":value.source,"render_state":actual.state.rawValue,"atoms":actual.atoms,"transforms":actual.transformedOperators])
      }
      records.append(["cell":cell.id,"input":cell.source,"operation":result.receipt.operation_id,"terminal":result.receipt.producer.terminal_status.rawValue,"outputs":outputs])
    }
    try JSONSerialization.data(withJSONObject:records,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("original-and-scientific-latex.json"))
    check(session.drafts.confirmed.revision.value==0,"rendering changed source")
    try await session.close();try await services.close();window.orderOut(nil);inlineWindow.orderOut(nil)
    print("Native SwiftMath \(rendered) real sizes/metrics, actual original53+scientific \(records.count) receipt/LaTeX records, raw fallback/inline baseline, missing-font/unknown/depth/byte/atoms and source-only rendering passed")
    print("CORPUS_EVIDENCE="+root.appendingPathComponent("original-and-scientific-latex.json").path)
  }
}
