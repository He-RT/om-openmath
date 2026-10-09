import Foundation

/// Native editing and owner cancel share this short lock with the final COMMIT admission.
/// MainActor never waits here for SQLite/fsync. The barrier closure may only call short native FFI.
final class PhysicalCommitGate:@unchecked Sendable {
  enum Phase:Sendable {case ready,committing,settled}
  private let lock=NSLock()
  private var phase:Phase = .ready
  private var invalid=false
  private var cancelRequested=false
  private let deadlineMS:UInt64
  private let now:@Sendable ()->UInt64
  init(deadlineMS:UInt64,now:@escaping @Sendable ()->UInt64) {self.deadlineMS=deadlineMS;self.now=now}
  func invalidateForNewInput() {lock.withLock {if phase == .ready {invalid=true}}}
  func cancel() {lock.withLock {cancelRequested=true}}
  func enter(_ ownerBarrier:()throws->Void) throws {
    try lock.withLock {
      guard phase == .ready else {throw StorageError.corruptIdentity}
      if cancelRequested {throw CommitPortError.cancelled}
      guard !invalid,now()<deadlineMS else {throw CommitPortError.editingBusy}
      // The native owner atomically checks operation/source/grants/cancel, without disk/parse work.
      try ownerBarrier();phase = .committing
    }
  }
  func settle() {lock.withLock {phase = .settled}}
  var entered:Bool {lock.withLock {phase != .ready}}
  var wasCancelled:Bool {lock.withLock {cancelRequested}}
}
enum CommitPortError:Error,Sendable {case editingBusy,cancelled,invalidFence,unknownOutcome,sourceConflict,operationInProgress}
struct SourceCommitControls:Sendable {
  let runtime:String
  let gate:PhysicalCommitGate
  let ownerBarrier:@Sendable ()throws->Void
}
