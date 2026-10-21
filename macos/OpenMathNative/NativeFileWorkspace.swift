import AppKit
import SwiftUI

/// Single current source document. All menus, panel opens and OS file opens use this owner;
/// an old session is never replaced before its explicit close/save policy succeeds.
@MainActor final class NativeFileWorkspace:NSObject,ObservableObject,NSWindowDelegate {
  @Published private(set) var session:NativeDocumentSession?
  @Published private(set) var busy=false
  @Published private(set) var errorText:String?
  @Published private(set) var libraryWarning:String?
  @Published private(set) var recoverableDocuments:[StartupDocumentRecord]=[]
  private var recoveryCursor:String?
  @Published private(set) var recoverableSessions:[SessionListRecord]=[]
  @Published private(set) var selectedSessionRecovery:SessionRecoveryPage?
  @Published private(set) var selectedSessionID:String?
  private var sessionListCursor:String?
  private var recoveryLedger:SessionLedgerPort?
  private let services:NativeAppStorage
  init(storage:NativeAppStorage = NativeAppStorage()) {services=storage;super.init()}
  private weak var window:NSWindow?
  private var controller:NSWindowController?
  private var allowingClose=false
  func attachWindow(_ window:NSWindow) {
    guard self.window !== window else {return}
    self.window=window;window.delegate=self
    // Full AppHost UI fixtures use a private bundle copy; shipping bundles omit these keys.
    if Bundle.main.object(forInfoDictionaryKey:"OpenMathFixtureRoot") is String,
       let fixture=Bundle.main.object(forInfoDictionaryKey:"OpenMathFixtureWindow") as? [String:Any] {
      if let width=fixture["width"] as? Double,let height=fixture["height"] as? Double,
         (640...2000).contains(width),(560...1600).contains(height) {
        DispatchQueue.main.async {
          let top=window.screen?.visibleFrame.maxY ?? window.frame.maxY
          window.setFrame(NSRect(x:0,y:top-height,width:width,height:height),display:true)
        }
      }
    }
    attachDocumentWindow()
  }
  private func attachDocumentWindow() {
    if let controller {session?.document.removeWindowController(controller)}
    controller=nil
    guard let session,let window else {return}
    let current=NSWindowController(window:window);session.document.addWindowController(current);controller=current
    window.delegate=self
  }
  func openPanel() async {
    guard !busy else {return};busy=true;defer {busy=false}
    guard let url=await NativeDocument.openWithPanel(window:window) else {return}
    _ = await replaceReserved(url)
  }
  func newDocument() async {_ = await replace(nil)}
  /// Settings share this installation lease even when no notebook has been opened yet.
  func prepareStorage() async throws {_ = try await services.open()}
  func refreshRecoverableDocuments(more:Bool=false) async {
    guard !busy else {return};busy=true;defer {busy=false}
    do {
      let storage=try await services.open();libraryWarning=await storage.libraryAvailability().warning
      let page=try await storage.startupDocuments(after:more ? recoveryCursor : nil)
      recoverableDocuments = more ? recoverableDocuments+page.records : page.records;recoveryCursor=page.next
    } catch {errorText="启动恢复检查失败："+error.localizedDescription}
  }
  var hasMoreRecoveryDocuments:Bool {recoveryCursor != nil}
  func refreshRecoverableSessions(more:Bool=false) async {
    guard !busy else {return};busy=true;defer {busy=false}
    do {
      let storage=try await services.open();libraryWarning=await storage.libraryAvailability().warning
      guard case .ready=await storage.libraryAvailability() else {recoverableSessions=[];return}
      let page=try await storage.agentSessions(after:more ? sessionListCursor : nil,limit:16)
      recoverableSessions=more ? recoverableSessions+page : page;sessionListCursor=page.count==16 ? page.last?.id : nil;errorText=nil
    } catch {errorText="会话恢复列表无法读取，原记录已保留："+error.localizedDescription}
  }
  var hasMoreRecoverySessions:Bool {sessionListCursor != nil}
  func inspectSessionRecovery(_ id:String,more:Bool=false) async {
    guard !busy else {return};busy=true;defer {busy=false}
    do {
      let storage=try await services.open()
      if selectedSessionID != id || !more {selectedSessionID=id;selectedSessionRecovery=nil;recoveryLedger=SessionLedgerPort(storage:storage,session:id)}
      guard let recoveryLedger else {throw StorageError.closing}
      let page=try await RecoveryService(storage:storage).recoverSession(id,ledger:recoveryLedger,cursor:more ? selectedSessionRecovery?.nextCursor : nil,limit:16)
      if more,let previous=selectedSessionRecovery {selectedSessionRecovery = .init(tools:previous.tools+page.tools,requests:previous.requests+page.requests,lastEventSequence:page.lastEventSequence,complete:page.complete,nextCursor:page.nextCursor)}
      else {selectedSessionRecovery=page}
      errorText=nil
    } catch {errorText="会话核对未完成，操作与请求未重发："+error.localizedDescription}
  }
  func recover(_ document:String) async {
    guard !busy,session?.drafts.confirmed.document_id != document else {return};busy=true;defer {busy=false}
    do {
      let storage=try await services.open()
      let candidate=try await NativeDocumentSession.recover(storage:storage,documentID:document)
      if let old=session,!(await old.document.prepareToClose()) {try await candidate.close();return}
      do {try await removeCurrent()} catch {try? await candidate.close();throw error}
      session=candidate;candidate.document.changed={ [weak self] in self?.objectWillChange.send() };attachDocumentWindow();errorText=nil
    } catch {errorText="无法恢复本地笔记本："+error.localizedDescription}
  }
  func reviewDraft(_ cell:String,keep:Bool) {
    guard let session else {return}
    do {try session.reviewDraft(cell,keep:keep);errorText=nil} catch {errorText="草稿确认失败，内容已保留："+error.localizedDescription}
  }
  func insertCell(_ kind:NativeCellKind,after:String?) async {
    guard !busy,let session,session.runtime.operation==nil,!session.assistant.busy else {return}
    busy=true;defer {busy=false;objectWillChange.send()}
    do {try await session.insertCell(kind,after:after);errorText=nil}
    catch {errorText="未能添加单元格："+error.localizedDescription}
  }
  func deleteCell(_ id:String) async {
    guard !busy,let session,session.runtime.operation==nil,!session.assistant.busy else {return}
    busy=true;defer {busy=false;objectWillChange.send()}
    do {try await session.deleteCell(id);errorText=nil}
    catch {errorText="未能删除单元格："+error.localizedDescription}
  }
  func open(_ url:URL) async->Bool {await replace(url)}
  private func replace(_ url:URL?) async->Bool {
    guard !busy else {return false};busy=true;defer {busy=false}
    return await replaceReserved(url)
  }
  /// Both OS opens and the interactive panel reserve the single-document owner for their
  /// whole lifetime, so a second menu/window action cannot create an overlapping panel.
  private func replaceReserved(_ url:URL?) async->Bool {
    do {
      // Prepare the new source without changing the old window. A bad/offline file leaves the
      // current source, marked input, scroll and file binding available.
      let storage=try await services.open()
      libraryWarning=await storage.libraryAvailability().warning
      let candidate=try await NativeDocumentSession.open(storage:storage,url:url)
      if let old=session,!((await old.document.prepareToClose())) {try await candidate.close();return false}
      do {try await removeCurrent()} catch {try? await candidate.close();throw error}
      // The single-document owner handles file opens and termination. Registering this façade
      // with AppKit's default controller also installs its independent draft-save/close flow,
      // which cannot preserve our source receipts or marked overlays.
      session=candidate
      candidate.document.changed={ [weak self] in self?.objectWillChange.send() }
      attachDocumentWindow();errorText=nil;return true
    } catch {errorText=error.localizedDescription;return false}
  }
  private func removeCurrent() async throws {
    guard let old=session else {return}
    if let controller {old.document.removeWindowController(controller)};controller=nil
    do {try await old.close(approvalRequired:true)} catch {attachDocumentWindow();throw error}
    session=nil;window?.title="OpenMath Preview";window?.representedURL=nil;window?.isDocumentEdited=false
  }
  func save(asNew:Bool=false) async {
    guard !busy,let session else {return};busy=true;defer {busy=false;objectWillChange.send()}
    do {
      if asNew || session.document.fileURL==nil {_ = try await session.document.saveAsWithPanel()}
      else {_ = try await session.document.saveNative()}
      errorText=nil
    } catch {errorText=error.localizedDescription}
  }
  func reconcile() async {
    guard !busy,let session else {return};busy=true;defer {busy=false;objectWillChange.send()}
    do {try await session.document.reconcileSave();errorText=nil} catch {errorText=error.localizedDescription}
  }
  func closeDocument() async->Bool {
    guard !busy else {return false};busy=true;defer {busy=false}
    guard let session else {return true}
    guard await session.document.prepareToClose() else {return false}
    do {try await removeCurrent();return true} catch {errorText=error.localizedDescription;return false}
  }
  func shutdown() async->Bool {
    guard await closeDocument() else {return false}
    do {try await services.close();return true} catch {errorText=error.localizedDescription;return false}
  }
  func windowShouldClose(_ sender:NSWindow)->Bool {
    if allowingClose || session==nil {return true}
    guard !busy else {return false}
    Task {if await closeDocument() {allowingClose=true;sender.performClose(nil)}}
    return false
  }
  func windowWillReturnUndoManager(_ window:NSWindow)->UndoManager? {session?.undo.manager}
  // SwiftUI's app-level default undo commands do not discover the embedded TextKit
  // editor's routed manager. Resolve the actual responder before falling back to the document.
  private var commandUndoManager:UndoManager? {
    let key=NSApp.keyWindow ?? window
    if key !== window {return (key?.firstResponder as? NSTextView)?.undoManager}
    guard !busy,window?.attachedSheet==nil else {return nil}
    if let composer=window?.firstResponder as? AssistantComposerTextView {return composer.undoManager}
    guard session?.assistant.busy != true else {return nil}
    return (window?.firstResponder as? NSTextView)?.undoManager ?? session?.undo.manager
  }
  var canUndo:Bool {commandUndoManager?.canUndo==true}
  var canRedo:Bool {commandUndoManager?.canRedo==true}
  func undo() {if let manager=commandUndoManager,manager.canUndo {manager.undo();objectWillChange.send()}}
  func redo() {if let manager=commandUndoManager,manager.canRedo {manager.redo();objectWillChange.send()}}
}

@MainActor final class NativeApplicationDelegate:NSObject,NSApplicationDelegate {
  weak var workspace:NativeFileWorkspace?
  private weak var scratch:NotebookViewModel?
  private var pending:(url:URL,reply:Bool)?
  private var terminating=false
  func attach(_ workspace:NativeFileWorkspace,scratch:NotebookViewModel?=nil) {
    self.workspace=workspace;self.scratch=scratch
    if let request=pending {pending=nil;Task {let opened=await workspace.open(request.url);if request.reply {NSApplication.shared.reply(toOpenOrPrint:opened ? .success : .failure)}}}
  }
  // SwiftUI forwards file URLs through this modern callback, bypassing openFiles.
  func application(_ sender:NSApplication,open urls:[URL]) {
    guard urls.count==1,let url=urls.first,url.isFileURL else {return}
    receive(url,sender:sender,reply:false)
  }
  func application(_ sender:NSApplication,openFiles filenames:[String]) {
    guard filenames.count==1,let filename=filenames.first else {sender.reply(toOpenOrPrint:.failure);return}
    receive(URL(fileURLWithPath:filename),sender:sender,reply:true)
  }
  private func receive(_ url:URL,sender:NSApplication,reply:Bool) {
    if let workspace {Task {let opened=await workspace.open(url);if reply {sender.reply(toOpenOrPrint:opened ? .success : .failure)}}}
    else if pending==nil {pending=(url,reply)} else if reply {sender.reply(toOpenOrPrint:.failure)}
  }
  func applicationShouldTerminate(_ sender:NSApplication)->NSApplication.TerminateReply {
    guard let workspace else {return .terminateNow}
    guard !terminating else {return .terminateLater};terminating=true
    Task {let allowed=await workspace.shutdown();if allowed {await scratch?.close()};terminating=false;sender.reply(toApplicationShouldTerminate:allowed)}
    return .terminateLater
  }
  func applicationShouldTerminateAfterLastWindowClosed(_ sender:NSApplication)->Bool {true}
  func applicationShouldOpenUntitledFile(_ sender:NSApplication)->Bool {false}
}

private final class WorkspaceWindowView:NSView {
  var attached:((NSWindow)->Void)?
  override func viewDidMoveToWindow() {super.viewDidMoveToWindow();if let window {attached?(window)}}
}
struct NativeWindowAttachment:NSViewRepresentable {
  let workspace:NativeFileWorkspace
  func makeNSView(context:Context)->NSView {let view=WorkspaceWindowView();view.attached={ [weak workspace] in workspace?.attachWindow($0) };return view}
  func updateNSView(_ nsView:NSView,context:Context) {if let window=nsView.window {workspace.attachWindow(window)}}
}

/// The notebook is the primary surface. Document-owned editors retain native input and undo.
struct NativeNotebookFileView:View {
  @ObservedObject var workspace:NativeFileWorkspace
  let session:NativeDocumentSession
  @ObservedObject private var runtime:NotebookRuntime
  @ObservedObject private var assistant:NativeAssistant
  init(workspace:NativeFileWorkspace,session:NativeDocumentSession) {self.workspace=workspace;self.session=session;_runtime=ObservedObject(wrappedValue:session.runtime);_assistant=ObservedObject(wrappedValue:session.assistant)}
  var body:some View {
    ScrollViewReader {proxy in
      ScrollView {
        VStack(alignment:.leading,spacing:28) {
          header
          LazyVStack(alignment:.leading,spacing:26) {
            ForEach(Array(session.drafts.confirmed.file.cells.enumerated()),id:\.element.id) {index,cell in
              cellView(cell,index:index).id(cell.id)
                .overlay(RoundedRectangle(cornerRadius:8).stroke(Theme.Color.agent,lineWidth:4)
                  .opacity(runtime.highlightedCell==cell.id ? runtime.highlightOpacity:0).allowsHitTesting(false))
            }
          }
          HStack(spacing:16) {
            Button("数学",systemImage:"plus") {Task {await workspace.insertCell(.math,after:session.drafts.confirmed.file.cells.last?.id)}}.accessibilityIdentifier("notebook-add-math")
            Button("文字",systemImage:"text.alignleft") {Task {await workspace.insertCell(.text,after:session.drafts.confirmed.file.cells.last?.id)}}.accessibilityIdentifier("notebook-add-text")
            Spacer()
            Text("⌘↩ 运行").font(.caption).foregroundStyle(.tertiary)
          }.buttonStyle(.borderless).disabled(!canChangeStructure).padding(.top,4)
          recoveryDetails
        }.frame(maxWidth:800,alignment:.leading).padding(.horizontal,48).padding(.vertical,30).frame(maxWidth:.infinity,alignment:.top)
      }.background(Color(nsColor:.textBackgroundColor))
        .onChange(of:runtime.navigationSequence) {if let cell=runtime.selectedCell {proxy.scrollTo(cell,anchor:.top)}}
    }
  }
  private var header:some View {
    VStack(alignment:.leading,spacing:8) {
      Text(session.document.fileURL?.deletingPathExtension().lastPathComponent ?? session.drafts.confirmed.file.title).font(Theme.Font.paperTitle).textSelection(.enabled)
      HStack(spacing:12) {
        Text("\(session.drafts.confirmed.file.cells.count) 个单元格").font(.callout).foregroundStyle(.secondary)
        NativeDraftBackupStatus(backup:session.backup)
      }
      if let error=workspace.errorText {Label(error,systemImage:"exclamationmark.circle").font(.callout).foregroundStyle(.red).textSelection(.enabled)}
      if let warning=workspace.libraryWarning {Text(warning).font(.callout).foregroundStyle(.secondary).textSelection(.enabled)}
      ForEach(Array(session.recoveryMessages.enumerated()),id:\.offset) {_,message in Text(message).font(.callout).foregroundStyle(.secondary).textSelection(.enabled)}
      if session.recoveryHistoryCursor != nil {Button("继续核对历史") {Task {await session.reconcileRecoveryHistory()}}}
    }
  }
  private var canChangeStructure:Bool {!workspace.busy && runtime.operation==nil && !session.assistant.busy}
  @ViewBuilder private func cellView(_ cell:NativeSourceCell,index:Int)->some View {
    VStack(alignment:.leading,spacing:12) {
      if cell.kind == .math {
        NotebookMathCell(session:session,cell:cell,index:index,workspace:workspace,result:cellResults(cell),tools:cellTools(cell))
      } else {
        HStack {Spacer();cellTools(cell)}
        NativeNotebookTextCell(workspace:workspace,session:session,cell:cell)
      }
      if session.drafts.overlay(cell.id)?.requiresReview==true {
        VStack(alignment:.leading,spacing:8) {
          Text("恢复的草稿尚未确认").font(.callout)
          HStack {
            Button("使用恢复草稿") {workspace.reviewDraft(cell.id,keep:true)}
            Button("使用已确认源码") {workspace.reviewDraft(cell.id,keep:false)}
          }
        }
      }
    }
  }
  private func cellTools(_ cell:NativeSourceCell)->some View {
    HStack(spacing:8) {
      if cell.kind == .math {
        Button("运行此单元格",systemImage:Theme.Symbol.run) {runtime.run(cell.id)}
          .labelStyle(.iconOnly).help("运行此单元格（⌘↩）")
          .disabled(!runtime.canRun || workspace.busy || session.assistant.busy)
          .accessibilityIdentifier("notebook-run-"+cell.id)
      }
      Menu("单元格操作",systemImage:Theme.Symbol.more) {
        Button("在下方添加数学") {Task {await workspace.insertCell(.math,after:cell.id)}}
        Button("在下方添加文字") {Task {await workspace.insertCell(.text,after:cell.id)}}
        Button("在助手中引用") {session.assistant.addReference(cell.id);runtime.navigation.show(.assistant)}
        Divider()
        Button("删除单元格",role:.destructive) {Task {await workspace.deleteCell(cell.id)}}
      }.labelStyle(.iconOnly).menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
        .disabled(!canChangeStructure).accessibilityIdentifier("notebook-cell-menu-"+cell.id)
    }.font(.callout).buttonStyle(.borderless)
  }
  @ViewBuilder private func cellResults(_ cell:NativeSourceCell)->some View {
    CellDiagnostics(session:session,cell:cell,workspace:workspace)
    if let result=runtime.results[cell.id] {
      CellPresentationScope(session:session,cell:cell,workspace:workspace) {state in
        VStack(alignment:.leading,spacing:12) {
          ForEach(result.bindings,id:\.result_ref) {binding in
            NativeResultView(presenter:runtime.presenter(for:binding),value:result.values.first{$0.binding.result_ref==binding.result_ref},displayContext:context(result,cell:cell.id),cellState:state,durationMs:runtime.durationsMs[result.receipt.operation_id],hasSourceDiagnostics:runtime.sourceDiagnostics[Data(cell.id.utf8)]?.contains{$0.severity=="Error"} ?? false,onShowSteps:{runtime.navigate(to:cell.id);runtime.navigation.show(.steps)})
              .accessibilityIdentifier("notebook-result-"+cell.id)
          }
          if result.bindings.isEmpty {Text("计算完成，结果暂无法读取。可在详情中核对。").font(.callout).foregroundStyle(.secondary)}
        }.opacity(resultOpacity(state))
      }
      if result.receipt.producer.terminal_status == .error,
        let previous=runtime.history.last(where:{$0.receipt.producer.cell_id.value==cell.id && $0.receipt.producer.terminal_status == .done}) {
        VStack(alignment:.leading,spacing:8) {
          Text("上次结果").font(.system(size:12)).foregroundStyle(Theme.Color.ink2)
          ForEach(previous.bindings,id:\.result_ref) {binding in
            NativeResultView(presenter:runtime.presenter(for:binding),value:nil,displayContext:.history)
          }
        }.opacity(0.65)
      }
    }
    let history=runtime.history.filter{$0.receipt.producer.cell_id.value==cell.id}
    if !history.isEmpty {
      DisclosureGroup("历史运行 · \(history.count)") {
        ForEach(history,id:\.receipt.operation_id) {previous in
          Text("历史计算 · 版本 \(previous.receipt.producer.kernel_state_revision.value)").font(.caption).foregroundStyle(.secondary)
          ForEach(previous.bindings,id:\.result_ref) {binding in NativeResultView(presenter:runtime.presenter(for:binding),value:nil,displayContext:.history)}
        }
      }.font(.callout).padding(.horizontal,10)
    }
  }
  private func resultOpacity(_ state:CellPresentationState)->Double {
    switch state.status {case .stale:return 0.42;case .running:return 0.5;default:return 1}
  }
  @ViewBuilder private var recoveryDetails:some View {
    if runtime.historyOmitted>0 {Text("显示本会话最近 32 次历史运行；更早的回执仍保留在本地记录中。").font(.caption).foregroundStyle(.secondary)}
    if !session.drafts.recoveredOrphans.isEmpty {
      DisclosureGroup("已删除单元格的恢复草稿") {
        ForEach(Array(session.drafts.recoveredOrphans.enumerated()),id:\.offset) {_,draft in
          Text(String(decoding:draft.sourceUTF8,as:UTF8.self)).font(.system(.body,design:.monospaced)).textSelection(.enabled)
        }
      }
    }
  }
  private func context(_ result:NotebookCellResult,cell:String)->NativeResultDisplayContext {
    if runtime.operationCell==cell {return .refreshing}
    if result.receipt.producer.execution_epoch.value != session.drafts.confirmed.execution_epoch.value || session.drafts.confirmed.file.cells.contains(where:{candidate in
      candidate.kind == .math && session.drafts.overlay(candidate.id).map{!$0.source.utf8.elementsEqual(candidate.source.utf8)}==true
    }) {return .sourceChanged}
    return .current
  }
}
private struct NativeDraftBackupStatus:View {
  @ObservedObject var backup:NativeDraftBackup
  var body:some View {Text(backup.status).font(.caption).foregroundStyle(.secondary).accessibilityIdentifier("notebook-draft-backup-status")}
}

private struct NativeNotebookTextCell:View {
  @ObservedObject var workspace:NativeFileWorkspace
  @ObservedObject var session:NativeDocumentSession
  let cell:NativeSourceCell
  @State private var message:String?
  private var source:String {session.drafts.overlay(cell.id)?.source ?? cell.source}
  private var reading:Bool {!session.editingTextCells.contains(cell.id)}
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      if reading {
        NativeMarkdownView(source:source,fontSize:16.5,paper:true,onDoubleClick:{session.beginTextEditing(cell.id)},onInsertCode:{code,language in
          Task {
            do {try await session.insertCell(.math,after:cell.id,source:code,dialect:["wolfram","mathematica"].contains(language.lowercased()) ? .wolfram:.modern);message=nil}
            catch {message="未插入代码，原文已保留；请核对源码或当前笔记本状态。"}
          }
        })
          .frame(maxWidth:640,alignment:.leading)
          .contextMenu {
            Button("编辑") {session.beginTextEditing(cell.id)}
          }
        HStack(spacing:12) {
          Button("编辑") {session.beginTextEditing(cell.id)}.accessibilityIdentifier("notebook-text-edit-"+cell.id)
          Button("复制 Markdown",systemImage:Theme.Symbol.copy) {copy()}.labelStyle(.iconOnly).help("复制完整 Markdown 原文")
        }.font(.system(size:12)).foregroundStyle(Theme.Color.ink2).buttonStyle(.borderless)
      } else {
        NativeSourceEditor(session:session,cell:cell,finishReading:{ [weak session,cell] in
          Task { @MainActor [weak session] in _ = session?.endTextEditing(cell.id) }
        })
          .frame(height:min(320,max(88,CGFloat(source.reduce(1){$1 == "\n" ? $0+1:$0})*21.6+20)))
          .background(Theme.Color.well,in:RoundedRectangle(cornerRadius:Theme.Radius.well))
          .overlay(alignment:.topLeading) {if source.isEmpty {Text("在这里记录文字或 Markdown…").font(Theme.Font.source).foregroundStyle(Theme.Color.ink3).padding(14).allowsHitTesting(false)}}
        HStack {
          Text("Esc 或 ⌘↩ 阅读").foregroundStyle(Theme.Color.ink2)
          Spacer()
          Button("阅读") {if !session.endTextEditing(cell.id) {message="请先完成正在输入的文字。"}}
        }.font(.system(size:12)).buttonStyle(.borderless)
      }
      if let message {Text(message).font(.caption).foregroundStyle(.secondary)}
    }.frame(maxWidth:.infinity,alignment:.leading)
  }
  private func copy() {NSPasteboard.general.clearContents();NSPasteboard.general.setString(source,forType:.string)}
}
