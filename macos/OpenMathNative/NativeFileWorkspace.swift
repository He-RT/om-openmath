import AppKit
import SwiftUI

/// Single current source document. All menus, panel opens and OS file opens use this owner;
/// an old session is never replaced before its explicit close/save policy succeeds.
@MainActor final class NativeFileWorkspace:NSObject,ObservableObject,NSWindowDelegate {
  @Published private(set) var session:NativeDocumentSession?
  @Published private(set) var busy=false
  @Published private(set) var errorText:String?
  private let services=NativeAppStorage()
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
    guard !busy,let url=await NativeDocument.openWithPanel() else {return}
    _ = await open(url)
  }
  func newDocument() async {_ = await replace(nil)}
  func open(_ url:URL) async->Bool {await replace(url)}
  private func replace(_ url:URL?) async->Bool {
    guard !busy else {return false};busy=true;defer {busy=false}
    do {
      // Prepare the new source without changing the old window. A bad/offline file leaves the
      // current source, marked input, scroll and file binding available.
      let storage=try await services.open()
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
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      HStack {Text(session.document.fileURL?.lastPathComponent ?? "未命名笔记本").font(.title2);Spacer();Text(session.document.statusText).foregroundStyle(.secondary).accessibilityIdentifier("notebook-file-status")}
      if let error=workspace.errorText {Text(error).foregroundStyle(.red).textSelection(.enabled)}
      HStack {
        Button("保存",systemImage:"square.and.arrow.down") {Task {await workspace.save()}}.disabled(workspace.busy).accessibilityIdentifier("notebook-save")
        Button("另存为") {Task {await workspace.save(asNew:true)}}.disabled(workspace.busy)
        Button("核对保存") {Task {await workspace.reconcile()}}.disabled(workspace.busy)
        Spacer()
        Button("关闭笔记本") {Task {_ = await workspace.closeDocument()}}.disabled(workspace.busy)
      }
      ScrollView {
        LazyVStack(alignment:.leading,spacing:14) {
          ForEach(session.drafts.confirmed.file.cells,id:\.id) {cell in
            VStack(alignment:.leading,spacing:5) {
              Text(cell.kind == .math ? "数学" : "文字").font(.caption).foregroundStyle(.secondary)
              NativeSourceFileEditor(session:session,cell:cell).frame(minHeight:110).border(.quaternary)
            }
          }
        }
      }
      Text("文件编辑与保存已接通；完整笔记本展示继续按计划接入。打开文件不会自动运行单元格。").font(.footnote).foregroundStyle(.secondary)
    }.padding(20)
  }
}
private struct NativeSourceFileEditor:NSViewRepresentable {
  let session:NativeDocumentSession
  let cell:NativeSourceCell
  func makeNSView(context:Context)->NSScrollView {
    let scroll=NSScrollView();scroll.hasVerticalScroller=true;scroll.hasHorizontalScroller=true
    do {
      let native=try session.editor(cell.id).textView
      native.font = .monospacedSystemFont(ofSize:14,weight:.regular);native.textContainerInset=NSSize(width:8,height:8)
      native.isVerticallyResizable=true;native.isHorizontallyResizable=false;native.autoresizingMask=[.width]
      native.textContainer?.widthTracksTextView=true
      native.setAccessibilityLabel(cell.kind == .math ? "数学源码" : "文字源码")
      native.setAccessibilityIdentifier("notebook-cell-"+cell.id);scroll.documentView=native
    } catch {
      let message=NSTextView();message.string="编辑器不可用："+error.localizedDescription;message.isEditable=false;scroll.documentView=message
    }
    return scroll
  }
  func updateNSView(_ nsView:NSScrollView,context:Context) {}
}
