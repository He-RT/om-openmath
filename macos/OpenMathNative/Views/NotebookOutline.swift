import AppKit
import SwiftUI

struct NotebookOutline:View {
  @ObservedObject var workspace:NativeFileWorkspace
  let session:NativeDocumentSession
  @ObservedObject private var runtime:NotebookRuntime
  @State private var headings:[String:String]=[:]
  init(workspace:NativeFileWorkspace,session:NativeDocumentSession) {
    self.workspace=workspace;self.session=session
    _runtime=ObservedObject(wrappedValue:session.runtime)
  }
  private var cells:[NativeSourceCell] {session.drafts.confirmed.file.cells}
  private var textSources:[String:String] {
    Dictionary(uniqueKeysWithValues:cells.filter{$0.kind == .text}.map{($0.id,session.drafts.overlay($0.id)?.source ?? $0.source)})
  }
  var body:some View {
    VStack(spacing:0) {
      List {
        ForEach(Array(cells.enumerated()),id:\.element.id) {index,cell in
          let heading=headings[cell.id]
          Button {runtime.navigate(to:cell.id)} label: {
            HStack(spacing:8) {
              if heading==nil {
                if cell.kind == .math {dot(cell)}
                else {Image(systemName:"text.alignleft").foregroundStyle(Theme.Color.ink3).frame(width:12)}
              }
              Text(heading ?? firstLine(cell))
                .font(heading==nil ? .system(size:12,design:cell.kind == .math ? .monospaced:.default):.system(size:12,weight:.semibold))
                .foregroundStyle(heading==nil ? Theme.Color.ink:Theme.Color.ink2)
                .lineLimit(1).truncationMode(.tail)
              Spacer(minLength:0)
            }.frame(maxWidth:.infinity,alignment:.leading).contentShape(Rectangle())
          }.buttonStyle(.plain).focusable(false)
            .listRowBackground(runtime.selectedCell==cell.id ? Theme.Color.verified.opacity(0.12):.clear)
            .listRowInsets(EdgeInsets(top:heading==nil ? 4:16,leading:12,bottom:4,trailing:8))
            .accessibilityLabel("单元格 \(index+1)，"+(heading ?? firstLine(cell)))
            .accessibilityIdentifier("outline-cell-"+cell.id)
        }
      }.listStyle(.sidebar).scrollContentBackground(.hidden)
        .accessibilityIdentifier("workspace-outline")
      Divider()
      HStack {
        Text("\(cells.count) 个单元格")
        Spacer()
        Text("⌘1").accessibilityHidden(true)
      }.font(.system(size:11.5)).foregroundStyle(Theme.Color.ink2)
        .padding(.horizontal,Theme.Space.md).padding(.vertical,Theme.Space.sm)
    }.frame(maxWidth:.infinity,maxHeight:.infinity,alignment:.topLeading)
      .background(.bar)
      .task(id:textSources) {
        let sources=textSources
        let parsed=await Task.detached(priority:.utility) {
          sources.compactMapValues{MarkdownRenderDocument.parse($0).firstOutlineHeading}
        }.value
        if !Task.isCancelled {headings=parsed}
      }
  }
  private func firstLine(_ cell:NativeSourceCell)->String {
    let source=session.drafts.overlay(cell.id)?.source ?? cell.source
    return source.split(whereSeparator:\.isNewline).first.map(String.init) ?? (cell.kind == .math ? "数学单元格":"文字单元格")
  }
  @ViewBuilder private func dot(_ cell:NativeSourceCell)->some View {
    let result=runtime.results[cell.id]
    let dirty=session.drafts.overlay(cell.id).map{!$0.source.utf8.elementsEqual(cell.source.utf8)} ?? false
    let stale=result.map{$0.receipt.producer.execution_epoch.value != session.drafts.confirmed.execution_epoch.value} ?? false
    let running=runtime.operationCell==cell.id
    let failed=result?.receipt.producer.terminal_status == .error
    let agent=runtime.agentModifiedCells.contains(cell.id) && !dirty
    if let binding=result?.bindings.last {
      OutlineResultDot(presenter:runtime.presenter(for:binding),running:running,stale:stale || dirty,failed:failed,byAgent:agent)
    } else {
      OutlineDot(color:failed ? Theme.Color.red:running ? Theme.Color.verified:Theme.Color.ink3,
        filled:failed || running,byAgent:agent,label:failed ? "计算出错":running ? "正在计算":"尚未运行")
    }
  }
}

private struct OutlineResultDot:View {
  @ObservedObject var presenter:ResultPresenter
  let running:Bool,stale:Bool,failed:Bool,byAgent:Bool
  private var conditional:Bool {
    presenter.presentation?.solutions?.solutions.contains{$0.condition_latex.value != nil} ?? false
  }
  var body:some View {
    OutlineDot(color:running ? Theme.Color.verified:stale ? Theme.Color.ink3:failed ? Theme.Color.red:conditional ? Theme.Color.conditional:Theme.Color.verified,
      filled:running || !stale,byAgent:byAgent,
      label:running ? "正在计算":stale ? "结果过期":failed ? "计算出错":conditional ? "有条件成立":"已有计算结果")
  }
}
private struct OutlineDot:View {
  let color:Color,filled:Bool,byAgent:Bool,label:String
  var body:some View {
    Circle().fill(filled ? color:.clear).frame(width:7,height:7)
      .overlay(Circle().stroke(color,lineWidth:filled ? 0:1.5))
      .frame(width:12,height:12)
      .overlay(Circle().stroke(byAgent ? Theme.Color.agent:.clear,lineWidth:1.5))
      .accessibilityLabel(label+(byAgent ? "，由助手修改":""))
  }
}
