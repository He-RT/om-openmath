import AppKit
import SwiftUI

@main struct WorkbenchUIFixture:App {
  @StateObject private var workspace:NativeFileWorkspace
  private let fixture:URL
  init() {
    guard let resource=Bundle.main.url(forResource:"FixtureRoot",withExtension:"txt"),let value=try? String(contentsOf:resource,encoding:.utf8) else {fatalError("fixture root missing")}
    let root=URL(fileURLWithPath:value.trimmingCharacters(in:.whitespacesAndNewlines),isDirectory:true)
    fixture=root.appendingPathComponent("fixture.omnb")
    _workspace=StateObject(wrappedValue:NativeFileWorkspace(storage:NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))))
  }
  var body:some Scene {
    Window("OpenMath Workbench Fixture",id:"fixture") {
      Group {
        if let session=workspace.session {WorkspaceView(workspace:workspace,session:session)}
        else if let error=workspace.errorText {Text("测试笔记本未打开："+error).textSelection(.enabled).padding()}
        else {ProgressView("正在打开测试笔记本")}
      }.frame(minWidth:640,minHeight:560)
        .background(NativeWindowAttachment(workspace:workspace).frame(width:0,height:0))
        .task {_ = await workspace.open(fixture)}
    }.defaultSize(width:1042,height:780)
  }
}
