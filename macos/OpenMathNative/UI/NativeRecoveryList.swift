import SwiftUI

struct NativeRecoveryList:View {
  @ObservedObject var workspace:NativeFileWorkspace
  @Binding var isPresented:Bool
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      Text("恢复本地记录").font(.title2)
      Text("选择本地笔记本恢复源码与草稿。不会自动执行单元格或发送模型请求。").font(.caption).foregroundStyle(.secondary)
      if let warning=workspace.libraryWarning {Text(warning).font(.caption).foregroundStyle(.secondary)}
      if let error=workspace.errorText {Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled)}
      if workspace.busy {WorkbenchProgress("正在核对本地记录…")}
      List(workspace.recoverableDocuments) {record in
        HStack(alignment:.top) {
          VStack(alignment:.leading,spacing:4) {
            Text(record.title)
            Text(record.hasDrafts ? "有待确认草稿" : record.fileDirty ? "文件尚未保存" : "本地记录与文件已保存状态一致").font(.caption).foregroundStyle(.secondary)
            if let warning=record.warning {Text(warning).font(.caption).foregroundStyle(.secondary)}
          }
          Spacer()
          Button("恢复") {Task {await workspace.recover(record.id);if workspace.session?.drafts.confirmed.document_id==record.id {isPresented=false}}}.disabled(!record.recoverable || workspace.busy).accessibilityIdentifier("restore-document-"+record.id)
        }.padding(.vertical,4)
      }
      HStack {
        Button("刷新") {Task {await workspace.refreshRecoverableDocuments()}}.disabled(workspace.busy)
        if workspace.hasMoreRecoveryDocuments {Button("查看更多") {Task {await workspace.refreshRecoverableDocuments(more:true)}}.disabled(workspace.busy)}
        Spacer();Button("关闭") {isPresented=false}.keyboardShortcut(.cancelAction).disabled(workspace.busy)
      }
    }.padding(20).frame(width:600,height:420)
  }
}
