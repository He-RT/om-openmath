import Foundation

struct NotebookSourceDiagnostic:Decodable,Sendable {
  struct Span:Decodable,Sendable {let start:UInt32,end:UInt32}
  let span:Span,severity:String,code:String,message:String
}
struct NotebookCellAnalysis:Decodable,Sendable {
  let cell_id:String,defines:[String],uses:[String],diagnostics:[NotebookSourceDiagnostic]
}
struct NotebookSourceAnalysis:Decodable,Sendable {
  let cells:[NotebookCellAnalysis]
  let edges:[[String]]
}
struct NotebookAnalysisReply:Decodable,Sendable {
  let protocol_version:UInt32,document_id:String,generation:HostSerial,base_hash:String
  let analysis:NotebookSourceAnalysis
}
struct NotebookAnalysisInput:Encodable,Sendable {
  let document_id:String,generation:HostSerial,base_hash:String,file:NativeSourceFile
}
struct NotebookSourceProjection:Sendable {
  let source:NativeSourceSnapshot
  let analysis:NotebookSourceAnalysis
  func staleReasons(since before:NotebookSourceProjection)->[String:String] {
    let old=Dictionary(uniqueKeysWithValues:before.source.file.cells.map{($0.id,$0)})
    let now=Dictionary(uniqueKeysWithValues:source.file.cells.map{($0.id,$0)})
    var changed=Set<String>(),reasons:[String:String]=[:]
    for id in Set(old.keys).union(now.keys) {
      let a=old[id],b=now[id]
      if a?.kind == .math || b?.kind == .math {
        if a==nil || b==nil || a?.source.utf8.elementsEqual(b!.source.utf8) != true || a?.dialect != b?.dialect || a?.kind != b?.kind {
          changed.insert(id);reasons[id]="源码已修改"
        }
      }
    }
    let oldOrder=before.source.file.cells.filter{$0.kind == .math}.map(\.id)
    let newOrder=source.file.cells.filter{$0.kind == .math}.map(\.id)
    if oldOrder != newOrder {
      for id in newOrder {changed.insert(id);reasons[id]="数学单元格的顺序已改变"}
    }
    let edges=before.analysis.edges+analysis.edges
    var progress=true
    while progress {
      progress=false
      for edge in edges where edge.count==2 && changed.contains(edge[0]) && !changed.contains(edge[1]) {
        changed.insert(edge[1]);progress=true
        let parent=edge[0]
        let symbols=(before.analysis.cells.first{$0.cell_id==parent}?.defines ?? [])+(analysis.cells.first{$0.cell_id==parent}?.defines ?? [])
        let uses=(before.analysis.cells.first{$0.cell_id==edge[1]}?.uses ?? [])+(analysis.cells.first{$0.cell_id==edge[1]}?.uses ?? [])
        if let symbol=symbols.first(where:{uses.contains($0)}) {reasons[edge[1]]="依赖的定义 "+symbol+" 已改变"}
        else {reasons[edge[1]]="依赖的单元格已改变"}
      }
    }
    return reasons
  }
}
