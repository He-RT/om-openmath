import AppKit
import SwiftUI

/// Live human/system IME fixture. No control synthesizes a composition or candidate selection.
@main struct CompositionUIFixture:App {
  @StateObject private var workspace:NativeFileWorkspace
  @State private var loaded=false
  @State private var opening=true
  private let fixture:URL
  init() {
    guard let resource=Bundle.main.url(forResource:"FixtureRoot",withExtension:"txt"),let value=try? String(contentsOf:resource,encoding:.utf8) else {fatalError("fixture root missing")}
    let root=URL(fileURLWithPath:value.trimmingCharacters(in:.whitespacesAndNewlines),isDirectory:true)
    fixture=root.appendingPathComponent("composition.omnb")
    _workspace=StateObject(wrappedValue:NativeFileWorkspace(storage:NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))))
  }
  var body:some Scene {
    Window("OpenMath Native IME Fixture",id:"composition") {
      VStack(spacing:0) {
        if let session=workspace.session {
          LiveCompositionStatus(session:session)
          Divider();WorkspaceView(workspace:workspace,session:session)
        } else if let error=workspace.errorText {Text(error).padding()}
        else {Text(opening ? "正在打开原生输入法验收文档" : "测试文档已关闭").padding()}
      }.frame(minWidth:861,minHeight:680)
        .background(NativeWindowAttachment(workspace:workspace).frame(width:0,height:0))
        .task {guard !loaded else {return};loaded=true;_ = await workspace.open(fixture);opening=false}
    }.defaultSize(width:1042,height:780)
  }
}
private struct LiveCompositionStatus:View {
  let session:NativeDocumentSession
  var body:some View {
    TimelineView(.periodic(from:.now,by:0.1)) {_ in
      VStack(alignment:.leading,spacing:6) {
        Text("使用系统中文或日文输入法，在数学源码的注释内输入。候选、Enter、Tab、Esc 由系统处理。").font(.callout)
        ForEach(session.drafts.confirmed.file.cells,id:\.id) {cell in
          if let editor=try? session.editor(cell.id),let draft=session.drafts.overlay(cell.id) {
            let actual=editor.textView.hasMarkedText(),range=editor.textView.markedRange()
            Text("\(cell.id)：系统合成 \(actual ? "进行中" : "已结束") · 原生 markedRange \(actual ? "\(range.location):\(range.length)" : "无") · 草稿 \(draft.sequence) · 已确认源码 \(session.drafts.confirmed.revision.value)")
              .font(.system(.caption,design:.monospaced)).accessibilityIdentifier("live-ime-status-"+cell.id)
          }
        }
      }.frame(maxWidth:.infinity,alignment:.leading).padding(12)
    }
  }
}
