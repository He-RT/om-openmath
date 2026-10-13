import AppKit
import SwiftUI
import Foundation

private actor ValueDeliveryPause {
  private var reached=false,released=false
  private var continuation:CheckedContinuation<Void,Never>?
  func pause()async {reached=true;if !released {await withCheckedContinuation {continuation=$0}}}
  func wait()async throws {
    let until=ContinuousClock.now.advanced(by:.seconds(10))
    while !reached {precondition(ContinuousClock.now<until);try await Task.sleep(for:.milliseconds(5))}
  }
  func release() {released=true;continuation?.resume();continuation=nil}
}
@main struct ValuePresenterFixtures {
  @MainActor static func check(_ value:@autoclosure ()->Bool,_ message:String="value presenter assertion",line:UInt=#line) {
    guard value() else {FileHandle.standardError.write(Data("Value presenter failed at \(line): \(message)\n".utf8));fatalError(message)}
  }
  @MainActor static func main()async throws {
    NSApplication.shared.setActivationPolicy(.accessory)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    let big="90071992547409931234567890",long=String(repeating:"中文🙂e\u{301}",count:5000)
    let quotedLong=String(data:try JSONEncoder().encode(long),encoding:.utf8)!
    let matrix="["+(0..<105).map{r in "["+(0..<35).map{c in
      if r==0 {switch c {case 0:return "1/3";case 1:return "decimal(\"0.1\",precision:80)";case 2:return big;case 3:return "\"é\"";case 4:return "\"e\u{301}\"";default:break}}
      return String(r*35+c+1)
    }.joined(separator:",")+"]"}.joined(separator:",")+"]"
    let table="DataTable(["+(0..<10).map{"\"col\($0)\""}.joined(separator:",")+"],["+(0..<40).map{r in "{"+(0..<10).reversed().map{"col\($0):\(r*10+$0)"}.joined(separator:",")+"}"}.joined(separator:",")+"])"
    var deep=big;for _ in 0..<17 {deep="{child:"+deep+"}"}
    let cases:[(String,String)]=[
      ("matrix",matrix),("table",table),("record","{title:\"中文🙂\",nested:[{exact:1/3,high:decimal(\"0.1\",precision:80)},[1,2]],flag:false,empty:null}"),
      ("large_record","{"+(0..<37).map{"field\($0):\($0)"}.joined(separator:",")+"}"),
      ("long","{text:"+quotedLong+"}"),("oversized_page","["+quotedLong+","+quotedLong+"]"),("deep",deep),("empty_list","[]"),("empty_record","{}"),
      ("empty_table","parse_csv(\"x,y\")"),("list","let localGuard=0; [1/3,\"中文🙂\",true,null,hold(assign(localGuard,99))]"),
      ("science","let actual=optimize((x-2)^2,x,scope:\"global\")"),("cached","actual"),
      ("fake","{converged:true,guarantee:\"certified_global\",point:[99]}"),
      ("wrapped","[optimize((x-2)^2,x,scope:\"global\")]"),
      ("fit","fit([[0,1],[1,3],[2,5]],model:a*x+b,parameters:{a:1,b:0})"),
      ("interpolation","interpolate([[0,0],[1,2],[2,4]])"),
      ("series","series(exp(x),x,order:4)"),("quantity","quantity(1/3,unit:\"m/s\")"),
      ("ode","ode(fn(t,y)=>y,initial:1,t:0..1)"),
      ("guard","let guard=2; seed_random(42); guard")]
    let cells=cases.filter{$0.0 != "oversized_page"}.map{NativeSourceCell(id:$0.0,kind:.math,source:$0.1,dialect:.modern)}
    let file=root.appendingPathComponent("values.omnb")
    try JSONEncoder().encode(NativeSourceFile(version:1,title:"结构值真实验收",cells:cells)).write(to:file)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let session=try await NativeDocumentSession.open(storage:services.open(),url:file)
    func run(_ cell:String)async throws->NotebookCellResult {
      session.runtime.run(cell)
      let until=ContinuousClock.now.advanced(by:.seconds(30))
      while session.runtime.operation != nil {check(ContinuousClock.now<until,"actual run timeout: "+cell);try await Task.sleep(for:.milliseconds(10))}
      guard let result=session.runtime.results[cell] else {fatalError("no actual result: "+cell+" "+session.runtime.message)};return result
    }
    let inputs=cases.map{["cell":$0.0,"input":$0.1,"dialect":"modern"]}
    try JSONSerialization.data(withJSONObject:inputs,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("actual-value-inputs.json"))
    var owners:[String:ResultPresenter]=[:],records:[[String:Any]]=[]
    func record(_ cell:String,_ label:String,_ page:NativeValuePage)throws {
      let actual=try JSONSerialization.jsonObject(with:JSONEncoder().encode(page))
      records.append(["cell":cell,"case":label,"page":actual])
      try JSONSerialization.data(withJSONObject:records,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("actual-value-pages.json"))
    }
    for cell in cells {
      let result=try await run(cell.id),binding=result.bindings.last!
      check(result.receipt.producer.terminal_status == .done,"actual math failed: "+cell.id+" "+result.sources.joined())
      let owner=session.runtime.presenter(for:binding);owners[cell.id]=owner
      if cell.id != "guard" {check(owner.valuePage != nil,"initial structured page missing: "+cell.id+" "+(owner.message ?? ""));try record(cell.id,"initial",owner.valuePage!)}
    }
    let matrixOwner=owners["matrix"]!,first=matrixOwner.valuePage!
    check(first.kind == .matrix && first.rows.count==32 && first.columns.count==8 && first.row_count==105 && first.column_count==35)
    check(first.rows[0].cells[0].source?.input_form=="1/3" && first.rows[0].cells[0].nature == .exact)
    check(first.rows[0].cells[1].nature == .high_precision && first.rows[0].cells[1].source!.input_form.contains("`"))
    check(first.rows[0].cells[2].source!.input_form==big,"big exact integer rounded")
    check(Data(first.rows[0].cells[3].source!.input_form.utf8) != Data(first.rows[0].cells[4].source!.input_form.utf8),"Unicode source normalized")
    let before=try await session.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
    await matrixOwner.loadValue(path:[],columnOffset:8);let columns=matrixOwner.valuePage!
    check(columns.rows[0].id.utf8.elementsEqual(first.rows[0].id.utf8) && columns.rows[0].cells[0].path==[0,8],"column page: "+(matrixOwner.message ?? "no message"))
    try record("matrix","second columns",columns)
    await matrixOwner.loadValue(path:[],offset:32,columnOffset:32);check(matrixOwner.valuePage!.rows.count==32 && matrixOwner.valuePage!.columns.count==3)
    try record("matrix","row/column boundary",matrixOwner.valuePage!)
    await matrixOwner.loadValue(path:[],offset:96);check(matrixOwner.valuePage!.rows.count==9 && matrixOwner.valuePage!.rows[0].label=="97")
    await matrixOwner.loadValue(path:[],limit:100,columnLimit:32)
    check(matrixOwner.message?.contains("RESULT_PAGE_TOO_LARGE")==true && matrixOwner.valuePage!.offset==96,"oversized actual reply should preserve prior page: "+(matrixOwner.message ?? ""))
    await matrixOwner.loadValue(path:[],limit:100,columnLimit:1);check(matrixOwner.valuePage!.rows.count==100 && matrixOwner.valuePage!.columns.count==1)
    try record("matrix","maximum row bound within byte budget",matrixOwner.valuePage!)
    await matrixOwner.loadValue(path:[],limit:1,columnLimit:32);check(matrixOwner.valuePage!.rows.count==1 && matrixOwner.valuePage!.columns.count==32)
    try record("matrix","maximum column bound within byte budget",matrixOwner.valuePage!)
    await matrixOwner.loadValue(path:[])
    await matrixOwner.openValue(first.rows[0].cells[2],label:"1 / 3")
    check(matrixOwner.valuePage!.path==[0,2] && matrixOwner.valuePage!.source!.input_form==big)
    let exactCopy=await matrixOwner.readValueSource(format:.input_form)
    check(exactCopy==big)
    let pasteboard=NSPasteboard.withUniqueName();defer {pasteboard.releaseGlobally()}
    check(NativeValueClipboard.write(exactCopy!,to:pasteboard) && pasteboard.string(forType:.string)==big,"complete clipboard acknowledgment failed")
    await matrixOwner.backValue();check(matrixOwner.valueTrail.isEmpty && matrixOwner.valuePage!.path.isEmpty)
    let tableOwner=owners["table"]!,tableFirst=tableOwner.valuePage!
    check(tableFirst.kind == .table && tableFirst.columns[0]=="col0" && tableFirst.rows[0].cells[0].source!.input_form=="0")
    check(tableFirst.rows[0].cells[0].path==[1,0,9,1],"record storage order confused with table column order")
    await tableOwner.loadValue(path:[],offset:32,columnOffset:8);check(tableOwner.valuePage!.rows.count==8 && tableOwner.valuePage!.columns==["col8","col9"])
    try record("table","last rows/columns",tableOwner.valuePage!)
    let nested=owners["record"]!,nestedEntry=nested.valuePage!.rows[1].cells[0]
    await nested.openValue(nestedEntry,label:"nested");check(nested.valuePage!.kind == .list)
    let exactRecord=nested.valuePage!.rows[0].cells[0];await nested.openValue(exactRecord,label:"nested / 1")
    check(nested.valuePage!.kind == .record && nested.valuePage!.rows[0].cells[0].source!.input_form=="1/3")
    try record("record","actual nested record",nested.valuePage!)
    await nested.backValue();await nested.backValue();check(nested.valueTrail.isEmpty)
    let large=owners["large_record"]!;await large.loadValue(path:[],offset:32);check(large.valuePage!.rows.count==5 && large.valuePage!.rows[0].label=="field32")
    let longOwner=owners["long"]!,longPage=longOwner.valuePage!,longEntry=longPage.rows[0].cells[0]
    check(longEntry.excerpt.contains("〔摘录〕") && !longPage.excerpt.contains(long))
    check(NativeValueClipboard.write(longPage.excerpt,to:pasteboard) && pasteboard.string(forType:.string)!.hasPrefix("OpenMath 页面摘录"))
    await longOwner.openValue(longEntry,label:"text")
    let full=await longOwner.readValueSource(format:.input_form),modern=await longOwner.readValueSource(format:.modern)
    let decodedLong=try JSONDecoder().decode(String.self,from:Data(full!.utf8))
    check(full != nil && modern != nil && full!.utf8.count>16384 && decodedLong==long)
    check(NativeValueClipboard.write(full!,to:pasteboard) && pasteboard.string(forType:.string)!.utf8.elementsEqual(full!.utf8),"complete fragmented Unicode copy altered")
    let deepOwner=owners["deep"]!,path=Array(repeating:[UInt32(0),UInt32(1)],count:16).flatMap{$0}
    await deepOwner.loadValue(path:path);check(deepOwner.valuePage!.path.count==32 && deepOwner.valuePage!.rows[0].cells[0].path.count==34)
    await deepOwner.loadValue(path:path+[0]);check(deepOwner.valuePage!.path==path && deepOwner.message != nil,"depth guard published invalid path")
    for id in ["empty_list","empty_record","empty_table"] {check(owners[id]!.valuePage!.rows.isEmpty)}
    check(owners["empty_table"]!.valuePage!.columns==["x","y"])
    let science=owners["science"]!,sciencePage=science.valuePage!
    check(sciencePage.authoritative && sciencePage.origin!.function_id=="fn_000179")
    check(sciencePage.fieldLabel("guarantee")=="数学保证")
    let guarantee=sciencePage.rows.first{$0.label=="guarantee"}!.cells[0]
    check(sciencePage.statusLabel(guarantee)=="精确全局认证")
    for id in ["cached","fake","wrapped"] {let page=owners[id]!.valuePage!;check(!page.authoritative && page.origin==nil && page.fieldLabel("converged")=="converged")}
    check(owners["fake"]!.valuePage!.statusLabel(owners["fake"]!.valuePage!.rows[1].cells[0])==nil)
    let point=sciencePage.rows.first{$0.label=="point"}!.cells[0]
    await science.openValue(point,label:"point");check(!science.valuePage!.authoritative && science.valuePage!.fieldLabel("guarantee")=="guarantee")
    let fit=owners["fit"]!,fitPage=fit.valuePage!
    await fit.openValue(fitPage.rows.first{$0.label=="model"}!.cells[0],label:"model");check(fit.valuePage!.kind == .model && fit.valuePage!.source != nil)
    try record("fit","actual model definition",fit.valuePage!)
    await fit.backValue();await fit.openValue(fitPage.rows.first{$0.label=="data"}!.cells[0],label:"data")
    check(fit.valuePage!.kind == .table && fit.valuePage!.row_count==3);try record("fit","actual samples and predictions",fit.valuePage!)
    for (id,kind) in [("interpolation",NativeValueKind.interpolation),("series",.series),("quantity",.quantity)] {
      check(owners[id]!.valuePage!.kind==kind && owners[id]!.valuePage!.source != nil)
      let opaqueSource=await owners[id]!.readValueSource(format:.input_form);check(opaqueSource != nil)
    }

    // Real AppKit data sources, exact displayed strings, stable outline item objects, no sort coercion.
    await matrixOwner.loadValue(path:[])
    let coordinator=NativeValueTableView.Coordinator(),tableView=NSTableView();coordinator.table=tableView
    tableView.dataSource=coordinator;tableView.delegate=coordinator;coordinator.update(matrixOwner.valuePage!,onOpen:{_,_ in})
    check(tableView.numberOfRows==32 && tableView.tableColumns.count==9 && tableView.tableColumns.allSatisfy{$0.sortDescriptorPrototype==nil})
    let bigCell=coordinator.tableView(tableView,viewFor:tableView.tableColumns[3],row:0) as! NativeValueCell
    check(bigCell.text.stringValue==big && bigCell.detail.stringValue=="精确数")
    var openedOutlineValue:NativeValueEntry?
    let hosted=NSHostingView(rootView:NativeRecordOutlineView(page:owners["fake"]!.valuePage!,onOpen:{entry,_ in openedOutlineValue=entry}))
    let window=NSWindow(contentRect:.init(x:0,y:0,width:700,height:350),styleMask:[.titled],backing:.buffered,defer:false)
    window.contentView=hosted;window.orderBack(nil)
    for _ in 0..<10 {try await Task.sleep(for:.milliseconds(10));hosted.layoutSubtreeIfNeeded()}
    func findOutline(_ view:NSView)->NSOutlineView? {if let outline=view as? NSOutlineView {return outline};return view.subviews.lazy.compactMap(findOutline).first}
    let outline=findOutline(hosted)!;check(outline.numberOfRows==3)
    let item=outline.item(atRow:0)!;outline.expandItem(item);check(outline.numberOfRows==4 && outline.isItemExpanded(item))
    hosted.layoutSubtreeIfNeeded();check(outline.numberOfRows==4,"stable outline item lost its expansion")
    outline.selectRowIndexes(IndexSet(integer:1),byExtendingSelection:false)
    check(window.firstResponder===outline,"actual outline selection did not own keyboard focus")
    let key=NSEvent.keyEvent(with:.keyDown,location:.zero,modifierFlags:[],timestamp:0,windowNumber:window.windowNumber,context:nil,characters:"\r",charactersIgnoringModifiers:"\r",isARepeat:false,keyCode:36)!
    outline.keyDown(with:key);check(openedOutlineValue?.path==owners["fake"]!.valuePage!.rows[0].cells[0].path,"actual Return did not open the retained entry")
    window.orderOut(nil)

    // Actual protocol rejects bad pagination/paths/view bindings; inspections do not run mathematics.
    let binding=matrixOwner.original,reader=ResultClient(host:session.host);try reader.select(binding)
    for query in [InspectResultPage(kind:.page,path:[],offset:106,limit:32,column_offset:0,column_limit:8),
      .init(kind:.page,path:[],offset:0,limit:101,column_offset:0,column_limit:8),
      .init(kind:.page,path:[],offset:0,limit:32,column_offset:0,column_limit:33),
      .init(kind:.page,path:Array(repeating:0,count:33),offset:0,limit:32,column_offset:0,column_limit:8),
      .init(kind:.page,path:[999],offset:0,limit:32,column_offset:0,column_limit:8)] {
      do {_ = try await reader.inspect(.inspect_result_page(query));fatalError("bad actual query accepted")} catch {}
    }
    var wrong=binding;wrong.view_id = .init("9999999999999999999");try reader.select(wrong)
    do {_ = try await reader.inspect(.inspect_result_page(.init(kind:.page,path:[],offset:0,limit:32,column_offset:0,column_limit:8)));fatalError("wrong view accepted")} catch {}
    try reader.select(owners["guard"]!.original)
    func readonly(_ text:String)async throws->NativeExpressionProjection {
      let reply=try await reader.inspect(.inspect_result_expression(.init(kind:.readonly_expression,source:text,numeric:false)))!
      return try JSONDecoder().decode(NativeExpressionProjection.self,from:JSONEncoder().encode(reply.payload))
    }
    let random1=try await readonly("RandomUniform[]"),out1=try await readonly("Out[\(owners["guard"]!.original.out_index.value)]")
    await owners["list"]!.openValue(owners["list"]!.valuePage!.rows.last!.cells[0],label:"held")
    let random2=try await readonly("RandomUniform[]"),out2=try await readonly("Out[\(owners["guard"]!.original.out_index.value)]"),guardValue=try await readonly("guard"),heldGuard=try await readonly("localGuard")
    let after=try await session.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
    check(before.kernel_state_revision.value==after.kernel_state_revision.value && before.source_revision.value==after.source_revision.value && guardValue.input_form=="2" && heldGuard.input_form=="0")
    check(random1.input_form.utf8.elementsEqual(random2.input_form.utf8) && out1.input_form.utf8.elementsEqual(out2.input_form.utf8))
    await reader.close()
    let pause=ValueDeliveryPause(),late=ResultPresenter(host:session.host,binding:binding,probes:.init(beforeDelivery:{reply in if reply.binding.value != nil {await pause.pause()}}))
    let pending=Task {await late.loadValue(path:[],offset:32)};try await pause.wait()
    let editor=try session.editor("guard");editor.textView.setSelectedRange(NSRange(location:0,length:(editor.textView.string as NSString).length))
    editor.textView.insertText("let guard=9; guard",replacementRange:editor.textView.selectedRange());_ = try await run("guard")
    await pause.release();await pending.value;check(late.valuePage==nil && late.message != nil,"late real structured reply published")
    await late.close();check(session.drafts.confirmed.revision.value==1)
    try JSONSerialization.data(withJSONObject:records,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("actual-value-pages.json"))
    try await session.close()
    // A separate real document isolates the large-page byte test from the unchanged 64MiB
    // snapshot retention budget; oversized fixtures must not evict earlier paging proofs.
    let oversized=cases.first{$0.0=="oversized_page"}!,budgetFile=root.appendingPathComponent("budget.omnb")
    try JSONEncoder().encode(NativeSourceFile(version:1,title:"实际页面字节边界",cells:[.init(id:oversized.0,kind:.math,source:oversized.1,dialect:.modern)])).write(to:budgetFile)
    let budgetSession=try await NativeDocumentSession.open(storage:services.open(),url:budgetFile)
    budgetSession.runtime.run(oversized.0)
    let budgetUntil=ContinuousClock.now.advanced(by:.seconds(30))
    while budgetSession.runtime.operation != nil {check(ContinuousClock.now<budgetUntil);try await Task.sleep(for:.milliseconds(10))}
    let budgetResult=budgetSession.runtime.results[oversized.0]!,budgetBinding=budgetResult.bindings.last!,budgetOwner=budgetSession.runtime.presenter(for:budgetBinding)
    check(budgetOwner.valuePage==nil && budgetOwner.summary != nil && budgetOwner.presentation != nil && budgetOwner.message?.contains("大小上限")==true,"byte refusal discarded accepted metadata")
    await budgetOwner.readOriginalValue();check(budgetOwner.originalValue != nil && budgetOwner.originalValue!.input_form.utf8.count>100000)
    check(NativeValueClipboard.write(budgetOwner.originalValue!.input_form,to:pasteboard),"oversized page complete source copy failed")
    let budgetReader=ResultClient(host:budgetSession.host);try budgetReader.select(budgetBinding)
    let budgetSummary=try await budgetReader.inspect(.inspect_result_summary(.init(kind:.summary)))!
    records.append(["cell":oversized.0,"case":"actual 256KiB refusal preserves summary and full source in separate real document","document":budgetBinding.document_id,"summary":try JSONSerialization.jsonObject(with:JSONEncoder().encode(budgetSummary.payload)),"full_source_bytes":budgetOwner.originalValue!.input_form.utf8.count])
    await budgetReader.close();try await budgetSession.close();try await services.close()
    try JSONSerialization.data(withJSONObject:records,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("actual-value-pages.json"))
    print("Actual native structured values: 21 complete inputs, 32x8/default, row100/column32 bounds and actual 256KiB reply refusal with accepted metadata/full source, stable pages/columns/nesting/depth32, exact/highprecision/UTF8 copies, real science vs fake/cached/nested, all structured kinds, Table/Outline, invalid scope and late callbacks, Out/random/source purity passed")
    print("VALUE_EVIDENCE="+root.appendingPathComponent("actual-value-pages.json").path)
    print("VALUE_INPUTS="+root.appendingPathComponent("actual-value-inputs.json").path)
  }
}
