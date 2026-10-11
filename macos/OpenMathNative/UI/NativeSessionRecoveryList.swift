import SwiftUI

/// Read/reconcile-only recovery UI; no task resume, executor or model dispatch action.
struct NativeSessionRecoveryList:View {
  @ObservedObject var workspace:NativeFileWorkspace
  @Binding var isPresented:Bool
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      Text("会话恢复").font(.title2)
      Text("查看历史并核对原操作。不会自动执行工具或重新发送模型请求。").font(.caption).foregroundStyle(.secondary)
      if let warning=workspace.libraryWarning {Text(warning).font(.caption).foregroundStyle(.secondary)}
      if let error=workspace.errorText {Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled)}
      if workspace.busy {ProgressView("正在核对…")}
      HSplitView {
        List(workspace.recoverableSessions) {record in
          VStack(alignment:.leading,spacing:6) {
            Text(record.id).font(.system(.caption,design:.monospaced)).textSelection(.enabled)
            Text("\(record.eventCount) 条原始记录").font(.caption).foregroundStyle(.secondary)
            Button("查看与核对") {Task {await workspace.inspectSessionRecovery(record.id)}}.disabled(workspace.busy).accessibilityIdentifier("recover-session-"+record.id)
          }.padding(.vertical,4)
        }.frame(minWidth:220)
        ScrollView {
          VStack(alignment:.leading,spacing:10) {
            if let page=workspace.selectedSessionRecovery {
              ForEach(Array(page.tools.enumerated()),id:\.offset) {_,tool in
                VStack(alignment:.leading,spacing:4) {
                  Text(tool.callID).font(.system(.caption,design:.monospaced)).textSelection(.enabled)
                  Text(toolText(tool.phase)).font(.body).accessibilityIdentifier("recovered-tool-"+tool.callID)
                  if let target=tool.target {Text("原操作："+target.operationID).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)}
                }
              }
              ForEach(Array(page.requests.enumerated()),id:\.offset) {_,request in
                VStack(alignment:.leading,spacing:4) {
                  Text(request.contextID).font(.system(.caption,design:.monospaced)).textSelection(.enabled)
                  Text(requestText(request.phase)).accessibilityIdentifier("recovered-request-"+request.contextID)
                }
              }
              if page.tools.isEmpty && page.requests.isEmpty {Text("这一页没有工具或模型请求记录。").foregroundStyle(.secondary)}
              if !page.complete {Button("继续核对下一页") {if let id=workspace.selectedSessionID {Task {await workspace.inspectSessionRecovery(id,more:true)}}}.disabled(workspace.busy)}
            } else {Text("选择会话查看已提交效果与待核对状态。").foregroundStyle(.secondary)}
          }.frame(maxWidth:.infinity,alignment:.leading).padding(12)
        }.frame(minWidth:280)
      }
      HStack {
        Button("刷新") {Task {await workspace.refreshRecoverableSessions()}}.disabled(workspace.busy)
        if workspace.hasMoreRecoverySessions {Button("更多会话") {Task {await workspace.refreshRecoverableSessions(more:true)}}.disabled(workspace.busy)}
        Spacer();Button("关闭") {isPresented=false}.keyboardShortcut(.cancelAction).disabled(workspace.busy)
      }
    }.padding(20).frame(width:700,height:480)
  }
  private func toolText(_ phase:RecoveredToolPhase)->String {
    switch phase {case .tool_not_started:return "尚未接纳，未执行";case .tool_outcome_unknown:return "效果未知，需要核对原操作";case .completed:return "原操作已确认完成";case .partial:return "已确认部分效果，未全部完成"}
  }
  private func requestText(_ phase:RecoveredRequestPhase)->String {
    switch phase {case .not_dispatched:return "尚未发送";case .dispatch_outcome_unknown:return "发送结果未知，未重发";case .response_observed:return "已收到响应，回复中断";case .finished:return "请求已完成";case .failed:return "请求已记录失败"}
  }
}
