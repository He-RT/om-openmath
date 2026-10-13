import AppKit
import SwiftUI
import Foundation

private actor PresenterDeliveryPause {
  private var reached=false,released=false
  private var continuation:CheckedContinuation<Void,Never>?
  func pause()async {reached=true;if !released {await withCheckedContinuation {continuation=$0}}}
  func wait()async throws {
    let until=ContinuousClock.now.advanced(by:.seconds(10))
    while !reached {precondition(ContinuousClock.now<until);try await Task.sleep(for:.milliseconds(5))}
  }
  func release() {released=true;continuation?.resume();continuation=nil}
}
@main struct ResultPresenterFixtures {
  @MainActor static func check(_ value:@autoclosure ()->Bool,_ message:String="result presenter assertion",line:UInt=#line) {
    guard value() else {FileHandle.standardError.write(Data(("Result presenter check failed at line \(line): \(message)\n").utf8));fatalError(message)}
  }
  @MainActor static func main()async throws {
    NSApplication.shared.setActivationPolicy(.accessory)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    let cells:[NativeSourceCell]=[
      .init(id:"finite",kind:.math,source:"Solve[x^2-5x+6==0,x]",dialect:.wolfram),
      .init(id:"none",kind:.math,source:"Solve[Sqrt[x]==-1,x]",dialect:.wolfram),
      .init(id:"all",kind:.math,source:"Solve[0==0,x]",dialect:.wolfram),
      .init(id:"region",kind:.math,source:"Reduce[x^2<4,x]",dialect:.wolfram),
      .init(id:"conditional",kind:.math,source:"Solve[Exp[x]==2,x]",dialect:.wolfram),
      .init(id:"roots",kind:.math,source:"Solve[x^5-x-1==0,x]",dialect:.wolfram),
      .init(id:"multiple",kind:.math,source:"Solve[(x-1)^20==0,x]",dialect:.wolfram),
      .init(id:"precision",kind:.math,source:"decimal(\"0.1\",precision:80)",dialect:.modern),
      .init(id:"held",kind:.math,source:"Solve[Sin[x]==x,x]",dialect:.wolfram),
      .init(id:"partial",kind:.math,source:"7\npartialLoop:=partialLoop\npartialLoop",dialect:.wolfram),
      .init(id:"failed",kind:.math,source:"let invalid=",dialect:.modern),
      .init(id:"guard",kind:.math,source:"let guard=2; seed_random(42); guard",dialect:.modern)]
    let source=NativeSourceFile(version:1,title:"实际结果展示",cells:cells),file=root.appendingPathComponent("presenter.omnb")
    try JSONEncoder().encode(source).write(to:file)
    let services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    let session=try await NativeDocumentSession.open(storage:services.open(),url:file)
    func run(_ cell:String)async throws->NotebookCellResult {
      session.runtime.run(cell)
      let until=ContinuousClock.now.advanced(by:.seconds(30))
      while session.runtime.operation != nil {check(ContinuousClock.now<until,"real run timed out: "+cell);try await Task.sleep(for:.milliseconds(10))}
      guard let actual=session.runtime.results[cell] else {fatalError("no actual result: "+cell+session.runtime.message)}
      return actual
    }
    var records:[[String:Any]]=[],owners:[String:ResultPresenter]=[:]
    for cell in cells {
      let result=try await run(cell.id)
      guard let binding=result.bindings.last else {fatalError("actual manifest unavailable: "+cell.id)}
      let owner=session.runtime.presenter(for:binding);owners[cell.id]=owner
      let before=try await session.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
      await owner.load();check(owner.summary != nil && owner.presentation != nil,"real typed projection not loaded: "+cell.id+" "+(owner.message ?? ""))
      let after=try await session.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
      check(before.kernel_state_revision.value==after.kernel_state_revision.value && before.source_revision.value==after.source_revision.value,"presentation executed math")
      let reader=ResultClient(host:session.host);try reader.select(binding)
      let raw=try await reader.inspect(.inspect_result_presentation(.init(kind:.presentation,offset:0,limit:16)))!
      let object=try JSONSerialization.jsonObject(with:JSONEncoder().encode(raw.payload))
      records.append(["cell":cell.id,"input":cell.source,"operation":result.receipt.operation_id,"terminal":result.receipt.producer.terminal_status.rawValue,"presentation":object])
      await reader.close()
    }
    try JSONSerialization.data(withJSONObject:records,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("actual-result-presentations.json"))
    let partial=session.runtime.results["partial"]!
    FileHandle.standardOutput.write(Data("Actual partial: terminal=\(partial.receipt.producer.terminal_status.rawValue), successful=\(partial.receipt.producer.successful_statements.value), freshness=\(owners["partial"]!.binding.freshness.rawValue), acceptance=\(owners["partial"]!.binding.acceptance.rawValue), source=\(partial.sources)\n".utf8))
    check(owners["finite"]!.presentation!.solutions!.solution_kind == .finite)
    check(owners["none"]!.presentation!.solutions!.solution_kind == .none)
    check(owners["all"]!.presentation!.solutions!.solution_kind == .all)
    check(owners["region"]!.presentation!.solutions!.solution_kind == .region)
    check(!owners["region"]!.presentation!.solutions!.intervals.isEmpty)
    check(owners["conditional"]!.presentation!.solutions!.solutions.contains{$0.condition_latex.value != nil})
    let roots=owners["roots"]!,rootPage=roots.presentation!.solutions!
    check(rootPage.solutions.contains{$0.bindings.contains{$0.root_index != nil}})
    let rootValue=rootPage.solutions[0].bindings[0]
    await roots.projectSolution(index:0,binding:0,source:rootValue.input_form)
    let rootKey=ResultPresenter.solutionNumericKey(index:0,binding:0,source:rootValue.input_form)
    check(roots.numeric[rootKey] != nil,"real Root numeric projection missing")
    check(roots.presentation!.solutions!.solutions[0].bindings[0].input_form.utf8.elementsEqual(rootValue.input_form.utf8))
    let multiple=owners["multiple"]!;check(multiple.presentation!.solutions!.groups.count==1 && multiple.presentation!.solutions!.groups[0].multiplicity==20)
    await multiple.load(offset:16)
    check(multiple.presentation!.solutions!.groups.count==1 && multiple.presentation!.solutions!.groups[0].multiplicity==20 && multiple.presentation!.solutions!.groups[0].firstIndex==0)
    let precision=owners["precision"]!;check(precision.summary!.statement.value?.nature=="high_precision")
    await precision.projectValue(digits:80);check(precision.numeric[Data("value/80".utf8)] != nil)
    check(session.runtime.results["precision"]!.values[0].source.contains("`"))
    check(owners["held"]!.presentation!.solutions==nil,"unevaluated expression became no solutions")
    check(session.runtime.results["partial"]!.receipt.producer.successful_statements.value>0 && session.runtime.results["partial"]!.receipt.producer.terminal_status == .error && owners["partial"]!.binding.freshness == .partial,"completed prefix was discarded or marked complete")
    check(owners["failed"]!.presentation!.kind=="error" && owners["failed"]!.binding.freshness == .partial)

    // Pure grouping negative: same value/pretty label, distinct actual conditions and raw bytes.
    let conditionJSON=#"{"kind":"solutions","solution_kind":"finite","vars":["x"],"region_latex":null,"total_solutions":2,"total_intervals":0,"offset":0,"solution_multiplicities":[1,1],"solution_first_indices":[0,1],"intervals":[],"solutions":[{"bindings":[{"var":"x","latex":"1","input_form":"1","modern_form":"1","numeric":null}],"condition_latex":"a>0","verified":"Exact"},{"bindings":[{"var":"x","latex":"1","input_form":"1","modern_form":"1","numeric":null}],"condition_latex":"a<0","verified":"Exact"}]}"#
    let different=try JSONDecoder().decode(NativeSolutionPage.self,from:Data(conditionJSON.utf8));check(different.groups.count==2,"different conditions merged by display equality")

    let guardResult=session.runtime.results["guard"]!,guardBinding=guardResult.bindings.last!
    let inspect=ResultClient(host:session.host);try inspect.select(guardBinding)
    func readonly(_ text:String)async throws->NativeExpressionProjection {
      let reply=try await inspect.inspect(.inspect_result_expression(.init(kind:.readonly_expression,source:text,numeric:false)))!
      return try JSONDecoder().decode(NativeExpressionProjection.self,from:JSONEncoder().encode(reply.payload))
    }
    let before=try await session.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
    let random1=try await readonly("RandomUniform[]"),out1=try await readonly("Out[\(guardBinding.out_index.value)]")
    await roots.projectSolution(index:0,binding:0,source:rootValue.input_form)
    await precision.projectValue(digits:80)
    let random2=try await readonly("RandomUniform[]"),out2=try await readonly("Out[\(guardBinding.out_index.value)]"),guardValue=try await readonly("guard")
    let after=try await session.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
    check(random1.input_form.utf8.elementsEqual(random2.input_form.utf8) && out1.input_form.utf8.elementsEqual(out2.input_form.utf8) && guardValue.input_form=="2")
    check(before.kernel_state_revision.value==after.kernel_state_revision.value && before.source_revision.value==after.source_revision.value,"numeric inspect changed main state")
    await inspect.close()

    // Actual decoded reply is withheld while another main result commits, not a mock bool.
    let pause=PresenterDeliveryPause()
    let late=ResultPresenter(host:session.host,binding:guardBinding,probes:.init(beforeDelivery:{reply in if reply.binding.value != nil {await pause.pause()}}))
    let delayed=Task {await late.load()};try await pause.wait()
    let editor=try session.editor("guard")
    editor.textView.setSelectedRange(NSRange(location:0,length:(editor.textView.string as NSString).length));editor.textView.insertText("let guard=9; guard",replacementRange:editor.textView.selectedRange())
    check(session.drafts.overlay("guard")!.source=="let guard=9; guard")
    _ = try await run("guard");await pause.release();await delayed.value
    check(late.presentation==nil && late.summary==nil && late.message != nil,"outdated real reply published")
    await late.close()
    check(session.runtime.history.contains{$0.receipt.operation_id==guardResult.receipt.operation_id} && session.runtime.history.allSatisfy{$0.values.isEmpty},"history was lost or duplicated large values")
    await owners["guard"]!.load();check(owners["guard"]!.binding.acceptance == .history_only && owners["guard"]!.binding.freshness == .stale)
    check(session.drafts.confirmed.revision.value==1,"viewing changed source in addition to the explicit native edit")
    try JSONSerialization.data(withJSONObject:records,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("actual-result-presentations.json"))
    try await session.close();try await services.close()
    print("Actual native result presenter: finite/none/all/region/conditional/Root, global 20-fold multiplicity on two pages, raw-condition isolation, 80-digit projection, real stale reply/history and main Out/random/source purity passed")
    print("PRESENTER_EVIDENCE="+root.appendingPathComponent("actual-result-presentations.json").path)
  }
}
