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
    guard let url=await NativeDocument.openWithPanel() else {return}
    _ = await replaceReserved(url)
  }
  func newDocument() async {_ = await replace(nil)}
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
}

@MainActor final class NativeApplicationDelegate:NSObject,NSApplicationDelegate {
  weak var workspace:NativeFileWorkspace?
  private weak var scratch:NotebookViewModel?
  private var pending:[URL]=[]
  private var terminating=false
  func attach(_ workspace:NativeFileWorkspace,scratch:NotebookViewModel) {
    self.workspace=workspace;self.scratch=scratch
    if let url=pending.first {pending=[];Task {let opened=await workspace.open(url);NSApplication.shared.reply(toOpenOrPrint:opened ? .success : .failure)}}
  }
  func application(_ sender:NSApplication,openFiles filenames:[String]) {
    guard filenames.count==1,let filename=filenames.first else {sender.reply(toOpenOrPrint:.failure);return}
    let url=URL(fileURLWithPath:filename)
    if let workspace {Task {let opened=await workspace.open(url);sender.reply(toOpenOrPrint:opened ? .success : .failure)}}
    else if pending.isEmpty {pending=[url]} else {sender.reply(toOpenOrPrint:.failure)}
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

/// Basic file editing path; rich notebook layout/results are implemented by R4.2. Editors are
/// retained by the document session so SwiftUI redraws cannot replace marked input or selection.
struct NativeNotebookFileView:View {
  @ObservedObject var workspace:NativeFileWorkspace
  let session:NativeDocumentSession
  @ObservedObject private var runtime:NotebookRuntime
  init(workspace:NativeFileWorkspace,session:NativeDocumentSession) {self.workspace=workspace;self.session=session;_runtime=ObservedObject(wrappedValue:session.runtime)}
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      HStack {Text(session.document.fileURL?.lastPathComponent ?? "未命名笔记本").font(.title2);Spacer();Text(session.document.statusText).foregroundStyle(.secondary).accessibilityIdentifier("notebook-file-status")}
      NativeDraftBackupStatus(backup:session.backup)
      if let error=workspace.errorText {Text(error).foregroundStyle(.red).textSelection(.enabled)}
      if let warning=workspace.libraryWarning {Text(warning).foregroundStyle(.secondary).textSelection(.enabled)}
      ForEach(Array(session.recoveryMessages.enumerated()),id:\.offset) {_,message in Text(message).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)}
      if session.recoveryHistoryCursor != nil {Button("继续核对历史") {Task {await session.reconcileRecoveryHistory()}}}
      HStack {
        Button("保存",systemImage:"square.and.arrow.down") {Task {await workspace.save()}}.disabled(workspace.busy).accessibilityIdentifier("notebook-save")
        Button("另存为") {Task {await workspace.save(asNew:true)}}.disabled(workspace.busy)
        Button("核对保存") {Task {await workspace.reconcile()}}.disabled(workspace.busy)
        Spacer()
        Button("关闭笔记本") {Task {_ = await workspace.closeDocument()}}.disabled(workspace.busy)
      }
      ScrollViewReader {proxy in ScrollView {
        LazyVStack(alignment:.leading,spacing:14) {
          ForEach(session.drafts.confirmed.file.cells,id:\.id) {cell in
            VStack(alignment:.leading,spacing:5) {
              Text(cell.kind == .math ? "数学" : "文字").font(.caption).foregroundStyle(.secondary)
              NativeSourceEditor(session:session,cell:cell).frame(minHeight:110).border(.quaternary)
              if cell.kind == .math {
                HStack {
                  Button("运行此单元格",systemImage:"play.fill") {runtime.run(cell.id)}.disabled(!runtime.canRun || workspace.busy)
                    .accessibilityIdentifier("notebook-run-"+cell.id)
                  if let result=runtime.results[cell.id] {
                    let stale=result.receipt.producer.execution_epoch.value != session.drafts.confirmed.execution_epoch.value || session.drafts.confirmed.file.cells.filter{$0.kind == .math}.contains{(session.drafts.overlay($0.id)?.source.utf8 ?? $0.source.utf8).elementsEqual($0.source.utf8)==false}
                    Text(stale ? "历史结果 · 源码已修改" : result.receipt.producer.terminal_status == .done ? "已接纳" : "计算有错误").font(.caption).foregroundStyle(.secondary)
                  }
                }
                if let result=runtime.results[cell.id] {
                  ForEach(Array(result.sources.enumerated()),id:\.offset) {_,text in
                    HStack(alignment:.top) {
                      Text(String(text.prefix(4000))).font(.system(.body,design:.monospaced)).textSelection(.enabled).frame(maxWidth:.infinity,alignment:.leading)
                      Button("复制结果") {NSPasteboard.general.clearContents();NSPasteboard.general.setString(text,forType:.string)}.labelStyle(.iconOnly)
                    }.padding(.vertical,6).accessibilityIdentifier("notebook-result-"+cell.id)
                    if text.count>4000 {Text("此处显示预览，可复制完整结果。").font(.caption).foregroundStyle(.secondary)}
                  }
                }
              }
              if session.drafts.overlay(cell.id)?.requiresReview==true {
                HStack {
                  Text("恢复的草稿尚未确认").font(.caption)
                  Button("使用恢复草稿") {workspace.reviewDraft(cell.id,keep:true)}
                  Button("使用已确认源码") {workspace.reviewDraft(cell.id,keep:false)}
                }
              }
            }.id(cell.id)
          }
        }
      }.onChange(of:runtime.navigationSequence) {if let cell=runtime.selectedCell {proxy.scrollTo(cell,anchor:.top)}}}
      if !session.drafts.recoveredOrphans.isEmpty {
        DisclosureGroup("已删除单元格的恢复草稿") {
          ForEach(Array(session.drafts.recoveredOrphans.enumerated()),id:\.offset) {_,draft in
            Text(String(decoding:draft.sourceUTF8,as:UTF8.self)).font(.system(.body,design:.monospaced)).textSelection(.enabled)
          }
        }
      }
    }.padding(20)
  }
}
private struct NativeDraftBackupStatus:View {
  @ObservedObject var backup:NativeDraftBackup
  var body:some View {Text(backup.status).font(.caption).foregroundStyle(.secondary).accessibilityIdentifier("notebook-draft-backup-status")}
}
