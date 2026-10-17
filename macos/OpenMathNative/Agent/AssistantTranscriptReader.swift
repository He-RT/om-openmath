import Foundation

/// A data-only projection of durable canonical messages. Presentation never dispatches tools.
actor AssistantTranscriptReader {
  private let storage:StorageService,document:String
  init(storage:StorageService,document:String) {self.storage=storage;self.document=document}
  private struct Reply {let body:Data;let changes:[String:TurnChangedCell.Kind]}
  func read(session:String)async throws->[AssistantMessage] {
    var events:[StoredSessionEvent]=[],after:UInt64=0
    while true {
      let page=try await storage.sessionEvents(session,after:after,limit:32)
      guard let last=page.last else {break};events+=page;after=last.event.event_sequence
    }
    let source=try await storage.committedSource(document)
    let order=Dictionary(uniqueKeysWithValues:(source?.file.cells ?? []).enumerated().map{($0.element.id,$0.offset+1)})
    var replies:[String:Reply]=[:],unavailable=Set<String>(),contexts:[String:AssistantMessageContext]=[:],stoppedRuns=Set<String>()
    func key(_ task:String,_ turn:String,_ call:String)->String {task+"\u{0}"+turn+"\u{0}"+call}
    for stored in events {
      let event=stored.event
      guard let text=event.payload_inline_json.value else {continue};let bytes=Data(text.utf8)
      if event.event_kind == .user_message,let input=try? JSONDecoder().decode(SessionCanonicalInput.self,from:bytes),let context=input.presentation,let task=event.task_id.value {contexts[task]=context}
      if event.event_kind == .task_transition,let transition=try? JSONDecoder().decode(SessionRunTransition.self,from:bytes),
        transition.kind == .run_child_stopped,transition.code=="CANCELLED",
        let group=events.first(where:{$0.event.event_id==transition.groupAdmission}),
        let admission=try? SessionValidation.payload(SessionToolAdmission.self,group.event) {
        stoppedRuns.insert(key(event.task_id.value ?? "",event.agent_turn_id.value ?? "",admission.callID))
      }
      if event.event_kind != .recovery {continue}
      let canonical:AgentCanonicalMessage,intent:NativeAgentToolIntent,hash:String,length:UInt64
      var changes:[String:TurnChangedCell.Kind]=[:]
      if let record=try SessionSemantics.readonlyRecord(event) {
        canonical=record.canonical;intent=record.intent;hash=record.blobHash;length=record.byteLength
      } else if let record=try SessionSemantics.effectRecord(event) {
        canonical=record.canonical;intent=record.intent;hash=record.blobHash;length=record.byteLength
        if let target=record.target,target.kind == .source,let receipt=try await storage.sourceReceipt(document:document,operation:target.operationID),let id=receipt.receipt.transaction_id.value,
          let transaction=try await storage.sourceTransactions(document:document,ids:[id]).first {
          let before=Set(transaction.plan.before.file.cells.map(\.id)),after=Set(transaction.plan.after.file.cells.map(\.id))
          for cell in transaction.plan.commit.changed_cell_ids {changes[cell] = !after.contains(cell) ? .deleted:before.contains(cell) ? .edited:.new}
        }
      } else {
        if let record=try? JSONDecoder().decode(AgentToolUnavailable.self,from:bytes),record.kind=="native_tool_unavailable" {
          unavailable.insert(key(event.task_id.value ?? "",record.intent.agent_turn_id,record.intent.tool_call_id))
        }
        continue
      }
      guard length<=262144 else {throw AgentSessionError.invalidFrame}
      let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:canonical.message_id),hash:hash)
      let body:Data
      do {body=try await storage.readBlob(lease,offset:0,count:Int(length));try await storage.closeBlob(lease)}
      catch {try? await storage.closeBlob(lease);throw error}
      guard UInt64(body.count)==length,KernelHashes.data(body)==hash else {throw AgentSessionError.ledgerMismatch}
      replies[key(event.task_id.value ?? "",intent.agent_turn_id,intent.tool_call_id)] = .init(body:body,changes:changes)
    }
    var messages:[AssistantMessage]=[]
    for stored in events {
      let event=stored.event,task=event.task_id.value ?? ""
      guard let raw=event.payload_inline_json.value else {continue};let bytes=Data(raw.utf8)
      let canonical:AgentCanonicalMessage
      if event.event_kind == .user_message {canonical=try JSONDecoder().decode(SessionCanonicalInput.self,from:bytes).message}
      else if event.event_kind == .assistant_end {
        canonical=try JSONDecoder().decode(AgentCanonicalMessage.self,from:Data(JSONDecoder().decode(SessionCanonicalAssistant.self,from:bytes).canonicalJSON.utf8))
      } else {continue}
      let turn=canonical.agent_turn_id.value ?? canonical.message_id
      var parts:[AssistantMessage.Part]=[]
      for part in canonical.parts {
        switch part {
        case .agent_text_part(let text):parts.append(.init(id:text.part_id,value:.text(text.text)))
        case .agent_tool_proposal_part(let proposal):
          let identity=key(task,turn,proposal.tool_call_id),reply=replies[identity]
          let action=AgentActionFormatter.make(turn:turn,call:proposal.tool_call_id,tool:proposal.name.rawValue,
            arguments:Data(proposal.arguments_json.utf8),reply:reply?.body,cellOrder:order,changes:reply?.changes ?? [:],
            fallback:stoppedRuns.contains(identity) ? .cancelled:unavailable.contains(identity) ? .failed:nil)
          if let last=parts.last,case .actions(var actions)=last.value {actions.append(action);parts[parts.count-1].value = .actions(actions)}
          else {parts.append(.init(id:proposal.part_id,value:.actions([action])))}
        default:break
        }
      }
      messages.append(.init(id:canonical.role == .assistant ? turn:canonical.message_id,taskID:task,role:canonical.role.rawValue,context:contexts[task],parts:parts))
    }
    return messages
  }
}
