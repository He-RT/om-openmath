import Foundation

@main struct SourceFeedbackFixtures {
  static func main() throws {
    let cells:[NativeSourceCell]=[
      .init(id:"a",kind:.math,source:"let a=2",dialect:.modern),
      .init(id:"b",kind:.math,source:"a+1",dialect:.modern),
      .init(id:"c",kind:.math,source:"9+1",dialect:.modern),
      .init(id:"text",kind:.text,source:"hello",dialect:.modern)]
    let graph=NotebookSourceAnalysis(cells:[
      .init(cell_id:"a",defines:["a"],uses:[],diagnostics:[]),
      .init(cell_id:"b",defines:[],uses:["a"],diagnostics:[]),
      .init(cell_id:"c",defines:[],uses:[],diagnostics:[])],edges:[["a","b"]])
    let snapshot=NativeSourceSnapshot(codec_version:1,document_id:"doc",revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"",cells:cells),cell_revisions:[],snapshot_hash:"hash")
    let baseline=NotebookSourceProjection(source:snapshot,analysis:graph)
    var edited=snapshot;edited.file.cells[0].source="let a=5"
    let after=NotebookSourceProjection(source:edited,analysis:graph)
    let stale=after.staleReasons(since:baseline)
    precondition(stale["a"]=="源码已修改" && stale["b"]=="依赖的定义 a 已改变" && stale["c"]==nil)
    precondition(baseline.staleReasons(since:baseline).isEmpty,"reverting source remained stale")
    var prose=snapshot;prose.file.cells[3].source="new prose";prose.file.title="new title"
    precondition(NotebookSourceProjection(source:prose,analysis:graph).staleReasons(since:baseline).isEmpty)
    var reordered=snapshot;reordered.file.cells.swapAt(0,1)
    precondition(NotebookSourceProjection(source:reordered,analysis:graph).staleReasons(since:baseline)["c"] != nil)
    let unicode="中文🙂\nlet f(x) ="
    let map=try SourceIndexMap(source:unicode,draftSequence:0)
    let position=try map.position(forUTF8:unicode.utf8.count)
    precondition(position.line==2 && position.column==11)
    let native=try map.nativeRange(start:0,end:10)
    precondition(native.length==4,"UTF8 diagnostic converted using byte count as UTF16 length")
    print("Source feedback: dependency changes, unrelated cells, reverting, prose/title isolation, order and Unicode positions passed")
  }
}
