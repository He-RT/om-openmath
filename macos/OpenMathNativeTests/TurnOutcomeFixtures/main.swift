import Foundation

@main struct TurnOutcomeFixtures {
  static func main() throws {
    for reason:TurnEndReason in [.modelFinished,.stopped,.failed("认证失败"),.budget(.context),.budget(.time),.budget(.steps)] {
      let outcome=TurnOutcome.assemble(task:"task",reason:reason,facts:[],saved:false,durationMs:250)
      precondition(outcome.endReason==reason && outcome.changedCells.isEmpty && outcome.ranCells.isEmpty)
      precondition(UIStrings.end(reason) != "已完成" && !UIStrings.end(reason).contains("任务已结束"))
    }
    precondition(UIStrings.end(.modelFinished)=="回合结束")
    precondition(UIStrings.end(.stopped)=="已停止，已完成的修改保留")
    let change=TurnChangedCell(cellID:"a",kind:.edited,transactionRef:"tx-1")
    let run=TurnRanCell(cellID:"b",status:"done",sourceHash:"source",diagnosticCount:0)
    let product=ProductFact(cellID:"b",resultRef:"r",sourceHash:"source",occurrence:"b:1:v",kind:.solutions,count:2,values:["x = 2","x = 3"],verified:true)
    let facts:[TurnFact]=[.request("request-1"),.request("request-1"),.changed([change],invalidated:["a","b"],deleted:[]),.ran(run),.product(product)]
    let outcome=TurnOutcome.assemble(task:"t",reason:.modelFinished,facts:facts,saved:false,durationMs:10)
    precondition(outcome.changedCells==[change] && outcome.ranCells==[run] && outcome.products==[product])
    precondition(outcome.staleAfterEdit==["a"] && outcome.modelRequests==1 && !outcome.saved)
    // A later edit invalidates the old inspection, even if the model says the task was done.
    let later=TurnOutcome.assemble(task:"t",reason:.modelFinished,facts:facts+[.changed([],invalidated:["b"],deleted:[])],saved:true,durationMs:10)
    precondition(later.products.isEmpty && later.staleAfterEdit==["a","b"] && later.saved)
    let deleted=TurnOutcome.assemble(task:"t",reason:.stopped,facts:facts+[.changed([],invalidated:[],deleted:["a","b"])],saved:false,durationMs:10)
    precondition(deleted.staleAfterEdit.isEmpty && deleted.products.isEmpty && deleted.changedCells==[change])
    let rerun=TurnOutcome.assemble(task:"t",reason:.modelFinished,facts:facts+[.ran(run)],saved:false,durationMs:10)
    precondition(rerun.products.isEmpty && rerun.ranCells.count==1)
    let readonly=TurnOutcome.assemble(task:"t",reason:.modelFinished,facts:[.request("read")],saved:false,durationMs:1)
    precondition(readonly.changedCells.isEmpty && readonly.ranCells.isEmpty && readonly.staleAfterEdit.isEmpty)

    let binding:[String:Any]=["cell_id":"b","source_hash":"source","out_index":1,"view_id":"v","result_ref":"notebook-result"]
    func inspect(_ payload:[String:Any],current:Bool=true,origin:String="notebook",ok:Bool=true)throws->ProductFact? {
      let bytes=try JSONSerialization.data(withJSONObject:["ok":ok,"data":["origin":origin,"is_current":current,"payload":payload]])
      return TurnProductParser.parse(bytes)
    }
    let geometry:[String:Any]=["binding":binding,"summary":["kind":"scene3d","available":true,"metadata":["meshes":[["triangles":12],["triangles":1174]]]]]
    let scene=try inspect(geometry)
    precondition(scene?.kind == .scene3d && scene?.count==1186 && scene?.verified==false)
    let staleScene=try inspect(geometry,current:false),scratchScene=try inspect(geometry,origin:"scratch"),failedScene=try inspect(geometry,ok:false)
    precondition(staleScene==nil && scratchScene==nil && failedScene==nil)
    let plot=try inspect(["binding":binding,"summary":["kind":"plot2d","available":true,"metadata":["curves":[["label":"sin(x)"],["label":"cos(x)"]]]]])
    precondition(plot?.kind == .plot && plot?.count==2 && plot?.resultRef=="notebook-result")
    let row:[String:Any]=["bindings":[["var":"x","input_form":"2"]],"verified":"Exact"]
    func solutions(_ total:Int,_ rows:[[String:Any]])throws->ProductFact? {
      try inspect(["binding":binding,"presentation":["presentation":["kind":"solutions","total_solutions":total,"offset":0,"solutions":rows]]])
    }
    let complete=try solutions(1,[row]),partial=try solutions(40,[row])
    precondition(complete?.verified==true && complete?.values==["x = 2"])
    precondition(partial?.verified==false && partial?.count==40)
    let unavailable=try inspect(["binding":binding,"summary":["kind":"scene3d","available":false,"metadata":[:]]])
    precondition(unavailable==nil)
    let encoded=try JSONEncoder().encode(outcome)
    let decoded=try JSONDecoder().decode(TurnOutcome.self,from:encoded)
    precondition(decoded==outcome)
    print("turn outcome fixtures passed: end reasons, durable-fact ordering, stale edits, partial inspections, geometry, request count")
  }
}
