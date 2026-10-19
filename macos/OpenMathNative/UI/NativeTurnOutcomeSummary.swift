import SwiftUI

/// Host facts remain historical after undo; command state never rewrites their provenance.
struct NativeTurnOutcomeSummary:View {
  @ObservedObject var assistant:NativeAssistant
  let outcome:TurnOutcome
  var documentSaved:Bool?=nil
  @Environment(\.accessibilityReduceMotion) private var systemReducedMotion
  private var reducedMotion:Bool {systemReducedMotion || WorkbenchMotion.fixtureReduced}
  private var discussion:Bool {assistant.messages.first{$0.taskID==outcome.id}?.context?.mode=="discuss"}
  private var undone:Bool {assistant.undoneTurns.contains(outcome.id)}
  private var saved:Bool {documentSaved ?? outcome.saved}
  private var stoppedCells:Set<String> {Set(assistant.messages.filter{$0.taskID==outcome.id}.flatMap(\.actions).filter{$0.tool=="run_cells" && $0.status == .cancelled}.flatMap(\.cells))}
  private var cells:[String] {Array(Set(outcome.changedCells.map(\.cellID)+outcome.ranCells.map(\.cellID))).filter{assistant.actionCellLabel?($0) != nil}.sorted()}
  var body:some View {
    if !discussion {
      VStack(alignment:.leading,spacing:9) {
        Text("本轮结果").font(.system(size:12.5,weight:.semibold))
        if undone {Label("本轮修改已撤销，以下为回合当时核验的结果",systemImage:"arrow.uturn.backward").font(.system(size:11.5)).foregroundStyle(Theme.Color.ink2)}
        ForEach(Array(TurnOutcomePresentation.lines(outcome,stopped:stoppedCells,label:{assistant.actionCellLabel?($0)}).enumerated()),id:\.offset) {_,line in
          HStack(alignment:.firstTextBaseline,spacing:7) {
            Image(systemName:line.icon).frame(width:13).foregroundStyle(line.tone == .warning ? Theme.Color.conditional:line.tone == .verified ? Theme.Color.verified:Theme.Color.ink2)
            Text(line.text).fixedSize(horizontal:false,vertical:true).textSelection(.enabled)
          }.font(.system(size:12))
        }
        Label(saved ? "已保存":"尚未保存到文件（⌘S）",systemImage:saved ? "checkmark":"clock")
          .font(.system(size:12)).foregroundStyle(saved ? Theme.Color.verified:Theme.Color.ink2)
        HStack(spacing:12) {
          if !outcome.transactions.isEmpty,assistant.changeOutcome != nil {
            if assistant.changingRound==outcome.id {WorkbenchProgress("正在核对修改…").controlSize(.mini)}
            else if assistant.unknownRoundChange==outcome.id {
              Button(assistant.unknownRoundRedo ? "核对重做":"核对撤销") {
                Task {if assistant.unknownRoundRedo {await assistant.redoTurn(outcome.id)} else {await assistant.undoTurn(outcome.id)}}
              }.accessibilityIdentifier("assistant-reconcile-turn")
            } else if undone {
              Text("已撤销").foregroundStyle(Theme.Color.ink2)
              Button("重做") {Task {await assistant.redoTurn(outcome.id)}}
                .disabled(!assistant.canChangeTurn(outcome,redo:true)).accessibilityIdentifier("assistant-redo-turn")
            } else {
              Button("撤销本轮修改") {Task {await assistant.undoTurn(outcome.id)}}
                .disabled(!assistant.canChangeTurn(outcome,redo:false)).accessibilityIdentifier("assistant-undo-turn")
            }
          }
          if cells.count==1,let cell=cells.first {
            Button("定位"+(assistant.actionCellLabel?(cell) ?? "单元格")) {assistant.locate(cell,reducedMotion:reducedMotion)}
              .accessibilityIdentifier("assistant-locate-turn")
          } else if !cells.isEmpty {
            Menu("定位单元格") {ForEach(cells,id:\.self) {cell in Button(assistant.actionCellLabel?(cell) ?? "单元格") {assistant.locate(cell,reducedMotion:reducedMotion)}}}.menuStyle(.borderlessButton).fixedSize()
          }
        }.font(.system(size:11.5)).buttonStyle(.borderless).foregroundStyle(Theme.Color.agent)
        if let error=assistant.roundErrors[outcome.id] {Text(error).font(.system(size:11.5)).foregroundStyle(Theme.Color.red)}
        Text(UIStrings.end(outcome.endReason)+"，用时 "+TurnOutcomePresentation.duration(outcome.durationMs)+(outcome.factsUnavailable ? "，模型请求次数未核实":"，\(outcome.modelRequests) 次模型请求"))
          .font(.system(size:11)).foregroundStyle(Theme.Color.ink2).textSelection(.enabled)
      }.padding(12).frame(maxWidth:.infinity,alignment:.leading)
        .background(Theme.Color.paper,in:RoundedRectangle(cornerRadius:8))
        .overlay(RoundedRectangle(cornerRadius:8).stroke(Theme.Color.line,lineWidth:1))
        .overlay(alignment:.leading) {UnevenRoundedRectangle(topLeadingRadius:8,bottomLeadingRadius:8).fill(Theme.Color.verified).frame(width:3)}
        .accessibilityElement(children:.contain).accessibilityIdentifier("assistant-turn-outcome")
    }
  }
}
