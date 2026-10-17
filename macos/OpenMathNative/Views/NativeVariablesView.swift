import SwiftUI

struct NativeVariablesView:View {
  @ObservedObject var runtime:NotebookRuntime
  let session:NativeDocumentSession,active:Bool
  @State private var selection:String?
  private var refreshIdentity:String {
    "\(active)/\(session.drafts.persistenceRevision)/\(session.drafts.confirmed.snapshot_hash)/\(runtime.projection?.active_checkpoint_id.value ?? "")"
  }
  var body:some View {
    VStack(alignment:.leading,spacing:8) {
      Text("当前计算中的定义，点按定位到单元格").font(.system(size:11.5)).foregroundStyle(.secondary).padding(.horizontal,16).padding(.top,16)
      if runtime.definitions.isEmpty {
        Text(runtime.definitionsBusy ? "正在读取变量…":"当前没有已接纳的定义").font(.callout).foregroundStyle(.secondary).padding(.horizontal,16)
      }
      Table(runtime.definitions,selection:$selection) {
        TableColumn("名称") {item in
          Text(item.name).font(.system(size:12,design:.monospaced)).foregroundStyle(item.stale ? .secondary:.primary)
            .onTapGesture {locate(item)}
        }.width(min:56,ideal:72,max:100)
        TableColumn("值") {item in
          VStack(alignment:.leading,spacing:3) {
            Text(item.error ? "未定义，单元格出错":item.value ?? "无值摘要")
              .font(.system(size:12.5)).foregroundStyle(item.stale ? Theme.Color.ink2:item.error ? Theme.Color.red:Theme.Color.ink)
              .lineLimit(3).help(item.value ?? "未定义，单元格出错")
            if item.stale {Text("已过期").font(.system(size:11)).foregroundStyle(.secondary)}
          }.onTapGesture {locate(item)}
        }.width(min:90,ideal:150)
        TableColumn("来自") {item in
          Text(owner(item)).font(.system(size:12)).foregroundStyle(.secondary).onTapGesture {locate(item)}
        }.width(min:70,ideal:80,max:90)
      }.tableStyle(.inset(alternatesRowBackgrounds:false)).accessibilityIdentifier("workspace-definitions-table")
        .onChange(of:selection) {if let item=runtime.definitions.first(where:{$0.id==selection}) {locate(item)}}
      if let error=runtime.definitionsError {
        HStack {Text(error).font(.caption).foregroundStyle(.secondary);Button("重新读取") {Task {await runtime.refreshDefinitions(session.drafts)}}}
          .padding(.horizontal,16).padding(.bottom,12)
      }
    }.frame(maxWidth:.infinity,maxHeight:.infinity,alignment:.topLeading)
      .task(id:refreshIdentity) {if active {await runtime.refreshDefinitions(session.drafts)}}
  }
  private func owner(_ item:NotebookDefinition)->String {
    session.drafts.confirmed.file.cells.firstIndex{$0.id==item.cell_id}.map{"单元格 \($0+1)"} ?? "已删除"
  }
  private func locate(_ item:NotebookDefinition) {
    if session.drafts.confirmed.file.cells.contains(where:{$0.id==item.cell_id}) {runtime.navigate(to:item.cell_id,force:true)}
  }
}
