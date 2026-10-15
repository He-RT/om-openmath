import Foundation

/// Pure event interpretation. It has no executor and treats uncertain effects as uncertain;
/// current document success comes only from verified external document facts.
enum SessionSemantics {
  static func effectRecord(_ bytes:Data)throws->AgentEffectRecord? {
    guard let object=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],object["kind"] as? String=="native_effect_result" else {return nil}
    return try JSONDecoder().decode(AgentEffectRecord.self,from:bytes)
  }
  static func effectRecord(_ event:NativeSessionEvent)throws->AgentEffectRecord? {
    guard event.event_kind == .recovery,let text=event.payload_inline_json.value else {return nil}
    return try effectRecord(Data(text.utf8))
  }
  static func effectPhase(_ record:AgentEffectRecord)throws->RecoveredToolPhase {
    guard record.canonical.parts.count==1,case .agent_tool_result_part(let part)=record.canonical.parts[0] else {throw StorageError.corruptIdentity}
    switch part.outcome {case .completed:return .completed;case .partial:return .partial;case .failed:return .failed;case .cancelled:return .cancelled;case .unknown:return .tool_outcome_unknown;default:throw StorageError.corruptIdentity}
  }
  static func readonlyRecord(_ bytes:Data)throws->AgentReadonlyRecord? {
    guard let object=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],object["kind"] as? String=="native_readonly_result" else {return nil}
    return try JSONDecoder().decode(AgentReadonlyRecord.self,from:bytes)
  }
  static func readonlyRecord(_ event:NativeSessionEvent)throws->AgentReadonlyRecord? {
    guard event.event_kind == .recovery,let text=event.payload_inline_json.value else {return nil}
    return try readonlyRecord(Data(text.utf8))
  }
  static func readonlyPhase(_ record:AgentReadonlyRecord)throws->RecoveredToolPhase {
    guard record.canonical.parts.count==1,case .agent_tool_result_part(let part)=record.canonical.parts[0] else {throw StorageError.corruptIdentity}
    switch part.outcome {case .completed:return .readonly_completed;case .failed:return .readonly_failed;default:throw StorageError.corruptIdentity}
  }
  private static func sameScope(_ input:SessionAppendInput,_ event:NativeSessionEvent)throws {
    guard input.turn==event.agent_turn_id.value,input.task==event.task_id.value,
      input.operation==event.operation_id.value,input.operationStore==event.operation_store_id.value else {throw SessionLedgerError.invalidTransition}
  }
  static func validate(_ input:SessionAppendInput,events:[NativeSessionEvent])throws {
    switch input.kind {
    case .assistant_end:
      let end=try JSONDecoder().decode(SessionAssistantEnd.self,from:input.payload)
      guard end.codecVersion==1,SourceValidation.identity(end.messageID),String(data:end.textUTF8,encoding:.utf8) != nil,
        end.calls.count<=32,Set(end.calls.map(\.callID)).count==end.calls.count else {throw StorageError.corruptIdentity}
      if !end.calls.isEmpty {guard input.turn != nil,input.task != nil else {throw SessionLedgerError.invalidTransition}}
      var previous=Set<String>()
      for event in events where event.event_kind == .assistant_end {
        let old=try SessionValidation.payload(SessionAssistantEnd.self,event)
        previous.formUnion(old.calls.map(\.callID))
      }
      for call in end.calls {
        guard SourceValidation.identity(call.callID),SourceValidation.identity(call.toolName),!previous.contains(call.callID) else {throw SessionLedgerError.invalidTransition}
        try SessionValidation.json(call.arguments)
      }
    case .assistant_batch:
      let batch=try JSONDecoder().decode(SessionAssistantEnd.self,from:input.payload)
      guard batch.codecVersion==1,batch.calls.isEmpty,SourceValidation.identity(batch.messageID),String(data:batch.textUTF8,encoding:.utf8) != nil,batch.textUTF8.count<=32768 else {throw SessionLedgerError.invalidTransition}
    case .tool_admitted:
      let admission=try JSONDecoder().decode(SessionToolAdmission.self,from:input.payload);try SessionValidation.target(admission.target)
      guard admission.codecVersion==1,input.operation==admission.target.operationID,input.operationStore==admission.target.storeID,
        KernelValidation.hash(admission.argumentsHash),input.parentEvent==admission.proposalEvent,
        let proposal=events.first(where:{$0.event_id==admission.proposalEvent && $0.event_kind == .assistant_end}),
        proposal.agent_turn_id.value==input.turn,proposal.task_id.value==input.task else {throw SessionLedgerError.invalidTransition}
      let end=try SessionValidation.payload(SessionAssistantEnd.self,proposal)
      guard let call=end.calls.first(where:{$0.callID==admission.callID}),call.toolName==admission.toolName,KernelHashes.data(call.arguments)==admission.argumentsHash else {throw SessionLedgerError.invalidTransition}
      try SessionRunSemantics.validateAdmission(admission,call:call)
      for event in events where event.event_kind == .tool_admitted {
        let old=try SessionValidation.payload(SessionToolAdmission.self,event)
        guard old.callID != admission.callID,old.target.storeID != admission.target.storeID || old.target.operationID != admission.target.operationID else {throw SessionLedgerError.executionAlreadyClaimed}
      }
    case .tool_started:
      let started=try JSONDecoder().decode(SessionToolStarted.self,from:input.payload)
      guard started.codecVersion==1,input.parentEvent==started.admissionEvent,
        let admission=events.first(where:{$0.event_id==started.admissionEvent && $0.event_kind == .tool_admitted}) else {throw SessionLedgerError.invalidTransition}
      try sameScope(input,admission)
      guard !events.contains(where:{$0.parent_event_id.value==started.admissionEvent && [.tool_started,.tool_result].contains($0.event_kind)}) else {throw SessionLedgerError.executionAlreadyClaimed}
    case .tool_result:
      let result=try JSONDecoder().decode(SessionToolResult.self,from:input.payload)
      guard result.codecVersion==1,input.parentEvent==result.admissionEvent,SourceValidation.identity(result.sourceEventID),
        KernelValidation.hash(result.sourceEffectHash),KernelValidation.hash(result.rawReceiptHash),
        let admission=events.first(where:{$0.event_id==result.admissionEvent && $0.event_kind == .tool_admitted}) else {throw SessionLedgerError.invalidTransition}
      try sameScope(input,admission)
      guard try SessionValidation.payload(SessionToolAdmission.self,admission).run==nil else {throw SessionLedgerError.invalidTransition}
      guard !events.contains(where:{$0.parent_event_id.value==result.admissionEvent && $0.event_kind == .tool_result}) else {throw SessionLedgerError.invalidTransition}
      if !result.recovered {guard events.contains(where:{$0.parent_event_id.value==result.admissionEvent && $0.event_kind == .tool_started}) else {throw SessionLedgerError.invalidTransition}}
    case .task_transition:
      if let run=try SessionRunSemantics.transition(input.payload) {try SessionRunSemantics.validate(input,transition:run,events:events);return}
      let lifecycle=try JSONDecoder().decode(SessionRequestLifecycle.self,from:input.payload)
      guard input.turn != nil,input.task != nil else {throw SessionLedgerError.invalidTransition}
      if lifecycle.kind == .request_planned {
        guard lifecycle.transition==nil,let plan=lifecycle.plan,plan.codecVersion==1,SourceValidation.identity(plan.contextID),
          KernelValidation.hash(plan.wireBlobHash),plan.wireByteLength>0,plan.wireByteLength<=128*1024*1024,
          SourceValidation.identity(plan.adapterVersion),!events.contains(where:{event in
            guard event.event_kind == .task_transition,let previous=try? SessionValidation.payload(SessionRequestLifecycle.self,event) else {return false}
            return previous.plan?.contextID==plan.contextID
          }) else {throw SessionLedgerError.invalidTransition}
      } else {
        guard lifecycle.plan==nil,let transition=lifecycle.transition,transition.codecVersion==1,transition.phase != .planned,
          let parentID=input.parentEvent,let parent=events.first(where:{$0.event_id==parentID && $0.event_kind == .task_transition}) else {throw SessionLedgerError.invalidTransition}
        guard parent.agent_turn_id.value==input.turn,parent.task_id.value==input.task else {throw SessionLedgerError.invalidTransition}
        let old=try SessionValidation.payload(SessionRequestLifecycle.self,parent)
        let context=old.plan?.contextID ?? old.transition?.contextID,phase=old.plan != nil ? SessionDispatchPhase.planned : old.transition!.phase
        guard context==transition.contextID,!events.contains(where:{$0.parent_event_id.value==parentID && $0.event_kind == .task_transition}) else {throw SessionLedgerError.invalidTransition}
        let allowed:Bool
        switch phase {
        case .planned:allowed=[.dispatch_started,.interrupted].contains(transition.phase)
        case .dispatch_started:allowed=[.response_started,.failed,.interrupted].contains(transition.phase)
        case .response_started:allowed=[.finished,.failed,.interrupted].contains(transition.phase)
        default:allowed=false
        }
        guard allowed else {throw SessionLedgerError.invalidTransition}
        if [.response_started,.finished].contains(transition.phase) {
          guard let hash=transition.responseBlobHash,KernelValidation.hash(hash),let length=transition.responseByteLength,length>0,length<=128*1024*1024 else {throw StorageError.corruptIdentity}
        } else {guard transition.responseBlobHash==nil,transition.responseByteLength==nil else {throw StorageError.corruptIdentity}}
      }
    case .recovery:
      if let record=try effectRecord(input.payload) {
        let intent=record.intent,message=record.canonical
        guard record.codecVersion==1,[.apply_notebook_patch,.run_cells].contains(intent.name),
          SourceValidation.identity(intent.operation_id),SourceValidation.identity(record.resultRef),KernelValidation.hash(record.blobHash),
          record.byteLength>0,record.byteLength<=262144,input.eventID=="effect-"+intent.operation_id,
          input.operation==record.target?.operationID,input.operationStore==record.target?.storeID,
          input.parentEvent==intent.proposal_event_ref,input.turn==intent.agent_turn_id,
          let proposal=events.first(where:{$0.event_kind == .assistant_end && $0.event_id==intent.proposal_event_ref}),
          proposal.task_id.value==input.task,proposal.agent_turn_id.value==input.turn,
          message.record_type == .canonical_agent_message,message.message_version==1,message.role == .tool,message.message_id==input.eventID,
          message.task_id.value==input.task,message.agent_turn_id.value==input.turn,message.parts.count==1,
          case .agent_tool_result_part(let part)=message.parts[0],part.part_id==input.eventID,part.tool_call_id==intent.tool_call_id,
          part.operation_ref.value==record.target?.operationID,part.result_ref.value==record.resultRef else {throw SessionLedgerError.invalidTransition}
        let end=try SessionValidation.payload(SessionAssistantEnd.self,proposal)
        guard let call=end.calls.first(where:{$0.callID==intent.tool_call_id}),call.toolName==intent.name.rawValue,
          KernelHashes.data(call.arguments)==intent.arguments_hash else {throw SessionLedgerError.invalidTransition}
        _ = try effectPhase(record)
        if let target=record.target {
          try SessionValidation.target(target)
          guard let admittedID=record.admissionEvent,let admitted=events.first(where:{$0.event_kind == .tool_admitted && $0.event_id==admittedID}),
            admitted.task_id.value==input.task,admitted.agent_turn_id.value==input.turn else {throw SessionLedgerError.invalidTransition}
          let admission=try SessionValidation.payload(SessionToolAdmission.self,admitted)
          guard admission.target==target,admission.callID==intent.tool_call_id,admission.argumentsHash==intent.arguments_hash else {throw SessionLedgerError.invalidTransition}
          if intent.name == .run_cells {
            guard admission.run != nil,let resultID=record.outcomeEvent,
              let result=events.first(where:{$0.event_id==resultID && $0.event_kind == .task_transition}),
              result.operation_id.value==target.operationID,result.operation_store_id.value==target.storeID,
              let run=try SessionRunSemantics.transition(result),run.groupAdmission==admittedID,run.kind == .run_finished,
              let phase=run.phase,phase.rawValue==part.outcome.rawValue else {throw SessionLedgerError.invalidTransition}
          } else if part.outcome == .completed {
            guard let resultID=record.outcomeEvent,let result=events.first(where:{$0.event_kind == .tool_result && $0.event_id==resultID && $0.parent_event_id.value==admittedID}),
              result.operation_id.value==target.operationID,result.operation_store_id.value==target.storeID,
              (try SessionValidation.payload(SessionToolResult.self,result)).outcome == .completed else {throw SessionLedgerError.invalidTransition}
          } else {guard record.outcomeEvent==nil else {throw SessionLedgerError.invalidTransition}}
        } else {guard record.admissionEvent==nil,record.outcomeEvent==nil,(part.outcome == .failed || intent.name == .run_cells && part.outcome == .cancelled) else {throw SessionLedgerError.invalidTransition}}
        for event in events where event.event_kind == .recovery {
          if let old=try effectRecord(event),old.intent.tool_call_id==intent.tool_call_id {throw SessionLedgerError.invalidTransition}
        }
      }
      if let record=try readonlyRecord(input.payload) {
        let intent=record.intent,message=record.canonical
        guard record.codecVersion==1,[.read_notebook,.search_functions,.get_function_docs,.preview_source,.inspect_result,.evaluate_scratch,.get_operation_status].contains(intent.name),
          SourceValidation.identity(intent.operation_id),SourceValidation.identity(record.resultRef),KernelValidation.hash(record.blobHash),
          record.byteLength>0,record.byteLength<=262144,input.eventID=="readonly-"+intent.operation_id,
          input.operation==nil,input.operationStore==nil,input.parentEvent==intent.proposal_event_ref,input.turn==intent.agent_turn_id,
          let proposal=events.first(where:{$0.event_kind == .assistant_end && $0.event_id==intent.proposal_event_ref}),
          proposal.task_id.value==input.task,proposal.agent_turn_id.value==input.turn,
          message.record_type == .canonical_agent_message,message.message_version==1,message.role == .tool,message.message_id==input.eventID,
          message.task_id.value==input.task,message.agent_turn_id.value==input.turn,message.parts.count==1,
          case .agent_tool_result_part(let part)=message.parts[0],part.part_id==input.eventID,part.tool_call_id==intent.tool_call_id,
          part.operation_ref.value==nil,part.result_ref.value==record.resultRef else {throw SessionLedgerError.invalidTransition}
        let end=try SessionValidation.payload(SessionAssistantEnd.self,proposal)
        guard let call=end.calls.first(where:{$0.callID==intent.tool_call_id}),call.toolName==intent.name.rawValue,
          KernelHashes.data(call.arguments)==intent.arguments_hash else {throw SessionLedgerError.invalidTransition}
        _ = try readonlyPhase(record)
        for previous in events where previous.event_kind == .recovery {
          if let old=try readonlyRecord(previous),old.intent.tool_call_id==intent.tool_call_id {throw SessionLedgerError.invalidTransition}
        }
      }
    case .user_message,.user_stop:
      break
    case .prompt_change,.compaction:
      // These records require Prompt/Context transaction adapters, implemented by R4.4.
      throw StorageError.unsupportedVersion
    }
  }
  static func recover(_ session:String,events:[NativeSessionEvent],complete:Bool)throws->SessionRecoveryPage {
    var tools:[RecoveredTool]=[],requests:[RecoveredRequest]=[]
    for event in events {
      if event.event_kind == .assistant_end {
        let end=try SessionValidation.payload(SessionAssistantEnd.self,event)
        for call in end.calls {
          var admissionEvent:NativeSessionEvent?,admission:SessionToolAdmission?
          for next in events where next.event_kind == .tool_admitted {
            let record=try SessionValidation.payload(SessionToolAdmission.self,next)
            if record.proposalEvent==event.event_id,record.callID==call.callID {guard admissionEvent==nil else {throw StorageError.corruptIdentity};admissionEvent=next;admission=record}
          }
          let result=admissionEvent.flatMap {admit in events.first(where:{$0.event_kind == .tool_result && $0.parent_event_id.value==admit.event_id})}
          var phase:RecoveredToolPhase
          if let result {phase=try SessionValidation.payload(SessionToolResult.self,result).outcome == .partial ? .partial : .completed}
          else {phase=admissionEvent==nil ? .tool_not_started : .tool_outcome_unknown}
          var resultID=result?.event_id
          var runSummary:RecoveredRunSummary?
          if let admissionEvent,let admission,admission.run != nil {
            var latest=admissionEvent
            for _ in 0..<3010 {
              let next=events.filter{$0.parent_event_id.value==latest.event_id && [.tool_started,.task_transition].contains($0.event_kind)}
              guard next.count<=1 else {throw StorageError.corruptIdentity}
              guard let child=next.first else {break};latest=child
            }
            let transition=try SessionRunSemantics.transition(latest)
            let snapshot=SessionRunSnapshot(admissionEvent:admissionEvent,admission:admission,latest:latest,transition:transition,progress:transition?.progress ?? .init(plan:admission.run!))
            phase=SessionRunSemantics.recoveredPhase(snapshot);runSummary=snapshot.summary;resultID=transition?.phase != nil ? latest.event_id:nil
          }
          for next in events where next.event_kind == .recovery && next.parent_event_id.value==event.event_id {
            if let record=try effectRecord(next),record.intent.tool_call_id==call.callID {
              guard record.target==admission?.target,record.admissionEvent==admissionEvent?.event_id else {throw StorageError.corruptIdentity}
              let effectPhase=try effectPhase(record)
              if !(effectPhase == .tool_outcome_unknown && [.completed,.partial].contains(phase)) {phase=effectPhase;resultID=next.event_id}
            }
            if let record=try readonlyRecord(next),record.intent.tool_call_id==call.callID {
              guard admissionEvent==nil,resultID==nil else {throw StorageError.corruptIdentity}
              phase=try readonlyPhase(record);resultID=next.event_id
            }
          }
          tools.append(.init(session:session,proposalEvent:event.event_id,callID:call.callID,admissionEvent:admissionEvent?.event_id,phase:phase,target:admission?.target,resultEvent:resultID,run:runSummary))
        }
      }
      if event.event_kind == .task_transition {
        if try SessionRunSemantics.transition(event) != nil {continue}
        let lifecycle=try SessionValidation.payload(SessionRequestLifecycle.self,event)
        guard let plan=lifecycle.plan else {continue}
        var latest=event,phase=SessionDispatchPhase.planned
        while let next=events.first(where:{$0.event_kind == .task_transition && $0.parent_event_id.value==latest.event_id}) {
          guard next.event_sequence>latest.event_sequence else {throw StorageError.corruptIdentity}
          let transition=try SessionValidation.payload(SessionRequestLifecycle.self,next)
          guard let body=transition.transition,body.contextID==plan.contextID else {throw StorageError.corruptIdentity}
          latest=next;phase=body.phase
        }
        let delivered=events.contains{event in
          guard event.event_kind == .task_transition,let body=try? SessionValidation.payload(SessionRequestLifecycle.self,event).transition else {return false}
          return body.contextID==plan.contextID && body.phase == .response_started
        }
        let recovery:RecoveredRequestPhase
        switch phase {
        case .planned:recovery = .not_dispatched
        case .finished:recovery = .finished
        case .failed:recovery = .failed
        case .dispatch_started:recovery = .dispatch_outcome_unknown
        case .response_started:recovery = .response_observed
        case .interrupted:
          let dispatched=events.contains{event in guard let body=try? SessionValidation.payload(SessionRequestLifecycle.self,event).transition else {return false};return body.contextID==plan.contextID && body.phase == .dispatch_started}
          recovery=delivered ? .response_observed : dispatched ? .dispatch_outcome_unknown : .not_dispatched
        }
        requests.append(.init(session:session,planEvent:event.event_id,contextID:plan.contextID,phase:recovery,latestEvent:latest.event_id))
      }
    }
    return .init(tools:tools,requests:requests,lastEventSequence:events.last?.event_sequence ?? 0,complete:complete)
  }
  static func effectOutcome(_ effect:DocumentEffect,target:SessionOperationTarget)throws->SessionToolOutcome {
    try RecoveryValidation.effect(effect);try SessionValidation.target(target)
    guard effect.sourceStoreID==target.storeID,effect.documentID==target.documentID,effect.operationID==target.operationID,effect.kind==target.kind else {throw SessionLedgerError.wrongSourceEffect}
    switch effect.kind {
    case .source:
      let actual=try JSONDecoder().decode(NativeDurableSourceReceipt.self,from:effect.payload)
      guard actual.receipt.request_hash==target.expectedHash else {throw SessionLedgerError.wrongSourceEffect};return .completed
    case .kernel:
      let actual=try JSONDecoder().decode(NativeKernelReceipt.self,from:effect.payload)
      guard actual.producer.source_snapshot_hash==target.expectedHash else {throw SessionLedgerError.wrongSourceEffect};return actual.producer.terminal_status == .error ? .partial : .completed
    case .save:
      let actual=try JSONDecoder().decode(ProjectedSaveOutcome.self,from:effect.payload)
      guard actual.fileHash==target.expectedHash else {throw SessionLedgerError.wrongSourceEffect};return .completed
    }
  }
}
