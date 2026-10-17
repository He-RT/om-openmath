import SwiftUI

struct AgentActionList:View {
  let actions:[AgentAction]
  var model:NativeAssistant?=nil
  var active=false
  var body:some View {
    VStack(alignment:.leading,spacing:10) {
      ForEach(AgentActionGroup.rows(actions)) {group in
        if group.actions.count>1 {
          DisclosureGroup(group.summary) {
            ForEach(group.actions) {action in row(action).padding(.top,6)}
          }.font(.system(size:12)).foregroundStyle(Theme.Color.ink2)
        } else if let action=group.actions.first {row(action)}
      }
    }.padding(.leading,12)
      .overlay(alignment:.leading) {Rectangle().fill(Theme.Color.agent.opacity(0.25)).frame(width:1)}
  }
  @ViewBuilder private func row(_ action:AgentAction)->some View {
    if let model {InteractiveAgentActionRow(action:action,model:model,active:active)}
    else {AgentActionRowLabel(action:action,active:active)}
  }
}
private struct AgentActionRowLabel:View {
  let action:AgentAction,active:Bool
  var undone=false
  private var state:String {
    if undone {return "已撤销"}
    return switch action.status {case .running:active ? "进行中":"等待回执";case .ok:"已返回";case .failed:"失败";case .unknown:"待核对";case .cancelled:"已取消"}
  }
  private var stateIcon:String {
    if undone {return "arrow.uturn.backward"}
    return switch action.status {case .running:"hourglass";case .ok:"checkmark";case .failed:"xmark";case .unknown:"questionmark.circle";case .cancelled:"stop.circle"}
  }
  private var icon:String {
    switch action.tool {
    case "read_notebook":"book"
    case "search_functions","get_function_docs":"magnifyingglass"
    case "preview_source":"eye"
    case "apply_notebook_patch":"pencil"
    case "run_cells":"play"
    case "inspect_result":"cube"
    case "undo_transaction":"arrow.uturn.backward"
    case "evaluate_scratch":"function"
    default:"clock.arrow.circlepath"
    }
  }
  var body:some View {
    VStack(alignment:.leading,spacing:4) {
      HStack(alignment:.firstTextBaseline,spacing:7) {
        Image(systemName:icon).font(.system(size:11)).foregroundStyle(Theme.Color.ink2).frame(width:13).accessibilityHidden(true)
        Text("\(Text(action.verb).fontWeight(.medium))  \(Text(action.object).foregroundColor(Theme.Color.ink2))")
          .fixedSize(horizontal:false,vertical:true)
        Spacer(minLength:0)
        if action.status == .running,active {ProgressView().controlSize(.mini).accessibilityLabel(state)}
        else {Image(systemName:stateIcon).font(.system(size:10)).foregroundStyle(action.status == .failed ? Theme.Color.red:action.status == .ok ? Theme.Color.verified:Theme.Color.ink2).accessibilityLabel(state)}
      }
      if let error=action.error {
        Text(error.message+(error.recovery.isEmpty ? "":"；"+error.recovery)).foregroundStyle(Theme.Color.red).padding(.leading,20).textSelection(.enabled)
          .help(error.code)
      }
    }.font(.system(size:12)).accessibilityElement(children:.contain)
      .accessibilityLabel(action.verb+"，"+action.object+"，"+state).accessibilityIdentifier("agent-action-"+action.callID)
  }
}
private struct InteractiveAgentActionRow:View {
  let action:AgentAction
  @ObservedObject var model:NativeAssistant
  let active:Bool
  @State private var showDiff=false
  @Environment(\.accessibilityReduceMotion) private var reducedMotion
  private var undone:Bool {action.tool != "preview_source" && (action.transactionRef.map{model.undoneActionTransactions.contains($0)} ?? false)}
  private var hasUndo:Bool {action.tool=="apply_notebook_patch" && action.status == .ok && action.transactionRef != nil}
  private var canUndo:Bool {
    !model.busy && !model.undoing && !model.historyLoading && (!undone || model.unknownActionUndo==action.id) && model.actionCanUndo?(action)==true &&
      action.transactionRef.map{model.knownActionTransactions.contains($0)}==true &&
      (model.unknownActionUndo==nil || model.unknownActionUndo==action.id)
  }
  var body:some View {
    VStack(alignment:.leading,spacing:5) {
      AgentActionRowLabel(action:action,active:active,undone:undone && model.unknownActionUndo != action.id)
      HStack(spacing:12) {
        let cells=action.cells.filter{model.actionCellLabel?($0) != nil}
        if cells.count==1,let cell=cells.first {
          Button("定位",systemImage:"scope") {model.locate(cell,reducedMotion:reducedMotion)}
        } else if cells.count>1 {
          Menu("定位",systemImage:"scope") {ForEach(cells,id:\.self) {cell in Button(model.actionCellLabel?(cell) ?? "单元格") {model.locate(cell,reducedMotion:reducedMotion)}}}
            .menuStyle(.borderlessButton).fixedSize()
        }
        if action.transactionRef != nil || (action.tool=="preview_source" && action.status == .ok) {
          Button(showDiff ? "收起差异":"查看差异") {
            showDiff.toggle()
          }.accessibilityIdentifier("agent-diff-"+action.callID)
        }
        if hasUndo {
          if model.actionUndoInProgress==action.id {ProgressView("正在撤销…").controlSize(.mini)}
          else if undone,model.unknownActionUndo != action.id {Text("已撤销").foregroundStyle(Theme.Color.ink2).accessibilityIdentifier("agent-undone-"+action.callID)}
          else {
            Button(model.unknownActionUndo==action.id ? "核对撤销":"撤销") {Task {await model.undoAction(action)}}
              .disabled(!canUndo).accessibilityIdentifier("agent-undo-"+action.callID)
          }
        }
      }.font(.system(size:11.5)).buttonStyle(.borderless).foregroundStyle(Theme.Color.agent).padding(.leading,20)
      if hasUndo,model.actionCanUndo?(action)==false {Text("历史修改，当前文稿仅可查看").font(.caption).foregroundStyle(.secondary).padding(.leading,20)}
      if showDiff {
        Group {
          if let diff=model.actionDiff(action) {AgentSourceDiffView(diff:diff).padding(.leading,20)}
          else if action.transactionRef==nil,action.preview==nil {Text("此预览没有保留完整的比对数据。").font(.caption).foregroundStyle(.secondary).padding(.leading,20)}
          else if model.actionErrors[action.id] != nil,!model.loadingActions.contains(action.id) {
            Button("重新读取差异") {Task {await model.loadActionDiff(action)}}.font(.caption).padding(.leading,20)
          }
          else {ProgressView("正在读取原始差异…").controlSize(.small)}
        }.task(id:action.transactionRef ?? action.id) {
          if model.actionDiff(action)==nil,(action.transactionRef != nil || action.preview != nil) {await model.loadActionDiff(action)}
        }
      }
      if let error=model.actionErrors[action.id] {Text(error).font(.caption).foregroundStyle(Theme.Color.red).padding(.leading,20)}
    }
  }
}
private struct AgentSourceDiffView:View {
  let diff:SourceTransactionDiff
  @State private var limit=80
  var body:some View {
    VStack(alignment:.leading,spacing:8) {
      ForEach(diff.notes,id:\.self) {Text($0).font(.caption)}
      ForEach(diff.cells) {cell in
        Text(cell.title).font(.system(size:11.5,weight:.medium))
        HStack(spacing:10) {
          if let before=cell.before {Button("复制修改前源码") {copy(before)}}
          if let after=cell.after {Button("复制修改后源码") {copy(after)}}
        }.font(.system(size:11)).buttonStyle(.borderless)
        ForEach(cell.notes,id:\.self) {Text($0).font(.caption).foregroundStyle(.secondary)}
        ScrollView(.horizontal) {
          VStack(alignment:.leading,spacing:1) {
            ForEach(Array(cell.lines.prefix(limit))) {line in
              Text(line.prefix+String(line.text.prefix(4096))+(line.text.count>4096 ? "…（此行较长，可复制完整源码）":"")).font(.system(size:11.5,design:.monospaced)).textSelection(.enabled)
                .foregroundStyle(line.kind == .added ? Theme.Color.verified:line.kind == .removed ? Theme.Color.red:Theme.Color.ink2)
                .frame(maxWidth:.infinity,alignment:.leading)
            }
          }.fixedSize(horizontal:true,vertical:true)
        }
        if cell.lines.count>limit {Button("再显示 80 行") {limit+=80}.font(.caption)}
      }
      if diff.cells.isEmpty,diff.notes.isEmpty {Text("这笔事务没有文字源码差异").font(.caption).foregroundStyle(.secondary)}
    }.padding(8).background(Theme.Color.well,in:RoundedRectangle(cornerRadius:6))
  }
  private func copy(_ value:String) {NSPasteboard.general.clearContents();NSPasteboard.general.setString(value,forType:.string)}
}
