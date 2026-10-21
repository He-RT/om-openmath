import AppKit
import OpenMathHost
import SwiftUI

@main struct OpenMathNativeApp:App {
  @NSApplicationDelegateAdaptor(NativeApplicationDelegate.self) private var delegate
  @StateObject private var workspace=NativeFileWorkspace()
  @StateObject private var settingsModel=NativeSettingsModel()
  @State private var showingRecovery=false
  @State private var showingSessionRecovery=false
  @AppStorage("workbench.appearance") private var appearance="system"
  private var selectedColorScheme:ColorScheme? {appearance=="light" ? .light:appearance=="dark" ? .dark:nil}
  private func applyAppearance() {NSApp.appearance=selectedColorScheme.flatMap{NSAppearance(named:$0 == .dark ? .darkAqua:.aqua)}}
  #if DEBUG
  @Environment(\.openWindow) private var openWindow
  #endif
  var body:some Scene {
    Window("OpenMath Preview",id:"workspace") {
      Group {
        if let session=workspace.session {WorkspaceView(workspace:workspace,session:session).id(ObjectIdentifier(session))}
        else {welcome}
      }.frame(minWidth:640,minHeight:560)
        .onAppear {delegate.attach(workspace)}
        .preferredColorScheme(selectedColorScheme)
        .onChange(of:appearance,initial:true) {applyAppearance()}
        .tint(Theme.Color.verified)
        .background(NativeWindowAttachment(workspace:workspace).frame(width:0,height:0))
        .toolbar {
          ToolbarItemGroup(placement:.automatic) {
            if workspace.session==nil {
            Menu("文件",systemImage:"folder") {
              Button("新建笔记本") {Task {await workspace.newDocument()}}.accessibilityIdentifier("notebook-new")
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
        }
        .sheet(isPresented:$showingRecovery) {NativeRecoveryList(workspace:workspace,isPresented:$showingRecovery)}
        .sheet(isPresented:$showingSessionRecovery) {NativeSessionRecoveryList(workspace:workspace,isPresented:$showingSessionRecovery)}
    }.defaultSize(width:1180,height:800).windowToolbarStyle(.unified).commands {
      WorkbenchCommands(workspace:workspace)
      CommandGroup(replacing:.appInfo) {
        Button("关于 OpenMath Preview") {
          let bundle=Bundle.main
          NSApp.orderFrontStandardAboutPanel(options:[
            .applicationVersion:bundle.object(forInfoDictionaryKey:"OpenMathReleaseVersion") as? String ?? "未知版本",
            .version:"源码 "+(bundle.object(forInfoDictionaryKey:"OpenMathSourceRevision") as? String ?? "development")
          ])
        }
      }
      CommandGroup(replacing:.newItem) {
        Button("新建笔记本") {Task {await workspace.newDocument()}}.keyboardShortcut("n").disabled(workspace.busy)
        Button("打开笔记本…") {Task {await workspace.openPanel()}}.keyboardShortcut("o").disabled(workspace.busy)
      }
      CommandGroup(replacing:.saveItem) {
        Button("保存") {Task {await workspace.save()}}.keyboardShortcut("s").disabled(workspace.session==nil || workspace.busy)
        Button("另存为…") {Task {await workspace.save(asNew:true)}}.keyboardShortcut("s",modifiers:[.command,.shift]).disabled(workspace.session==nil || workspace.busy)
      }
      CommandGroup(after:.saveItem) {
        Button("核对保存状态") {Task {await workspace.reconcile()}}.disabled(workspace.session==nil)
        Button("恢复本地笔记本…") {showingRecovery=true;Task {await workspace.refreshRecoverableDocuments()}}
        Button("恢复助手会话…") {showingSessionRecovery=true;Task {await workspace.refreshRecoverableSessions()}}
        Button("关闭笔记本") {Task {_ = await workspace.closeDocument()}}.disabled(workspace.session==nil)
      }
      CommandGroup(replacing:.undoRedo) {
        Button("撤销") {workspace.undo()}.keyboardShortcut("z").disabled(!workspace.canUndo)
        Button("重做") {workspace.redo()}.keyboardShortcut("z",modifiers:[.command,.shift]).disabled(!workspace.canRedo)
      }
      #if DEBUG
      CommandMenu("开发") {
        Button("工作台令牌预览") {openWindow(id:"theme-preview")}
      }
      #endif
    }
    Settings {
      NativeSettingsView(model:settingsModel,workspace:workspace).preferredColorScheme(selectedColorScheme)
        .onChange(of:appearance,initial:true) {applyAppearance()}
    }
    #if DEBUG
    Window("工作台令牌预览",id:"theme-preview") {
      ThemePreview()
    }.defaultSize(width:760,height:720)
    #endif
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
      if workspace.busy {WorkbenchProgress().controlSize(.small)}
      if let error=workspace.errorText {Text(error).foregroundStyle(.red).textSelection(.enabled).frame(maxWidth:480)}
    }.frame(maxWidth:.infinity,maxHeight:.infinity).background(Color(nsColor:.textBackgroundColor))
  }
}
