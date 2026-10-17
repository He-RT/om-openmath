import SwiftUI

struct WorkbenchCommands:Commands {
  @ObservedObject var workspace:NativeFileWorkspace
  var body:some Commands {
    CommandMenu("笔记本") {
      Button("显示或隐藏目录") {workspace.session?.runtime.navigation.toggleOutline()}
        .keyboardShortcut("1").disabled(workspace.session==nil)
      Button("显示或隐藏检查器") {workspace.session?.runtime.navigation.toggleInspector()}
        .keyboardShortcut("0",modifiers:[.command,.option]).disabled(workspace.session==nil)
      Divider()
      Button("助手") {workspace.session?.runtime.navigation.show(.assistant)}
        .keyboardShortcut("1",modifiers:[.command,.option]).disabled(workspace.session==nil)
      Button("步骤") {workspace.session?.runtime.navigation.show(.steps)}
        .keyboardShortcut("2",modifiers:[.command,.option]).disabled(workspace.session==nil)
      Button("变量") {workspace.session?.runtime.navigation.show(.variables)}
        .keyboardShortcut("3",modifiers:[.command,.option]).disabled(workspace.session==nil)
      Divider()
      Button("核对原运行") {Task {await workspace.session?.runtime.reconcile()}}
        .disabled(workspace.session?.runtime.unknownOperation==nil)
      Button("停止当前工作") {
        guard let session=workspace.session else {return}
        Task {if session.assistant.busy {await session.assistant.stop()} else {await session.runtime.stop()}}
      }.keyboardShortcut(".").disabled(workspace.session==nil)
    }
  }
}
