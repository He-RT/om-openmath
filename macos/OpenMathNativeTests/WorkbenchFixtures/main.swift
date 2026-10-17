import AppKit
import SwiftUI

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
    let ui=actionUI || CommandLine.arguments.contains("--assistant-ui") || Bundle.main.object(forInfoDictionaryKey:"OpenMathAssistantFixture") as? Bool==true
    let app=NSApplication.shared;app.setActivationPolicy(ui ? .regular:.accessory)
    let rootBase=URL(fileURLWithPath:CommandLine.arguments.count>1 ? CommandLine.arguments[1]:Bundle.main.object(forInfoDictionaryKey:"OpenMathFixtureRoot") as! String,isDirectory:true)
    let root=ui ? rootBase.appendingPathComponent("run-"+UUID().uuidString.lowercased()):rootBase
    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    if ui {
      Task {
        do {if actionUI {try await actionLog(root:root,hold:true)} else {try await assistantTranscript(root:root,hold:true)}}
        catch {FileHandle.standardError.write(Data(("Assistant UI fixture failed: \(error)\n").utf8));NSApp.terminate(nil)}
      }
      desktopLoop()
      return
    }
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
