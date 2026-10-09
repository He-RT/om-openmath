import Foundation
import SwiftUI

/// Confirmed host projection plus a local scratch draft. Persistent notebook source arrives
/// from the document coordinator in R4.1; this model never stores a second writable notebook.
@MainActor final class NotebookViewModel:ObservableObject {
  @Published var draft="2+2"
  @Published private(set) var projection:HostConfirmedProjection?
  @Published private(set) var output=""
  @Published private(set) var message="正在连接内核"
  @Published private(set) var operationID:String?
  @Published private(set) var cancelPending=false
  @Published private(set) var evaluatedSource:String?
  private var session:NativeHostSession?
  private var watch:Task<Void,Never>?
  private var draftSequence:UInt64=0
  private var draftDelivery:Task<Void,Never>?
  private var alive=true
  var isStale:Bool { evaluatedSource != nil && evaluatedSource != draft }
  func start() async {
    guard session==nil,alive else { return }
    do {
      let host=try await NativeHostSession.open()
      guard alive else { try await host.close();return }
      session=host;draftSequence=0;draftDelivery=nil
      let updates=try await host.updates()
      watch=Task { [weak self] in
        for await projection in updates {
          guard !Task.isCancelled,let self,self.alive else { break }
          self.projection=projection
          if self.operationID==nil && self.evaluatedSource==nil { self.message=projection.needsResync ? "正在同步状态" : "可以试算" }
        }
      }
    } catch { message="内核连接失败" }
  }
  func noteDraft() {
    guard draftSequence<HostSerial.maximum else { message="编辑计数已达上限";return }
    draftSequence += 1
    let sequence=draftSequence
    if let session {
      let previous=draftDelivery
      draftDelivery=Task { await previous?.value;try? await session.noteDraft(sequence:sequence) }
    }
  }
  func run() async {
    guard let session,operationID==nil,alive else { return }
    let id=UUID().uuidString.lowercased(),source=draft
    operationID=id;cancelPending=false;message="正在试算"
    do {
      let status=try await session.perform(.evaluate_scratch(.init(kind:.evaluate_scratch,source:source,
        dialect:.modern,definition_snapshot_ref:.init(nil),use_notebook_definitions:false,timeout_ms:10000)),operation:id)
      let display=await Task.detached { Self.display(status) }.value
      guard alive,operationID==id else { return }
      output=display.output;message=display.message;evaluatedSource=source
    } catch {
      guard alive,operationID==id else { return }
      message="试算未完成，请检查状态";output=""
    }
    if operationID==id { operationID=nil;cancelPending=false }
  }
  func stop() async {
    guard let operationID,let session,!cancelPending else { return }
    cancelPending=true
    message="正在停止"
    await session.cancel(operationID)
  }
  func close() async {
    guard alive else { return }
    alive=false
    watch?.cancel();watch=nil
    if let session { try? await session.close() }
    session=nil
  }
  nonisolated private static func display(_ status:HostOperationStatus)->(output:String,message:String) {
    if status.phase == .cancelled { return ("","已停止") }
    guard case .object(let result)=status.result.value else {
      return (status.error_code.value ?? "结果详情不可用","试算未完成")
    }
    guard case .object(let response)=result["response"] else { return ("","结果不可用") }
    var lines:[String]=[]
    if case .object(let output)=response["output"],case .array(let items)=output["items"] {
      for item in items {
        if case .object(let item)=item,case .string(let exact)=item["input_form"] { lines.append(exact) }
      }
      if case .array(let messages)=output["messages"] {
        for message in messages {
          if case .object(let fields)=message,case .string(let text)=fields["text"] { lines.append(text) }
        }
      }
    }
    if case .string(let error)=response["message"] { lines.append(error) }
    let text=lines.joined(separator:"\n")
    let clipped=text.count>8000 ? String(text.prefix(8000))+"\n（结果较长，预览已截断）" : text
    return (clipped,result["run_outcome"] == .string("failed") ? "计算有错误" : "试算完成")
  }
}
