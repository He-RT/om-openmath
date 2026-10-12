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
    let file=NativeSourceFile(version:1,title:"原生工作台验收",cells:[
      .init(id:"a",kind:.math,source:"let a=2",dialect:.modern),
      .init(id:"b",kind:.math,source:"a+1",dialect:.modern),
      .init(id:"long",kind:.math,source:"map(fn(k)=>sin(k),range(1,1000000))",dialect:.modern)]+(0..<12).map{NativeSourceCell(id:"text-\($0)",kind:.text,source:"用于滚动验收的原生文字 \($0)",dialect:.auto)})
    let url=root.appendingPathComponent("fixture.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let workspace=NativeFileWorkspace(storage:services)
    guard await workspace.open(url),let session=workspace.session else {fatalError("actual fixture notebook did not open")}
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1100,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    window.title="OpenMath Workbench Fixture"
    let host=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));window.contentView=host
    window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    try await until {descendants(host).contains{$0 is WorkbenchSplitView}}
    let split=descendants(host).compactMap{$0 as? WorkbenchSplitView}.first!
    let center=ObjectIdentifier(split.notebook),editor=try session.editor("a").textView
    try await until {editor.window != nil}
    window.makeFirstResponder(editor)
    editor.selectAll(nil);editor.insertText("let a=5 # 中文🙂",replacementRange:editor.selectedRange())
    editor.setSelectedRange(NSRange(location:4,length:1))
    let source=Data(editor.string.utf8),selection=editor.selectedRange()
    var widths:[Int]=[]
    for width in [1042,861,640,1100] {
      window.setContentSize(NSSize(width:width,height:780));window.contentView?.layoutSubtreeIfNeeded();window.displayIfNeeded()
      try await Task.sleep(for:.milliseconds(100))
      require(ObjectIdentifier(split.notebook)==center && editor.window===window)
      require(Data(editor.string.utf8)==source && editor.selectedRange()==selection)
      let mode=WorkbenchLayout.forWidth(CGFloat(width))
      let actual:[String:Any]=["requested_width":width,"split_width":split.bounds.width,"notebook_width":split.notebook.frame.width,"outline_hidden":split.outline.isHidden,"inspector_hidden":split.inspector.isHidden,"editor_attached":editor.window===window,"selection_location":editor.selectedRange().location,"selection_length":editor.selectedRange().length]
      FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject:actual,options:[.sortedKeys]));FileHandle.standardOutput.write(Data("\n".utf8))
      require(split.outline.isHidden==(mode != .three) && split.inspector.isHidden==(mode == .compact))
      require(split.notebook.frame.width>=400)
      if let bitmap=host.bitmapImageRepForCachingDisplay(in:host.bounds) {
        host.cacheDisplay(in:host.bounds,to:bitmap)
        try bitmap.representation(using:.png,properties:[:])?.write(to:root.appendingPathComponent("workspace-\(width).png"))
      }
      widths.append(width)
    }
    session.runtime.run("a");try await until {session.runtime.operation==nil}
    require(session.runtime.results["a"]?.sources==["5"])
    session.runtime.run("b");try await until {session.runtime.operation==nil}
    require(session.runtime.results["b"]?.sources==["6"])
    let notebookScroll=descendants(split.notebook).compactMap{$0 as? NSScrollView}.first {scroll in
      guard let document=scroll.documentView,document !== editor else {return false};return descendants(document).contains{$0===editor}
    }!
    notebookScroll.contentView.scroll(to:NSPoint(x:0,y:100));notebookScroll.reflectScrolledClipView(notebookScroll.contentView)
    let scrollOwner=ObjectIdentifier(notebookScroll),scrollPosition=notebookScroll.contentView.bounds.origin
    require(scrollPosition.y>0,"real notebook did not scroll")
    let navigation=session.runtime.navigation
    let runtimeOwner=ObjectIdentifier(session.runtime)
    for destination in WorkbenchDestination.allCases {
      navigation.inspector=destination
      navigation.outlineVisible=false;navigation.inspectorVisible=false
      host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(60))
      navigation.outlineVisible=true;navigation.inspectorVisible=true
      host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(60))
      require(navigation.inspector==destination && ObjectIdentifier(session.runtime)==runtimeOwner)
      require(ObjectIdentifier(split.notebook)==center && editor.selectedRange()==selection)
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
    let report:[String:Any]=["widths":widths,"destinations":WorkbenchDestination.allCases.map(\.rawValue),"navigation_document_owned":true,"real_scroll_retained":true,"center_identity_retained":true,"real_native_selection_retained":true,"result_a":"5","result_b":"6","long_cas_edit_stop":true,"pending_late_draft":"let a=9 # 中文🙂"]
    print(String(decoding:try JSONSerialization.data(withJSONObject:report,options:[.sortedKeys]),as:UTF8.self))
    window.orderOut(nil)
    try await session.close();try await services.close()
    print("Actual native window 1042/861/640, same notebook view/editor/selection, physical kernel receipts 5/6, live CAS resize/edit/explicit stop and draft retention passed")
  }
}
