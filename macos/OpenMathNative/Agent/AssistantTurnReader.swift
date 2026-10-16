import Foundation

/// Reads original receipts even if stopping prevented delivery of the final tool result to Pi.
/// No method here evaluates, replays an operation, or uses a model's claimed accomplishments.
actor AssistantTurnReader {
  private let storage:StorageService,document:String
  init(storage:StorageService,document:String) {self.storage=storage;self.document=document}
  func read(session:String,task:String)async throws->[TurnFact] {
    var events:[StoredSessionEvent]=[],after:UInt64=0
    while true {
      let page=try await storage.sessionEvents(session,after:after,limit:32)
      events+=page.filter{$0.event.task_id.value==task}
      guard let last=page.last else {break};after=last.event.event_sequence
    }
    var result:[TurnFact]=[]
    var patchData:[String:[String:Any]]=[:],runData:[String:[String:Any]]=[:]
    // Result envelopes enrich receipts; absence never erases a committed transaction.
    for stored in events {
      guard let text=stored.event.payload_inline_json.value,
        let record=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(text.utf8)),record.kind=="native_effect_result" else {continue}
      let bytes=try await blob(hash:record.blobHash,length:record.byteLength,owner:record.canonical.message_id)
      guard let root=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],
        let data=root["data"] as? [String:Any] else {continue}
      if record.intent.name == .run_cells {runData[record.intent.operation_id]=data}
      else if root["ok"] as? Bool==true {patchData[record.intent.operation_id]=data}
    }
    for stored in events {
      let event=stored.event
      guard let text=event.payload_inline_json.value else {continue};let bytes=Data(text.utf8)
      if event.event_kind == .tool_admitted {
        let admission=try JSONDecoder().decode(SessionToolAdmission.self,from:bytes)
        if admission.target.kind == .source,let receipt=try await storage.sourceReceipt(document:document,operation:admission.target.operationID),let transactionID=receipt.receipt.transaction_id.value {
          let transactions=try await storage.sourceTransactions(document:document,ids:[transactionID])
          guard let transaction=transactions.first,transaction.plan.commit.operation_id==admission.target.operationID,
            transaction.plan.commit.task_id.value==nil || transaction.plan.commit.task_id.value==task else {throw AgentSessionError.ledgerMismatch}
          let plan=transaction.plan,before=Set(plan.before.file.cells.map(\.id)),after=Set(plan.after.file.cells.map(\.id))
          let changes=plan.commit.changed_cell_ids.map {id in
            TurnChangedCell(cellID:id,kind:!after.contains(id) ? .deleted:before.contains(id) ? .edited:.new,transactionRef:transactionID)
          }
          let deleted=Array(before.subtracting(after))
          let changedMath=plan.after.file.cells.filter{$0.kind == .math && plan.commit.changed_cell_ids.contains($0.id)}.map(\.id)
          let invalidated=patchData[admission.target.operationID]?["invalidated_cell_ids"] as? [String] ?? []
          result.append(.changed(changes,invalidated:Array(Set(changedMath+invalidated)),deleted:deleted))
        }
      } else if event.event_kind == .task_transition {
        if let transition=try? JSONDecoder().decode(SessionRunTransition.self,from:bytes),transition.kind == .run_child_result,
          let receipt=transition.original?.receipt,let cell=receipt.producer.cell_id.value {
          let group=events.first{$0.event.event_id==transition.groupAdmission}
          let admission=try group.map{try SessionValidation.payload(SessionToolAdmission.self,$0.event)}
          let summary=(runData[admission?.target.operationID ?? ""]?["cells"] as? [[String:Any]])?.first{$0["cell_id"] as? String==cell}
          result.append(.ran(.init(cellID:cell,status:receipt.producer.terminal_status.rawValue,sourceHash:receipt.producer.cell_source_hash.value,diagnosticCount:(summary?["diagnostics"] as? [Any])?.count)))
        } else if let lifecycle=try? JSONDecoder().decode(SessionRequestLifecycle.self,from:bytes),let transition=lifecycle.transition,transition.phase == .dispatch_started {
          result.append(.request(transition.contextID))
        }
      } else if event.event_kind == .recovery,
        let record=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:bytes),record.kind=="native_readonly_result",record.intent.name == .inspect_result {
        let original=try await blob(hash:record.blobHash,length:record.byteLength,owner:record.canonical.message_id)
        if let fact=TurnProductParser.parse(original) {result.append(.product(fact))}
      }
    }
    return result
  }
  private func blob(hash:String,length:UInt64,owner:String)async throws->Data {
    guard length<=262144 else {throw AgentSessionError.invalidFrame}
    let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:owner),hash:hash)
    let bytes:Data
    do {bytes=try await storage.readBlob(lease,offset:0,count:Int(length));try await storage.closeBlob(lease)}
    catch {try? await storage.closeBlob(lease);throw error}
    guard UInt64(bytes.count)==length,KernelHashes.data(bytes)==hash else {throw AgentSessionError.ledgerMismatch}
    return bytes
  }
}
