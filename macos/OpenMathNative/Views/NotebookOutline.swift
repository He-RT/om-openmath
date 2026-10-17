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
  private func dot(_ cell:NativeSourceCell)->some View {
    CellPresentationScope(session:session,cell:cell,workspace:workspace) {state in
      Circle().fill(state.muted ? Color.clear:state.color).frame(width:7,height:7)
        .overlay(Circle().stroke(state.color,lineWidth:state.muted ? 1.5:0))
        .frame(width:12,height:12)
        .overlay(Circle().stroke(state.byAgent ? Theme.Color.agent:.clear,lineWidth:1.5))
        .accessibilityLabel(state.label+(state.byAgent ? "，由助手修改":""))
    }
  }
}
