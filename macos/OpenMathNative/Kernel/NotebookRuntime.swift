import Foundation
import SwiftUI

struct NotebookCellResult:Sendable {
  let receipt:NativeKernelReceipt
  let bindings:[ResultBinding]
  let sources:[String]
}
/// The document owns this model, not a pane's lifetime. Requests/stop retain their original
/// operation ID and accepted results come only from the physical kernel receipt/read channel.
@MainActor final class NotebookRuntime:ObservableObject {
  @Published private(set) var operation:String?
  @Published private(set) var cancelPending=false
  @Published private(set) var phase:NativeKernelHostReplyPhase?
  @Published private(set) var message="就绪"
  @Published private(set) var results:[String:NotebookCellResult]=[:]
  @Published private(set) var unknownOperation:String?
  @Published var selectedCell:String?
  @Published private(set) var navigationSequence:UInt64=0
  private let kernel:KernelCommitPort,host:NativeHostClient
  private var runTask:Task<Void,Never>?
  private var closing=false
  var beforeRun:(@MainActor () async throws->Void)?
  var changed:(@MainActor ()->Void)?
  init(host:NativeHostClient,storage:StorageService,source:DocCommitPort) {
    self.host=host;kernel=KernelCommitPort(client:host,storage:storage,source:source)
  }
  var canRun:Bool {!closing && operation==nil && unknownOperation==nil}
  var canStop:Bool {operation != nil && !cancelPending && phase != .completed && phase != .failed && phase != .discarded}
  func navigate(to cell:String?) {guard !closing,cell != selectedCell else {return};selectedCell=cell;navigationSequence+=1}
  func run(_ cell:String) {
    guard canRun else {return}
    let id="notebook-run-"+UUID().uuidString.lowercased()
    selectedCell=cell;operation=id;phase=nil;message="正在提交运行请求";cancelPending=false
    runTask=Task { [weak self] in
      guard let self else {return}
      let polling=Task { [weak self] in
        while !Task.isCancelled {
          guard let self,!closing,operation==id else {return}
          if let actual=try? await kernel.status(id),actual.operation_id.value==id {phase=actual.phase}
          try? await Task.sleep(for:.milliseconds(100))
        }
      }
      defer {polling.cancel();if operation==id {operation=nil;cancelPending=false};changed?()}
      do {
        try await beforeRun?()
        let receipt=try await kernel.runCell(cell,operation:id)
        guard !closing else {return}
        // A late stop cannot overwrite an actual committed result.
        phase = .completed;message=receipt.producer.terminal_status == .done ? "已完成" : "计算结束，有错误"
        do {try await display(receipt)}
        catch {if !closing {results[cell] = .init(receipt:receipt,bindings:[],sources:[]);message="计算已提交，结果暂无法读取："+String(describing:error)}}
      } catch KernelPortError.unknownOutcome {unknownOperation=id;phase = .unknown;message="运行结果待核对"}
      catch {if !closing {message="运行未完成："+String(describing:error)}}
    }
  }
  private func display(_ receipt:NativeKernelReceipt) async throws {
    guard let cell=receipt.producer.cell_id.value,let root=receipt.result_id.value else {return}
    let reader=ResultClient(host:host)
    var offset:UInt32=0,bindings:[ResultBinding]=[],sources:[String]=[]
    do {
      while true {
        let manifest=try await reader.manifest(root,offset:offset,limit:16)
        for binding in manifest.bindings {
          guard binding.cell_id==cell,binding.operation_id==receipt.operation_id else {throw ResultClientError.invalidReply}
          bindings.append(binding)
          if binding.out_index.value>0 {
            try reader.select(binding)
            if let source=try await reader.completeSource() {sources.append(source)}
          }
        }
        let next=offset+UInt32(manifest.bindings.count)
        if next>=manifest.total {break}
        guard next>offset else {throw ResultClientError.invalidReply};offset=next
      }
      await reader.close()
      guard !closing else {return};results[cell] = .init(receipt:receipt,bindings:bindings,sources:sources)
    } catch {await reader.close();throw error}
  }
  func stop() async {
    guard let operation,!cancelPending else {return};cancelPending=true
    await kernel.cancel(operation)
    // Keep pending until the original producer/physical outcome actually settles.
  }
  func reconcile() async {
    guard let id=unknownOperation,operation==nil,!closing else {return}
    operation=id;defer {operation=nil;changed?()}
    do {
      if let receipt=try await kernel.reconcile(id) {
        phase = .completed;unknownOperation=nil;message="已核对原运行结果"
        do {try await display(receipt)} catch {message="已核对计算提交，结果暂无法读取："+String(describing:error)}
      }
      else {phase = .discarded;message="已确认本次运行未提交"}
      unknownOperation=nil
    } catch {message="运行结果仍待核对："+String(describing:error)}
  }
  func close() async {
    guard !closing else {return};closing=true
    if let operation {await kernel.cancel(operation)}
    runTask?.cancel();await runTask?.value;await kernel.close();runTask=nil
  }
}
