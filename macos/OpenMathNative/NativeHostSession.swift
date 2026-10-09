import Foundation

enum NativeHostSessionError:Error,Sendable {
  case admissionClosed, duplicateOperation, tooManyOperations, tooManySubscribers, staleEvent, unknownOutcome(String)
}
/// One event consumer, bounded request correlation and full confirmed snapshots for UI subscribers.
/// Slow UI never owns terminal delivery: awaiters and the Rust registry settle independently.
actor NativeHostSession {
  private let client:NativeHostClient
  private var router:AppEventRouter
  private var pending:[String:Pending]=[:]
  private var subscribers:[UUID:AsyncStream<HostConfirmedProjection>.Continuation]=[:]
  private var pumpTask:Task<Void,Never>?
  private let eventByteBudget:Int
  private var shutdownTask:Task<Void,any Error>?
  private var closing=false
  private var recovering=false
  private var recoveryRevision:UInt64=0
  private var lastInvalidatedRustSequence:UInt64=0
  private struct Pending {
    let request:RequestEnvelope
    let continuation:CheckedContinuation<HostOperationStatus,any Error>
    var admitted=false
    var stopRequested=false
    var stopIssued=false
    var early:HostOperationStatus?
  }
  private init(client:NativeHostClient,runtime:String,eventByteBudget:Int) {
    self.eventByteBudget=eventByteBudget
    self.client=client; router=AppEventRouter(runtime:runtime)
    router.registerLocalProducer("draft",identity:runtime,generation:1)
  }
  static func open(runtime:String=UUID().uuidString.lowercased(),eventCapacity:UInt32=128,eventByteBudget:Int=512*1024) async throws -> NativeHostSession {
    let client=try await NativeHostClient.open(runtime:runtime,eventCapacity:eventCapacity)
    let session=NativeHostSession(client:client,runtime:runtime,eventByteBudget:eventByteBudget)
    try await session.recover()
    await session.startPump()
    return session
  }
  private func startPump() {
    pumpTask=Task { [weak self] in await self?.pump() }
  }
  func projection()->HostConfirmedProjection { router.projection }
  func updates() throws ->AsyncStream<HostConfirmedProjection> {
    guard subscribers.count<8 else { throw NativeHostSessionError.tooManySubscribers }
    let id=UUID()
    let stream=AsyncStream<HostConfirmedProjection>(bufferingPolicy:.bufferingNewest(1)) { continuation in
      subscribers[id]=continuation
      continuation.yield(router.projection)
      continuation.onTermination={ [weak self] _ in Task { await self?.removeSubscriber(id) } }
    }
    return stream
  }
  private func removeSubscriber(_ id:UUID) { subscribers.removeValue(forKey:id) }
  func noteDraft(sequence:UInt64) throws {
    guard !closing,router.projection.hostPhase != .closed,router.projection.hostPhase != .failed else { throw NativeHostSessionError.admissionClosed }
    if try router.applyLocal("draft",identity:router.projection.runtime,generation:1,sequence:sequence) { publish() }
  }
  func perform(_ body:HostRequestBody,operation:String=UUID().uuidString.lowercased()) async throws -> HostOperationStatus {
    guard !closing,router.projection.hostPhase != .closed,router.projection.hostPhase != .failed else { throw NativeHostSessionError.admissionClosed }
    guard pending[operation]==nil else { throw NativeHostSessionError.duplicateOperation }
    guard pending.count<32 else { throw NativeHostSessionError.tooManyOperations }
    try Task.checkCancellation()
    let request=RequestEnvelope(protocol_version:1,runtime_instance_id:router.projection.runtime,
      request_ref:operation,operation_id:.init(operation),document_binding:.init(router.projection.documentBinding),
      task_binding:.init(nil),body:body)
    return try await withTaskCancellationHandler {
      try await withCheckedThrowingContinuation { continuation in
        pending[operation]=Pending(request:request,continuation:continuation)
        Task { await self.admit(operation) }
      }
    } onCancel: { Task { await self.cancel(operation) } }
  }
  private func admit(_ id:String) async {
    guard let item=pending[id] else { return }
    if item.stopRequested { remove(id,throwing:CancellationError());return }
    do {
      _ = try await client.submit(item.request)
      guard var admitted=pending[id] else { return }
      admitted.admitted=true;pending[id]=admitted
      if let early=admitted.early { resolve(early) }
      else if admitted.stopRequested { await cancel(id) }
      else { try await recover() }
    } catch { remove(id,throwing:error) }
  }
  func cancel(_ id:String) async {
    guard var item=pending[id] else { return }
    item.stopRequested=true;pending[id]=item
    guard item.admitted && !item.stopIssued else { return } // preserve stop until actual admission
    item.stopIssued=true;pending[id]=item
    do { _ = try await client.cancel(operation:id);try await recover() }
    catch { /* Preserve the original ID; a later snapshot can establish the real outcome. */ }
  }
  private func pump() async {
    while !Task.isCancelled {
      do {
        let batch=try await client.nextDecodedEvents(maxBytes:eventByteBudget)
        let terminals=try router.apply(batch)
        if router.projection.needsResync {
          if batch.lastRustSequence>lastInvalidatedRustSequence {
            guard recoveryRevision<HostSerial.maximum else { throw HostContractError.invalidSerial }
            recoveryRevision += 1;lastInvalidatedRustSequence=batch.lastRustSequence
          }
          try await recover()
        }
        else {
          for result in terminals {
            if let item=pending[result.operation_ref], let event=batch.events.first(where:{$0.operation?.operation_ref==result.operation_ref}) {
              guard event.envelope.request_ref.value==item.request.request_ref,
                sameDocument(event.envelope.document_binding.value,item.request.document_binding.value) else { continue }
              resolve(result)
            }
          }
          publish()
        }
      } catch {
        if closing || Task.isCancelled { break }
        try? router.markFailed();publish()
        for id in Array(pending.keys) { remove(id,throwing:NativeHostSessionError.unknownOutcome(id)) }
        break
      }
    }
  }
  /// Readback bypasses event/admission queues, so it works even when terminal events were dropped.
  private func recover() async throws {
    guard !recovering else { return }
    recovering=true
    defer { recovering=false }
    var stable=false
    while !stable {
    let revision=recoveryRevision
    var queue=Array(pending.keys).sorted()
    if queue.isEmpty {
      let snapshot=try await client.snapshot()
      if try !router.install(snapshot) { continue }
    }
    var queried=Set<String>()
    while !queue.isEmpty {
      let group=Array(queue.prefix(4));queue.removeFirst(group.count);queried.formUnion(group)
      let snapshot=try await client.snapshot(operations:group,includeResults:true)
      if try !router.install(snapshot) { continue }
      for status in snapshot.operations where pending[status.operation_ref] != nil && status.phase.isTerminal {
        if status.phase == .completed && status.result.value == nil && status.error_code.value == nil {
          guard !group.contains(status.operation_ref) else { throw NativeHostClientError.invalidReply }
          // New operations may settle during readback. Fetch their details before advancing
          // past the fence, rather than waiting for an event already included in that fence.
          if !queue.contains(status.operation_ref) { queue.append(status.operation_ref) }
        } else { resolve(status) }
      }
      for id in snapshot.unavailable_operation_refs where pending[id]?.admitted==true {
        remove(id,throwing:NativeHostSessionError.unknownOutcome(id))
      }
      // Admission can reenter this actor during IO; include new IDs in the same recovery.
      for id in pending.keys where !queried.contains(id) && !queue.contains(id) { queue.append(id) }
    }
    stable = revision==recoveryRevision && !router.projection.needsResync
    }
    publish()
  }
  private func resolve(_ status:HostOperationStatus) {
    guard var item=pending[status.operation_ref] else { return }
    if !item.admitted { item.early=status;pending[status.operation_ref]=item;return }
    pending.removeValue(forKey:status.operation_ref)
    publish() // terminal projection is available before its awaiting caller resumes
    item.continuation.resume(returning:status)
  }
  private func remove(_ id:String,throwing error:any Error) {
    pending.removeValue(forKey:id)?.continuation.resume(throwing:error)
  }
  private func publish() { for continuation in subscribers.values { continuation.yield(router.projection) } }
  func close() async throws {
    if let shutdownTask { return try await shutdownTask.value }
    let task=Task { try await self.finishClosing() }
    shutdownTask=task
    try await task.value
  }
  private func finishClosing() async throws {
    closing=true
    try await client.beginClose()
    // Keep polling/readback until the owner has actually settled, including jobs still in admission.
    while !pending.isEmpty {
      try await recover()
      if !pending.isEmpty { try await Task.sleep(for:.milliseconds(10)) }
    }
    pumpTask?.cancel()
    await pumpTask?.value
    pumpTask=nil
    try await client.close()
    try router.confirmClosed();publish()
    for continuation in subscribers.values { continuation.finish() }
    subscribers.removeAll()
  }
}
