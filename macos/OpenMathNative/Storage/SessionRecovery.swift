import Foundation

extension RecoveryService {
  /// Caller opens the known document writer under the current root. Missing/invalid locators
  /// stay unknown. Recovery never has a tool/HTTP callback and never restarts a request.
  func recoverSession(_ session:String,ledger:SessionLedgerPort,faults:StorageFaults = .init()) async throws->SessionRecoveryPage {
    let current=try await storage.sessionRecovery(session)
    guard current.complete else {return current}
    for tool in current.tools where tool.phase == .tool_outcome_unknown {
      guard let target=tool.target,let admittedID=tool.admissionEvent,
        let admissionEvent=try await storage.sessionEvent(session,event:admittedID) else {continue}
      do {
        guard let original=try await storage.originalOperationEffect(target) else {continue}
        _ = try await storage.projectDocumentEffect(original,faults:faults)
        let result=SessionToolResult(codecVersion:1,admissionEvent:admittedID,sourceEventID:original.eventID,sourceEffectHash:try original.stableHash,rawReceiptHash:original.rawReceiptHash,recovered:true,outcome:try SessionSemantics.effectOutcome(original,target:target))
        _ = try await ledger.record(result,id:"recovered-"+admittedID,kind:.tool_result,parent:admittedID,turn:admissionEvent.event.agent_turn_id.value,task:admissionEvent.event.task_id.value,target:target,faults:faults)
      } catch StorageError.closing {continue}
    }
    return try await storage.sessionRecovery(session)
  }
}
