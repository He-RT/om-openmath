import Foundation

struct SourceDiffLine:Identifiable,Sendable,Equatable {
  enum Kind:String,Sendable {case context,added,removed,omitted}
  let id:Int,kind:Kind,text:String
  var prefix:String {switch kind {case .context:"  ";case .added:"+ ";case .removed:"- ";case .omitted:"  "}}
}
struct SourceCellDiff:Identifiable,Sendable {
  let id:String,title:String,notes:[String],lines:[SourceDiffLine],before:String?,after:String?
}
struct SourceTransactionDiff:Sendable {
  let transaction:String,document:String,cells:[SourceCellDiff],notes:[String]
  static func make(preview:AgentPreviewDisplay,identity:String)->Self {
    .init(transaction:identity,document:"",cells:preview.cells.map{
      .init(id:$0.id,title:$0.title,notes:$0.notes,lines:lines(before:$0.before,after:$0.after),before:$0.before,after:$0.after)
    },notes:preview.notes)
  }
  static func make(_ value:NativeStoredSourceTransaction)->Self {
    let before=value.plan.before.file,after=value.plan.after.file
    let old=Dictionary(uniqueKeysWithValues:before.cells.enumerated().map{($0.element.id,($0.offset,$0.element))})
    let now=Dictionary(uniqueKeysWithValues:after.cells.enumerated().map{($0.element.id,($0.offset,$0.element))})
    let ids=before.cells.map(\.id)+after.cells.filter{old[$0.id]==nil}.map(\.id)
    var cells:[SourceCellDiff]=[]
    for id in ids {
      let a=old[id],b=now[id]
      guard a?.1.source != b?.1.source || a?.1.kind != b?.1.kind || a?.1.dialect != b?.1.dialect || a?.0 != b?.0 else {continue}
      let kind=a==nil ? "新增":b==nil ? "删除":"修改"
      var notes:[String]=[]
      if let a,let b {
        if a.0 != b.0 {notes.append("从单元格 \(a.0+1) 移到 \(b.0+1)")}
        if a.1.kind != b.1.kind {notes.append("单元格类型已改变")}
        if a.1.dialect != b.1.dialect {notes.append("源码语法已改变")}
      }
      cells.append(.init(id:id,title:"单元格 \((b?.0 ?? a!.0)+1) · "+kind,notes:notes,lines:lines(before:a?.1.source,after:b?.1.source),before:a?.1.source,after:b?.1.source))
    }
    let notes=before.title==after.title ? []:["标题：\(before.title) → \(after.title)"]
    return .init(transaction:value.transaction_id,document:value.plan.commit.document_id,cells:cells,notes:notes)
  }
  static func lines(before:String?,after:String?)->[SourceDiffLine] {
    let a=before.map{$0.components(separatedBy:"\n")} ?? [],b=after.map{$0.components(separatedBy:"\n")} ?? []
    var output:[(SourceDiffLine.Kind,String)]=[],truncated=false
    func append<S:Sequence>(_ values:S,_ kind:SourceDiffLine.Kind) where S.Element==String {
      for value in values {if output.count>=10_000 {truncated=true;break};output.append((kind,value))}
    }
    if a.count+b.count<=4000 && a.count*b.count<=1_000_000 {
      let difference=b.difference(from:a)
      var removed=Set<Int>(),added=Set<Int>()
      for change in difference {switch change {case .remove(let i,_,_):removed.insert(i);case .insert(let i,_,_):added.insert(i)}}
      var i=0,j=0
      while i<a.count || j<b.count {
        if i<a.count,removed.contains(i) {output.append((.removed,a[i]));i+=1}
        else if j<b.count,added.contains(j) {output.append((.added,b[j]));j+=1}
        else if i<a.count,j<b.count {output.append((.context,a[i]));i+=1;j+=1}
        else {break}
      }
    } else {
      var start=0,end=0
      while start<min(a.count,b.count),a[start]==b[start] {start+=1}
      while end<min(a.count,b.count)-start,a[a.count-1-end]==b[b.count-1-end] {end+=1}
      if start>3 {output.append((.omitted,"…"))}
      append(a[max(0,start-3)..<start],.context)
      append(a[start..<(a.count-end)],.removed)
      append(b[start..<(b.count-end)],.added)
      append(a[(a.count-end)..<min(a.count,a.count-end+3)],.context)
    }
    var visible=Set<Int>()
    for i in output.indices where output[i].0 != .context {
      visible.formUnion(max(0,i-3)...min(output.count-1,i+3))
    }
    var result:[SourceDiffLine]=[],omitted=false
    for i in output.indices {
      if visible.contains(i) {result.append(.init(id:i,kind:output[i].0,text:output[i].1));omitted=false}
      else if !omitted {result.append(.init(id:i,kind:.omitted,text:"…"));omitted=true}
    }
    if truncated {result.append(.init(id:output.count,kind:.omitted,text:"后续差异未展开；可复制完整源码"))}
    return result
  }
}
