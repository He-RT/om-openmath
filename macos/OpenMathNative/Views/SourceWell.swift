import AppKit
import SwiftUI

struct SourceWell<Tools:View>:View {
  let session:NativeDocumentSession,cell:NativeSourceCell,selected:Bool
  let tools:Tools
  @State private var hovered=false
  private var source:String {session.drafts.overlay(cell.id)?.source ?? cell.source}
  private var height:CGFloat {
    min(240,max(42,CGFloat(source.reduce(1){$1 == "\n" ? $0+1:$0})*21.6+20))
  }
  var body:some View {
    NativeSourceEditor(session:session,cell:cell).padding(.trailing,76)
      .frame(height:height)
      .background(Theme.Color.well,in:RoundedRectangle(cornerRadius:Theme.Radius.well))
      .overlay(RoundedRectangle(cornerRadius:Theme.Radius.well).stroke(selected ? Theme.Color.lineStrong:.clear,lineWidth:1))
      .overlay(alignment:.topLeading) {
        if source.isEmpty {Text("输入表达式，⌘↩ 运行").font(Theme.Font.source).foregroundStyle(Theme.Color.ink3).padding(.horizontal,14).padding(.vertical,10).allowsHitTesting(false)}
      }
      .overlay(alignment:.topTrailing) {
        tools.padding(.horizontal,10).padding(.top,9)
          .opacity(hovered || selected || NSApp.isFullKeyboardAccessEnabled ? 1:0)
      }
      .onHover {hovered=$0}
  }
}

struct NotebookMathCell<Result:View,Tools:View>:View {
  let session:NativeDocumentSession,cell:NativeSourceCell,index:Int
  @ObservedObject var workspace:NativeFileWorkspace
  let result:Result,tools:Tools
  var body:some View {
    CellPresentationScope(session:session,cell:cell,workspace:workspace) {state in
      VStack(alignment:.leading,spacing:Theme.Space.sm) {
        SourceWell(session:session,cell:cell,selected:session.runtime.selectedCell==cell.id,tools:tools)
        result.padding(.leading,14)
        HStack(spacing:6) {
          if state.status == .running {ProgressView().controlSize(.mini)}
          else {Image(systemName:symbol(state)).frame(width:13)}
          Text(state.label)
          if state.byAgent {Text("· 助手修改").foregroundStyle(Theme.Color.agent)}
        }.font(.system(size:12)).foregroundStyle(state.color).padding(.leading,14)
      }.padding(.leading,20)
        .overlay(alignment:.leading) {CellRail(state:state)}
        .accessibilityElement(children:.contain)
        .accessibilityLabel("单元格 \(index+1)，"+state.label+(state.byAgent ? "，由助手修改":""))
        .accessibilityIdentifier("notebook-cell-state-"+cell.id)
    }
  }
  private func symbol(_ state:CellPresentationState)->String {
    switch state.status {
    case .verified:return Theme.Symbol.verified
    case .error:return Theme.Symbol.error
    case .conditional:return Theme.Symbol.conditional
    case .pending,.unconfirmed:return "circle"
    case .stale:return "clock"
    case .stopped:return Theme.Symbol.stop
    case .numeric:return "equal"
    default:return Theme.Symbol.success
    }
  }
}
