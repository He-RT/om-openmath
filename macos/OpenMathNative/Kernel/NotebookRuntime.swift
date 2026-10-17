import Foundation
import SwiftUI

enum WorkbenchLayout:String {case three,two,compact
  static func forWidth(_ width:CGFloat)->Self {width>=1042 ? .three : width>=861 ? .two : .compact}
}
enum WorkbenchDestination:String,CaseIterable,Identifiable {
  case document,result,assistant,steps,variables
  var id:String {rawValue}
  var title:String {switch self {case .document:"文档";case .result:"结果";case .assistant:"助手";case .steps:"步骤";case .variables:"变量"}}
}
/// Document-owned navigation survives hiding, popovers and reconstruction of the workspace view.
@MainActor final class WorkbenchNavigation:ObservableObject {
  @Published var outlineVisible=true
  @Published var inspectorVisible=false
  @Published var layout:WorkbenchLayout = .three
  @Published var inspector:WorkbenchDestination = .assistant
  @Published var outlinePopover=false
  @Published var inspectorPopover=false
  func toggleOutline() {
    if layout == .three {outlineVisible.toggle()} else {outlinePopover.toggle()}
  }
  func toggleInspector() {
    if layout == .compact {inspectorPopover.toggle()} else {inspectorVisible.toggle()}
  }
  func show(_ destination:WorkbenchDestination) {
    inspector=destination
    if layout == .compact {inspectorPopover=true} else {inspectorVisible=true}
  }
}

struct NotebookValueResult:Sendable,Identifiable {
  let binding:ResultBinding
  let source:String
  let modern:String?
  let latex:String?
  var id:String {binding.result_id+"/"+String(binding.out_index.value)+"/"+(binding.view_id.value ?? "")}
}
struct NotebookCellResult:Sendable {
  let receipt:NativeKernelReceipt
  let bindings:[ResultBinding]
  let values:[NotebookValueResult]
  var sources:[String] {values.map(\.source)}
}
/// The document owns this model, not a pane's lifetime. Requests/stop retain their original
/// operation ID and accepted results come only from the physical kernel receipt/read channel.
@MainActor final class NotebookRuntime:ObservableObject {
  @Published private(set) var operation:String?
  @Published private(set) var operationCell:String?
  @Published private(set) var cancelPending=false
  @Published private(set) var phase:NativeKernelHostReplyPhase?
  @Published private(set) var message="就绪"
  @Published private(set) var results:[String:NotebookCellResult]=[:]
  @Published private(set) var history:[NotebookCellResult]=[]
  @Published private(set) var historyOmitted:UInt64=0
  @Published private(set) var unknownOperation:String?
  @Published private(set) var projection:NativeKernelHostReply?
  @Published private(set) var projectionError:String?
  @Published var selectedCell:String?
  @Published private(set) var navigationSequence:UInt64=0
  @Published private(set) var agentModifiedCells=Set<String>()
  @Published private(set) var stoppedCells=Set<String>()
  @Published private(set) var durationsMs:[String:Double]=[:]
  @Published private(set) var staleReasons:[String:String]=[:]
  @Published private(set) var sourceDiagnostics:[String:[NotebookSourceDiagnostic]]=[:]
  @Published private(set) var sourceProjection:NotebookSourceProjection?
  private var resultSources:[String:NotebookSourceProjection]=[:]
  private var sourceAnalysisTask:Task<Void,Never>?
  private var sourceAnalysisSerial:UInt64=0
  private var analysisRequestSerial:UInt64=0
  let kernel:KernelCommitPort
  private let host:NativeHostClient
  private var runTask:Task<Void,Never>?
  private var closing=false
  private var unknownCell:String?
  private var presenters:[String:ResultPresenter]=[:]
  var beforeRun:(@MainActor () async throws->Void)?
  var changed:(@MainActor ()->Void)?
  var runObserved:(@MainActor (String)->Void)?
  let navigation=WorkbenchNavigation()
  init(host:NativeHostClient,storage:StorageService,source:DocCommitPort) {
    self.host=host;kernel=KernelCommitPort(client:host,storage:storage,source:source)
  }
  var canRun:Bool {!closing && operation==nil && unknownOperation==nil}
  var canStop:Bool {operation != nil && !cancelPending && phase != .completed && phase != .failed && phase != .discarded}
  func presenter(for binding:ResultBinding)->ResultPresenter {
    if let existing=presenters[binding.result_ref] {return existing}
    let value=ResultPresenter(host:host,binding:binding);presenters[binding.result_ref]=value;return value
  }
  private func accept(_ result:NotebookCellResult,cell:String) {
    if let previous=results[cell],previous.receipt.operation_id != result.receipt.operation_id {
      // History keeps small original receipts/bindings. Complete values are read on demand
      // from the real immutable result; never duplicate every large output in Swift memory.
      history.append(.init(receipt:previous.receipt,bindings:previous.bindings,values:[]))
      if history.count>32 {history.removeFirst();historyOmitted+=1}
    }
    results[cell]=result
  }
  func refreshProjection() async {
    guard !closing else {return}
    do {
      let actual=try await kernel.state()
      guard !closing else {return};projection=actual;projectionError=nil
    } catch {if !closing {projectionError="定义状态暂无法核对；已有计算记录仍保留。"}}
  }
  func navigate(to cell:String?) {guard !closing,cell != selectedCell else {return};selectedCell=cell;navigationSequence+=1}
  func markAgentChanges(changed:[String],deleted:[String]) {
    agentModifiedCells.formUnion(changed);agentModifiedCells.subtract(deleted)
  }
  func markManualEdit(_ cell:String) {if agentModifiedCells.contains(cell) {agentModifiedCells.remove(cell)}}
  func scheduleSourceAnalysis(_ drafts:DraftStore) {
    guard !closing else {return}
    sourceAnalysisSerial+=1;let serial=sourceAnalysisSerial
    sourceAnalysisTask?.cancel()
    sourceAnalysisTask=Task { [weak self] in
      do {try await Task.sleep(for:.milliseconds(150))} catch {return}
      guard let self,!closing,sourceAnalysisSerial==serial else {return}
      await analyzeSourceNow(drafts)
    }
  }
  func analyzeSourceNow(_ drafts:DraftStore) async {
    guard !closing else {return}
    analysisRequestSerial+=1;let requestSerial=analysisRequestSerial
    let confirmed=drafts.confirmed
    var input=confirmed
    for index in input.file.cells.indices {
      if let draft=drafts.overlay(input.file.cells[index].id),!draft.composing {input.file.cells[index].source=draft.source}
    }
    let sequence=drafts.persistenceRevision
    do {
      let reply=try await host.analyzeSource(.init(document_id:confirmed.document_id,generation:try HostSerial(drafts.documentGeneration),base_hash:confirmed.snapshot_hash,file:input.file))
      guard !closing,!Task.isCancelled,requestSerial==analysisRequestSerial,sequence==drafts.persistenceRevision,confirmed.snapshot_hash==drafts.confirmed.snapshot_hash else {return}
      let projection=NotebookSourceProjection(source:input,analysis:reply.analysis)
      sourceProjection=projection
      sourceDiagnostics=Dictionary(uniqueKeysWithValues:reply.analysis.cells.map{($0.cell_id,$0.diagnostics)})
      refreshStaleReasons(projection)
    } catch { /* Existing source/result facts remain intact until another exact analysis is ready. */ }
  }
  private func refreshStaleReasons(_ current:NotebookSourceProjection) {
    var reasons:[String:String]=[:],compared:[String:[String:String]]=[:]
    for (cell,baseline) in resultSources {
      let hash=baseline.source.snapshot_hash
      if compared[hash]==nil {compared[hash]=current.staleReasons(since:baseline)}
      reasons[cell]=compared[hash]?[cell]
    }
    staleReasons=reasons
  }
  func hasResultSource(_ cell:String)->Bool {resultSources[cell] != nil}
  func run(_ cell:String) {
    guard canRun else {return}
    stoppedCells.remove(cell)
    let id="notebook-run-"+UUID().uuidString.lowercased()
    selectedCell=cell;operation=id;operationCell=cell;phase=nil;message="正在提交运行请求";cancelPending=false
    runTask=Task { [weak self] in
      guard let self else {return}
      let polling=Task { [weak self] in
        while !Task.isCancelled {
          guard let self,!closing,operation==id else {return}
          if let actual=try? await kernel.status(id),actual.operation_id.value==id {phase=actual.phase}
          try? await Task.sleep(for:.milliseconds(100))
        }
      }
      defer {polling.cancel();if operation==id {operation=nil;operationCell=nil;cancelPending=false};changed?();runObserved?("finished")}
      do {
        try await beforeRun?()
        let runProjection=sourceProjection
        runObserved?("source_synchronized")
        let started=ContinuousClock.now
        let receipt=try await kernel.runCell(cell,operation:id)
        let elapsed=started.duration(to:.now).components
        durationsMs[receipt.operation_id]=Double(elapsed.seconds)*1000+Double(elapsed.attoseconds)/1e15
        runObserved?("kernel_received")
        guard !closing else {return}
        if let runProjection {
          resultSources[cell]=runProjection
          if let current=sourceProjection {refreshStaleReasons(current)}
        }
        // A late stop cannot overwrite an actual committed result.
        phase = .completed;message=receipt.producer.terminal_status == .done ? "计算结束" : "计算结束，有错误"
        do {try await display(receipt)}
        catch {if !closing {accept(.init(receipt:receipt,bindings:[],values:[]),cell:cell);message="计算已提交，结果暂无法读取："+String(describing:error)}}
        runObserved?("result_read")
      } catch KernelPortError.unknownOutcome {unknownOperation=id;unknownCell=cell;phase = .unknown;message="运行结果待核对"}
      catch KernelPortError.cancelled {if !closing {message="已停止";stoppedCells.insert(cell)}}
      catch KernelPortError.failed("CANCELLED") {if !closing {message="已停止";stoppedCells.insert(cell)}}
      catch {if !closing {message="运行未完成："+String(describing:error)}}
      await refreshProjection()
    }
  }
  private func display(_ receipt:NativeKernelReceipt) async throws {
    guard let cell=receipt.producer.cell_id.value,let root=receipt.result_id.value else {return}
    let reader=ResultClient(host:host)
    var offset:UInt32=0,bindings:[ResultBinding]=[],values:[NotebookValueResult]=[]
    do {
      while true {
        let manifest=try await reader.manifest(root,offset:offset,limit:16)
        for binding in manifest.bindings {
          guard binding.cell_id==cell,binding.operation_id==receipt.operation_id else {throw ResultClientError.invalidReply}
          bindings.append(binding)
          if binding.out_index.value>0 {
            try reader.select(binding)
            guard let source=try await reader.completeSource() else {throw ResultClientError.staleSelection}
            let modern=try? await reader.completeSource(format:.modern)
            let latex=try? await reader.completeSource(format:.latex)
            values.append(.init(binding:binding,source:source,modern:modern,latex:latex))
          }
          // The first visible projection arrives with its accepted output, so an inspector
          // switch cannot accidentally coincide with a delayed type/solution height change.
          await presenter(for:binding).load()
        }
        let next=offset+UInt32(manifest.bindings.count)
        if next>=manifest.total {break}
        guard next>offset else {throw ResultClientError.invalidReply};offset=next
      }
      await reader.close()
      guard !closing else {return};accept(.init(receipt:receipt,bindings:bindings,values:values),cell:cell)
    } catch {await reader.close();throw error}
  }
  func displayAgentReceipts(_ receipts:[NativeKernelReceipt]) async {
    for receipt in receipts {
      guard !closing,receipt.producer.cell_id.value != nil,results[receipt.producer.cell_id.value!]?.receipt.operation_id != receipt.operation_id else {continue}
      if let sourceProjection,sourceProjection.source.snapshot_hash==receipt.producer.source_snapshot_hash {
        resultSources[receipt.producer.cell_id.value!]=sourceProjection
      }
      do {try await display(receipt)}
      catch {if let cell=receipt.producer.cell_id.value {accept(.init(receipt:receipt,bindings:[],values:[]),cell:cell)}}
    }
    if let sourceProjection {refreshStaleReasons(sourceProjection)}
  }
  func stop() async {
    guard let operation,!cancelPending else {return};cancelPending=true
    await kernel.cancel(operation)
    // Keep pending until the original producer/physical outcome actually settles.
  }
  func reconcile() async {
    guard let id=unknownOperation,operation==nil,!closing else {return}
    operation=id;operationCell=unknownCell;defer {operation=nil;operationCell=nil;changed?()}
    do {
      if let receipt=try await kernel.reconcile(id) {
        phase = .completed;unknownOperation=nil;message="已核对原运行结果"
        do {try await display(receipt)} catch {message="已核对计算提交，结果暂无法读取："+String(describing:error)}
      }
      else {phase = .discarded;message="已确认本次运行未提交"}
      unknownOperation=nil;unknownCell=nil
    } catch {message="运行结果仍待核对："+String(describing:error)}
  }
  func close() async {
    guard !closing else {return};closing=true
    if let operation {await kernel.cancel(operation)}
    runTask?.cancel();await runTask?.value
    sourceAnalysisTask?.cancel();sourceAnalysisTask=nil
    for owner in presenters.values {await owner.close()};presenters.removeAll()
    await kernel.close();runTask=nil
  }
}
