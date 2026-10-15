import AppKit
import OpenMathHost
import SwiftUI

@main struct OpenMathNativeApp:App {
  @NSApplicationDelegateAdaptor(NativeApplicationDelegate.self) private var delegate
  @StateObject private var workspace=NativeFileWorkspace()
  @State private var showingRecovery=false
  @State private var showingSessionRecovery=false
  var body:some Scene {
    Window("OpenMath Preview",id:"workspace") {
      Group {
        if let session=workspace.session {WorkspaceView(workspace:workspace,session:session)}
        else {welcome}
      }.frame(minWidth:640,minHeight:560)
        .background(NativeWindowAttachment(workspace:workspace).frame(width:0,height:0))
        .toolbar {
          ToolbarItemGroup(placement:.automatic) {
            Button("新建笔记本",systemImage:"square.and.pencil") {Task {await workspace.newDocument()}}.disabled(workspace.busy).help("新建笔记本（⌘N）").accessibilityIdentifier("notebook-new")
            Menu("文件",systemImage:"folder") {
              Button("打开笔记本…") {Task {await workspace.openPanel()}}.accessibilityIdentifier("notebook-open")
              Divider()
              Button("另存为…") {Task {await workspace.save(asNew:true)}}.disabled(workspace.session==nil)
              Button("核对保存状态") {Task {await workspace.reconcile()}}.disabled(workspace.session==nil)
              Divider()
              Button("恢复本地笔记本…") {showingRecovery=true;Task {await workspace.refreshRecoverableDocuments()}}.accessibilityIdentifier("notebook-recovery-menu")
              Button("恢复助手会话…") {showingSessionRecovery=true;Task {await workspace.refreshRecoverableSessions()}}.accessibilityIdentifier("session-recovery-menu")
              Divider()
              Button("关闭笔记本") {Task {_ = await workspace.closeDocument()}}.disabled(workspace.session==nil)
            }.disabled(workspace.busy).help("打开、保存与恢复")
          }
        }
        .sheet(isPresented:$showingRecovery) {NativeRecoveryList(workspace:workspace,isPresented:$showingRecovery)}
        .sheet(isPresented:$showingSessionRecovery) {NativeSessionRecoveryList(workspace:workspace,isPresented:$showingSessionRecovery)}
    }.defaultSize(width:1180,height:800).windowToolbarStyle(.unified).commands {
      CommandGroup(replacing:.newItem) {
        Button("新建笔记本") {Task {await workspace.newDocument()}}.keyboardShortcut("n").disabled(workspace.busy)
        Button("打开笔记本…") {Task {await workspace.openPanel()}}.keyboardShortcut("o").disabled(workspace.busy)
      }
      CommandGroup(replacing:.saveItem) {
        Button("保存") {Task {await workspace.save()}}.keyboardShortcut("s").disabled(workspace.session==nil || workspace.busy)
        Button("另存为…") {Task {await workspace.save(asNew:true)}}.keyboardShortcut("s",modifiers:[.command,.shift]).disabled(workspace.session==nil || workspace.busy)
      }
    }
  }
  private var welcome:some View {
    VStack(spacing:24) {
      Image(systemName:"function").font(.system(size:46,weight:.light)).foregroundStyle(.tint)
      VStack(spacing:8) {
        Text("OpenMath").font(.system(size:30,weight:.semibold))
        Text("在笔记本中计算、推导与记录。").font(.title3).foregroundStyle(.secondary)
      }
      HStack(spacing:12) {
        Button("新建笔记本") {Task {await workspace.newDocument()}}.buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
        Button("打开笔记本…") {Task {await workspace.openPanel()}}.buttonStyle(.bordered)
      }.controlSize(.large).disabled(workspace.busy)
      if workspace.busy {ProgressView().controlSize(.small)}
      if let error=workspace.errorText {Text(error).foregroundStyle(.red).textSelection(.enabled).frame(maxWidth:480)}
    }.frame(maxWidth:.infinity,maxHeight:.infinity).background(Color(nsColor:.textBackgroundColor))
  }
}
private struct NativeRecoveryList:View {
  @ObservedObject var workspace:NativeFileWorkspace
  @Binding var isPresented:Bool
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      Text("恢复本地记录").font(.title2)
      Text("选择本地笔记本恢复源码与草稿。不会自动执行单元格或发送模型请求。").font(.caption).foregroundStyle(.secondary)
      if let warning=workspace.libraryWarning {Text(warning).font(.caption).foregroundStyle(.secondary)}
      if let error=workspace.errorText {Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled)}
      if workspace.busy {ProgressView("正在核对本地记录…")}
      List(workspace.recoverableDocuments) {record in
        HStack(alignment:.top) {
          VStack(alignment:.leading,spacing:4) {
            Text(record.title)
            Text(record.hasDrafts ? "有待确认草稿" : record.fileDirty ? "文件尚未保存" : "本地记录与文件已保存状态一致").font(.caption).foregroundStyle(.secondary)
            if let warning=record.warning {Text(warning).font(.caption).foregroundStyle(.secondary)}
          }
          Spacer()
          Button("恢复") {Task {await workspace.recover(record.id);if workspace.session?.drafts.confirmed.document_id==record.id {isPresented=false}}}.disabled(!record.recoverable || workspace.busy).accessibilityIdentifier("restore-document-"+record.id)
        }.padding(.vertical,4)
      }
      HStack {
        Button("刷新") {Task {await workspace.refreshRecoverableDocuments()}}.disabled(workspace.busy)
        if workspace.hasMoreRecoveryDocuments {Button("查看更多") {Task {await workspace.refreshRecoverableDocuments(more:true)}}.disabled(workspace.busy)}
        Spacer();Button("关闭") {isPresented=false}.keyboardShortcut(.cancelAction).disabled(workspace.busy)
      }
    }.padding(20).frame(width:600,height:420)
  }
}
