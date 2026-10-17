import SwiftUI

/// Minimal factual controls for A5; D3 supplies the final workbench-v2 presentation.
struct NativeTurnOutcomeSummary:View {
  @ObservedObject var assistant:NativeAssistant
  let outcome:TurnOutcome
  var body:some View {
    VStack(alignment:.leading,spacing:5) {
      Text("本轮结果").fontWeight(.medium)
      if outcome.factsUnavailable {Text("本轮事实暂无法读取，请核对笔记本")}
      else {
        if !outcome.changedCells.isEmpty {Text("修改 \(Set(outcome.changedCells.map(\.cellID)).count) 个单元格")}
        if !outcome.ranCells.isEmpty {
          let errors=outcome.ranCells.filter{$0.status=="error"}.count
          let known=outcome.ranCells.allSatisfy{$0.diagnosticCount != nil}
          let diagnostics=outcome.ranCells.compactMap(\.diagnosticCount).reduce(0,+)
          Text("运行 \(outcome.ranCells.count) 个单元格"+(errors>0 ? "，\(errors) 个出错":known && diagnostics==0 ? "，没有诊断":known ? "，\(diagnostics) 条诊断":"，诊断尚未核实"))
        }
        ForEach(Array(outcome.products.enumerated()),id:\.offset) {_,product in
          switch product.kind {
          case .scene3d:Text("三维场景已生成，\(product.count ?? 0) 个三角形")
          case .plot:Text("二维图已生成，\(product.count ?? 0) 条曲线")
          case .solutions:Text("\(product.count ?? 0) 个解"+(product.verified ? "，已验证":""))
          case .expression:Text(product.values.first ?? "表达式结果已检查")
          }
        }
        if !outcome.staleAfterEdit.isEmpty {Text("\(outcome.staleAfterEdit.count) 个单元格修改后没有重新运行，结果过期")}
        Text(outcome.saved ? "已保存":"尚未保存到文件（⌘S）")
      }
      if !outcome.changedCells.isEmpty,assistant.undoOutcome != nil {
        Button(assistant.undoneTurns.contains(outcome.id) ? "已撤销":"撤销本轮修改") {Task {await assistant.undoTurn(outcome.id)}}
          .disabled(assistant.busy || assistant.undoing || assistant.undoneTurns.contains(outcome.id))
          .accessibilityIdentifier("assistant-undo-turn")
      }
      if let error=assistant.undoError {Text(error).foregroundStyle(.secondary)}
      Text(UIStrings.end(outcome.endReason)+"，\(outcome.durationMs / 1000) 秒，\(outcome.modelRequests) 次模型请求").foregroundStyle(.secondary)
    }.font(.caption).padding(.vertical,6).accessibilityIdentifier("assistant-turn-outcome")
  }
}
