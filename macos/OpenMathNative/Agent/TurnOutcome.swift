import Foundation

enum TurnBudgetKind:String,Codable,Sendable {case steps,time,context}
enum TurnEndReason:Equatable,Codable,Sendable {
  case modelFinished,stopped,failed(String),budget(TurnBudgetKind)
}
struct TurnChangedCell:Equatable,Codable,Sendable {
  enum Kind:String,Codable,Sendable {case new,edited,deleted}
  let cellID:String,kind:Kind,transactionRef:String
}
struct TurnRanCell:Equatable,Codable,Sendable {
  let cellID:String,status:String,sourceHash:String?
  /// nil means the durable receipt exists but its diagnostics have not been inspected.
  let diagnosticCount:Int?
}
struct ProductFact:Equatable,Codable,Sendable {
  enum Kind:String,Codable,Sendable {case expression,solutions,plot,scene3d}
  let cellID:String,resultRef:String,sourceHash:String,occurrence:String
  let kind:Kind,count:Int?,values:[String],verified:Bool
}
enum TurnFact:Sendable {
  case changed([TurnChangedCell],invalidated:[String],deleted:[String])
  case ran(TurnRanCell)
  case product(ProductFact)
  case request(String)
}
/// A projection of committed host facts. Assistant prose is deliberately not an input.
struct TurnOutcome:Equatable,Codable,Sendable,Identifiable {
  let id:String,endReason:TurnEndReason
  let changedCells:[TurnChangedCell],ranCells:[TurnRanCell],products:[ProductFact],staleAfterEdit:[String]
  let saved:Bool,durationMs:Int,modelRequests:Int
  var factsUnavailable=false

  static func assemble(task:String,reason:TurnEndReason,facts:[TurnFact],saved:Bool,durationMs:Int)->Self {
    var changes:[TurnChangedCell]=[],runs:[TurnRanCell]=[],products:[ProductFact]=[]
    var stale=Set<String>(),requests=Set<String>()
    for fact in facts {
      switch fact {
      case .changed(let cells,let invalidated,let deleted):
        changes+=cells;stale.formUnion(invalidated);stale.subtract(deleted)
        let affected=Set(invalidated+deleted)
        products.removeAll{affected.contains($0.cellID)}
      case .ran(let run):
        runs.removeAll{$0.cellID==run.cellID};runs.append(run)
        stale.remove(run.cellID)
        // An inspection of an earlier run cannot describe a subsequent run of the same cell.
        products.removeAll{$0.cellID==run.cellID}
      case .product(let product):
        guard !stale.contains(product.cellID) else {continue}
        products.removeAll{$0.occurrence==product.occurrence};products.append(product)
      case .request(let id):requests.insert(id)
      }
    }
    return .init(id:task,endReason:reason,changedCells:changes,ranCells:runs,products:products,
      staleAfterEdit:stale.sorted(),saved:saved,durationMs:max(0,durationMs),modelRequests:requests.count)
  }
}

enum TurnProductParser {
  /// Parse only a successful, current notebook inspection. Data-only geometry proves generation,
  /// never that the model saw a rendered image. A page cannot prove all solutions were verified.
  static func parse(_ bytes:Data)->ProductFact? {
    guard let root=try? JSONSerialization.jsonObject(with:bytes) as? [String:Any],root["ok"] as? Bool==true,
      let data=root["data"] as? [String:Any],data["origin"] as? String=="notebook",data["is_current"] as? Bool==true,
      let payload=data["payload"] as? [String:Any],let binding=payload["binding"] as? [String:Any],
      let cell=binding["cell_id"] as? String,let hash=binding["source_hash"] as? String,
      let inspectedRef=binding["result_ref"] as? String else {return nil}
    let occurrence=cell+":"+String(describing:binding["out_index"] ?? "")+":"+String(describing:binding["view_id"] ?? "")
    let presentation=((payload["presentation"] as? [String:Any])?["presentation"] as? [String:Any]) ?? [:]
    let geometry=(payload["summary"] as? [String:Any]) ?? (payload["geometry"] as? [String:Any]) ?? (presentation["geometry"] as? [String:Any])
    if let geometry,geometry["available"] as? Bool==true,let meta=geometry["metadata"] as? [String:Any] {
      if geometry["kind"] as? String=="scene3d",let meshes=meta["meshes"] as? [[String:Any]] {
        let counts=meshes.compactMap{$0["triangles"] as? Int}
        guard counts.count==meshes.count else {return nil}
        return .init(cellID:cell,resultRef:inspectedRef,sourceHash:hash,occurrence:occurrence,kind:.scene3d,count:counts.reduce(0,+),values:[],verified:false)
      }
      if geometry["kind"] as? String=="plot2d",let curves=meta["curves"] as? [[String:Any]] {
        return .init(cellID:cell,resultRef:inspectedRef,sourceHash:hash,occurrence:occurrence,kind:.plot,count:curves.count,values:[],verified:false)
      }
    }
    if presentation["kind"] as? String=="solutions",let total=presentation["total_solutions"] as? Int,
      let solutions=presentation["solutions"] as? [[String:Any]] {
      let verified=total>0 && solutions.count==total && presentation["offset"] as? Int==0 && solutions.allSatisfy{$0["verified"] as? String=="Exact"}
      let values=solutions.prefix(4).flatMap{($0["bindings"] as? [[String:Any]] ?? []).compactMap {b->String? in
        guard let name=b["var"] as? String,let value=b["input_form"] as? String else {return nil};return name+" = "+value
      }}
      return .init(cellID:cell,resultRef:inspectedRef,sourceHash:hash,occurrence:occurrence,kind:.solutions,count:total,values:values,verified:verified)
    }
    if presentation["kind"] as? String=="expression" {
      return .init(cellID:cell,resultRef:inspectedRef,sourceHash:hash,occurrence:occurrence,kind:.expression,count:nil,values:[],verified:false)
    }
    if data["part"] as? String=="value",let source=(payload["page"] as? [String:Any])?["source"] as? [String:Any],let form=source["input_form"] as? String {
      return .init(cellID:cell,resultRef:inspectedRef,sourceHash:hash,occurrence:occurrence,kind:.expression,count:nil,values:[String(form.prefix(160))],verified:false)
    }
    return nil
  }
}
