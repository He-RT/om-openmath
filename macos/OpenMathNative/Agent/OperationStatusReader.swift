import Foundation

/// Reads only managed current-document owners. No model input supplies a path, store or receipt.
struct OperationStatusReader:Sendable {
  let storage:StorageService
  var kernel:KernelCommitPort?=nil
  private func reconcileOriginal(_ operation:String)async throws {
    guard let kernel else {return}
    let state:NativeKernelHostReply
    do {state=try await kernel.status(operation)}
    catch NativeHostClientError.rejected(let error) where error.code == .invalid_reference {return}
    guard state.operation_id.value==operation else {throw SessionLedgerError.wrongSourceEffect}
    if [.unknown,.committing,.completed].contains(state.phase) {_ = try await kernel.reconcile(operation)}
  }
  func read(_ lookup:NativeOperationLookup)async throws->NativeOperationObservation {
    let identity=try await storage.documentIdentity(lookup.document_id)
    guard identity.storeID==lookup.store_id else {throw SessionLedgerError.wrongSourceEffect}
    func value(source:NativeDurableSourceReceipt?=nil,kernel:NativeKernelReceipt?=nil,run:NativeOperationRunObservation?=nil,claim:NativeOperationClaimObservation?=nil)->NativeOperationObservation {
      .init(operation_ref:lookup.operation_ref,document_id:lookup.document_id,store_id:lookup.store_id,source_receipt:.init(source),kernel_receipt:.init(kernel),run:.init(run),claim:.init(claim))
    }
    if let receipt=try await storage.sourceReceipt(document:lookup.document_id,operation:lookup.operation_ref) {return value(source:receipt)}
    if let receipt=try await storage.kernelReceipt(document:lookup.document_id,operation:lookup.operation_ref) {try await reconcileOriginal(lookup.operation_ref);return value(kernel:receipt)}
    guard let claim=try await storage.operationClaim(document:lookup.document_id,store:lookup.store_id,operation:lookup.operation_ref) else {return value()}
    if let admission=claim.runAdmission {
      let before=try await storage.sessionRunSnapshot(claim.event.event.agent_session_id,admission:admission)
      if !(await storage.sessionRunIsLive(claim.event.event.agent_session_id,admission:admission)),let active=before.summary.originalActiveOperation {try await reconcileOriginal(active)}
      let ledger=SessionLedgerPort(storage:storage,session:claim.event.event.agent_session_id)
      // This recovery method has no executor/callback. Live groups are pinned and left untouched.
      let snapshot=try await SessionRunPort(storage:storage,ledger:ledger).reconcile(admission:admission)
      guard let plan=snapshot.admission.run,snapshot.admission.target.operationID==lookup.operation_ref,
        let latest=try await storage.sessionEvent(ledger.session,event:snapshot.latest.event_id) else {throw SessionLedgerError.wrongSourceEffect}
      let live=await storage.sessionRunIsLive(ledger.session,admission:admission)
      let state:NativeOperationRunObservationPhase
      if let terminal=snapshot.transition?.phase {state=NativeOperationRunObservationPhase(rawValue:terminal.rawValue)!}
      else {state=live ? (snapshot.latest.event_kind == .tool_admitted ? .queued:.running):.unknown}
      var receipts:[NativeKernelReceipt]=[],effects=snapshot.progress.acceptedCells
      if plan.initialCheckpointID==nil,let first=plan.children.first,first.cellID==nil,snapshot.progress.nextChild>0,
        let bootstrap=try await storage.kernelReceipt(document:lookup.document_id,operation:first.operationID) {
        guard bootstrap.producer.cell_id.value==nil else {throw SessionLedgerError.wrongSourceEffect};effects+=1
      }
      for child in plan.children.prefix(snapshot.progress.nextChild).reversed() where child.cellID != nil {
        if let receipt=try await storage.kernelReceipt(document:lookup.document_id,operation:child.operationID) {
          guard receipt.producer.cell_id.value==child.cellID,receipt.producer.cell_source_hash.value==child.cellSourceHash else {throw SessionLedgerError.wrongSourceEffect}
          receipts.append(receipt);if receipts.count==32 {break}
        }
      }
      let fact=NativeOperationRunObservation(phase:state,accepted_cells:try HostSerial(UInt64(snapshot.progress.acceptedCells)),accepted_effects:try HostSerial(UInt64(effects)),original_active_operation:.init(snapshot.summary.originalActiveOperation),event:.init(event_id:latest.event.event_id,event_sequence:try HostSerial(latest.receipt.eventSequence),receipt_hash:latest.receipt.recordHash),kernel_receipts:Array(receipts.reversed()))
      return value(run:fact)
    }
    return value(claim:.init(phase:.unknown,event:.init(event_id:claim.event.event.event_id,event_sequence:try HostSerial(claim.event.receipt.eventSequence),receipt_hash:claim.event.receipt.recordHash)))
  }
}
