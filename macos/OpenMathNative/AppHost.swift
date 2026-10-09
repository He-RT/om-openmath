import AppKit
import OpenMathHost
import SwiftUI

@main struct OpenMathNativeApp:App {
  var body:some Scene {
    Window("OpenMath Preview",id:"workspace") {
      HostScratchView().frame(minWidth:640,minHeight:560)
    }.defaultSize(width:1100,height:780)
  }
}
private struct HostScratchView:View {
  @StateObject private var model=NotebookViewModel()
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
      Text("此处为隔离试算。笔记本工作台与文件保存正在接入。").font(.footnote).foregroundStyle(.secondary)
    }.padding(24).tint(.green)
      .task { await model.start() }
      .onDisappear { Task { await model.close() } }
  }
}
