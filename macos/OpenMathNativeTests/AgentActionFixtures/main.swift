import Foundation

@main struct AgentActionFixtures {
  static func main() throws {
    let order=["stable-a":1,"stable-b":2]
    func action(_ tool:String="read_notebook",call:String="call",reply:[String:Any]?=nil,args:[String:Any]=[:],changes:[String:TurnChangedCell.Kind]=[:])throws->AgentAction {
      AgentActionFormatter.make(turn:"turn",call:call,tool:tool,arguments:try JSONSerialization.data(withJSONObject:args),
        reply:try reply.map{try JSONSerialization.data(withJSONObject:$0)},cellOrder:order,changes:changes)
    }
    let pending=try action(),read=try action(reply:["ok":true,"status":"completed","data":["cells":[["cell_id":"stable-a"],["cell_id":"stable-b"]]]])
    precondition(pending.id==read.id && pending.status == .running && read.status == .ok)
    precondition(read.object=="2 个单元格" && read.cells==["stable-a","stable-b"])
    let failed=try action("preview_source",reply:["ok":false,"status":"failed","data":NSNull(),"error":["code":"INVALID_ARGUMENT","message":"缺少 cell_ids","recovery":"fix_arguments"]])
    precondition(failed.status == .failed && failed.error?.code=="INVALID_ARGUMENT" && failed.error?.message=="缺少 cell_ids")
    precondition(failed.error?.recovery=="按错误信息补齐参数后重试")
    let unknown=try action("apply_notebook_patch",reply:["ok":false,"status":"unknown","error":["code":"UNKNOWN_OUTCOME","message":"回执待核对","recovery":"query_operation"]])
    precondition(unknown.status == .unknown && unknown.error?.code=="UNKNOWN_OUTCOME")
    let patch=try action("apply_notebook_patch",reply:["ok":true,"status":"completed","data":["transaction_ref":"txn","changed_cell_ids":["stable-b"]]],changes:["stable-b":.new])
    precondition(patch.object=="新增单元格 2" && patch.transactionRef=="txn" && patch.cells==["stable-b"])
    let moved=AgentActionFormatter.make(turn:"turn",call:"call",tool:"apply_notebook_patch",arguments:Data("{}".utf8),reply:try JSONSerialization.data(withJSONObject:["ok":true,"status":"completed","data":["changed_cell_ids":["stable-b"]]]),cellOrder:["stable-b":7])
    precondition(moved.cells==["stable-b"] && moved.object=="修改单元格 7")
    let cancelled=try action("run_cells",reply:["ok":false,"status":"cancelled"])
    precondition(cancelled.status == .cancelled)
    let stopped=AgentActionFormatter.make(turn:"t",call:"c",tool:"run_cells",arguments:Data("{}".utf8),reply:nil,cellOrder:[:],fallback:.cancelled)
    precondition(stopped.status == .cancelled)
    let invalid=try action("preview_source",reply:["ok":true,"status":"completed","data":["valid":false,"diagnostics":[["message":"表达式未完成"]]]])
    precondition(invalid.status == .failed && invalid.error?.message=="表达式未完成")
    let a=try action(call:"read-1",reply:["ok":true]),b=try action("get_function_docs",call:"read-2",reply:["ok":true],args:["name":"scene"]),c=try action("get_function_docs",call:"read-3",reply:["ok":true],args:["name":"disk"])
    let groups=AgentActionGroup.rows([a,b,c,failed,patch])
    precondition(groups.count==3 && groups[0].actions.count==3 && groups[0].summary=="读取笔记本和 2 个函数说明")
    precondition(groups[1].actions==[failed] && groups[2].actions==[patch])
    precondition(AgentActionGroup.rows([a,b]).count==2)
    precondition(AgentActionGroup.rows([a,failed,b,c]).count==4)
    let message=AssistantMessage(id:"turn",taskID:"task",role:"assistant",context:.init(model:"configured-model",mode:"execute"),parts:[.init(id:"t1",value:.text("先读")),.init(id:"a1",value:.actions([a])),.init(id:"t2",value:.text("再修改")),.init(id:"a2",value:.actions([patch]))])
    let decoded=try JSONDecoder().decode(AssistantMessage.self,from:JSONEncoder().encode(message))
    precondition(decoded==message && decoded.parts.map(\.id)==["t1","a1","t2","a2"] && decoded.actions.map(\.callID)==["read-1","call"])
    let first=AssistantMessage(id:"u1",taskID:"task",role:"user",context:nil,parts:[]),second=AssistantMessage(id:"u2",taskID:"later",role:"user",context:nil,parts:[])
    let merged=AssistantMessageProjection.merge(durable:[first,second],partial:[message],pendingInput:nil)
    precondition(merged.map(\.id)==["u1","turn","u2"])
    precondition(AssistantMessageProjection.merge(durable:[first,message,second],partial:[message],pendingInput:nil).count==3)
    print("agent action fixtures passed: pairing identity, errors, unknown/cancelled outcomes, cell identity, transaction refs, consecutive reads, ordered message parts")
  }
}
