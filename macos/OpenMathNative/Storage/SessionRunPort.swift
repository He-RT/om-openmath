import Foundation

enum SessionRunNativeOutcome:Sendable {case accepted,stopped(String)}
/// Durable group/child producer. Only a fresh group claim permits a computation callback.
/// Reconciliation is a different method with no executor, kernel, helper or recovered grant.
actor SessionRunPort {
  private let storage:StorageService,ledger:SessionLedgerPort
  private var current:SessionRunSnapshot?,liveAdmission:String?,busy=false
  private var lifetime:SessionRunLifetime?
  init(storage:StorageService,ledger:SessionLedgerPort) {self.storage=storage;self.ledger=ledger}
  func begin(_ admission:SessionToolAdmission,id:String,turn:String,task:String,faults:StorageFaults = .init())async throws->SessionRunSnapshot {
    try Task.checkCancellation()
    guard !busy,liveAdmission==nil,admission.run != nil else {throw StorageError.inUse}
    busy=true;defer {busy=false}
    let identity=try await storage.documentIdentity(admission.target.documentID)
    guard identity.storeID==admission.target.storeID else {throw SessionLedgerError.wrongSourceEffect}
    let pin=SessionRunLifetime()
    try await storage.pinSessionRun(ledger.session,admission:id,lifetime:pin)
    do {
    let actual=try await ledger.record(admission,id:id,kind:.tool_admitted,parent:admission.proposalEvent,turn:turn,task:task,target:admission.target,faults:faults)
    guard actual.inserted else {throw SessionLedgerError.reconciliationRequired}
    try faults.reach("run_group_admission_acknowledged")
    try Task.checkCancellation()
    let started=try await ledger.record(SessionToolStarted(codecVersion:1,admissionEvent:id),id:"run-started-"+admission.target.operationID,kind:.tool_started,parent:id,turn:turn,task:task,target:admission.target,faults:faults)
    guard started.inserted else {throw SessionLedgerError.reconciliationRequired}
    try faults.reach("run_group_execution_claimed")
    let snapshot=SessionRunSnapshot(admissionEvent:actual.stored.event,admission:admission,latest:started.stored.event,transition:nil,progress:.init(plan:admission.run!))
    current=snapshot;liveAdmission=id;lifetime=pin;return snapshot
    } catch {await storage.unpinSessionRun(ledger.session,admission:id,lifetime:pin);throw error}
  }
  private func record(_ t:SessionRunTransition,id:String,parent:SessionRunSnapshot,target:SessionOperationTarget,faults:StorageFaults)async throws->SessionRunSnapshot {
    let actual=try await ledger.record(t,id:id,kind:.task_transition,parent:parent.latest.event_id,turn:parent.admissionEvent.agent_turn_id.value,task:parent.admissionEvent.task_id.value,target:target,faults:faults)
    guard actual.inserted else {throw SessionLedgerError.reconciliationRequired}
    let next=SessionRunSnapshot(admissionEvent:parent.admissionEvent,admission:parent.admission,latest:actual.stored.event,transition:t,progress:t.progress)
    current=next;return next
  }
  /// Native adapter acknowledges its frozen child to Rust only inside execute, after both ACKs.
  /// If any ACK is lost, this method never invokes that callback again under any identity.
  func performChild(index:Int,faults:StorageFaults = .init(),execute:@escaping @Sendable (SessionRunChild)async throws->SessionRunNativeOutcome)async throws->SessionRunSnapshot {
    do {try Task.checkCancellation()} catch {await revokeLifetime();throw error}
    guard !busy,let start=current,liveAdmission==start.admissionEvent.event_id,
      start.activeChild==nil,start.transition?.phase==nil,index==start.progress.nextChild,
      let plan=start.admission.run,plan.children.indices.contains(index) else {throw SessionLedgerError.reconciliationRequired}
    busy=true;defer {busy=false}
    let child=plan.children[index],target=try SessionRunSemantics.target(start.admission,index:index),admitID="run-admit-"+child.operationID
    do {
      let admit=SessionRunTransition(kind:.run_child_admitted,codecVersion:1,groupAdmission:start.admissionEvent.event_id,childIndex:index,childAdmission:admitID,original:nil,code:nil,phase:nil,recovered:false,progress:start.progress)
      let admitted=try await record(admit,id:admitID,parent:start,target:target,faults:faults)
      try faults.reach("run_child_admission_acknowledged")
      try Task.checkCancellation()
      let claim=SessionRunTransition(kind:.run_child_started,codecVersion:1,groupAdmission:start.admissionEvent.event_id,childIndex:index,childAdmission:admitID,original:nil,code:nil,phase:nil,recovered:false,progress:start.progress)
      let started=try await record(claim,id:"run-start-"+child.operationID,parent:admitted,target:target,faults:faults)
      try faults.reach("run_child_execution_claimed")
      try Task.checkCancellation()
      guard try await storage.originalOperationEffect(target)==nil else {throw SessionLedgerError.reconciliationRequired}
      let outcome=try await execute(child)
      try faults.reach("run_child_effect_returned")
      switch outcome {
      case .accepted:
        guard let original=try await storage.originalOperationEffect(target) else {throw SessionLedgerError.reconciliationRequired}
        return try await settle(started,original:original,recovered:false,faults:faults)
      case .stopped(let code):
        guard try await storage.originalOperationEffect(target)==nil else {throw SessionLedgerError.reconciliationRequired}
        var progress=started.progress;progress.nextChild+=1;progress.stoppedChildren+=1
        let t=SessionRunTransition(kind:.run_child_stopped,codecVersion:1,groupAdmission:start.admissionEvent.event_id,childIndex:index,childAdmission:admitID,original:nil,code:code,phase:nil,recovered:false,progress:progress)
        return try await record(t,id:"run-stopped-"+child.operationID,parent:started,target:target,faults:faults)
      }
    } catch {await revokeLifetime();throw error}
  }
  private func revokeLifetime()async {
    if let lifetime,let admission=liveAdmission {await storage.unpinSessionRun(ledger.session,admission:admission,lifetime:lifetime)}
    liveAdmission=nil;lifetime=nil
  }
  private func settle(_ snapshot:SessionRunSnapshot,original:DocumentEffect,recovered:Bool,faults:StorageFaults)async throws->SessionRunSnapshot {
    guard let index=snapshot.activeChild,let childAdmission=snapshot.transition?.childAdmission,let plan=snapshot.admission.run else {throw SessionLedgerError.invalidTransition}
    let target=try SessionRunSemantics.target(snapshot.admission,index:index)
    _ = try SessionSemantics.effectOutcome(original,target:target)
    _ = try await storage.projectDocumentEffect(original,faults:faults)
    let receipt=try JSONDecoder().decode(NativeKernelReceipt.self,from:original.payload)
    let actual=SessionRunOriginalReceipt(sourceEventID:original.eventID,sourceEffectHash:try original.stableHash,rawReceiptHash:original.rawReceiptHash,receipt:receipt)
    let progress=try SessionRunSemantics.applying(actual,to:snapshot.progress,child:plan.children[index],plan:plan,target:target)
    let t=SessionRunTransition(kind:.run_child_result,codecVersion:1,groupAdmission:snapshot.admissionEvent.event_id,childIndex:index,childAdmission:childAdmission,original:actual,code:nil,phase:nil,recovered:recovered,progress:progress)
    let next=try await record(t,id:"run-result-"+target.operationID,parent:snapshot,target:target,faults:faults)
    try faults.reach("run_child_receipt_acknowledged")
    return next
  }
  func finish(_ phase:SessionRunPhase,code:String?=nil,faults:StorageFaults = .init())async throws->SessionRunSnapshot {
    guard !busy,let snapshot=current else {throw SessionLedgerError.reconciliationRequired}
    busy=true;defer {busy=false}
    do {let result=try await finish(snapshot,phase:phase,code:code,faults:faults);await revokeLifetime();return result}
    catch {await revokeLifetime();throw error}
  }
  private func finish(_ snapshot:SessionRunSnapshot,phase:SessionRunPhase,code:String?,faults:StorageFaults)async throws->SessionRunSnapshot {
    var progress=snapshot.progress
    if phase == .unknown {progress.outcomeWasUnknown=true}
    let t=SessionRunTransition(kind:.run_finished,codecVersion:1,groupAdmission:snapshot.admissionEvent.event_id,
      childIndex:phase == .unknown ? snapshot.activeChild:nil,childAdmission:phase == .unknown ? snapshot.transition?.childAdmission:nil,
      original:nil,code:code,phase:phase,recovered:false,progress:progress)
    let result=try await record(t,id:"run-finished-"+snapshot.admission.target.operationID+"-"+phase.rawValue,parent:snapshot,target:snapshot.admission.target,faults:faults)
    try faults.reach("run_group_outcome_acknowledged");return result
  }
  /// Read originals only. An unfinished child remains unknown if there is no actual receipt.
  /// This function cannot acknowledge admission to an executor or invoke a computation callback.
  func reconcile(admission:String,faults:StorageFaults = .init())async throws->SessionRunSnapshot {
    guard !busy else {throw StorageError.inUse};busy=true;defer {busy=false}
    var snapshot=try await storage.sessionRunSnapshot(ledger.session,admission:admission)
    if await storage.sessionRunIsLive(ledger.session,admission:admission) {return snapshot}
    current=snapshot;liveAdmission=nil;lifetime=nil
    if let phase=snapshot.transition?.phase,phase != .unknown {return snapshot}
    if let index=snapshot.activeChild {
      let target=try SessionRunSemantics.target(snapshot.admission,index:index)
      guard let original=try await storage.reconcileOriginalOperationEffect(target) else {return snapshot}
      snapshot=try await settle(snapshot,original:original,recovered:true,faults:faults)
    }
    guard snapshot.latest.event_kind != .tool_admitted else {return snapshot}
    let run=snapshot.admission.run!,p=snapshot.progress
    let complete=p.nextChild==run.children.count && p.errorCells==0 && p.stoppedChildren==0 && run.blockedCells.isEmpty
    return try await finish(snapshot,phase:complete ? .completed:p.acceptedCells>0 ? .partial:.failed,
      code:complete ? nil:"RECOVERED_INTERRUPTED",faults:faults)
  }
}
