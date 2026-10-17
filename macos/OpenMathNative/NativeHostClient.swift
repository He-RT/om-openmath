import Foundation
import OpenMathHost

/// The wrapper never dereferences the opaque pointer; Rust guards every live call.
/// Its immutable pointer may cross background queues. Closing/freeing remain Rust operations.
private final class NativeHostHandle: @unchecked Sendable {
  let pointer: OpaquePointer
  private let lock = NSLock()
  private var consumed = false
  init(_ pointer: OpaquePointer) { self.pointer = pointer }
  func claimFinish() -> Bool { lock.withLock { if consumed { return false }; consumed = true; return true } }
  deinit {
    if !consumed {
      let address = UInt(bitPattern: pointer)
      NativeHostQueues.shutdown.async {
        guard let pointer = OpaquePointer(bitPattern: address) else { return }
        om_host_buffer_free(om_host_close_begin(pointer))
        _ = om_host_close_finish(pointer)
      }
    }
  }
}
private enum NativeHostQueues {
  static let control = DispatchQueue(label: "org.openmath.native.control", qos: .userInitiated)
  static let events = DispatchQueue(label: "org.openmath.native.events", qos: .userInitiated)
  static let recovery = DispatchQueue(label: "org.openmath.native.recovery", qos: .userInitiated)
  static let kernel = DispatchQueue(label: "org.openmath.native.kernel.control", qos: .userInitiated)
  static let agent = DispatchQueue(label: "org.openmath.native.agent.control", qos: .userInitiated)
  static let model = DispatchQueue(label: "org.openmath.native.model.codec", qos: .userInitiated)
  static let modelCancel = DispatchQueue(label: "org.openmath.native.model.cancel", qos: .userInitiated)
  static let results = DispatchQueue(label: "org.openmath.native.results.control", qos: .userInitiated)
  static let analysis = DispatchQueue(label: "org.openmath.native.source.analysis", qos: .utility)
  static let cancel = DispatchQueue(label: "org.openmath.native.cancel", qos: .userInitiated)
  static let shutdown = DispatchQueue(label: "org.openmath.native.shutdown", qos: .utility)
}

enum NativeHostClientError: Error, Sendable {
  case rejected(HostError), closing, emptyBuffer, incompatibleVersion, invalidReply, status(Int32)
}
/// Synchronous revocation reaches a native HTTP task even while an actor/ledger is suspended.
final class NativeModelLifetime:@unchecked Sendable {
  private let lock=NSLock()
  private var closed=false,hook:(@Sendable ()->Void)?
  var isClosed:Bool {lock.withLock {closed}}
  func bind(_ callback:@escaping @Sendable ()->Void) {
    let invoke=lock.withLock {if closed{return true};hook=callback;return false}
    if invoke {callback()}
  }
  func close() {
    let callback=lock.withLock {()->(@Sendable ()->Void)? in guard !closed else {return nil};closed=true;let value=hook;hook=nil;return value}
    callback?()
  }
}

/// Transport only: it owns ABI lifetime, not a second document or a synthesized success state.
/// All encoding, bounded polling, decoding/freeing and owner joins execute off the caller actor.
actor NativeHostClient {
  let runtimeInstanceID: String
  private var handle: NativeHostHandle?
  private var closeTask:Task<Void,any Error>?
  private var modelLifetimes:[String:NativeModelLifetime]=[:]
  private init(runtime: String, handle: NativeHostHandle) {
    runtimeInstanceID = runtime
    self.handle = handle
  }
  static func open(runtime: String = UUID().uuidString.lowercased(), eventCapacity: UInt32 = 128) async throws -> NativeHostClient {
    let handle: NativeHostHandle = try await background(NativeHostQueues.control) {
      guard om_host_abi_version() == 1 else { throw NativeHostClientError.incompatibleVersion }
      let frame = HostInit(protocol_version: 1, runtime_instance_id: runtime,
        max_pending_operations: 32, event_capacity: eventCapacity)
      let data = try JSONEncoder().encode(frame)
      let result = data.withUnsafeBytes { raw in
        om_host_create(raw.bindMemory(to: UInt8.self).baseAddress, raw.count)
      }
      guard let pointer = result.handle else {
        let failure: HostFailure = try read(result.error)
        throw NativeHostClientError.rejected(failure.error)
      }
      guard result.error.ptr == nil && result.error.len == 0 else {
        om_host_buffer_free(result.error)
        om_host_buffer_free(om_host_close_begin(pointer))
        _ = om_host_close_finish(pointer)
        throw NativeHostClientError.invalidReply
      }
      return NativeHostHandle(pointer)
    }
    return NativeHostClient(runtime: runtime, handle: handle)
  }
  func submit(_ request: RequestEnvelope) async throws -> AdmissionReceipt {
    guard let handle else { throw NativeHostClientError.closing }
    guard request.runtime_instance_id == runtimeInstanceID else { throw NativeHostClientError.invalidReply }
    return try await Self.background(NativeHostQueues.control) {
      let data = try JSONEncoder().encode(request)
      guard data.count <= 2 * 1024 * 1024 else { throw NativeHostClientError.invalidReply }
      let buffer = data.withUnsafeBytes { raw in
        om_host_submit(handle.pointer, raw.bindMemory(to: UInt8.self).baseAddress, raw.count)
      }
      let receipt: AdmissionReceipt = try Self.read(buffer)
      guard receipt.protocol_version == 1, receipt.request_ref == request.request_ref,
        receipt.operation_ref.value == (request.operation_id.value ?? request.request_ref),
        receipt.accepted, receipt.error.value == nil else { throw NativeHostClientError.invalidReply }
      return receipt
    }
  }
  func nextEvents() async throws -> EventBatch {
    guard let handle else { throw NativeHostClientError.closing }
    return try await Self.background(NativeHostQueues.events) {
      let batch: EventBatch = try Self.read(om_host_next_events(handle.pointer, 100, 512 * 1024))
      guard batch.protocol_version == 1 else { throw NativeHostClientError.incompatibleVersion }
      return batch
    }
  }
  func nextDecodedEvents(maxBytes:Int=512*1024) async throws -> NativeDecodedBatch {
    guard let handle else { throw NativeHostClientError.closing }
    return try await Self.background(NativeHostQueues.events) {
      let batch: EventBatch = try Self.read(om_host_next_events(handle.pointer,100,maxBytes))
      guard batch.protocol_version==1 else { throw NativeHostClientError.incompatibleVersion }
      let events = try batch.events.map { event in
        var status:HostOperationStatus?
        if event.event_kind == .operation_finished || event.event_kind == .operation_progress {
          status = try JSONDecoder().decode(HostOperationStatus.self, from:JSONEncoder().encode(event.payload))
        }
        return NativeDecodedEvent(envelope:event,operation:status)
      }
      return NativeDecodedBatch(needsResync:batch.needs_resync,lastRustSequence:batch.last_rust_event_sequence.value,events:events)
    }
  }
  func snapshot(operations:[String]=[],includeResults:Bool=false) async throws -> HostSnapshot {
    guard let handle else { throw NativeHostClientError.closing }
    let runtime=runtimeInstanceID
    return try await Self.background(NativeHostQueues.recovery) {
      let query=HostSnapshotQuery(protocol_version:1,runtime_instance_id:runtime,
        operation_refs:operations,include_results:includeResults)
      let data=try JSONEncoder().encode(query)
      let packet=data.withUnsafeBytes { raw in
        om_host_read_snapshot(handle.pointer,raw.bindMemory(to:UInt8.self).baseAddress,raw.count)
      }
      let snapshot:HostSnapshot=try Self.read(packet)
      guard snapshot.protocol_version==1,snapshot.runtime_instance_id==runtime else { throw NativeHostClientError.invalidReply }
      return snapshot
    }
  }
  func beginClose() async throws {
    guard let handle else { return }
    try await Self.background(NativeHostQueues.cancel) {
      let packet=om_host_close_begin(handle.pointer)
      let _:HostJSONValue=try Self.read(packet)
    }
  }
  func sourceCommand(_ command:NativeSourceHostCommand) async throws ->NativeSourceHostReply {
    guard let handle else {throw NativeHostClientError.closing}
    return try await Self.background(NativeHostQueues.control) {
      let data=try JSONEncoder().encode(command)
      guard data.count<=2*1024*1024 else {throw NativeHostClientError.invalidReply}
      let packet=data.withUnsafeBytes {om_host_source_command(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NativeSourceHostReply=try Self.read(packet)
      guard reply.protocol_version==1 else {throw NativeHostClientError.incompatibleVersion}
      return reply
    }
  }
  func analyzeSource(_ input:NotebookAnalysisInput) async throws->NotebookAnalysisReply {
    guard let handle else {throw NativeHostClientError.closing}
    return try await Self.background(NativeHostQueues.analysis) {
      let data=try JSONEncoder().encode(input)
      guard data.count<=2*1024*1024 else {throw NativeHostClientError.invalidReply}
      let packet=data.withUnsafeBytes {om_host_analyze_source(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NotebookAnalysisReply=try Self.read(packet)
      guard reply.protocol_version==1,reply.document_id==input.document_id,reply.generation.value==input.generation.value,reply.base_hash==input.base_hash else {throw NativeHostClientError.invalidReply}
      return reply
    }
  }
  /// Main mathematics uses a separate control queue; Rust still owns every source/acceptance fact.
  func listDefinitions(_ input:NotebookAnalysisInput) async throws->NotebookDefinitionsReply {
    guard let handle else {throw NativeHostClientError.closing}
    return try await Self.background(NativeHostQueues.results) {
      let data=try JSONEncoder().encode(input)
      guard data.count<=2*1024*1024 else {throw NativeHostClientError.invalidReply}
      let packet=data.withUnsafeBytes {om_host_list_definitions(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NotebookDefinitionsReply=try Self.read(packet)
      guard reply.protocol_version==1,reply.document_id==input.document_id,reply.generation.value==input.generation.value,
        reply.base_hash==input.base_hash,Set(reply.items.map(\.name)).count==reply.items.count,
        reply.items.allSatisfy({!$0.name.isEmpty && !$0.cell_id.isEmpty && ($0.value?.unicodeScalars.count ?? 0)<=120}) else {throw NativeHostClientError.invalidReply}
      return reply
    }
  }
  /// Main mathematics uses a separate control queue; Rust still owns every source/acceptance fact.
  func kernelCommand(_ command:NativeKernelHostCommand) async throws ->NativeKernelHostReply {
    guard let handle else {throw NativeHostClientError.closing}
    return try await Self.background(NativeHostQueues.kernel) {
      let data=try JSONEncoder().encode(command)
      guard data.count<=2*1024*1024 else {throw NativeHostClientError.invalidReply}
      let packet=data.withUnsafeBytes {om_host_kernel_command(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NativeKernelHostReply=try Self.read(packet)
      guard reply.protocol_version==1,reply.document_generation.value>0,reply.kernel_build.utf8.count==64,
        reply.kernel_build.utf8.allSatisfy({($0>=48 && $0<=57)||($0>=97 && $0<=102)}),reply.blob_bytes.allSatisfy({$0<=255}) else {throw NativeHostClientError.invalidReply}
      return reply
    }
  }
  /// Immutable result admission/status. Source/CAS/physical IO owners remain independent.
  func resultCommand(_ command:NativeResultHostCommand) async throws ->NativeResultHostReply {
    guard let handle else {throw NativeHostClientError.closing}
    let runtime=runtimeInstanceID
    return try await Self.background(NativeHostQueues.results) {
      let data=try JSONEncoder().encode(command)
      guard data.count<=2*1024*1024 else {throw NativeHostClientError.invalidReply}
      let packet=data.withUnsafeBytes {om_host_result_command(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NativeResultHostReply=try Self.read(packet)
      guard reply.protocol_version==1,reply.runtime_instance_id==runtime,reply.document_generation.value>0 else {throw NativeHostClientError.invalidReply}
      if let binding=reply.binding.value {
        guard binding.runtime_instance_id==runtime,binding.document_id==reply.document_id,binding.document_generation.value==reply.document_generation.value else {throw NativeHostClientError.invalidReply}
      }
      return reply
    }
  }
  /// Private task control, bound by Rust to the currently opened source owner.
  func agentCommand(_ command:NativeAgentHostCommand) async throws->NativeAgentHostReply {
    guard let handle else {throw NativeHostClientError.closing}
    let runtime=runtimeInstanceID
    return try await Self.background(NativeHostQueues.agent) {
      let data=try JSONEncoder().encode(command)
      guard data.count<=2*1024*1024 else {throw NativeHostClientError.invalidReply}
      let packet=data.withUnsafeBytes {om_host_agent_command(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NativeAgentHostReply=try Self.read(packet)
      guard reply.protocol_version==1,reply.runtime_instance_id==runtime,reply.document_generation.value>0,
        reply.scope.document_generation.value==reply.document_generation.value else {throw NativeHostClientError.invalidReply}
      return reply
    }
  }
  /// Pure model channel on its own queue; cancellation never waits for the CAS owner.
  func modelCodecCommand(_ command:NativeModelCodecCommand) async throws->NativeModelCodecReply {
    guard let handle else {throw NativeHostClientError.closing}
    let runtime=runtimeInstanceID
    let cancelling:Bool
    if case .native_model_codec_cancel=command {cancelling=true} else {cancelling=false}
    return try await Self.background(cancelling ? NativeHostQueues.modelCancel:NativeHostQueues.model) {
      let data=try JSONEncoder().encode(command)
      guard data.count<=2*1024*1024 else {throw NativeHostClientError.invalidReply}
      let buffer=data.withUnsafeBytes {om_host_model_codec_command(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NativeModelCodecReply=try Self.read(buffer)
      guard reply.protocol_version==1,reply.runtime_instance_id==runtime else {throw NativeHostClientError.invalidReply}
      return reply
    }
  }
  func registerModelLifetime(_ id:String,_ lifetime:NativeModelLifetime)throws {
    guard handle != nil,closeTask==nil,modelLifetimes[id]==nil,modelLifetimes.count<8 else {lifetime.close();throw NativeHostClientError.closing}
    modelLifetimes[id]=lifetime
  }
  func removeModelLifetime(_ id:String){modelLifetimes.removeValue(forKey:id)?.close()}
  /// Short final Rust owner check; prepared off MainActor and called by the physical SQL writer.
  func kernelBarrier(operation:String) async throws ->(@Sendable ()throws->Void) {
    guard let handle else {throw NativeHostClientError.closing}
    let data=try JSONEncoder().encode(NativeKernelHostCommand.kernel_barrier(.init(type:.kernel_barrier,operation_id:operation)))
    return {
      precondition(!Thread.isMainThread)
      let packet=data.withUnsafeBytes {om_host_kernel_command(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NativeKernelHostReply=try Self.read(packet)
      guard reply.operation_id.value==operation,reply.phase == .committing else {throw NativeHostClientError.invalidReply}
    }
  }
  /// Prepared off MainActor; execution holds the native fence lock only for this short owner RPC.
  func sourceBarrier(operation:String,fenceID:String) async throws ->(@Sendable ()throws->Void) {
    guard let handle else {throw NativeHostClientError.closing}
    let command=NativeSourceHostCommand.source_barrier(.init(type:.source_barrier,operation_id:operation,fence_id:fenceID))
    let data=try JSONEncoder().encode(command)
    return {
      precondition(!Thread.isMainThread)
      let packet=data.withUnsafeBytes {om_host_source_command(handle.pointer,$0.bindMemory(to:UInt8.self).baseAddress,$0.count)}
      let reply:NativeSourceHostReply=try Self.read(packet)
      guard reply.operation.value?.operation_id==operation,reply.operation.value?.phase == .committing else {throw NativeHostClientError.invalidReply}
    }
  }
  /// This queue is independent of event polling and submission, and of the Rust CAS workers.
  func cancel(operation: String) async throws -> Bool {
    guard let handle else { throw NativeHostClientError.closing }
    return try await Self.background(NativeHostQueues.cancel) {
      let data = Data(operation.utf8)
      let status = data.withUnsafeBytes { raw in
        om_host_cancel(handle.pointer, raw.bindMemory(to: UInt8.self).baseAddress, raw.count)
      }
      guard status >= 0 else { throw NativeHostClientError.status(status) }
      return status == 1
    }
  }
  /// Callers stop their event consumer first. Outstanding C calls are guarded and settled in Rust.
  func close() async throws {
    for token in modelLifetimes.values {token.close()};modelLifetimes.removeAll()
    if let closeTask { return try await closeTask.value }
    guard let handle else { return }
    self.handle = nil
    guard handle.claimFinish() else { return }
    let task=Task {
      try await Self.background(NativeHostQueues.shutdown) {
        let begin = om_host_close_begin(handle.pointer)
        defer { om_host_buffer_free(begin) }
        let status = om_host_close_finish(handle.pointer)
        guard status == 0 else { throw NativeHostClientError.status(status) }
      }
    }
    closeTask=task
    try await task.value
  }
  private static func read<T: Decodable>(_ buffer: om_host_buffer) throws -> T {
    defer { om_host_buffer_free(buffer) }
    guard let pointer = buffer.ptr, buffer.len > 0 else { throw NativeHostClientError.emptyBuffer }
    let data = Data(bytes: pointer, count: buffer.len)
    let decoder = JSONDecoder()
    if let failure = try? decoder.decode(HostFailure.self, from: data) {
      guard failure.protocol_version == 1 else { throw NativeHostClientError.incompatibleVersion }
      throw NativeHostClientError.rejected(failure.error)
    }
    return try decoder.decode(T.self, from: data)
  }
  private static func background<T: Sendable>(_ queue: DispatchQueue,
    _ work: @escaping @Sendable () throws -> T) async throws -> T {
    try await withCheckedThrowingContinuation { continuation in
      queue.async {
        precondition(!Thread.isMainThread, "Native ABI/encoding/decoding must stay off the UI thread")
        do { continuation.resume(returning: try work()) }
        catch { continuation.resume(throwing: error) }
      }
    }
  }
}
