import Foundation

/// Presentation metadata only. Reopening a message never restores tools, credentials or grants.
struct AssistantMessageContext:Codable,Equatable,Sendable {
  let model:String,mode:String
  var references:[AssistantCellReference]?=nil
  var attachments:[AssistantAttachment]?=nil
}
struct AgentPreviewDisplay:Codable,Equatable,Sendable {
  struct Cell:Codable,Equatable,Sendable {let id:String,title:String,before:String?,after:String?,notes:[String]}
  let cells:[Cell],notes:[String]
}
struct AgentAction:Identifiable,Codable,Equatable,Sendable {
  enum Status:String,Codable,Sendable {case running,ok,failed,cancelled,unknown}
  struct Failure:Codable,Equatable,Sendable {let code:String,message:String,recovery:String}
  let id:String,turnID:String,callID:String,tool:String,verb:String,object:String
  let status:Status,cells:[String]
  var transactionRef:String?
  let error:Failure?
  var documentID:String?=nil
  var preview:AgentPreviewDisplay?=nil
}
struct AssistantMessage:Identifiable,Codable,Equatable,Sendable {
  struct Part:Identifiable,Codable,Equatable,Sendable {
    enum Value:Codable,Equatable,Sendable {case text(String),actions([AgentAction])}
    let id:String
    var value:Value
  }
  let id:String,taskID:String,role:String,context:AssistantMessageContext?
  var parts:[Part]
  var text:String {parts.compactMap{if case .text(let text)=$0.value {return text};return nil}.joined()}
  var actions:[AgentAction] {parts.flatMap{if case .actions(let actions)=$0.value {return actions};return []}}
}
struct AgentActionGroup:Identifiable,Equatable,Sendable {
  let actions:[AgentAction]
  var id:String {actions[0].id}
  var summary:String {
    let books=actions.filter{$0.tool=="read_notebook"}.count,functions=actions.count-books
    if books>0 && functions>0 {return "读取笔记本和 \(functions) 个函数说明"}
    return books>0 ? "读取笔记本 \(books) 次":"查阅 \(functions) 个函数说明"
  }
  /// Keep all calls and errors as data; only compact consecutive successful reads for display.
  static func rows(_ actions:[AgentAction])->[Self] {
    var rows:[Self]=[],pending:[AgentAction]=[]
    func flush() {
      if pending.count>=3 {rows.append(.init(actions:pending))}
      else {rows+=pending.map{.init(actions:[$0])}}
      pending=[]
    }
    for action in actions {
      if ["read_notebook","search_functions","get_function_docs"].contains(action.tool),action.status == .ok {pending.append(action)}
      else {flush();rows.append(.init(actions:[action]))}
    }
    flush();return rows
  }
  static func summary(_ actions:[AgentAction])->String {
    var parts:[String]=[]
    let edits=actions.filter{$0.tool=="apply_notebook_patch" && $0.status == .ok}.count
    let runs=actions.filter{$0.tool=="run_cells"}.count,failed=actions.filter{$0.status == .failed}.count
    let unknown=actions.filter{$0.status == .unknown}.count
    if edits>0 {parts.append("修改 \(edits) 处")}
    if runs>0 {parts.append("运行 \(runs) 次")}
    if failed>0 {parts.append("\(failed) 次失败")}
    if unknown>0 {parts.append("\(unknown) 次待核对")}
    return "\(actions.count) 个操作"+(parts.isEmpty ? "":"："+parts.joined(separator:"，"))
  }
  static func expanded(_ actions:[AgentAction],streaming:Bool,preference:Bool?)->Bool {
    streaming || actions.contains{$0.status == .running} || (preference ?? (actions.count<=6))
  }
  static func visible(_ actions:[AgentAction],expanded:Bool)->[AgentAction] {
    expanded ? actions:actions.filter{$0.status == .failed || $0.status == .unknown}
  }
}
enum AssistantMessageProjection {
  static func merge(durable:[AssistantMessage],partial:[AssistantMessage],pendingInput:AssistantMessage?)->[AssistantMessage] {
    var output=durable
    for message in partial where !output.contains(where:{$0.id==message.id}) {
      if let last=output.lastIndex(where:{$0.taskID==message.taskID}) {output.insert(message,at:last+1)}
      else {output.append(message)}
    }
    if let pendingInput,!output.contains(where:{$0.id==pendingInput.id}) {output.append(pendingInput)}
    return output
  }
}

enum AgentActionFormatter {
  /// The only tool-to-language mapping. Inputs are a durable proposal and verified host result.
  static func make(turn:String,call:String,tool:String,arguments:Data,reply:Data?,cellOrder:[String:Int],
    changes:[String:TurnChangedCell.Kind]=[:],fallback:AgentAction.Status?=nil)->AgentAction {
    let args=(try? JSONSerialization.jsonObject(with:arguments) as? [String:Any]) ?? [:]
    let root=reply.flatMap{try? JSONSerialization.jsonObject(with:$0) as? [String:Any]} ?? [:]
    let data=root["data"] as? [String:Any] ?? [:],problem=root["error"] as? [String:Any] ?? [:]
    var status:AgentAction.Status = reply==nil ? (fallback ?? .running):root["ok"] as? Bool==true ? .ok:.failed
    let rawStatus=root["status"] as? String
    if rawStatus=="unknown" || problem["code"] as? String=="UNKNOWN_OUTCOME" {status = .unknown}
    else if rawStatus=="cancelled" {status = .cancelled}
    else if rawStatus=="partial" {status = .failed}
    if tool=="preview_source",data["valid"] as? Bool==false {status = .failed}
    if tool=="evaluate_scratch",data["evaluation_status"] as? String=="error" {status = .failed}
    if tool=="get_operation_status",data["outcome_known"] as? Bool==false {status = .unknown}
    var cells=(data["changed_cell_ids"] as? [String]) ?? (data["executed_cell_ids"] as? [String]) ?? (args["cell_ids"] as? [String]) ?? []
    func labels(_ ids:[String])->String {
      ids.map {id in cellOrder[id].map{"单元格 \($0)"} ?? "已删除或当前不可定位的单元格"}.joined(separator:"、")
    }
    var verb="操作",object="笔记本"
    switch tool {
    case "read_notebook":
      verb="读取笔记本"
      if let read=data["cells"] as? [[String:Any]] {
        cells=read.compactMap{$0["cell_id"] as? String}
        let paged=(root["pagination"] as? [String:Any])?["truncated"] as? Bool==true
        object="\(read.count) 个单元格"+(paged ? "（本页）":"")
      } else {object=cells.isEmpty ? "当前笔记本":labels(cells)}
    case "search_functions":verb="查阅函数";object=String((args["query"] as? String ?? "函数目录").prefix(100))
    case "get_function_docs":verb="查阅函数";object=args["name"] as? String ?? data["name"] as? String ?? "函数说明"
    case "preview_source":
      verb="预览修改"
      let input=args["input"] as? [String:Any],ops=input?["operations"] as? [[String:Any]] ?? []
      let inserted=ops.filter{$0["type"] as? String=="insert_cell"}
      if !inserted.isEmpty {
        let math=inserted.allSatisfy{($0["kind"] as? String)?.lowercased()=="math"}
        object="新增 \(inserted.count) 个"+(math ? "数学":"")+"单元格"
      } else {object=ops.isEmpty ? "源码检查":"\(ops.count) 处操作"}
      let diagnostics=data["diagnostics"] as? [Any]
      if let diagnostics {object+=diagnostics.isEmpty ? "，无诊断":"，\(diagnostics.count) 条诊断"}
      cells+=(data["assigned_cell_ids"] as? [String:String] ?? [:]).values.sorted()
    case "apply_notebook_patch","undo_transaction":
      verb=tool=="undo_transaction" ? "撤销修改":"修改笔记本"
      object=cells.isEmpty ? "文稿信息":cells.map {id in
        let prefix=switch changes[id] {case .new?:"新增";case .deleted?:"删除";default:"修改"}
        return prefix+labels([id])
      }.joined(separator:"、")
    case "run_cells":
      verb="运行";object=cells.isEmpty ? "笔记本单元格":labels(cells)
      if let summaries=data["cells"] as? [[String:Any]] {
        let errors=summaries.filter{($0["status"] as? String ?? "").lowercased()=="error"}.count
        if errors>0 {object+="，\(errors) 个出错"}
      }
    case "inspect_result":
      verb="检查结果";object="内核结果"
      let payload=data["payload"] as? [String:Any] ?? [:],binding=payload["binding"] as? [String:Any] ?? [:]
      if let cell=binding["cell_id"] as? String {cells=[cell]}
      if let reply,let product=TurnProductParser.parse(reply) {
        switch product.kind {
        case .scene3d:object="三维场景，\(product.count ?? 0) 个三角形"
        case .plot:object="二维图，\(product.count ?? 0) 条曲线"
        case .solutions:object=product.values.isEmpty ? "\(product.count ?? 0) 个解":"解 "+product.values.joined(separator:"，")
        case .expression:object=product.values.first ?? "表达式结果"
        }
      } else if data["is_current"] as? Bool==false {object="历史结果（已过期）"}
    case "evaluate_scratch":verb="临时计算";object="临时环境"+(data["evaluation_status"] as? String=="error" ? "，有诊断":"")
    case "get_operation_status":
      verb="核对操作"
      let names=["queued":"等待执行","running":"正在执行","completed":"已提交","partial":"部分已提交","failed":"操作失败","cancelled":"已取消","unknown":"结果待核对"]
      object=(data["operation_state"] as? String).flatMap{names[$0]} ?? "原操作回执"
    case "read_attachment":verb="读取附件";object="本条附件"
    case "prepare_attachment":verb="准备附件";object="本条附件"
    default:break
    }
    let error:AgentAction.Failure?
    if status == .failed || status == .unknown {
      let diagnostic=(data["diagnostics"] as? [[String:Any]])?.first
      let message=problem["message"] as? String ?? diagnostic?["message"] as? String ?? (status == .unknown ? "操作结果待核对":"操作未返回有效结果")
      let recovery=problem["recovery"] as? String ?? ""
      let next=switch recovery {case "retry_read","refresh_snapshot":"重新读取后继续";case "fix_arguments":"按错误信息补齐参数后重试";case "query_operation":"核对原操作，避免重复提交";case "reduce_request":"缩小本次请求后重试";default:""}
      error = .init(code:problem["code"] as? String ?? diagnostic?["code"] as? String ?? (status == .unknown ? "UNKNOWN_OUTCOME":"TOOL_FAILED"),message:String(message.prefix(240)),recovery:next)
    } else {error=nil}
    return .init(id:turn+":"+call,turnID:turn,callID:call,tool:tool,verb:verb,object:object,status:status,
      cells:Array(NSOrderedSet(array:cells)) as? [String] ?? cells,transactionRef:data["transaction_ref"] as? String ?? data["undo_transaction_ref"] as? String,error:error)
  }
}
