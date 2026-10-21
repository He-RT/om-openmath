import AppKit
import SwiftUI
import PDFKit
import ImageIO

private actor ComposerFixtureInputCapture {
  var value:AssistantInputSnapshot?
  func observe(_ value:AssistantInputSnapshot) {self.value=value}
}
@MainActor private final class ComposerFixtureController:ObservableObject {
  weak var editor:AssistantComposerTextView?
  let assistant:NativeAssistant
  @Published var submissions=0
  var changeAppearance:(()->Void)?
  init(assistant:NativeAssistant) {self.assistant=assistant}
  func composeMarked() {
    guard let editor else {return};editor.window?.makeFirstResponder(editor)
    editor.setMarkedText("中文拼音",selectedRange:NSRange(location:0,length:4),replacementRange:NSRange(location:0,length:0))
  }
}
private struct ComposerFixtureControls:View {
  @ObservedObject var controller:ComposerFixtureController
  @ObservedObject var assistant:NativeAssistant
  var body:some View {
    HStack(spacing:12) {
      Text("输入框测试，不连接模型 · Return 接收 \(controller.submissions) 次 · "+(assistant.composerMarked ? "合成中":"非合成"))
      Spacer()
      Button("准备合成文字") {controller.composeMarked()}.accessibilityIdentifier("fixture-compose-marked")
      Button("切换窗口主题") {controller.changeAppearance?()}.accessibilityIdentifier("fixture-composer-appearance")
    }.font(.caption).padding(8)
  }
}
@MainActor private final class AssistantTranscriptFixtureController:ObservableObject {
  let storage:StorageService,assistant:NativeAssistant,sessionID:String
  var count=24
  var changeAppearance:(()->Void)?
  init(storage:StorageService,assistant:NativeAssistant,sessionID:String) {self.storage=storage;self.assistant=assistant;self.sessionID=sessionID}
  func append()async {
    do {
      count+=1
      try await WorkbenchFixtures.recordConversationPair(storage:storage,session:sessionID,index:count,
        question:"新的界面测试问题 \(count)",answer:"第 \(count) 条新增回复。\n\n这是追加到真实会话存储中的测试内容，检查向上阅读时是否保留位置。$x^2 + 1$。")
      await assistant.selectConversation(sessionID)
    } catch {fatalError("fixture append failed: \(error)")}
  }
}
private struct AssistantTranscriptFixtureControls:View {
  @ObservedObject var controller:AssistantTranscriptFixtureController
  var body:some View {
    HStack {
      Text("界面测试数据，不连接模型").foregroundStyle(.secondary)
      Spacer()
      Button("追加测试回复") {Task {await controller.append()}}.accessibilityIdentifier("fixture-append-message")
      Button("切换窗口主题") {controller.changeAppearance?()}.accessibilityIdentifier("fixture-toggle-appearance")
    }.font(.caption).padding(8)
  }
}

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
    let actionUI=CommandLine.arguments.contains("--action-ui") || Bundle.main.object(forInfoDictionaryKey:"OpenMathActionFixture") as? Bool==true
    let outcomeUI=CommandLine.arguments.contains("--outcome-ui") || Bundle.main.object(forInfoDictionaryKey:"OpenMathOutcomeFixture") as? Bool==true
    let composerUI=CommandLine.arguments.contains("--composer-ui") || Bundle.main.object(forInfoDictionaryKey:"OpenMathComposerFixture") as? Bool==true
    let ui=actionUI || outcomeUI || composerUI || CommandLine.arguments.contains("--assistant-ui") || Bundle.main.object(forInfoDictionaryKey:"OpenMathAssistantFixture") as? Bool==true
    let app=NSApplication.shared;app.setActivationPolicy(ui ? .regular:.accessory)
    let rootBase=URL(fileURLWithPath:CommandLine.arguments.count>1 ? CommandLine.arguments[1]:Bundle.main.object(forInfoDictionaryKey:"OpenMathFixtureRoot") as! String,isDirectory:true)
    let root=ui ? rootBase.appendingPathComponent("run-"+UUID().uuidString.lowercased()):rootBase
    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    if ui {
      Task {
        do {if composerUI {try await composer(root:root,hold:true)} else if outcomeUI {try await turnOutcome(root:root,hold:true)} else if actionUI {try await actionLog(root:root,hold:true)} else {try await assistantTranscript(root:root,hold:true)}}
        catch {FileHandle.standardError.write(Data(("Assistant UI fixture failed: \(error)\n").utf8));NSApp.terminate(nil)}
      }
      desktopLoop()
      return
    }
    if CommandLine.arguments.contains("--composer") || CommandLine.arguments.contains("--attachments") {try await composer(root:root,hold:false);return}
    if CommandLine.arguments.contains("--turn-outcome") {try await turnOutcome(root:root,hold:false);return}
    if CommandLine.arguments.contains("--action-log") {
      try await actionLog(root:root,hold:false)
      return
    }
    if CommandLine.arguments.contains("--assistant-transcript") {
      try await assistantTranscript(root:root,hold:false)
      return
    }
    if CommandLine.arguments.contains("--inspector") {
      try await inspector(root:root)
      return
    }
    if CommandLine.arguments.contains("--text-reading") {
      try await textReading(root:root)
      return
    }
    if CommandLine.arguments.contains("--source-feedback") {
      try await sourceFeedback(root:root)
      return
    }
    if CommandLine.arguments.contains("--cell-states") {
      try await cellStates(root:root)
      return
    }
    if CommandLine.arguments.contains("--file-events") {
      try await fileEvents(root:root)
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

  @MainActor static func fileEvents(root:URL) async throws {
    let file=NativeSourceFile(version:1,title:"系统文件事件",cells:[.init(id:"math",kind:.math,source:"2+2",dialect:.modern)])
    let url=root.appendingPathComponent("file-event.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let workspace=NativeFileWorkspace(storage:services),delegate=NativeApplicationDelegate()
    delegate.application(NSApp,open:[URL(string:"https://example.invalid/file.omnb")!])
    delegate.application(NSApp,open:[url,url])
    delegate.attach(workspace)
    require(workspace.session==nil,"non-file or multiple URLs created a document")
    let cold=NativeApplicationDelegate()
    cold.application(NSApp,open:[url])
    cold.attach(workspace)
    try await until {workspace.session != nil}
    require(workspace.session!.drafts.confirmed.file.cells.map(\.source)==["2+2"],"pending file URL did not load actual source")
    let first=workspace.session!
    let closed=await workspace.closeDocument();require(closed,"file event document did not close")
    cold.application(NSApp,open:[url])
    try await until {workspace.session != nil}
    require(workspace.session !== first,"warm file event did not create a fresh source owner")
    require(workspace.session!.drafts.confirmed.file.cells.map(\.source)==["2+2"])
    let stopped=await workspace.shutdown();require(stopped,"file event storage did not close")
    print("Actual early/warm application URL events loaded source through the workspace; non-file/multiple URLs rejected and storage closed")
  }

  static func recordConversationPair(storage:StorageService,session:String,index:Int,question:String,answer:String) async throws {
    let task="fixture-task-\(index)",turn="fixture-turn-\(index)"
    let ledger=SessionLedgerPort(storage:storage,session:session,sourceInstance:"fixture-writer-\(session)-\(index)")
    let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"fixture-user-\(index)",role:.user,agent_turn_id:.init(nil),task_id:.init(task),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:question))])
    _ = try await ledger.record(SessionCanonicalInput(codecVersion:1,delivery:"user_input",message:input,presentation:.init(model:index%2==0 ? "gpt-6.1-sol":"gpt-6-luna",mode:index%2==0 ? "execute":"discuss")),id:"fixture-input-\(index)",kind:.user_message,task:task)
    let message=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"fixture-answer-\(index)",role:.assistant,agent_turn_id:.init(turn),task_id:.init(task),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:answer))])
    _ = try await ledger.record(SessionCanonicalAssistant(codecVersion:1,messageID:message.message_id,textUTF8:Data(answer.utf8),calls:[],canonicalJSON:String(decoding:try JSONEncoder().encode(message),as:UTF8.self),peerMessageID:"fixture-peer-\(index)",connectionID:"fixture-connection",sourceSequence:UInt64(index+1)),id:"fixture-answer-event-\(index)",kind:.assistant_end,turn:turn,task:task)
  }
  @MainActor static func assistantTranscript(root:URL,hold:Bool)async throws {
    let file=NativeSourceFile(version:1,title:"助手对话",cells:[.init(id:"math",kind:.math,source:"solve(x^2 - 5x + 6 = 0, x)",dialect:.modern)])
    let url=root.appendingPathComponent("assistant.omnb");try JSONEncoder().encode(file).write(to:url)
    let paths=NativeStoragePaths(root:root.appendingPathComponent("storage"),channel:.preview)
    let services=NativeAppStorage(paths:paths),storage=try await services.open()
    let workspace=NativeFileWorkspace(storage:services)
    guard await workspace.open(url),let session=workspace.session else {fatalError("assistant document missing")}
    session.assistant.prepare={throw AgentSessionError.inactive} // UI fixtures never contact a model.
    let document=session.drafts.confirmed.document_id
    let title="先求解二次方程，再说明每一步为什么成立，保留精确结果。"
    _ = try await storage.createAgentSession("fixture-main",documentID:document,title:title)
    _ = try await storage.createAgentSession("fixture-other",documentID:"another-document",title:"其他文稿")
    _ = try await storage.createAgentSession("fixture-legacy")
    _ = try await storage.createAgentSession("fixture-history",documentID:document,title:"另一段对话")
    try await recordConversationPair(storage:storage,session:"fixture-history",index:100,question:"另一段对话",answer:"历史回复。")
    for i in 0..<24 {
      let answer=i==23 ? "两个根为 $x=2$、$x=3$。\n\n```openmath\nsolve(x^2 - 5x + 6 = 0, x)\n```\n\n代码只有主动插入后才会成为单元格。" : "第 \(i+1) 条回复：先整理原方程，再检查因子与候选解。\n\n将 $x=2$ 代回原式，得到 $2^2-5\\times2+6=0$。这段说明用于检查原生 Markdown 和阅读位置。"
      try await recordConversationPair(storage:storage,session:"fixture-main",index:i,question:i==0 ? title:"第 \(i+1) 个问题：为什么需要代回原方程验证？",answer:answer)
    }
    await session.assistant.refreshHistory()
    require(Set(session.assistant.conversations.map(\.id))==["fixture-main","fixture-history"],"conversation history crossed document scope")
    require(session.assistant.conversations.first{$0.id=="fixture-main"}?.title==String(title.prefix(20)))
    let countBefore=try await storage.agentSessions(limit:32).first{$0.id=="fixture-main"}!.eventCount
    await session.assistant.selectConversation("fixture-main")
    require(session.assistant.messages.count==48 && session.assistant.conversationTitle==String(title.prefix(20)))
    require(session.assistant.messages.first(where:{$0.role=="assistant"})?.context?.mode=="execute")
    let countAfter=try await storage.agentSessions(limit:32).first{$0.id=="fixture-main"}!.eventCount
    require(countAfter==countBefore,"opening history wrote session events")
    await session.assistant.selectConversation("fixture-other")
    require(session.assistant.messages.count==48,"foreign conversation was opened")
    session.runtime.navigation.inspectorVisible=true
    session.runtime.navigation.inspector = .assistant
    let controller=AssistantTranscriptFixtureController(storage:storage,assistant:session.assistant,sessionID:"fixture-main")
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1400,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    window.title="助手对话 · 界面测试数据";window.appearance=NSAppearance(named:.aqua)
    controller.changeAppearance={ [weak window] in guard let window else {return};window.appearance=NSAppearance(named:window.effectiveAppearance.bestMatch(from:[.aqua,.darkAqua]) == .darkAqua ? .aqua:.darkAqua)}
    let host=NSHostingView(rootView:VStack(spacing:0) {WorkspaceView(workspace:workspace,session:session);AssistantTranscriptFixtureControls(controller:controller)})
    window.contentView=host;workspace.attachWindow(window);window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    try await until {descendants(host).compactMap{$0 as? MarkdownTextView}.count>0}
    print("Document-scoped conversation history: 48 canonical messages, original model/mode, 20-character title, foreign/legacy exclusion and readonly reopening passed; no model calls")
    if hold {while !Task.isCancelled {try await Task.sleep(for:.seconds(30))}}
    window.orderOut(nil);try await session.close();try await services.close()
    let (reopened,_)=try await StorageService.open(paths:paths)
    let recovered=try await reopened.agentConversations(document:document)
    require(Set(recovered.map(\.id))==["fixture-main","fixture-history"])
    let reopenedSession=try await NativeDocumentSession.open(storage:reopened,url:url)
    require(reopenedSession.drafts.confirmed.document_id != document,"ordinary file open reused a source authority")
    await reopenedSession.assistant.refreshHistory()
    require(Set(reopenedSession.assistant.conversations.map(\.id))==["fixture-main","fixture-history"],"reopening the actual file lost its bound history")
    await reopenedSession.assistant.selectConversation("fixture-main")
    require(reopenedSession.assistant.messages.count==48)
    let fresh=try await reopenedSession.agent.activate(session:"fixture-main",grants:[.read_notebook],readonly:true,coreVersion:"fixture",adapterVersion:"fixture")
    require(fresh.document_id==reopenedSession.drafts.confirmed.document_id,"history restored the old document grant")
    await reopenedSession.agent.resetTask()
    _ = try await reopenedSession.files.save(operation:"fixture-history-resave")
    await reopenedSession.assistant.refreshHistory()
    require(Set(reopenedSession.assistant.conversations.map(\.id))==["fixture-main","fixture-history"],"saving the reopened file lost older conversations")
    try await reopenedSession.close()
    let againSession=try await NativeDocumentSession.open(storage:reopened,url:url)
    await againSession.assistant.refreshHistory()
    require(Set(againSession.assistant.conversations.map(\.id))==["fixture-main","fixture-history"],"history was lost after resumed save and another open")
    try await againSession.close()
    let copy=root.appendingPathComponent("same-content-copy.omnb");try FileManager.default.copyItem(at:url,to:copy)
    let copiedSession=try await NativeDocumentSession.open(storage:reopened,url:copy)
    await copiedSession.assistant.refreshHistory()
    require(copiedSession.assistant.conversations.isEmpty,"a same-content copy inherited another file's conversations")
    try await copiedSession.close()
    try await reopened.close()
    print("Physical file reopen retained history through verified saved bindings, fresh activation used the new document owner, identical-content copy kept separate history")
  }
  @MainActor static func desktopLoop() {
    let menu=NSMenu(),item=NSMenuItem(),application=NSMenu()
    let quit=NSMenuItem(title:"退出界面测试",action:#selector(NSApplication.terminate(_:)),keyEquivalent:"q");quit.target=NSApp
    application.addItem(quit);item.submenu=application;menu.addItem(item);NSApp.mainMenu=menu
    let editItem=NSMenuItem(),edit=NSMenu(title:"编辑");editItem.submenu=edit;menu.addItem(editItem)
    for (title,action,key) in [("撤销","undo:","z"),("剪切","cut:","x"),("复制","copy:","c"),("粘贴","paste:","v"),("全选","selectAll:","a")] {
      edit.addItem(NSMenuItem(title:title,action:NSSelectorFromString(action),keyEquivalent:key))
    }
    NSApp.run()
  }
  @MainActor static func actionLog(root:URL,hold:Bool)async throws {
    let casePath=ProcessInfo.processInfo.environment["OPENMATH_ACTION_FIXTURE_CASE"] ?? Bundle.main.object(forInfoDictionaryKey:"OpenMathActionCase") as? String
    guard let casePath else {throw StorageError.unsafePath}
    let caseRoot=URL(fileURLWithPath:casePath,isDirectory:true)
    let services=NativeAppStorage(paths:.init(root:caseRoot.appendingPathComponent("storage"),channel:.preview))
    let storage=try await services.open(),workspace=NativeFileWorkspace(storage:services)
    let documents=try await storage.startupDocuments(limit:32)
    guard let document=documents.records.first?.id else {throw StorageError.corruptIdentity}
    await workspace.recover(document)
    guard let session=workspace.session else {throw StorageError.recoveryRequired}
    session.assistant.prepare={throw AgentSessionError.inactive}
    guard let conversation=try await storage.agentSessions(limit:32).first?.id else {throw StorageError.corruptIdentity}
    let originalMessages=try await session.agent.recordedTranscript(session:conversation)
    guard var unsubmitted=originalMessages.flatMap(\.actions).first(where:{$0.tool=="preview_source" && $0.status == .ok}) else {throw StorageError.corruptIdentity}
    require(unsubmitted.preview?.cells.contains{$0.after?.contains("scene")==true}==true,"accepted S2 preview lost its original proposal")
    unsubmitted.transactionRef=nil
    await session.assistant.loadActionDiff(unsubmitted)
    require(session.assistant.actionDiffs[unsubmitted.id]?.notes.contains("尚未提交的预览")==true,"unsubmitted preview depended on a later transaction")
    if let submitted=originalMessages.flatMap(\.actions).first(where:{$0.id==unsubmitted.id}),submitted.transactionRef != nil {
      require(session.assistant.actionDiff(submitted)==nil,"an unsubmitted preview cache was relabeled as a committed diff")
      await session.assistant.loadActionDiff(submitted)
      require(session.assistant.actionDiff(submitted)?.transaction==submitted.transactionRef)
    }
    let baseline="let a=2",baselineHash=KernelHashes.data(Data(baseline.utf8))
    let proposal:[String:Any]=["input":["kind":"patch","operations":[["type":"update_cell","target":["cell_id":"a"],"expected_source_hash":baselineHash,"source":"let a=5"]]]]
    let verified=AgentPreviewFormatter.make(arguments:proposal,result:["valid":true],snapshot:["cells":[["cell_id":"a","source_complete":true,"source":baseline,"source_hash":baselineHash]]])!
    require(verified.cells[0].before==baseline && verified.cells[0].after=="let a=5")
    let tampered=AgentPreviewFormatter.make(arguments:proposal,result:["valid":true],snapshot:["cells":[["cell_id":"a","source_complete":true,"source":"not the baseline","source_hash":baselineHash]]])!
    require(tampered.cells.allSatisfy{$0.before==nil && $0.notes.contains(where:{$0.contains("完整基线")})},"preview rendered an unverified read as its baseline")
    guard let action=originalMessages.flatMap(\.actions).first(where:{$0.tool=="apply_notebook_patch" && $0.status == .ok}),let transaction=action.transactionRef else {throw StorageError.corruptIdentity}
    var facts=try await storage.sourceUndoFacts(document:document,ids:[transaction])
    if facts.undone.contains(transaction) {
      let history=try await storage.sourceHistory(document:document)
      var restored=false
      for entry in history where entry.retained {
        let record=try await storage.sourceTransactions(document:document,ids:[entry.transaction])[0]
        if record.plan.undo_group?.transaction_ids.contains(transaction)==true {
          let status=try await storage.sourceUndoFacts(document:document,ids:[entry.transaction])
          if !status.undone.contains(entry.transaction) {
            _ = try await session.source.undo(transactions:[entry.transaction],group:"d2-restore",operation:"d2-restore-"+UUID().uuidString.lowercased())
            restored=true;break
          }
        }
      }
      require(restored,"the actual S2 inverse was not available to restore")
    }
    try await session.assistant.restoreRecordedConversation(conversation)
    facts=try await storage.sourceUndoFacts(document:document,ids:[transaction])
    require(!facts.undone.contains(transaction))
    let before=session.drafts.confirmed
    await session.assistant.loadActionDiff(action)
    guard let diff=session.assistant.actionDiffs[action.id] else {fatalError("actual S2 diff unavailable")}
    require(diff.cells.contains{$0.lines.contains{$0.kind == .added && $0.text.contains("scene")}},"the S2 action did not read the actual geometry source diff")
    let delta=SourceTransactionDiff.lines(before:"中文\na\nb\n",after:"中文\na\nc\n")
    require(delta.filter{$0.kind == .removed}.map(\.text)==["b"] && delta.filter{$0.kind == .added}.map(\.text)==["c"])
    let longDelta=SourceTransactionDiff.lines(before:String(repeating:"a\n",count:6000),after:String(repeating:"b\n",count:6000))
    require(longDelta.count<15_000 && longDelta.contains{$0.kind == .omitted && $0.text.contains("复制完整源码")},"large source diff was not bounded")
    if let cell=action.cells.first {
      let sequence=session.runtime.navigationSequence;session.assistant.locate(cell,reducedMotion:true)
      require(session.runtime.selectedCell==cell && session.runtime.navigationSequence>sequence && session.runtime.highlightedCell==nil)
      session.assistant.locate(cell,reducedMotion:false)
      require(session.runtime.highlightedCell==cell)
    }
    session.runtime.navigation.inspectorVisible=true;session.runtime.navigation.inspector = .assistant
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1400,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    window.title="A5 S2 操作记录 · 不连接模型";window.appearance=NSAppearance(named:.aqua)
    let host=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));window.contentView=host;workspace.attachWindow(window)
    window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    if hold {while !Task.isCancelled {try await Task.sleep(for:.seconds(30))}}
    await session.assistant.undoAction(action)
    require(session.assistant.undoneActionTransactions.contains(transaction),"row undo was not confirmed by source history")
    let after=session.drafts.confirmed
    require(after.revision.value==before.revision.value+1)
    await session.assistant.undoAction(action)
    require(session.drafts.confirmed.revision.value==after.revision.value,"repeated row undo wrote another transaction")
    session.undo.manager.undo();try await until {session.undo.phase != .pending}
    await session.assistant.refreshActionFacts()
    try await until {!session.assistant.undoneActionTransactions.contains(transaction)}
    require(!session.assistant.undoneActionTransactions.contains(transaction),"Cmd-Z did not redo the explicit row undo")
    require(session.drafts.confirmed.file.cells.map(\.source)==before.file.cells.map(\.source))
    let actual=session.assistant.undoActionTransaction!
    var lose=true
    session.assistant.undoActionTransaction={action,operation in
      try await actual(action,operation)
      if lose {lose=false;throw CommitPortError.unknownOutcome}
    }
    await session.assistant.undoAction(action)
    require(session.assistant.unknownActionUndo==action.id)
    let uncertain=session.drafts.confirmed.revision.value
    await session.assistant.undoAction(action)
    require(session.assistant.unknownActionUndo==nil && session.drafts.confirmed.revision.value==uncertain,"unknown row undo did not reconcile the original ID")
    print("Actual A5 S2 source action: verified original diff, reduced-motion locate, single transaction undo, repeat safety, native Cmd-Z redo, durable inverse graph and original-ID uncertain readback passed; no model calls")
    window.orderOut(nil);try await session.close()
    let archived=try await storage.sourceTransactionForDisplay(document:document,id:transaction)
    require(archived.transaction_id==transaction && archived.plan.after.file.cells.map(\.source)==before.file.cells.map(\.source),"archived diff no longer matched the restored S2 source")
    do {_ = try await storage.documentIdentity(document);fatalError("reading an archived diff opened a source writer")} catch StorageError.closing { }
    try await services.close()
  }
  @MainActor static func turnOutcome(root:URL,hold:Bool)async throws {
    struct NativeSummary:Decodable {let outcomes:[TurnOutcome]}
    let casePath=ProcessInfo.processInfo.environment["OPENMATH_OUTCOME_FIXTURE_CASE"] ?? Bundle.main.object(forInfoDictionaryKey:"OpenMathOutcomeCase") as? String
    guard let casePath else {throw StorageError.unsafePath}
    let caseRoot=URL(fileURLWithPath:casePath,isDirectory:true)
    let captured=try JSONDecoder().decode(NativeSummary.self,from:Data(contentsOf:caseRoot.appendingPathComponent("summary.json"))).outcomes[0]
    let services=NativeAppStorage(paths:.init(root:caseRoot.appendingPathComponent("storage"),channel:.preview))
    let storage=try await services.open(),workspace=NativeFileWorkspace(storage:services)
    let documents=try await storage.startupDocuments(limit:32)
    guard let document=documents.records.first?.id else {throw StorageError.corruptIdentity}
    await workspace.recover(document)
    guard let session=workspace.session,let conversation=try await storage.agentSessions(limit:32).first?.id else {throw StorageError.recoveryRequired}
    session.assistant.prepare={throw AgentSessionError.inactive}
    let facts=try await session.agent.outcomeFacts(session:conversation,task:captured.id)
    let verified=TurnOutcome.assemble(task:captured.id,reason:captured.endReason,facts:facts,saved:captured.saved,durationMs:captured.durationMs)
    require(verified.changedCells==captured.changedCells && verified.ranCells==captured.ranCells && verified.products==captured.products && verified.staleAfterEdit==captured.staleAfterEdit && verified.modelRequests==captured.modelRequests,"native A5 summary did not match independently reread host facts")
    let refs=Set(verified.transactions)
    if !refs.isEmpty {
      let state=try await storage.sourceUndoFacts(document:document,ids:refs)
      if state.undone==refs {
        let inverse=try await storage.sourceRedoTransactions(document:document,ids:refs)
        _ = try await session.source.undo(transactions:inverse,group:"d3-restore",operation:"d3-restore-"+UUID().uuidString.lowercased())
      }
    }
    try await session.assistant.restoreRecordedConversation(conversation)
    await session.assistant.acceptOutcome(verified)
    try await session.assistant.recordConclusion(verified)
    try await session.assistant.restoreRecordedConversation(conversation)
    require(session.assistant.outcomes.first{$0.id==captured.id}?.products==captured.products,"stored native conclusion did not rebuild host products")
    // A restored conclusion is read-only; explicitly registering this test round uses the same
    // verified transaction group as the live completion path.
    await session.assistant.acceptOutcome(verified)
    let before=session.drafts.confirmed
    session.runtime.navigation.inspectorVisible=true;session.runtime.navigation.inspector = .assistant
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1400,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    window.title="A5 本轮结果 · 不连接模型";window.appearance=NSAppearance(named:.aqua)
    let host=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));window.contentView=host;workspace.attachWindow(window);window.makeKeyAndOrderFront(nil)
    if hold {while !Task.isCancelled {try await Task.sleep(for:.seconds(30))}}
    if !refs.isEmpty {
      await session.assistant.undoTurn(verified.id)
      require(session.assistant.roundErrors[verified.id]==nil,"round undo failed: \(session.assistant.roundErrors[verified.id] ?? "")")
      try await until {session.assistant.undoneTurns.contains(verified.id)}
      let undoneRevision=session.drafts.confirmed.revision.value
      require(undoneRevision==before.revision.value+1,"round undo did not use one source commit")
      await session.assistant.undoTurn(verified.id)
      require(session.drafts.confirmed.revision.value==undoneRevision)
      await session.assistant.redoTurn(verified.id)
      require(session.assistant.roundErrors[verified.id]==nil,"round redo failed: \(session.assistant.roundErrors[verified.id] ?? "")")
      try await until {!session.assistant.undoneTurns.contains(verified.id)}
      require(session.drafts.confirmed.file.cells.map(\.source)==before.file.cells.map(\.source),"round redo changed unrelated source")
      session.undo.manager.undo();try await until {session.undo.phase != .pending}
      await session.assistant.refreshActionFacts();try await until {session.assistant.undoneTurns.contains(verified.id)}
      session.undo.manager.undo();try await until {session.undo.phase != .pending}
      await session.assistant.refreshActionFacts();try await until {!session.assistant.undoneTurns.contains(verified.id)}
      let changing=session.assistant.changeOutcome!
      var lose=true
      session.assistant.changeOutcome={outcome,redo,operation in
        try await changing(outcome,redo,operation)
        if lose {lose=false;throw CommitPortError.unknownOutcome}
      }
      await session.assistant.undoTurn(verified.id)
      require(session.assistant.unknownRoundChange==verified.id)
      let unknownRevision=session.drafts.confirmed.revision.value
      let readFacts=session.assistant.actionUndoFacts
      session.assistant.actionUndoFacts={_ in throw StorageError.inaccessible}
      await session.assistant.refreshActionFacts()
      require(session.assistant.canChangeTurn(verified,redo:false),"unavailable cached facts disabled original-ID reconciliation")
      session.assistant.changeOutcome={_,_,_ in throw StorageError.inaccessible}
      await session.assistant.undoTurn(verified.id)
      require(session.assistant.unknownRoundChange==verified.id)
      session.assistant.changeOutcome=changing;session.assistant.actionUndoFacts=readFacts
      await session.assistant.undoTurn(verified.id)
      require(session.assistant.unknownRoundChange==nil && session.drafts.confirmed.revision.value==unknownRevision,"uncertain round result was not reconciled by the original ID")
      await session.assistant.redoTurn(verified.id)
      try await until {!session.assistant.undoneTurns.contains(verified.id)}
    }
    print("Actual native round: independently verified A5 facts, durable end metadata, grouped single-commit undo, repeat safety, redo and keyboard state tracking passed; model requests unchanged: \(verified.modelRequests)")
    window.orderOut(nil);try await session.close();try await services.close()
  }
  @MainActor static func composer(root:URL,hold:Bool)async throws {
    let file=NativeSourceFile(version:1,title:"输入框验收",cells:[.init(id:"math",kind:.math,source:"solve(x^2 - 5x + 6 = 0, x)",dialect:.modern)])
    let url=root.appendingPathComponent("composer.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let storage=try await services.open()
    let workspace=NativeFileWorkspace(storage:services)
    guard await workspace.open(url),let session=workspace.session else {throw StorageError.recoveryRequired}
    session.assistant.draftEdited("尚未配置时保留草稿")
    require(!session.assistant.canSend,"an unconfigured composer must not accept a message")
    let configuration=AssistantConfiguration(baseURL:"https://example.invalid/v1",model:"fixture-tool-model",credentialReference:"fixture-never-used")
    try session.assistant.configure(configuration)
    require(session.assistant.canSend,"a configured text model accepts the retained draft")
    session.assistant.draftEdited("")
    var registry=ModelRegistryDocument.current(configuration)
    registry.providers[0].models.append(.init(id:"fixture-text",apiName:"fixture-text",displayName:"仅文字模型",tools:false,images:false,contextLimit:16000))
    registry.providers[0].models.append(.init(id:"fixture-image",apiName:"fixture-image",displayName:"图片与工具模型",tools:true,images:true,contextLimit:32000))
    let catalog=try ModelRegistry(file:root.appendingPathComponent("models.json"))
    try await catalog.save(registry)
    let reloaded=try await catalog.load()
    require(reloaded==registry)
    session.assistant.registry=registry
    session.assistant.prepare={throw AgentSessionError.inactive}
    let schemaRoot=Bundle.main.object(forInfoDictionaryKey:"OpenMathRepository") as? String ?? FileManager.default.currentDirectoryPath
    let schema=URL(fileURLWithPath:schemaRoot).appendingPathComponent("docs/design/agent-tools.schema.json")
    let previewSession="composer-preview-"+UUID().uuidString.lowercased(),previewTask="composer-preview-task"
    _ = try await storage.createAgentSession(previewSession,documentID:session.drafts.confirmed.document_id,title:"只读输入检查")
    let reference=session.assistant.referenceCandidates!()[0]
    let message=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"composer-preview-user",role:.user,agent_turn_id:.init(nil),task_id:.init(previewTask),parts:[
      .agent_text_part(.init(kind:.text,part_id:"notebook-references",text:AssistantComposerRules.referencePart([reference]))),
      .agent_text_part(.init(kind:.text,part_id:"question",text:"只读检查：中文与公式 x^2"))])
    let capture=ComposerFixtureInputCapture()
    let context=AssistantContext(host:session.host,storage:storage,session:previewSession,task:previewTask,configuration:configuration,mode:.discuss,tools:AssistantMode.discuss.tools,schema:schema,observed:{await capture.observe($0)})
    let planned=try await context.preview(message)
    require(planned.fullInput.contains("notebook-references") && planned.fullInput.contains(reference.sourceHash) && planned.estimatedTokens>0)
    require(!planned.fullInput.contains("fixture-never-used") && !planned.fullInput.contains("\"name\":\"apply_notebook_patch\""))
    let ledger=SessionLedgerPort(storage:storage,session:previewSession)
    _ = try await ledger.record(SessionCanonicalInput(codecVersion:1,delivery:"user_input",message:message,presentation:.init(model:configuration.model,mode:"discuss",references:[reference])),id:"composer-preview-input",kind:.user_message,task:previewTask)
    let frozen=try await context.prepare(.init(operation_id:"composer-freeze",agent_turn_id:"composer-freeze-turn",request_id:"composer-freeze-request"))
    let observed=await capture.value
    require(observed?.actual==true && observed?.fullInput==String(decoding:frozen.model.body,as:UTF8.self),"full-input observer did not receive the actual Rust wire body")
    let wire=try JSONSerialization.jsonObject(with:frozen.model.body) as! [String:Any]
    require(wire["model"] as? String==configuration.model && !observed!.fullInput.contains(configuration.credentialReference))
    await frozen.model.cancel()
    let previewEvents=try await storage.sessionEvents(previewSession,limit:32)
    require(previewEvents.count==1,"reading or freezing input dispatched a model request")
    session.runtime.navigation.inspectorVisible=true;session.runtime.navigation.inspector = .assistant
    let controller=ComposerFixtureController(assistant:session.assistant)
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1400,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    window.title="输入框 · 测试配置，不连接模型";window.appearance=NSAppearance(named:.aqua)
    let host=NSHostingView(rootView:VStack(spacing:0) {WorkspaceView(workspace:workspace,session:session);if hold {ComposerFixtureControls(controller:controller,assistant:session.assistant)}});window.contentView=host;workspace.attachWindow(window);window.makeKeyAndOrderFront(nil)
    try await until {descendants(host).contains{$0 is AssistantComposerTextView}}
    let editor=descendants(host).compactMap{$0 as? AssistantComposerTextView}.first!
    controller.editor=editor
    controller.changeAppearance={ [weak window] in guard let window else {return};window.appearance=NSAppearance(named:window.effectiveAppearance.bestMatch(from:[.aqua,.darkAqua]) == .darkAqua ? .aqua:.darkAqua)}
    let scroll=editor.enclosingScrollView as! AssistantComposerScrollView
    window.makeFirstResponder(editor)
    var sends=0;let submitted=editor.submit;editor.submit={sends+=1}
    func key(_ modifiers:NSEvent.ModifierFlags=[])->NSEvent {NSEvent.keyEvent(with:.keyDown,location:.zero,modifierFlags:modifiers,timestamp:0,windowNumber:window.windowNumber,context:nil,characters:"\r",charactersIgnoringModifiers:"\r",isARepeat:false,keyCode:36)!}
    editor.setMarkedText("中文拼音",selectedRange:NSRange(location:0,length:4),replacementRange:NSRange(location:0,length:0))
    require(editor.hasMarkedText())
    editor.keyDown(with:key());require(sends==0,"marked native Return submitted the message")
    editor.unmarkText();editor.keyDown(with:key(.shift));require(sends==0 && editor.string.contains("\n"))
    editor.shortcut = .enter;editor.keyDown(with:key());require(sends==1)
    editor.shortcut = .commandReturn;editor.keyDown(with:key());require(sends==1)
    _=editor.performKeyEquivalent(with:key(.command));require(sends==2)
    editor.submit=submitted
    editor.string=String(repeating:"中文与公式 x^2 + 1\n",count:80)
    let measured=scroll.measure(width:340)
    require(measured<=scroll.maximumHeight && scroll.hasVerticalScroller && editor.frame.height>measured)
    session.assistant.composerMarked=false;session.assistant.draftEdited("保留失败时的输入")
    session.assistant.addReference("math")
    session.assistant.setMode(execute:true);session.assistant.selectModel("fixture-text")
    require(session.assistant.selectedModelID != "fixture-text","execute mode selected a model without tools")
    session.assistant.setMode(execute:false);session.assistant.selectModel("fixture-text")
    require(session.assistant.selectedModelID=="fixture-text")
    session.assistant.setMode(execute:true);require(!session.assistant.canSend)
    session.assistant.selectModel("fixture-image");require(session.assistant.canSend)
    session.assistant.send();require(session.assistant.draft=="保留失败时的输入","draft was cleared before native acceptance")
    session.assistant.draftEdited("等待期间继续输入的新草稿")
    try await until {!session.assistant.busy}
    require(session.assistant.draft=="等待期间继续输入的新草稿" && session.assistant.references.count==1)
    require(session.drafts.confirmed.file.cells[0].source==file.cells[0].source)
    require(!session.document.isDocumentEdited,"a failed assistant preparation marked unchanged notebook source as edited")
    require(session.assistant.messages.isEmpty,"preparation failure left an unsent message in the transcript")
    session.assistant.prepare={
      while !session.assistant.stopping {try await Task.sleep(for:.milliseconds(20))}
      throw CancellationError()
    }
    session.assistant.send();try await until {session.assistant.busy}
    require(editor.isEditable,"running assistant disabled the next draft")
    session.assistant.selectModel("current-model")
    require(session.assistant.modelChangeNote?.contains("从下一轮开始使用")==true)
    session.assistant.draftEdited("运行期间保留的新草稿")
    await session.assistant.stop();try await until {!session.assistant.busy}
    require(session.assistant.status==UIStrings.end(.stopped) && session.assistant.draft=="运行期间保留的新草稿")
    require(session.assistant.messages.isEmpty,"stopping before acceptance left an unsent message in the transcript")
    session.assistant.prepare={throw AgentSessionError.inactive}
    print("Native composer: marked Return blocked, Shift/newline and Cmd-Return verified, height capped with real scrolling, configured capability selection, reference chips and failed-send/new-draft retention passed; no model calls")
    var attachmentConversation:String?
    if CommandLine.arguments.contains("--attachments") || Bundle.main.object(forInfoDictionaryKey:"OpenMathAttachmentFixture") as? Bool==true {attachmentConversation=try await attachments(root:root,session:session,storage:storage)}
    if hold {
      session.assistant.newConversation()
      if let attachmentConversation {await session.assistant.refreshHistory();await session.assistant.selectConversation(attachmentConversation)}
      editor.submit={controller.submissions+=1}
      while !Task.isCancelled {try await Task.sleep(for:.seconds(30))}
    }
    window.orderOut(nil);try await session.close();try await services.close()
  }
  @MainActor static func attachments(root:URL,session:NativeDocumentSession,storage:StorageService)async throws->String {
    let sourceURL=root.appendingPathComponent("composer.omnb"),originalSource=try Data(contentsOf:sourceURL)
    let repository=Bundle.main.object(forInfoDictionaryKey:"OpenMathRepository") as? String ?? FileManager.default.currentDirectoryPath
    let fixtures=URL(fileURLWithPath:repository).appendingPathComponent("macos/OpenMathNativeTests/Fixtures/Media")
    let imageData=try Data(contentsOf:fixtures.appendingPathComponent("rgb-pattern.png"))
    let board=NSPasteboard.withUniqueName();defer {board.releaseGlobally()}
    board.setData(imageData,forType:.png)
    require(AssistantAttachmentInput.read(board,assistant:session.assistant));try await until {session.assistant.attachments.count==1 && session.assistant.importingAttachments==0}
    let unknown=root.appendingPathComponent("保留文件.xyz");try Data("unsupported bytes retained".utf8).write(to:unknown)
    let scan=root.appendingPathComponent("扫描页.pdf")
    var box=CGRect(x:0,y:0,width:240,height:320)
    let pdf=CGContext(scan as CFURL,mediaBox:&box,nil)!
    pdf.beginPDFPage(nil);pdf.setFillColor(CGColor(gray:0.6,alpha:1));pdf.fill(CGRect(x:20,y:20,width:50,height:30));pdf.endPDFPage();pdf.closePDF()
    let finder=NSPasteboard.withUniqueName();defer {finder.releaseGlobally()}
    finder.writeObjects([fixtures.appendingPathComponent("math-text.pdf") as NSURL,unknown as NSURL,scan as NSURL])
    require(AssistantAttachmentInput.read(finder,assistant:session.assistant));try await until {session.assistant.attachments.count==4 && session.assistant.importingAttachments==0}
    let items=session.assistant.attachments,image=items.first(where:{$0.kind == .image})!,text=items.first(where:{$0.text != nil})!
    require(image.original.hash==KernelHashes.data(imageData) && image.thumbnail != nil && image.image != nil)
    require(items.first(where:{$0.name==unknown.lastPathComponent})?.note=="暂不能发送给模型")
    require(items.first(where:{$0.name==scan.lastPathComponent})?.note=="没有可提取的文字")
    session.assistant.selectModel("current-model");require(!session.assistant.canSend && image.explanation(images:false)=="当前模型不能读取图片")
    session.assistant.selectModel("fixture-image");require(session.assistant.canSend)
    let textBytes=try await AssistantAttachmentStore.bytes(storage:storage,owner:text.owner,descriptor:text.text!,maximum:128*1024)
    let extracted=String(decoding:textBytes,as:UTF8.self);require(extracted.contains("〔第 1 页〕") && extracted.contains("x^2+y^2=1"))
    let doc=session.drafts.confirmed.document_id,scope=session.assistant.conversationID
    let identity=try await storage.createAgentSession(scope,documentID:doc,title:"检查附件颜色和公式")
    require(identity.title=="检查附件颜色和公式","attachment-only session lost its first question title")
    for item in items {
      let retained=try await AssistantAttachmentStore.bytes(storage:storage,owner:item.owner,descriptor:item.original,maximum:32*1024*1024)
      require(KernelHashes.data(retained)==item.original.hash)
    }
    let contextSession="attachment-input-"+UUID().uuidString.lowercased(),task="attachment-freeze-task"
    _ = try await storage.createAgentSession(contextSession,documentID:doc)
    let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"attachment-question",role:.user,agent_turn_id:.init(nil),task_id:.init(task),parts:[
      .agent_media_part(.init(kind:.media,part_id:image.id,media_ref:image.id,submission_kind:.image)),
      .agent_text_part(.init(kind:.text,part_id:text.id,text:extracted)),.agent_text_part(.init(kind:.text,part_id:"question",text:"检查图片和 PDF"))])
    let configured=AssistantConfiguration(baseURL:"https://example.invalid/v1",model:"fixture-image",credentialReference:"never-used")
    let context=AssistantContext(host:session.host,storage:storage,session:contextSession,task:task,configuration:configured,mode:.discuss,tools:AssistantMode.discuss.tools,schema:URL(fileURLWithPath:repository).appendingPathComponent("docs/design/agent-tools.schema.json"))
    let preview=try await context.preview(input,attachments:[image,text]);require(preview.attachmentCharacters>0)
    _ = try await SessionLedgerPort(storage:storage,session:contextSession).record(SessionCanonicalInput(codecVersion:1,delivery:"user_input",message:input,presentation:.init(model:configured.model,mode:"discuss",attachments:[image,text])),id:"attachment-input",kind:.user_message,task:task)
    let prepared=try await context.prepare(.init(operation_id:"attachment-freeze",agent_turn_id:"attachment-turn",request_id:"attachment-request"))
    let wire=try JSONSerialization.jsonObject(with:prepared.model.body) as! [String:Any]
    let messages=wire["messages"] as! [[String:Any]],content=messages.last!["content"] as! [[String:Any]]
    require((content[0]["image_url"] as? [String:Any])?["url"] as? String != nil)
    let uri=(content[0]["image_url"] as! [String:Any])["url"] as! String
    require(uri.hasPrefix("data:image/jpeg;base64,") && !uri.contains(fixtures.path))
    let projected=Data(base64Encoded:String(uri.dropFirst("data:image/jpeg;base64,".count)))!
    require(KernelHashes.data(projected)==image.image!.hash && CGImageSourceCreateWithData(projected as CFData,nil) != nil)
    require((content[1]["text"] as? String)?.contains("x^2+y^2=1")==true)
    await prepared.model.cancel()
    let transcript=try await AssistantTranscriptReader(storage:storage,document:doc).read(session:contextSession)
    require(transcript.last?.context?.attachments?.count==2 && transcript.last?.text=="检查图片和 PDF")
    let before=session.assistant.attachments;session.assistant.send();try await until {!session.assistant.busy}
    require(session.assistant.attachments==before,"failed preparation lost attachments")
    session.assistant.send();session.assistant.removeAttachment(image.id);try await until {!session.assistant.busy}
    require(!session.assistant.attachments.contains(where:{$0.id==image.id}),"failure resurrected an attachment explicitly removed from the next draft")
    await session.assistant.addAttachmentData(imageData,name:"粘贴图片.png")
    require(session.drafts.confirmed.file.cells.count==1 && !session.document.isDocumentEdited)
    let unchanged=try Data(contentsOf:sourceURL);require(unchanged==originalSource,"attachments modified .omnb source bytes")
    print("Native attachments: clipboard PNG and Finder URLs stored in BlobStore, JPEG/thumbnail decoded, PDF page text, scan/unknown retained, capability block, first-question title, canonical image/PDF -> actual Rust body, transcript cards and failure retention passed; no network")
    return contextSession
  }
  @MainActor static func inspector(root:URL) async throws {
    let definitions=(0..<40).map{"let v\($0)=\($0);"}.joined(separator:"\n")
    let file=NativeSourceFile(version:1,title:"检查器验收",cells:[
      .init(id:"definitions",kind:.math,source:definitions,dialect:.modern),
      .init(id:"dependent",kind:.math,source:"let b=v0+1",dialect:.modern),
      .init(id:"solve",kind:.math,source:"solve(x^4 - 5*x^2 + 4 = 0, x)",dialect:.modern),
      .init(id:"independent",kind:.math,source:"let independent=7",dialect:.modern),
      .init(id:"broken",kind:.math,source:"let broken=2; solve(",dialect:.modern)])
    let url=root.appendingPathComponent("inspector.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let workspace=NativeFileWorkspace(storage:services)
    guard await workspace.open(url),let session=workspace.session else {fatalError("inspector notebook missing")}
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1400,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    let host=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));window.contentView=host
    session.runtime.navigation.inspectorVisible=true
    window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    for cell in file.cells {session.runtime.run(cell.id);try await until {session.runtime.operation==nil}}
    let before=try await session.runtime.kernel.state(),sourceBefore=session.drafts.confirmed.snapshot_hash
    session.runtime.navigation.inspector = .variables
    await session.runtime.refreshDefinitions(session.drafts)
    try await until {session.runtime.definitions.count==43}
    require(session.runtime.definitions.first{$0.name=="b"}?.value=="1")
    require(session.runtime.definitions.first{$0.name=="broken"}?.error==true)
    host.layoutSubtreeIfNeeded()
    guard let split=descendants(host).compactMap({$0 as? WorkbenchSplitView}).first else {fatalError("split missing")}
    try await until {descendants(split.inspector).compactMap{$0 as? NSTableView}.contains{$0.numberOfRows==43}}
    let table=descendants(split.inspector).compactMap{$0 as? NSTableView}.first{$0.numberOfRows==43}!
    table.scrollRowToVisible(30)
    let tableScroll=table.enclosingScrollView!,tablePosition=table.enclosingScrollView!.contentView.bounds.origin
    require(tablePosition.y>0,"variables table did not scroll")
    session.runtime.navigate(to:"solve");session.runtime.navigation.inspector = .steps
    let binding=session.runtime.results["solve"]!.bindings.last!,presenter=session.runtime.presenter(for:binding),model=presenter.stepsModel()
    await model.load();await presenter.readOriginalInput(cellSource:true)
    try await until {presenter.originalCellSource != nil}
    require(presenter.originalCellSource==file.cells[2].source,"inspector header does not contain the original cell source")
    if let branch=model.page?.entries.first(where:{$0.child_count>0}) {
      model.expandedPaths.insert(branch.path);let child=model.childModel(for:branch);await child.load()
      require(child.error==nil && child.page?.parent_path==branch.path)
    }
    try await until {descendants(split.inspector).compactMap{$0 as? NSScrollView}.contains {scroll in
      scroll !== tableScroll && scroll.bounds.height>400 && (scroll.documentView?.bounds.height ?? 0)>scroll.contentSize.height+50
    }}
    let stepsScroll=descendants(split.inspector).compactMap{$0 as? NSScrollView}.first {scroll in
      scroll !== tableScroll && scroll.bounds.height>400 && (scroll.documentView?.bounds.height ?? 0)>scroll.contentSize.height+50
    }!
    stepsScroll.contentView.scroll(to:NSPoint(x:0,y:90));stepsScroll.reflectScrolledClipView(stepsScroll.contentView)
    let stepPosition=stepsScroll.contentView.bounds.origin
    session.assistant.draft="切换分段后保留的草稿"
    for destination in [WorkbenchDestination.assistant,.variables,.steps,.variables] {
      session.runtime.navigation.inspector=destination;host.layoutSubtreeIfNeeded();try await Task.sleep(for:.milliseconds(80))
      require(descendants(split.inspector).contains{$0===table} && tableScroll.contentView.bounds.origin==tablePosition,"variables scroll reset on tab change")
      require(descendants(split.inspector).contains{$0===stepsScroll} && stepsScroll.contentView.bounds.origin==stepPosition,"steps scroll reset on tab change")
      require(session.assistant.draft=="切换分段后保留的草稿")
    }
    require(session.drafts.confirmed.snapshot_hash==sourceBefore,"inspector reads committed editor source")
    let editor=try session.editor("definitions").textView
    let edited=definitions.replacingOccurrences(of:"let v0=0;",with:"let v0=9;")
    editor.selectAll(nil);editor.insertText(edited,replacementRange:editor.selectedRange())
    require(session.drafts.overlay("definitions")?.source==edited,"native edit did not reach the draft owner")
    await session.runtime.analyzeSourceNow(session.drafts);await session.runtime.refreshDefinitions(session.drafts)
    try await until {session.runtime.definitions.first{$0.name=="b"}?.stale==true || session.runtime.definitionsError != nil}
    require(session.runtime.definitions.first{$0.name=="b"}?.stale==true,"dependent definition did not become stale: \(session.runtime.definitionsError ?? "no query error")")
    require(session.runtime.definitions.first{$0.name=="b"}?.value=="1")
    require(session.runtime.definitions.first{$0.name=="independent"}?.stale==false)
    let after=try await session.runtime.kernel.state()
    require(before.active_checkpoint_id.value==after.active_checkpoint_id.value && before.kernel_state_revision.value==after.kernel_state_revision.value,"inspector reads changed the accepted kernel")
    print("Native inspector: accepted 43 definitions, failed declaration, dependent stale values, independent freshness, original step source, inline children and distinct table/steps scroll owners retained; no accepted checkpoint/source changed")
    window.orderOut(nil);try await session.close();try await services.close()
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
    guard let diagnostic=session.runtime.sourceDiagnostics[Data("a".utf8)]?.first(where:{$0.severity=="Error"}) else {fatalError("real parser error missing")}
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
  @MainActor static func textReading(root:URL) async throws {
    let file=NativeSourceFile(version:1,title:"文字阅读",cells:[.init(id:"text",kind:.text,source:"# 中文标题\n\n正文 $x$",dialect:.modern)])
    let url=root.appendingPathComponent("text.omnb");try JSONEncoder().encode(file).write(to:url)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let workspace=NativeFileWorkspace(storage:services)
    guard await workspace.open(url),let session=workspace.session else {fatalError("text notebook missing")}
    require(session.editingTextCells.isEmpty,"existing prose did not default to reading")
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:1100,height:780),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    let host=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));window.contentView=host
    window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    session.beginTextEditing("text")
    let editor=try session.editor("text").textView
    try await until {editor.window===window}
    window.makeFirstResponder(editor)
    editor.setMarkedText("拼音",selectedRange:NSRange(location:0,length:2),replacementRange:NSRange(location:0,length:0))
    require(editor.hasMarkedText() && !session.endTextEditing("text"),"reading switch discarded marked text")
    editor.unmarkText()
    let original=editor.string
    guard let escape=NSEvent.keyEvent(with:.keyDown,location:.zero,modifierFlags:[],timestamp:0,windowNumber:window.windowNumber,context:nil,characters:"\u{1b}",charactersIgnoringModifiers:"\u{1b}",isARepeat:false,keyCode:53) else {fatalError()}
    editor.keyDown(with:escape)
    try await until {!session.editingTextCells.contains("text")}
    require(editor.string==original,"reading switch rewrote source")
    try await session.insertCell(.text,after:"text")
    let new=session.drafts.confirmed.file.cells.last!.id
    require(session.editingTextCells.contains(new),"new prose did not enter editing")
    window.orderOut(nil);try await session.close();try await services.close()
    print("Text reading: existing prose reads, real marked text blocks exit, Esc retains source, new prose edits immediately passed")
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
