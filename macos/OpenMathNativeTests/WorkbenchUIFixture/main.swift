import AppKit
import SwiftUI

@main struct WorkbenchUIFixture:App {
  @StateObject private var workspace:NativeFileWorkspace
  private let fixture:URL
  private let transcript:[AssistantMessage]?
  init() {
    guard let resource=Bundle.main.url(forResource:"FixtureRoot",withExtension:"txt"),let value=try? String(contentsOf:resource,encoding:.utf8) else {fatalError("fixture root missing")}
    let root=URL(fileURLWithPath:value.trimmingCharacters(in:.whitespacesAndNewlines),isDirectory:true)
    fixture=root.appendingPathComponent("fixture.omnb")
    transcript=Bundle.main.url(forResource:"ActionTranscript",withExtension:"json").flatMap{try? JSONDecoder().decode([AssistantMessage].self,from:Data(contentsOf:$0))}
    _workspace=StateObject(wrappedValue:NativeFileWorkspace(storage:NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))))
  }
  var body:some Scene {
    Window("OpenMath Workbench Fixture",id:"fixture") {
      Group {
        if let transcript {
          ScrollView {VStack(alignment:.leading,spacing:18) {
            Text("操作记录组件预览（回环数据，不调用模型）").font(.caption).foregroundStyle(.secondary)
            ForEach(transcript) {message in NativeAssistantMessageBody(message:message)}
          }.padding(20)}.frame(width:400)
        }
        else if let session=workspace.session {WorkspaceView(workspace:workspace,session:session)}
        else if let error=workspace.errorText {Text("测试笔记本未打开："+error).textSelection(.enabled).padding()}
        else {ProgressView("正在打开测试笔记本")}
      }.frame(minWidth:transcript==nil ? 640:400,minHeight:560)
        .background(NativeWindowAttachment(workspace:workspace).frame(width:0,height:0))
        .task {if transcript==nil {_ = await workspace.open(fixture)}}
    }.defaultSize(width:transcript==nil ? 1042:400,height:780)
  }
}
