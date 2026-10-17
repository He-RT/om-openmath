import Foundation

/// Display an accepted preview from the original tool input and hash-verified read snapshot.
/// This formats the proposal only; it creates no plan, receipt, evaluation, or document mutation.
enum AgentPreviewFormatter {
  static func make(arguments:[String:Any],result:[String:Any],snapshot:[String:Any]?)->AgentPreviewDisplay? {
    guard result["valid"] as? Bool==true,let input=arguments["input"] as? [String:Any] else {return nil}
    if input["kind"] as? String=="source",let source=input["source"] as? String {
      return .init(cells:[.init(id:"source",title:"源码预览",before:nil,after:source,notes:[])],notes:["源码检查，尚未修改笔记本"])
    }
    guard input["kind"] as? String=="patch",let operations=input["operations"] as? [[String:Any]] else {return nil}
    var originals:[String:String]=[:],sources:[String:String]=[:],changes:[String:AgentPreviewDisplay.Cell]=[:],order:[String]=[],notes=["尚未提交的预览"]
    for cell in snapshot?["cells"] as? [[String:Any]] ?? [] {
      guard cell["source_complete"] as? Bool==true,let id=cell["cell_id"] as? String,let source=cell["source"] as? String,
        let hash=cell["source_hash"] as? String,KernelHashes.data(Data(source.utf8))==hash else {continue}
      originals[id]=source;sources[id]=source
    }
    let assigned=result["assigned_cell_ids"] as? [String:String] ?? [:]
    func target(_ value:Any?)->String? {
      guard let selector=value as? [String:Any] else {return nil}
      return selector["cell_id"] as? String ?? (selector["client_key"] as? String).flatMap{assigned[$0]}
    }
    func put(_ id:String,title:String,after:String?,extra:[String]=[]) {
      if !order.contains(id) {order.append(id)}
      changes[id] = .init(id:id,title:title,before:originals[id],after:after,notes:(changes[id]?.notes ?? [])+extra)
    }
    for (index,operation) in operations.enumerated() {
      guard let type=operation["type"] as? String else {return nil}
      if type=="rename_notebook",let title=operation["title"] as? String {
        notes.append("标题：\(snapshot?["title"] as? String ?? "原标题") → \(title)");continue
      }
      if type=="insert_cell",let key=operation["client_key"] as? String,let id=assigned[key],let source=operation["source"] as? String {
        sources[id]=source;put(id,title:"预览新增单元格",after:source);continue
      }
      guard let id=target(operation["target"]) else {return nil}
      if type=="move_cell" {put(id,title:"预览移动单元格",after:sources[id],extra:["调整单元格位置"]);continue}
      guard let old=originals[id],let expected=operation["expected_source_hash"] as? String,
        KernelHashes.data(Data(old.utf8))==expected else {
        let key="missing-\(index)"
        put(key,title:"预览操作 \(index+1)",after:nil,extra:["原工具记录没有保留完整基线，无法显示这处源码比对"]);continue
      }
      switch type {
      case "update_cell":
        let source=operation["source"] as? String ?? sources[id] ?? old
        var extra:[String]=[]
        if let kind=operation["kind"] as? String {extra.append("类型："+(kind.lowercased()=="math" ? "数学":"文字"))}
        if let dialect=operation["dialect"] as? String {extra.append("语法："+dialect)}
        sources[id]=source;put(id,title:"预览修改单元格",after:source,extra:extra)
      case "delete_cell":sources[id]=nil;put(id,title:"预览删除单元格",after:nil)
      case "replace_text":
        guard let match=operation["match"] as? String,!match.isEmpty,let replacement=operation["replacement"] as? String,
          let current=sources[id],current.components(separatedBy:match).count==2 else {return nil}
        let source=current.replacingOccurrences(of:match,with:replacement)
        sources[id]=source;put(id,title:"预览修改单元格",after:source)
      default:return nil
      }
    }
    return .init(cells:order.compactMap{changes[$0]},notes:notes)
  }
}
