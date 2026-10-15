import Foundation

struct SessionOperationClaim:Sendable {
  let event:StoredSessionEvent,target:SessionOperationTarget,runAdmission:String?
}
