import Foundation

/// Ephemeral liveness only, never serialized or usable as a mathematical/write grant.
final class SessionRunLifetime:Sendable {}
/// Native-only frozen execution facts. Decoding this record cannot recreate an executor/grant.
struct SessionRunChild:Codable,Sendable,Equatable {
  let operationID:String,cellID:String?,cellSourceHash:String?
}
struct SessionRunBlockedCell:Codable,Sendable,Equatable {let cellID:String,code:String}
struct SessionRunPlan:Codable,Sendable {
  let codecVersion:Int
  let scope:String,requestedCellIDs:[String],children:[SessionRunChild],blockedCells:[SessionRunBlockedCell]
  let continueOnError:Bool
  let documentGeneration:UInt64,sourceRevision:UInt64,executionEpoch:UInt64,configRevision:UInt64
  let kernelBuild:String,generalHash:String
  let initialCheckpointID:String?,initialCheckpointHash:String?,initialKernelRevision:UInt64?
}
enum SessionRunTransitionKind:String,Codable,Sendable {
  case run_child_admitted,run_child_started,run_child_result,run_child_stopped,run_finished
}
enum SessionRunPhase:String,Codable,Sendable {case completed,partial,failed,cancelled,unknown}
/// Running receipt chain is checked against the preceding immutable event and real effects.
struct SessionRunProgress:Codable,Sendable,Equatable {
  var nextChild:Int,acceptedCells:Int,errorCells:Int,stoppedChildren:Int
  var receiptChainHash:String,lastCheckpointID:String?,lastCheckpointHash:String?,lastKernelRevision:UInt64?
  var outcomeWasUnknown:Bool
  init(plan:SessionRunPlan) {
    nextChild=0;acceptedCells=0;errorCells=0;stoppedChildren=0
    receiptChainHash=KernelHashes.data(Data("openmath/run-receipts/v1".utf8))
    lastCheckpointID=plan.initialCheckpointID;lastCheckpointHash=plan.initialCheckpointHash;lastKernelRevision=plan.initialKernelRevision
    outcomeWasUnknown=false
  }
}
struct SessionRunOriginalReceipt:Codable,Sendable {
  let sourceEventID:String,sourceEffectHash:String,rawReceiptHash:String
  let receipt:NativeKernelReceipt
}
struct SessionRunTransition:Codable,Sendable {
  let kind:SessionRunTransitionKind,codecVersion:Int,groupAdmission:String
  let childIndex:Int?,childAdmission:String?
  let original:SessionRunOriginalReceipt?
  let code:String?,phase:SessionRunPhase?,recovered:Bool
  let progress:SessionRunProgress
}
struct RecoveredRunSummary:Codable,Sendable {
  let phase:SessionRunPhase?,acceptedCells:Int,errorCells:Int,stoppedChildren:Int,nextChild:Int,totalChildren:Int
  let originalActiveOperation:String?,receiptChainHash:String,outcomeWasUnknown:Bool
}
struct SessionRunSnapshot:Sendable {
  let admissionEvent:NativeSessionEvent,admission:SessionToolAdmission,latest:NativeSessionEvent
  let transition:SessionRunTransition?,progress:SessionRunProgress
  var activeChild:Int? {
    guard let transition else {return nil}
    switch transition.kind {
    case .run_child_admitted,.run_child_started:return transition.childIndex
    case .run_finished:return transition.phase == .unknown ? transition.childIndex:nil
    default:return nil
    }
  }
  var summary:RecoveredRunSummary {
    let run=admission.run!
    return .init(phase:transition?.phase,acceptedCells:progress.acceptedCells,errorCells:progress.errorCells,stoppedChildren:progress.stoppedChildren,
      nextChild:progress.nextChild,totalChildren:run.children.count,originalActiveOperation:activeChild.map {run.children[$0].operationID},
      receiptChainHash:progress.receiptChainHash,outcomeWasUnknown:progress.outcomeWasUnknown)
  }
}
