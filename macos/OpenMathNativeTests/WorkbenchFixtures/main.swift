import AppKit
import SwiftUI

@main struct WorkbenchFixtures {
  @MainActor static func require(_ condition:@autoclosure ()->Bool,_ message:String="actual workspace assertion failed",file:StaticString=#fileID,line:UInt=#line) {
    guard condition() else {
      FileHandle.standardError.write(Data(("Native workspace check failed at \(file):\(line): \(message)\n").utf8))
      fatalError(message,file:file,line:line)
    }
  }
  @MainActor static func until(_ check:@escaping @MainActor ()->Bool) async throws {
    let deadline=ContinuousClock.now.advanced(by:.seconds(20))
    while !check() {require(ContinuousClock.now<deadline,"actual workspace update timed out");try await Task.sleep(for:.milliseconds(10))}
  }
  @MainActor static func descendants(_ view:NSView)->[NSView] { [view]+view.subviews.flatMap{descendants($0)} }
  @MainActor static func main() async throws {
    let app=NSApplication.shared;app.setActivationPolicy(.accessory)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    if CommandLine.arguments.contains("--source-feedback") {
      try await sourceFeedback(root:root)
      return
    }
    if CommandLine.arguments.contains("--cell-states") {
      try await cellStates(root:root)
      return
    }
    let file=NativeSourceFile(version:1,title:"原生工作台验收",cells:[
      .init(id:"a",kind:.math,source:"let a=2",dialect:.modern),
      .init(id:"b",kind:.math,source:"a+1",dialect:.modern),
      .init(id:"long",kind:.math,source:"map(fn(k)=>sin(k),range(1,1000000))",dialect:.modern)]+(0..<12).map{NativeSourceCell(id:"text-\($0)",kind:.text,source:"用于滚动验收的原生文字 \($0)",dialect:.auto)})
    let url=root.appendingPathComponent("fixture.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let workspace=NativeFileWorkspace(storage:services)
    guard await workspace.open(url),let session=workspace.session else {fatalError("actual fixture notebook did not open")}
    // The product opens with the notebook dominant; this fixture explicitly exercises all panes.
    session.runtime.navigation.inspectorVisible=true
    if CommandLine.arguments.contains("--navigation-probe") {
      try await probeNavigation(workspace:workspace,session:session,services:services)
      return
    }
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1100,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    window.title="OpenMath Workbench Fixture"
    let host=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));window.contentView=host
    window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    try await until {descendants(host).contains{$0.identifier?.rawValue=="workspace-notebook"}}
    let notebook=descendants(host).first{$0.identifier?.rawValue=="workspace-notebook"}!
    let center=ObjectIdentifier(notebook),editor=try session.editor("a").textView
    try await until {editor.window != nil}
    window.makeFirstResponder(editor)
    editor.selectAll(nil);editor.insertText("let a=5 # 中文🙂",replacementRange:editor.selectedRange())
    editor.setSelectedRange(NSRange(location:4,length:1))
    let source=Data(editor.string.utf8),selection=editor.selectedRange()
    var widths:[Int]=[]
    for width in [1042,861,640,1100] {
      window.setContentSize(NSSize(width:width,height:780));window.contentView?.layoutSubtreeIfNeeded();window.displayIfNeeded()
      try await Task.sleep(for:.milliseconds(100))
      require(descendants(host).contains{ObjectIdentifier($0)==center} && editor.window===window)
      require(Data(editor.string.utf8)==source && editor.selectedRange()==selection)
      let mode=WorkbenchLayout.forWidth(CGFloat(width))
      let actual:[String:Any]=["requested_width":width,"host_width":host.bounds.width,"notebook_width":notebook.frame.width,"layout":session.runtime.navigation.layout.rawValue,"editor_attached":editor.window===window,"selection_location":editor.selectedRange().location,"selection_length":editor.selectedRange().length]
      FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject:actual,options:[.sortedKeys]));FileHandle.standardOutput.write(Data("\n".utf8))
      require(session.runtime.navigation.layout==mode,"native container breakpoint mismatch")
      require(notebook.frame.width>=400,"notebook is narrower than its minimum")
      require(mode == .compact ? notebook.frame.width>CGFloat(width)-50 : notebook.frame.width<CGFloat(width)-300,"native panes did not follow the breakpoint")
      if let bitmap=host.bitmapImageRepForCachingDisplay(in:host.bounds) {
        host.cacheDisplay(in:host.bounds,to:bitmap)
        try bitmap.representation(using:.png,properties:[:])?.write(to:root.appendingPathComponent("workspace-\(width).png"))
      }
      widths.append(width)
    }
    session.runtime.run("a");try await until {session.runtime.operation==nil}
    require(session.runtime.results["a"]?.sources==["5"])
    require(window.firstResponder===editor,"first result update lost the native editor focus")
    session.runtime.run("b");try await until {session.runtime.operation==nil}
    require(session.runtime.results["b"]?.sources==["6"])
    let notebookScroll=descendants(notebook).compactMap{$0 as? NSScrollView}.first {scroll in
      guard let document=scroll.documentView,document !== editor else {return false};return descendants(document).contains{$0===editor}
    }!
    notebookScroll.contentView.scroll(to:NSPoint(x:0,y:100));notebookScroll.reflectScrolledClipView(notebookScroll.contentView)
    let scrollOwner=ObjectIdentifier(notebookScroll),scrollPosition=notebookScroll.contentView.bounds.origin
    require(scrollPosition.y>0,"real notebook did not scroll")
    let navigation=session.runtime.navigation
    let runtimeOwner=ObjectIdentifier(session.runtime)
    for destination in [WorkbenchDestination.assistant,.steps,.variables] {
      navigation.inspector=destination
      navigation.outlineVisible=false;navigation.inspectorVisible=false
      host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(60))
      navigation.outlineVisible=true;navigation.inspectorVisible=true
      host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(60))
      require(navigation.inspector==destination && ObjectIdentifier(session.runtime)==runtimeOwner)
      require(descendants(host).contains{ObjectIdentifier($0)==center} && editor.selectedRange()==selection)
      require(ObjectIdentifier(notebookScroll)==scrollOwner && notebookScroll.contentView.bounds.origin==scrollPosition,"destination switch reset actual notebook scroll")
    }
    require(session.runtime.projection?.definitions_current==true)
    session.runtime.run("long");try await until {session.runtime.phase == .running || session.runtime.operation==nil}
    FileHandle.standardOutput.write(Data(("Long CAS observed: phase=\(String(describing:session.runtime.phase)), message=\(session.runtime.message), active=\(session.runtime.operation != nil)\n").utf8))
    require(session.runtime.operation != nil,"long actual CAS did not remain active")
    window.setContentSize(NSSize(width:640,height:780));host.layoutSubtreeIfNeeded()
    editor.setSelectedRange(NSRange(location:6,length:1));editor.insertText("9",replacementRange:editor.selectedRange())
    require(editor.string=="let a=9 # 中文🙂")
    await session.runtime.stop();try await until {session.runtime.operation==nil}
    // Switching destinations or shrinking the pane never invokes close/cancel. Explicit stop
    // settles the original operation, and the late draft remains pending rather than overwritten.
    require(session.drafts.overlay("a")?.source=="let a=9 # 中文🙂")
    let runtime=ObjectIdentifier(session.runtime)
    window.setContentSize(NSSize(width:1042,height:780));host.layoutSubtreeIfNeeded()
    require(ObjectIdentifier(session.runtime)==runtime)
    let report:[String:Any]=["widths":widths,"destinations":["assistant","steps","variables"],"navigation_document_owned":true,"real_scroll_retained":true,"center_identity_retained":true,"real_native_selection_retained":true,"result_a":"5","result_b":"6","long_cas_edit_stop":true,"pending_late_draft":"let a=9 # 中文🙂"]
    print(String(decoding:try JSONSerialization.data(withJSONObject:report,options:[.sortedKeys]),as:UTF8.self))
    for (name,width,dark) in [("mac-light",1440,false),("mac-dark",1440,true),("mac-1000",1000,false),("mac-800",800,false)] {
      window.appearance=NSAppearance(named:dark ? .darkAqua:.aqua)
      window.setContentSize(NSSize(width:width,height:900));host.layoutSubtreeIfNeeded()
      try await Task.sleep(for:.milliseconds(150));window.displayIfNeeded()
      if let bitmap=host.bitmapImageRepForCachingDisplay(in:host.bounds) {
        host.cacheDisplay(in:host.bounds,to:bitmap)
        try bitmap.representation(using:.png,properties:[:])?.write(to:root.appendingPathComponent(name+".png"))
      }
    }
    window.orderOut(nil)
    try await session.close();try await services.close()
    print("Actual native window 1042/861/640, same notebook view/editor/selection, physical kernel receipts 5/6, live CAS resize/edit/explicit stop and draft retention passed")
  }

  @MainActor static func cellStates(root:URL) async throws {
    let sources:[(String,String)]=[
      ("verified","solve(x^2 - 5x + 6 = 0, x)"),("conditional","solve(x/x = 1, x)"),
      ("exact","2+2"),("numeric","numeric(pi,precision:20)"),("unevaluated","foo(x)"),
      ("error","let f(x) ="),("pending","3+4"),("stale","let a=2"),
      ("running","factor(x^10007 - 1)")]
    let file=NativeSourceFile(version:1,title:"真实左轨状态",cells:sources.map{
      NativeSourceCell(id:$0.0,kind:.math,source:$0.1,dialect:.modern)
    })
    let url=root.appendingPathComponent("states.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let workspace=NativeFileWorkspace(storage:services)
    guard await workspace.open(url),let session=workspace.session else {fatalError("state notebook did not open")}
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1200,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    let host=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));window.contentView=host
    window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    let expected:[String:CellPresentationState.Status]=["verified":.verified,"conditional":.conditional,"exact":.exact,"numeric":.numeric,"unevaluated":.unevaluated,"error":.error]
    for (cell,_) in sources where expected[cell] != nil || cell=="stale" {
      session.runtime.run(cell);try await until {session.runtime.operation==nil}
      if let binding=session.runtime.results[cell]?.bindings.last {await session.runtime.presenter(for:binding).load()}
      guard let actualCell=session.drafts.confirmed.file.cells.first(where:{$0.id==cell}) else {fatalError()}
      let state=session.runtime.presentation(actualCell,session:session,presenter:session.runtime.results[cell]?.bindings.last.map{session.runtime.presenter(for:$0)})
      if let expected=expected[cell] {require(state.status==expected,"\(cell) status was \(state.label)")}
      session.runtime.navigate(to:cell);host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(100))
      save(host,to:root.appendingPathComponent("state-"+cell+".png"))
      print("Real state \(cell): \(state.label)")
    }
    let stale=try session.editor("stale").textView
    stale.selectAll(nil);stale.insertText("let a=5",replacementRange:stale.selectedRange())
    session.runtime.navigate(to:"stale");host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(100))
    let staleCell=session.drafts.confirmed.file.cells.first{$0.id=="stale"}!
    let staleState=session.runtime.presentation(staleCell,session:session,presenter:nil)
    require(staleState.status == .stale("源码已修改"))
    save(host,to:root.appendingPathComponent("state-stale.png"));print("Real state stale: \(staleState.label)")
    session.runtime.run("running");try await until {session.runtime.phase == .running || session.runtime.operation==nil}
    require(session.runtime.operation != nil)
    let runningCell=session.drafts.confirmed.file.cells.first{$0.id=="running"}!
    require(session.runtime.presentation(runningCell,session:session,presenter:nil).status == .running)
    host.layoutSubtreeIfNeeded();window.displayIfNeeded();save(host,to:root.appendingPathComponent("state-running.png"))
    await session.runtime.stop();try await until {session.runtime.operation==nil}
    require(session.runtime.presentation(runningCell,session:session,presenter:nil).status == .stopped)
    host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(100));save(host,to:root.appendingPathComponent("state-stopped.png"))
    let pending=session.drafts.confirmed.file.cells.first{$0.id=="pending"}!
    require(session.runtime.presentation(pending,session:session,presenter:nil).status == .pending)
    print("Real pending/running/stopped states passed; screenshots: \(root.path)")
    window.orderOut(nil);try await session.close();try await services.close()
  }
  @MainActor private static func save(_ host:NSView,to url:URL) {
    guard let bitmap=host.bitmapImageRepForCachingDisplay(in:host.bounds) else {return}
    host.cacheDisplay(in:host.bounds,to:bitmap)
    try? bitmap.representation(using:.png,properties:[:])?.write(to:url)
  }
  @MainActor static func sourceFeedback(root:URL) async throws {
    let file=NativeSourceFile(version:1,title:"过期与诊断",cells:[
      .init(id:"a",kind:.math,source:"let a=2",dialect:.modern),
      .init(id:"b",kind:.math,source:"a+1",dialect:.modern),
      .init(id:"independent",kind:.math,source:"7+1",dialect:.modern)])
    let url=root.appendingPathComponent("feedback.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let workspace=NativeFileWorkspace(storage:services)
    guard await workspace.open(url),let session=workspace.session else {fatalError("feedback source did not open")}
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1200,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    let host=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));window.contentView=host
    window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    for cell in ["a","b","independent"] {session.runtime.run(cell);try await until {session.runtime.operation==nil}}
    let editor=try session.editor("a").textView
    editor.selectAll(nil);editor.insertText("let a=5",replacementRange:editor.selectedRange())
    await session.runtime.analyzeSourceNow(session.drafts)
    require(session.runtime.staleReasons["b"]=="依赖的定义 a 已改变","real host dependency did not invalidate b")
    require(session.runtime.staleReasons["independent"]==nil,"unrelated result became stale")
    save(host,to:root.appendingPathComponent("source-stale.png"))
    editor.selectAll(nil);editor.insertText("let a=2",replacementRange:editor.selectedRange())
    await session.runtime.analyzeSourceNow(session.drafts)
    require(session.runtime.staleReasons.isEmpty,"reverted source stayed stale")
    let broken="# 中文🙂\nlet f(x) ="
    editor.selectAll(nil);editor.insertText(broken,replacementRange:editor.selectedRange())
    await session.runtime.analyzeSourceNow(session.drafts)
    guard let diagnostic=session.runtime.sourceDiagnostics["a"]?.first(where:{$0.severity=="Error"}) else {fatalError("real parser error missing")}
    let map=try SourceIndexMap(source:broken,draftSequence:0)
    let position=try map.position(forUTF8:Int(diagnostic.span.start))
    require(position.line==2,"diagnostic line lost Unicode/newline")
    session.locateDiagnostic("a",source:broken,start:diagnostic.span.start,end:diagnostic.span.end)
    let expectedRange=try map.nativeRange(start:Int(diagnostic.span.start),end:Int(diagnostic.span.end))
    require(editor.selectedRange()==expectedRange)
    require(window.firstResponder===editor)
    session.runtime.run("a");try await until {session.runtime.operation==nil}
    require(session.runtime.results["a"]?.receipt.producer.terminal_status == .error)
    require(session.runtime.history.last(where:{$0.receipt.producer.cell_id.value=="a"})?.receipt.producer.terminal_status == .done)
    host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(100))
    save(host,to:root.appendingPathComponent("source-error.png"))
    print("Real C ABI source feedback: a edit invalidates b only, reverting restores freshness, Unicode diagnostic \(position.line):\(position.column) locates exact native range, prior valid receipt retained")
    window.orderOut(nil);try await session.close();try await services.close()
  }

  /// C1: use the production notebook/editor inside the proposed native container.
  @MainActor static func probeNavigation(workspace:NativeFileWorkspace,session:NativeDocumentSession,services:NativeAppStorage) async throws {
    let state=NavigationProbeState()
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1100,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    let host=NSHostingView(rootView:NavigationProbe(workspace:workspace,session:session,state:state))
    window.contentView=host;window.makeKeyAndOrderFront(nil)
    let editor=try session.editor("a").textView
    try await until {editor.window===window}
    window.makeFirstResponder(editor)
    editor.selectAll(nil);editor.insertText("let a=5 # 中文🙂",replacementRange:editor.selectedRange())
    editor.setSelectedRange(NSRange(location:4,length:1))
    let source=editor.string,selection=editor.selectedRange()
    guard let scroll=descendants(host).compactMap({$0 as? NSScrollView}).first(where:{view in
      guard let content=view.documentView,content !== editor else {return false}
      return descendants(content).contains{$0===editor}
    }) else {fatalError("native navigation notebook scroll missing")}
    scroll.contentView.scroll(to:NSPoint(x:0,y:100));scroll.reflectScrolledClipView(scroll.contentView)
    let origin=scroll.contentView.bounds.origin
    require(origin.y>0)
    var allPassed=true
    for width in [1042,861,640,1100] {
      state.width=CGFloat(width)
      window.setContentSize(NSSize(width:width,height:780));host.layoutSubtreeIfNeeded()
      try await Task.sleep(for:.milliseconds(200))
      window.displayIfNeeded()
      let attached=editor.window===window
      let retainedScroll=descendants(host).contains{$0===scroll} && scroll.contentView.bounds.origin==origin
      let retainedSelection=editor.selectedRange()==selection && editor.string==source
      let actualWidth=host.bounds.width
      allPassed = allPassed && attached && retainedScroll && retainedSelection && abs(actualWidth-CGFloat(width))<1
      let row:[String:Any]=["width":width,"actual_width":actualWidth,"editor_attached":attached,"same_scroll_and_offset":retainedScroll,"same_source_and_selection":retainedSelection,"offset_y":scroll.contentView.bounds.origin.y]
      print(String(decoding:try JSONSerialization.data(withJSONObject:row,options:[.sortedKeys]),as:UTF8.self))
    }
    print("NavigationSplitView + inspector retention: \(allPassed ? "passed":"failed")")
    window.orderOut(nil);try await session.close();try await services.close()
  }
}

@MainActor private final class NavigationProbeState:ObservableObject {
  @Published var width:CGFloat=1100
  @Published var columns:NavigationSplitViewVisibility = .all
  @Published var inspector=true
}
private struct NavigationProbe:View {
  let workspace:NativeFileWorkspace,session:NativeDocumentSession
  @ObservedObject var state:NavigationProbeState
  var body:some View {
    NavigationSplitView(columnVisibility:$state.columns) {
      List {Text("目录")}.navigationSplitViewColumnWidth(min:220,ideal:240,max:260)
    } detail: {
      NativeNotebookFileView(workspace:workspace,session:session)
    }
    .inspector(isPresented:$state.inspector) {Text("检查器").inspectorColumnWidth(min:360,ideal:380,max:420)}
    .onChange(of:state.width) {
      state.columns=state.width>=1042 ? .all:.detailOnly
      state.inspector=state.width>=861
    }
  }
}
