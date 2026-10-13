import SwiftUI
import Foundation

private struct RecordedInput:Decodable {let cell:String,input:String,dialect:String?}
@MainActor private final class ResultFixtureWorkspace:ObservableObject {
  let storage:NativeAppStorage
  let workspace:NativeFileWorkspace
  private let file:URL
  private let cells:[RecordedInput]
  @Published private(set) var preparing=false
  @Published private(set) var status="打开后可准备固定用例的实际结果。"
  init() {
    guard let url=Bundle.main.url(forResource:"ActualInputs",withExtension:"json"),let data=try? Data(contentsOf:url),let cells=try? JSONDecoder().decode([RecordedInput].self,from:data) else {fatalError("actual fixture inputs missing")}
    self.cells=cells
    let root=Bundle.main.bundleURL.deletingLastPathComponent().appendingPathComponent("data",isDirectory:true)
    try! FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    file=root.appendingPathComponent("results.omnb")
    if !FileManager.default.fileExists(atPath:file.path) {
      let source=NativeSourceFile(version:1,title:"结果展示验收",cells:cells.map{.init(id:$0.cell,kind:.math,source:$0.input,dialect:$0.dialect=="modern" || ["precision","guard","failed"].contains($0.cell) ? .modern : .wolfram)})
      try! JSONEncoder().encode(source).write(to:file)
    }
    storage=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    workspace=NativeFileWorkspace(storage:storage)
  }
  func open()async {_ = await workspace.open(file);status="固定用例已打开，可准备实际结果。"}
  func prepare()async {
    guard !preparing,let session=workspace.session else {return};preparing=true;defer {preparing=false}
    for (index,cell) in cells.enumerated() {
      status="正在准备 \(index+1)/\(cells.count)：\(cell.cell)"
      session.runtime.run(cell.cell)
      while session.runtime.operation != nil {try? await Task.sleep(for:.milliseconds(20))}
    }
    session.runtime.navigate(to:cells.contains{$0.cell=="roots"} ? "roots" : cells[0].cell);status="实际结果已接纳，可检查公式、结构值、数值和历史。"
  }
}
@main struct ResultPresenterUIFixture:App {
  @StateObject private var model=ResultFixtureWorkspace()
  @State private var dark=false
  var body:some Scene {
    Window("OpenMath Native Result Fixture",id:"results") {
      VStack(spacing:10) {
        HStack {Text(model.status).font(.caption);Spacer();Button("准备固定用例") {Task {await model.prepare()}}.disabled(model.preparing || model.workspace.session==nil);Toggle("深色",isOn:$dark).toggleStyle(.switch)}.padding(12)
        ResultFixtureSurface(workspace:model.workspace)
      }.preferredColorScheme(dark ? .dark : .light).frame(minWidth:640,minHeight:600).task {await model.open()}
    }.defaultSize(width:1100,height:860)
  }
}
private struct ResultFixtureSurface:View {
  @ObservedObject var workspace:NativeFileWorkspace
  var body:some View {
    Group {
      if let session=workspace.session {WorkspaceView(workspace:workspace,session:session)}
      else if let error=workspace.errorText {Text(error).textSelection(.enabled).padding()}
      else {Text("正在打开或已关闭测试笔记本。").foregroundStyle(.secondary)}
    }.background(NativeWindowAttachment(workspace:workspace).frame(width:0,height:0))
  }
}
