import Foundation

/// Data-only run interpretation. All accepted effects are independently verified in Library.
enum SessionRunSemantics {
  static func transition(_ bytes:Data)throws->SessionRunTransition? {
    guard let object=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],
      let kind=object["kind"] as? String,kind.hasPrefix("run_") else {return nil}
    guard SessionRunTransitionKind(rawValue:kind) != nil else {throw SessionLedgerError.invalidTransition}
    return try JSONDecoder().decode(SessionRunTransition.self,from:bytes)
  }
  static func transition(_ event:NativeSessionEvent)throws->SessionRunTransition? {
    guard event.event_kind == .task_transition,let text=event.payload_inline_json.value else {return nil}
    return try transition(Data(text.utf8))
  }
  static func childOperation(_ admission:SessionToolAdmission,ordinal:Int)throws->String {
    let bytes=try JSONSerialization.data(withJSONObject:["openmath/run-child/v1",admission.target.operationID,admission.argumentsHash,ordinal],options:[.withoutEscapingSlashes])
    return "run-"+KernelHashes.data(bytes)
  }
  static func target(_ admission:SessionToolAdmission,index:Int)throws->SessionOperationTarget {
    guard let plan=admission.run,plan.children.indices.contains(index) else {throw SessionLedgerError.invalidTransition}
    return .init(documentID:admission.target.documentID,storeID:admission.target.storeID,operationID:plan.children[index].operationID,kind:.kernel,expectedHash:admission.target.expectedHash)
  }
  static func validateAdmission(_ admission:SessionToolAdmission,call:SessionToolCall)throws {
    guard let run=admission.run else {return}
    guard admission.toolName=="run_cells",admission.target.kind == .kernel,run.codecVersion==1,
      ["selected","required","affected"].contains(run.scope),(1...50).contains(run.requestedCellIDs.count),
      Set(run.requestedCellIDs).count==run.requestedCellIDs.count,run.requestedCellIDs.allSatisfy(SourceValidation.identity),
      run.children.count<=1001,run.blockedCells.count<=1000,
      run.documentGeneration>0,run.documentGeneration<=HostSerial.maximum,
      [run.sourceRevision,run.executionEpoch,run.configRevision].allSatisfy({$0<=HostSerial.maximum}),
      KernelValidation.hash(run.kernelBuild),KernelValidation.hash(run.generalHash),
      Set(run.children.map(\.operationID)).count==run.children.count,
      let args=try JSONSerialization.jsonObject(with:call.arguments) as? [String:Any],
      args["cell_ids"] as? [String]==run.requestedCellIDs,
      (args["scope"] as? String ?? "required")==run.scope,
      (args["continue_on_error"] as? Bool ?? false)==run.continueOnError else {throw SessionLedgerError.invalidTransition}
    if let id=run.initialCheckpointID {
      guard SourceValidation.identity(id),let hash=run.initialCheckpointHash,KernelValidation.hash(hash),
        let revision=run.initialKernelRevision,revision<=HostSerial.maximum else {throw StorageError.corruptIdentity}
    } else {guard run.initialCheckpointHash==nil,run.initialKernelRevision==nil else {throw StorageError.corruptIdentity}}
    var cells=Set<String>(),hasBootstrap=false
    for (index,child) in run.children.enumerated() {
      let ordinal:Int
      if let id=child.cellID {
        guard SourceValidation.identity(id),cells.insert(id).inserted,let hash=child.cellSourceHash,KernelValidation.hash(hash) else {throw StorageError.corruptIdentity}
        ordinal=index+(hasBootstrap ? 0:1)
      } else {
        guard index==0,run.initialCheckpointID==nil,child.cellSourceHash==nil else {throw StorageError.corruptIdentity}
        hasBootstrap=true;ordinal=0
      }
      guard child.operationID == (try childOperation(admission,ordinal:ordinal)) else {throw SessionLedgerError.invalidTransition}
    }
    guard run.children.filter({$0.cellID != nil}).count<=1000,
      run.children.isEmpty || hasBootstrap == (run.initialCheckpointID==nil) else {throw SessionLedgerError.invalidTransition}
    for blocked in run.blockedCells {
      guard SourceValidation.identity(blocked.cellID),cells.insert(blocked.cellID).inserted,
        ["DEPENDENCY_CYCLE","DEPENDENCY_BLOCKED"].contains(blocked.code) else {throw StorageError.corruptIdentity}
    }
    guard Set(run.requestedCellIDs).isSubset(of:cells) else {throw SessionLedgerError.invalidTransition}
  }
  static func prior(_ parent:NativeSessionEvent,group:NativeSessionEvent,admission:SessionToolAdmission)throws->SessionRunProgress {
    if parent.event_kind == .tool_started {
      let started=try SessionValidation.payload(SessionToolStarted.self,parent)
      guard started.admissionEvent==group.event_id,parent.parent_event_id.value==group.event_id else {throw SessionLedgerError.invalidTransition}
      return .init(plan:admission.run!)
    }
    guard let old=try transition(parent),old.groupAdmission==group.event_id else {throw SessionLedgerError.invalidTransition}
    return old.progress
  }
  static func applying(_ original:SessionRunOriginalReceipt,to prior:SessionRunProgress,child:SessionRunChild,plan:SessionRunPlan,target:SessionOperationTarget)throws->SessionRunProgress {
    let r=original.receipt,p=r.producer
    guard original.sourceEventID==r.outbox_event_id,SourceValidation.identity(original.sourceEventID),
      KernelValidation.hash(original.sourceEffectHash),KernelValidation.hash(original.rawReceiptHash),
      r.protocol_version==1,r.store_id==target.storeID,r.document_id==target.documentID,r.operation_id==child.operationID,
      p.document_id==target.documentID,p.document_generation.value==plan.documentGeneration,p.source_revision.value==plan.sourceRevision,
      p.execution_epoch.value==plan.executionEpoch,p.config_revision.value==plan.configRevision,p.general_hash==plan.generalHash,
      p.kernel_build==plan.kernelBuild,p.source_snapshot_hash==target.expectedHash,p.cell_id.value==child.cellID,
      p.cell_source_hash.value==child.cellSourceHash,r.accepted_snapshot_hash==target.expectedHash,r.accepted_source_revision.value==plan.sourceRevision,
      r.kernel_state_revision.value==p.kernel_state_revision.value else {throw SessionLedgerError.wrongSourceEffect}
    var progress=prior
    if child.cellID==nil {
      guard prior.lastCheckpointID==nil,p.kernel_state_revision.value==0,p.terminal_status == .unexecuted,r.result_id.value==nil else {throw SessionLedgerError.wrongSourceEffect}
    } else {
      guard let revision=prior.lastKernelRevision,revision<HostSerial.maximum,p.kernel_state_revision.value==revision+1,
        [.done,.error].contains(p.terminal_status),r.result_id.value != nil else {throw SessionLedgerError.wrongSourceEffect}
      progress.acceptedCells+=1
      if p.terminal_status == .error {progress.errorCells+=1}
    }
    let chain=try RecoveryValidation.encode([prior.receiptChainHash,original.sourceEffectHash,original.rawReceiptHash])
    progress.receiptChainHash=KernelHashes.data(chain);progress.nextChild+=1
    progress.lastCheckpointID=r.checkpoint_id;progress.lastCheckpointHash=r.checkpoint_blob_hash;progress.lastKernelRevision=r.kernel_state_revision.value
    return progress
  }
  static func validate(_ input:SessionAppendInput,transition t:SessionRunTransition,events:[NativeSessionEvent])throws {
    guard t.codecVersion==1,input.turn != nil,input.task != nil,
      let group=events.first(where:{$0.event_id==t.groupAdmission && $0.event_kind == .tool_admitted}),
      group.task_id.value==input.task,group.agent_turn_id.value==input.turn,
      let parent=events.first(where:{$0.event_id==input.parentEvent}),parent.task_id.value==input.task,parent.agent_turn_id.value==input.turn else {throw SessionLedgerError.invalidTransition}
    let admission=try SessionValidation.payload(SessionToolAdmission.self,group)
    guard let plan=admission.run else {throw SessionLedgerError.invalidTransition}
    let old=try transition(parent)
    let before=try prior(parent,group:group,admission:admission)
    guard !events.contains(where:{$0.parent_event_id.value==parent.event_id && $0.event_kind == .task_transition}) else {throw SessionLedgerError.invalidTransition}
    var expected=before
    if t.kind == .run_finished {
      guard t.original==nil,t.recovered==false,let phase=t.phase,input.operation==admission.target.operationID,input.operationStore==admission.target.storeID else {throw SessionLedgerError.invalidTransition}
      let active=old.map { [.run_child_admitted,.run_child_started].contains($0.kind) } ?? false
      if phase == .unknown {
        guard active,t.childIndex==old?.childIndex,t.childAdmission==old?.childAdmission,t.code=="UNKNOWN_OUTCOME" else {throw SessionLedgerError.invalidTransition}
        expected.outcomeWasUnknown=true
      } else {
        guard !active,t.childIndex==nil,t.childAdmission==nil,old?.kind != .run_finished else {throw SessionLedgerError.invalidTransition}
        let allCells=plan.children.filter{$0.cellID != nil}.count
        let completed=before.nextChild==plan.children.count && before.acceptedCells==allCells && before.errorCells==0 && before.stoppedChildren==0 && plan.blockedCells.isEmpty
        switch phase {
        case .completed:guard completed,t.code==nil else {throw SessionLedgerError.invalidTransition}
        case .partial:guard before.acceptedCells>0,!completed,t.code.map(SourceValidation.identity) ?? true else {throw SessionLedgerError.invalidTransition}
        case .failed:guard before.acceptedCells==0,let code=t.code,SourceValidation.identity(code) else {throw SessionLedgerError.invalidTransition}
        case .cancelled:guard t.code=="CANCELLED" else {throw SessionLedgerError.invalidTransition}
        case .unknown:break
        }
      }
    } else {
      guard t.phase==nil,let i=t.childIndex,i==before.nextChild,plan.children.indices.contains(i),let childAdmit=t.childAdmission else {throw SessionLedgerError.invalidTransition}
      let child=plan.children[i],target=try target(admission,index:i)
      guard input.operation==target.operationID,input.operationStore==target.storeID else {throw SessionLedgerError.invalidTransition}
      switch t.kind {
      case .run_child_admitted:
        guard t.original==nil,t.code==nil,!t.recovered,childAdmit==input.eventID,
          parent.event_kind == .tool_started || old?.kind == .run_child_result || old?.kind == .run_child_stopped,
          !before.outcomeWasUnknown,plan.continueOnError || before.errorCells+before.stoppedChildren==0 && plan.blockedCells.isEmpty else {throw SessionLedgerError.invalidTransition}
      case .run_child_started:
        guard old?.kind == .run_child_admitted,childAdmit==parent.event_id,t.original==nil,t.code==nil,!t.recovered else {throw SessionLedgerError.invalidTransition}
      case .run_child_result,.run_child_stopped:
        guard let childEvent=events.first(where:{$0.event_id==childAdmit}),let childRecord=try transition(childEvent),
          childRecord.kind == .run_child_admitted,childRecord.groupAdmission==t.groupAdmission,childRecord.childIndex==i,
          old?.childAdmission==childAdmit,
          old?.kind == .run_child_started || t.recovered && (old?.kind == .run_child_admitted || old?.kind == .run_finished && old?.phase == .unknown) else {throw SessionLedgerError.invalidTransition}
        if t.kind == .run_child_result {
          guard let original=t.original,t.code==nil else {throw SessionLedgerError.invalidTransition}
          expected=try applying(original,to:before,child:child,plan:plan,target:target)
        } else {
          guard t.original==nil,!t.recovered,let code=t.code,SourceValidation.identity(code),old?.kind != .run_finished else {throw SessionLedgerError.invalidTransition}
          expected.nextChild+=1;expected.stoppedChildren+=1
        }
      case .run_finished:throw SessionLedgerError.invalidTransition
      }
    }
    guard expected==t.progress else {throw SessionLedgerError.invalidTransition}
  }
  static func recoveredPhase(_ snapshot:SessionRunSnapshot)->RecoveredToolPhase {
    guard let phase=snapshot.transition?.phase else {return .tool_outcome_unknown}
    switch phase {case .completed:return .completed;case .partial:return .partial;case .failed:return .failed;case .cancelled:return .cancelled;case .unknown:return .tool_outcome_unknown}
  }
}
