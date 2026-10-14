import Foundation
import CryptoKit

enum KernelPortError:Error,Sendable {case cancelled,failed(String),unknownOutcome,sourceChanged,recoveryRequired,busy}
struct KernelRunOutcome:Sendable {
  let operation:String
  let receipts:[NativeKernelReceipt]
  let stopped:String?
}
/// IO adapter only. The Rust source/kernel owner makes all active-state and cancellation decisions.
actor KernelCommitPort {
  private let client:NativeHostClient
  private let storage:StorageService
  private let source:DocCommitPort
  private var closing=false
  private var inFlight=Set<String>()
  private var preparing=Set<String>()
  private var stopped=Set<String>()
  private var unknown:[String:NativeKernelCommit]=[:]
  private var groups:[String:Set<String>]=[:]
  private var seeds:[String:String]=[:]
  init(client:NativeHostClient,storage:StorageService,source:DocCommitPort) {self.client=client;self.storage=storage;self.source=source}
  func state() async throws ->NativeKernelHostReply {try await client.kernelCommand(.kernel_state(.init(type:.kernel_state)))}
  func status(_ operation:String) async throws ->NativeKernelHostReply {try await client.kernelCommand(.kernel_status(.init(type:.kernel_status,operation_id:operation)))}
  private func admitted(_ operation:String) async throws {
    if stopped.contains(operation) {_ = try await client.cancel(operation:operation)}
  }
  private func waitForCandidate(_ operation:String) async throws ->NativeKernelHostReply {
    while true {
      let result=try await status(operation)
      switch result.phase {
      case .candidate,.completed:return result
      case .failed,.discarded:throw KernelPortError.failed(result.error_code.value ?? "CANCELLED")
      case .committing,.unknown:throw KernelPortError.unknownOutcome
      default:
        if Task.isCancelled || stopped.contains(operation) {_ = try await client.cancel(operation:operation)}
        // Cancellation cannot abandon an actual owner/SQL outcome. Poll settles its original ID.
        try? await Task.sleep(for:.milliseconds(10))
      }
    }
  }
  private func actualBytes(_ plan:NativeKernelCommit) async throws ->Data {
    guard plan.checkpoint_byte_length.value<=64*1024*1024 else {throw NativeHostClientError.invalidReply}
    var data=Data(),offset:UInt64=0
    while offset<plan.checkpoint_byte_length.value {
      let count=UInt32(min(65536,plan.checkpoint_byte_length.value-offset))
      let result=try await client.kernelCommand(.kernel_read_blob(.init(type:.kernel_read_blob,operation_id:plan.operation_id,offset:try HostSerial(offset),count:count)))
      guard result.phase == .blob,result.operation_id.value==plan.operation_id,result.blob_offset.value?.value==offset,result.blob_bytes.count==Int(count) else {throw NativeHostClientError.invalidReply}
      data.append(contentsOf:result.blob_bytes.map(UInt8.init));offset+=UInt64(count)
    }
    guard KernelHashes.data(data)==plan.checkpoint_blob_hash else {throw BlobError.corrupt}
    return data
  }
  private func settleNoCommit(_ plan:NativeKernelCommit) async throws {
    let proof=try await storage.confirmedNoKernelCommit(plan)
    let result=try await client.kernelCommand(.kernel_settled(.init(type:.kernel_settled,operation_id:plan.operation_id,proof:proof)))
    guard result.phase == .discarded else {throw NativeHostClientError.invalidReply}
    unknown.removeValue(forKey:plan.operation_id)
  }
  /// Read original actual facts; never construct another ID or replay computation after lost ACK.
  func reconcile(_ operation:String) async throws ->NativeKernelReceipt? {
    let result=try await status(operation)
    if result.phase == .completed {
      guard let receipt=result.receipt.value else {throw NativeHostClientError.invalidReply}
      if let plan=unknown[operation] {guard receipt.request_hash==plan.request_hash else {throw NativeHostClientError.invalidReply}}
      unknown.removeValue(forKey:operation);return receipt
    }
    guard let plan=unknown[operation] ?? result.plan.value else {throw KernelPortError.unknownOutcome}
    if let receipt=try await storage.kernelReceipt(document:plan.producer.document_id,operation:operation) {
      let accepted=try await client.kernelCommand(.kernel_completed(.init(type:.kernel_completed,operation_id:operation,receipt:receipt)))
      guard accepted.phase == .completed,accepted.receipt.value?.request_hash==plan.request_hash else {throw NativeHostClientError.invalidReply}
      unknown.removeValue(forKey:operation);return accepted.receipt.value
    }
    try await settleNoCommit(plan);return nil
  }
  private func persist(_ reply:NativeKernelHostReply,faults:StorageFaults) async throws ->NativeKernelReceipt {
    if reply.phase == .completed,let receipt=reply.receipt.value {return receipt}
    guard let plan=reply.plan.value,reply.operation_id.value==plan.operation_id else {throw NativeHostClientError.invalidReply}
    if stopped.contains(plan.operation_id) || Task.isCancelled {
      _ = try await client.cancel(operation:plan.operation_id)
      _ = try await client.kernelCommand(.kernel_discard(.init(type:.kernel_discard,operation_id:plan.operation_id)))
      throw KernelPortError.cancelled
    }
    var publication:BlobPublication?
    do {
      let bytes=try await actualBytes(plan)
      publication=try await storage.publishBlob(data:bytes,expectedHash:plan.checkpoint_blob_hash,faults:faults)
      let barrier=try await client.kernelBarrier(operation:plan.operation_id)
      let receipt=try await storage.commitKernel(plan,publication:publication!,controls:.init(ownerBarrier:barrier),faults:faults)
      let accepted=try await client.kernelCommand(.kernel_completed(.init(type:.kernel_completed,operation_id:plan.operation_id,receipt:receipt)))
      guard accepted.phase == .completed,accepted.receipt.value?.request_hash==plan.request_hash else {throw NativeHostClientError.invalidReply}
      if let publication {try await storage.releaseBlob(publication)}
      return receipt
    } catch {
      let actualError=error
      // The physical task has settled. Only actual readback/absence can release its shared gate.
      unknown[plan.operation_id]=plan
      if let current=try? await status(plan.operation_id),current.phase == .committing {
        _ = try? await client.kernelCommand(.kernel_unknown(.init(type:.kernel_unknown,operation_id:plan.operation_id)))
      }
      do {
        if let receipt=try await reconcile(plan.operation_id) {
          if let publication {try await storage.releaseBlob(publication)}
          return receipt
        }
      } catch {
        if let publication {try? await storage.releaseBlob(publication)}
        throw KernelPortError.unknownOutcome
      }
      if let publication {try? await storage.releaseBlob(publication)}
      throw actualError
    }
  }
  /// Source barrier completes first; restore/calculation never happens on MainActor or this actor.
  func finishExecution(_ operation:String,faults:StorageFaults = .init()) async throws ->NativeKernelReceipt {
    guard !closing,!inFlight.contains(operation),!preparing.contains(operation),unknown.isEmpty else {throw KernelPortError.busy}
    inFlight.insert(operation);defer {inFlight.remove(operation);stopped.remove(operation)}
    try await admitted(operation)
    return try await persist(waitForCandidate(operation),faults:faults)
  }
  /// Source barrier completes first; restore/calculation never happens on MainActor or this actor.
  func runCell(_ cell:String,operation:String="kernel-"+UUID().uuidString.lowercased(),faults:StorageFaults = .init()) async throws ->NativeKernelReceipt {
    guard !closing else {throw StorageError.closing}
    guard unknown.isEmpty else {throw KernelPortError.unknownOutcome}
    guard !inFlight.contains(operation),!preparing.contains(operation),inFlight.count+preparing.count<32 else {throw KernelPortError.busy}
    preparing.insert(operation);defer {preparing.remove(operation);stopped.remove(operation);if let seed=seeds.removeValue(forKey:operation) {stopped.remove(seed)}}
    try await source.synchronizeDrafts()
    let head=try await client.sourceCommand(.source_read(.init(type:.source_read)))
    guard let snapshot=head.snapshot.value else {throw NativeHostClientError.invalidReply}
    let current=try await state()
    if current.active_checkpoint_id.value==nil {
      guard try await storage.acceptedKernelHead(document:snapshot.document_id)==nil else {throw KernelPortError.recoveryRequired}
      let seed="bootstrap-"+UUID().uuidString.lowercased()
      seeds[operation]=seed
      _ = try await client.kernelCommand(.kernel_bootstrap(.init(type:.kernel_bootstrap,operation_id:seed,expected_source_hash:snapshot.snapshot_hash)))
      if stopped.contains(operation) {stopped.insert(seed);_ = try await client.cancel(operation:seed)}
      _ = try await persist(waitForCandidate(seed),faults:faults);seeds.removeValue(forKey:operation);stopped.remove(seed)
    }
    if stopped.contains(operation) || Task.isCancelled {throw KernelPortError.cancelled}
    _ = try await client.kernelCommand(.kernel_run_cell(.init(type:.kernel_run_cell,operation_id:operation,cell_id:cell,expected_source_hash:snapshot.snapshot_hash)))
    inFlight.insert(operation);defer {inFlight.remove(operation)}
    try await admitted(operation)
    return try await persist(waitForCandidate(operation),faults:faults)
  }
  /// Actual accepted prefix survives errors/stop/source changes; no whole-run rollback is invented.
  func runCells(_ cells:[String],operation:String="run-"+UUID().uuidString.lowercased()) async ->KernelRunOutcome {
    guard !closing,!cells.isEmpty,cells.count<=10000,Set(cells).count==cells.count,groups[operation]==nil else {return .init(operation:operation,receipts:[],stopped:"INVALID_ARGUMENT")}
    groups[operation]=[];defer {groups.removeValue(forKey:operation);stopped.remove(operation)}
    var receipts:[NativeKernelReceipt]=[]
    let epoch:UInt64
    do {try await source.synchronizeDrafts();epoch=try await state().execution_epoch.value} catch {return .init(operation:operation,receipts:[],stopped:String(describing:error))}
    for (index,cell) in cells.enumerated() {
      if stopped.contains(operation) || Task.isCancelled {return .init(operation:operation,receipts:receipts,stopped:"CANCELLED")}
      let child=operation+".\(index+1)";groups[operation]?.insert(child)
      do {
        guard try await state().execution_epoch.value==epoch else {return .init(operation:operation,receipts:receipts,stopped:"STALE_SOURCE")}
        let receipt=try await runCell(cell,operation:child);receipts.append(receipt)
        if receipt.producer.terminal_status == .error {return .init(operation:operation,receipts:receipts,stopped:"FAILED")}
      } catch {return .init(operation:operation,receipts:receipts,stopped:String(describing:error))}
    }
    return .init(operation:operation,receipts:receipts,stopped:nil)
  }
  func cancel(_ operation:String) async {
    stopped.insert(operation)
    if let seed=seeds[operation] {stopped.insert(seed);_ = try? await client.cancel(operation:seed)}
    for child in groups[operation] ?? [operation] {_ = try? await client.cancel(operation:child)}
  }
  /// No new calls; actual writes/preparation/readback settle before native lifetime is closed.
  func close() async {
    closing=true
    for operation in inFlight.union(preparing) {await cancel(operation)}
    while !inFlight.isEmpty || !preparing.isEmpty {try? await Task.sleep(for:.milliseconds(10))}
  }
}
