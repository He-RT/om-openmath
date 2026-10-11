import Foundation

/// Calibrate a native owner's monotonic epoch once; Rust independently validates actual deadlines.
final class SourceOwnerClock:@unchecked Sendable {
  private let lock=NSLock()
  private var nativeMS:UInt64=0
  private var receivedNS:UInt64=DispatchTime.now().uptimeNanoseconds
  func calibrate(_ value:UInt64) {lock.withLock {nativeMS=value;receivedNS=DispatchTime.now().uptimeNanoseconds}}
  func now()->UInt64 {lock.withLock {nativeMS+(DispatchTime.now().uptimeNanoseconds-receivedNS)/1_000_000}}
}
/// Native caller owns this physical port. A model passes only a preview_ref to the actual owner,
/// not SQL/source/grants/fences. IO waits never hold MainActor or native document/CAS locks.
actor DocCommitPort {
  private let client:NativeHostClient
  private let storage:StorageService
  private let drafts:DraftStore
  private let clock:SourceOwnerClock
  private var closing=false
  private var inFlight=Set<String>()
  private var preparing=Set<String>()
  private var activeCalls=0
  private var cancelRequests=Set<String>()
  private var acknowledgements:[String:[DraftAcknowledgement]]=[:]
  private var gates:[String:PhysicalCommitGate]=[:]
  private var blockedUnknown:String?
  private var admissionOnlyUnknown=Set<String>()
  init(client:NativeHostClient,storage:StorageService,drafts:DraftStore,clock:SourceOwnerClock) {
    self.client=client;self.storage=storage;self.drafts=drafts;self.clock=clock
  }
  private func beginCall() throws {
    guard !closing else {throw StorageError.closing}
    guard activeCalls<32 else {throw CommitPortError.operationInProgress}
    activeCalls+=1
  }
  /// Used before read/run/save/agent preview. Marked text is protected, invalid manual syntax allowed.
  @discardableResult func synchronizeDrafts(faults:StorageFaults = .init(),inputGroup:String?=nil,excludingMarked:Bool=false) async throws ->NativeCommitState? {
    try beginCall();defer {activeCalls-=1}
    guard !closing else {throw StorageError.closing}
    guard blockedUnknown==nil else {throw CommitPortError.unknownOutcome}
    let pending=try await drafts.pendingEdits(excludingMarked:excludingMarked)
    guard !pending.isEmpty else {return nil}
    let operations=pending.map { NativeSourceOperation.update_source_cell(.init(kind:.update_cell,cell:$0.0)) }
    let reply=try await client.sourceCommand(.source_prepare_manual(.init(type:.source_prepare_manual,operations:operations,input_group_id:inputGroup)))
    guard let plan=reply.plan.value else {throw NativeHostClientError.invalidReply}
    let committed=try await execute(plan,acknowledgements:pending.map(\.1),manual:true,faults:faults)
    guard committed.phase == .completed else {throw committed.phase == .cancelled ? CommitPortError.cancelled : CommitPortError.sourceConflict}
    return committed
  }
  func preview(_ input:NativePreviewInput) async throws ->NativePreviewData {
    try beginCall();defer {activeCalls-=1}
    try await synchronizeDrafts()
    let source=await drafts.confirmed
    let nativeState=try await drafts.editorState()
    _ = try await client.sourceCommand(.source_editor_changed(.init(type:.source_editor_changed,state:nativeState)))
    let reference=try await client.sourceCommand(.source_snapshot_ref(.init(type:.source_snapshot_ref,complete_cell_ids:source.file.cells.map(\.id))))
    guard let snapshotRef=reference.reference.value else {throw NativeHostClientError.invalidReply}
    let argument=NativePreviewArgs(snapshot_ref:snapshotRef,input:input)
    let json=try await Task.detached {try JSONDecoder().decode(HostJSONValue.self,from:JSONEncoder().encode(argument))}.value
    let reply=try await client.sourceCommand(.source_preview(.init(type:.source_preview,arguments:json)))
    guard let value=reply.preview.value else {throw NativeHostClientError.invalidReply}
    return try await Task.detached {try JSONDecoder().decode(NativePreviewData.self,from:JSONEncoder().encode(value))}.value
  }
  /// Duplicate/ref stale reconciliation precedes any new permission to write; actual authority
  func undo(transactions:[String],group:String,operation:String,faults:StorageFaults = .init()) async throws ->NativeCommitState {
    try beginCall();defer {activeCalls-=1}
    guard !closing else {throw StorageError.closing}
    guard !inFlight.contains(operation),!preparing.contains(operation) else {throw CommitPortError.operationInProgress}
    preparing.insert(operation);defer {preparing.remove(operation)}
    let source=await drafts.confirmed
    if let actual=try await storage.sourceReceipt(document:source.document_id,operation:operation) {
      guard actual.receipt.operation_kind == .undo,actual.receipt.transaction_id.value != nil else {throw StorageError.idempotencyConflict}
      let metadata=try await storage.sourceUndoMetadata(document:source.document_id,operation:operation)
      guard let metadata,metadata.group_id==group,
        Set(metadata.transaction_ids)==Set(transactions),metadata.transaction_ids.count==transactions.count else {throw StorageError.idempotencyConflict}
      let reply=try await client.sourceCommand(.source_recover_undo(.init(type:.source_recover_undo,receipt:actual)))
      guard let state=reply.operation.value else {throw NativeHostClientError.invalidReply}
      if let snapshot=reply.snapshot.value {try await drafts.acknowledge(snapshot)}
      if blockedUnknown==operation {blockedUnknown=nil}
      acknowledgements.removeValue(forKey:operation);admissionOnlyUnknown.remove(operation);cancelRequests.remove(operation)
      return state
    }
    try await synchronizeDrafts()
    let records=try await storage.sourceTransactions(document:source.document_id,ids:transactions,faults:faults)
    let reply=try await client.sourceCommand(.source_prepare_undo(.init(type:.source_prepare_undo,operation_id:operation,group_id:group,records:records)))
    guard let plan=reply.plan.value else {
      if let state=reply.operation.value {return state}
      throw NativeHostClientError.invalidReply
    }
    guard !inFlight.contains(operation) else {throw CommitPortError.operationInProgress}
    return try await execute(plan,acknowledgements:[],manual:false,faults:faults)
  }
  /// Duplicate/ref stale reconciliation precedes any new permission to write; actual authority
  /// keeps the original operation mapping. No source is reassembled from current drafts here.
  func apply(_ previewRef:String,faults:StorageFaults = .init()) async throws ->NativeCommitState {
    try beginCall();defer {activeCalls-=1}
    guard !closing else {throw StorageError.closing}
    let reply=try await client.sourceCommand(.source_begin(.init(type:.source_begin,preview_ref:previewRef)))
    if reply.plan.value==nil,let state=reply.operation.value {
      guard !inFlight.contains(state.operation_id) else {throw CommitPortError.operationInProgress}
      if state.phase == .completed {return try await reconcile(state.operation_id)}
      if state.phase == .cancelled || state.phase == .failed {return state}
      throw CommitPortError.unknownOutcome
    }
    guard let plan=reply.plan.value else {throw NativeHostClientError.invalidReply}
    guard !inFlight.contains(plan.commit.operation_id) else {throw CommitPortError.operationInProgress}
    if let receipt=try await storage.sourceReceipt(document:plan.commit.document_id,operation:plan.commit.operation_id) {
      let actual=try await client.sourceCommand(.source_completed(.init(type:.source_completed,operation_id:plan.commit.operation_id,receipt:receipt)))
      if let source=actual.snapshot.value {try await drafts.acknowledge(source,ownDrafts:acknowledgements[plan.commit.operation_id] ?? [])}
      acknowledgements.removeValue(forKey:plan.commit.operation_id)
      if blockedUnknown==plan.commit.operation_id {blockedUnknown=nil}
      guard let state=actual.operation.value else {throw NativeHostClientError.invalidReply};return state
    }
    guard blockedUnknown==nil else {throw CommitPortError.unknownOutcome}
    return try await execute(plan,acknowledgements:[],manual:false,faults:faults)
  }
  private func execute(_ plan:NativeSourceCommit,acknowledgements:[DraftAcknowledgement],manual:Bool,faults:StorageFaults) async throws ->NativeCommitState {
    guard !closing else {throw StorageError.closing}
    let operation=plan.commit.operation_id
    guard !inFlight.contains(operation) else {throw CommitPortError.operationInProgress}
    inFlight.insert(operation);self.acknowledgements[operation]=acknowledgements
    defer {inFlight.remove(operation);if blockedUnknown != operation {self.acknowledgements.removeValue(forKey:operation);cancelRequests.remove(operation)}}
    let runtime=client.runtimeInstanceID
    if cancelRequests.contains(operation) {_ = try? await client.cancel(operation:operation)}
    let admitted:NativeSourceHostReply
    do {
      let admission=try await storage.admitSource(plan,runtime:runtime,faults:faults)
      admitted=try await client.sourceCommand(.source_admitted(.init(type:.source_admitted,operation_id:operation,admission:admission)))
    } catch {
      // No source COMMIT has started, but durable admission may have lost its acknowledgement.
      // Keep the original ID and gate for explicit readback; never manufacture a fresh retry.
      blockedUnknown=operation
      admissionOnlyUnknown.insert(operation)
      _ = try? await client.sourceCommand(.source_unknown(.init(type:.source_unknown,operation_id:operation)))
      throw CommitPortError.unknownOutcome
    }
    if let state=admitted.operation.value,state.phase == .completed {
      if let source=admitted.snapshot.value {try await drafts.acknowledge(source,ownDrafts:acknowledgements)}
      self.acknowledgements.removeValue(forKey:operation);return state
    }
    if let state=admitted.operation.value,state.phase == .cancelled || state.phase == .failed {
      return try await settleNoCommit(operation,document:plan.commit.document_id,cancelled:state.phase == .cancelled)
    }
    guard admitted.operation.value?.phase == .awaiting_fence else {
      blockedUnknown=operation;throw CommitPortError.unknownOutcome
    }
    let fence:DraftFence
    let barrier:@Sendable ()throws->Void
    var acquiredFence:String?
    do {
    let read=try await client.sourceCommand(.source_read(.init(type:.source_read)))
    clock.calibrate(read.owner_time_ms.value)
    let targets=plan.commit.changed_cell_ids.filter {id in plan.before.file.cells.contains{$0.id.utf8.elementsEqual(id.utf8)}}
      fence=try await drafts.acquireFence(before:plan.before,targets:targets,manualAfter:manual ? plan.after : nil)
    acquiredFence=fence.value.fence_id
    gates[operation]=fence.gate
    if cancelRequests.contains(operation) {fence.gate.cancel()}
    _ = try await client.sourceCommand(.source_fenced(.init(type:.source_fenced,operation_id:operation,fence:fence.value)))
      barrier=try await client.sourceBarrier(operation:operation,fenceID:fence.value.fence_id)
    } catch {
      gates.removeValue(forKey:operation)
      if let acquiredFence {await drafts.releaseFence(acquiredFence)}
      // The physical source transaction has not begun. Admission cancellation/failure itself is
      // a durable fact, and does not pretend that a partially committed source was rolled back.
      _ = try await settleNoCommit(operation,document:plan.commit.document_id,cancelled:cancelRequests.contains(operation))
      throw error
    }
    defer {gates.removeValue(forKey:operation);Task {await drafts.releaseFence(fence.value.fence_id)}}
    let controls=SourceCommitControls(runtime:runtime,gate:fence.gate,ownerBarrier:barrier)
    do {
      let receipt=try await storage.commitSource(plan,faults:faults,controls:controls)
      let result=try await client.sourceCommand(.source_completed(.init(type:.source_completed,operation_id:operation,receipt:receipt)))
      guard let source=result.snapshot.value,let state=result.operation.value,state.phase == .completed else {throw NativeHostClientError.invalidReply}
      try await drafts.acknowledge(source,ownDrafts:acknowledgements)
      self.acknowledgements.removeValue(forKey:operation)
      return state
    } catch StorageError.unknownCommit {
      blockedUnknown=operation
      _ = try? await client.sourceCommand(.source_unknown(.init(type:.source_unknown,operation_id:operation)))
      throw CommitPortError.unknownOutcome
    } catch {
      // A rejection before the barrier is known rollback. After barrier/COMMIT, verify the
      // original physical receipt before claiming no effects or releasing a dependent gate.
      if fence.gate.entered {
        blockedUnknown=operation
        _ = try? await client.sourceCommand(.source_unknown(.init(type:.source_unknown,operation_id:operation)))
        throw CommitPortError.unknownOutcome
      }
      _ = try await settleNoCommit(operation,document:plan.commit.document_id,cancelled:fence.gate.wasCancelled)
      throw error
    }
  }
  private func settleNoCommit(_ operation:String,document:String,cancelled:Bool) async throws ->NativeCommitState {
    do {
      _ = try await storage.settleSourceAdmission(document:document,operation:operation,cancelled:cancelled)
      let actual=try await client.sourceCommand(.source_settled(.init(type:.source_settled,operation_id:operation,cancelled:cancelled)))
      guard let state=actual.operation.value else {throw NativeHostClientError.invalidReply}
      return state
    } catch {
      blockedUnknown=operation
      _ = try? await client.sourceCommand(.source_unknown(.init(type:.source_unknown,operation_id:operation)))
      throw CommitPortError.unknownOutcome
    }
  }
  /// Synchronous cancellation signal reaches Swift fence now; native operation stop is independent
  /// of the writer queue. A committing state stays pending until the actual receipt is known.
  func activeOperations()->[String] {Array(inFlight.union(preparing)).sorted()}
  func unresolvedOperation()->String? {blockedUnknown}
  func cancel(_ operation:String) async {
    guard inFlight.contains(operation) || preparing.contains(operation) || blockedUnknown==operation else {return}
    cancelRequests.insert(operation)
    gates[operation]?.cancel()
    _ = try? await client.cancel(operation:operation)
  }
  /// Close native document work before consuming its Host handle or storage connections.
  /// IO owns its original resource references until all active operations actually return.
  func close() async {
    closing=true
    for id in Array(inFlight.union(preparing)) {await cancel(id)}
    while !inFlight.isEmpty || activeCalls>0 {try? await Task.sleep(for:.milliseconds(5))}
    await drafts.invalidateDocument()
  }
  func reconcile(_ operation:String) async throws ->NativeCommitState {
    try beginCall();defer {activeCalls-=1}
    guard !inFlight.contains(operation) else {throw CommitPortError.operationInProgress}
    let source=await drafts.confirmed
    if let receipt=try await storage.sourceReceipt(document:source.document_id,operation:operation) {
      let result=try await client.sourceCommand(.source_completed(.init(type:.source_completed,operation_id:operation,receipt:receipt)))
      guard let snapshot=result.snapshot.value,let state=result.operation.value else {throw NativeHostClientError.invalidReply}
      try await drafts.acknowledge(snapshot,ownDrafts:acknowledgements[operation] ?? []);acknowledgements.removeValue(forKey:operation)
      admissionOnlyUnknown.remove(operation)
      cancelRequests.remove(operation)
      if blockedUnknown==operation {blockedUnknown=nil}
      return state
    }
    let status=try await client.sourceCommand(.source_status(.init(type:.source_status,operation_id:operation)))
    if admissionOnlyUnknown.contains(operation) {
      // This live port knows source IO never began. Verify healthy durable admission after the
      // lost ACK, then settle the same ID. Restart recovery cannot rely on this in-memory fact.
      let admitted=try await storage.sourceAdmission(document:source.document_id,operation:operation)
      guard admitted?.phase != .completed else {throw CommitPortError.unknownOutcome}
      let settled:NativeCommitState
      if admitted != nil {
        settled=try await settleNoCommit(operation,document:source.document_id,cancelled:status.operation.value?.cancel_requested == true)
      } else {
        let reply=try await client.sourceCommand(.source_settled(.init(type:.source_settled,operation_id:operation,cancelled:status.operation.value?.cancel_requested == true)))
        guard let state=reply.operation.value else {throw NativeHostClientError.invalidReply};settled=state
      }
      admissionOnlyUnknown.remove(operation);acknowledgements.removeValue(forKey:operation)
      cancelRequests.remove(operation)
      if blockedUnknown==operation {blockedUnknown=nil}
      return settled
    }
    if let state=status.operation.value,state.phase == .cancelled {
      let actual=try await settleNoCommit(operation,document:source.document_id,cancelled:true)
      if blockedUnknown==operation {blockedUnknown=nil}
      return actual
    }
    // Missing healthy receipt is reported separately; restarting a new operation is forbidden.
    throw CommitPortError.unknownOutcome
  }
}
