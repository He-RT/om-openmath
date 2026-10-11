import AppKit
import OpenMathHost
import SwiftUI

@main struct OpenMathNativeApp:App {
  @NSApplicationDelegateAdaptor(NativeApplicationDelegate.self) private var delegate
  @StateObject private var workspace=NativeFileWorkspace()
  @StateObject private var scratch=NotebookViewModel()
  var body:some Scene {
    Window("OpenMath Preview",id:"workspace") {
      VStack(spacing:0) {
        HStack {
          Button("新建笔记本",systemImage:"doc.badge.plus") {Task {await workspace.newDocument()}}.disabled(workspace.busy).accessibilityIdentifier("notebook-new")
          Button("打开笔记本",systemImage:"folder") {Task {await workspace.openPanel()}}.disabled(workspace.busy).accessibilityIdentifier("notebook-open")
          Spacer()
        }.padding(12)
        Divider()
        if let session=workspace.session {NativeNotebookFileView(workspace:workspace,session:session)}
        else {HostScratchView(model:scratch);if let error=workspace.errorText {Text(error).foregroundStyle(.red).padding(8)}}
      }.frame(minWidth:640,minHeight:560)
        .background(NativeWindowAttachment(workspace:workspace).frame(width:0,height:0))
        .task {delegate.attach(workspace,scratch:scratch);await scratch.start()}
        .onDisappear {Task {await scratch.close()}}
    }.defaultSize(width:1100,height:780).commands {
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
}
private struct HostScratchView:View {
  @ObservedObject var model:NotebookViewModel
  var body:some View {
    VStack(alignment:.leading,spacing:16) {
      HStack {
        Label("OpenMath",systemImage:"function").font(.title)
        Spacer()
        Text("开发预览").foregroundStyle(.secondary)
      }
      Text("试算").font(.headline)
      TextEditor(text:$model.draft).font(.system(.body,design:.monospaced))
        .frame(minHeight:120,maxHeight:240).padding(4)
        .overlay(RoundedRectangle(cornerRadius:8).stroke(.quaternary))
        .accessibilityLabel("数学源码").accessibilityIdentifier("scratch-source")
        .onChange(of:model.draft) { model.noteDraft() }
      HStack {
        Button("运行",systemImage:"play.fill") { Task { await model.run() } }
          .keyboardShortcut(.return,modifiers:[.command])
          .disabled(model.projection?.hostPhase != .ready || model.operationID != nil)
          .accessibilityIdentifier("scratch-run")
        Button("停止",systemImage:"stop.fill") { Task { await model.stop() } }
          .disabled(model.operationID==nil || model.cancelPending).accessibilityIdentifier("scratch-stop")
        Text(model.message).foregroundStyle(.secondary).accessibilityIdentifier("scratch-status")
        Spacer()
        if model.isStale { Label("源码已修改",systemImage:"clock").foregroundStyle(.secondary) }
      }
      Divider()
      ScrollView {
        Text(model.output.isEmpty ? "输入表达式后运行，结果将显示在这里。" : model.output)
          .font(.system(.body,design:.monospaced)).textSelection(.enabled)
          .frame(maxWidth:.infinity,alignment:.leading).accessibilityIdentifier("scratch-output")
      }.frame(maxWidth:.infinity,maxHeight:.infinity)
      Text(model.storageMessage).font(.footnote).foregroundStyle(.secondary)
      Text("此处为隔离试算。使用上方新建或打开入口编辑和保存笔记本。").font(.footnote).foregroundStyle(.secondary)
    }.padding(24).tint(.green)
  }
}
