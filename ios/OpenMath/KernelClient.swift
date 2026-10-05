import Foundation
import OpenMathKernel

private final class RequestCancellation: @unchecked Sendable {
  private let lock = NSLock()
  private var cancelled = false
  private var running = false
  func start() -> Bool {
    lock.lock()
    defer { lock.unlock() }
    if cancelled { return false }
    running = true
    return true
  }
  func finish() -> Bool {
    lock.lock()
    defer { lock.unlock() }
    running = false
    return cancelled
  }
  func cancel(_ interrupt: () -> Void) {
    lock.lock()
    defer { lock.unlock() }
    cancelled = true
    if running { interrupt() }
  }
}

final class KernelClient: @unchecked Sendable {
  private let handle: UInt64
  private let queue = DispatchQueue(label: "org.openmath.ios.client", qos: .userInitiated)
  private let lock = NSLock()
  private var nextID: UInt64 = 1
  private var closed = false
  init(config: JSONValue? = nil) throws {
    guard om_ios_abi_version() == 1 else { throw KernelError.message("内核 ABI 不匹配") }
    let bytes = try config?.data() ?? Data()
    handle = bytes.withUnsafeBytes {
      om_ios_create($0.bindMemory(to: UInt8.self).baseAddress, $0.count)
    }
    guard handle != 0 else { throw KernelError.message("内核初始化失败") }
  }
  private func correlation() throws -> UInt64 {
    lock.lock()
    defer { lock.unlock() }
    guard !closed else { throw KernelError.message("内核已关闭") }
    let result = nextID
    nextID += 1
    return result
  }
  private func decode(_ buffer: OmBuffer) throws -> KernelPacket {
    defer { om_ios_buffer_free(buffer) }
    guard let ptr = buffer.data, buffer.len > 0 else { throw KernelError.message("内核返回空缓冲区") }
    let data = Data(bytes: ptr, count: buffer.len)
    let packet = try JSONDecoder().decode(KernelPacket.self, from: data)
    if packet.response.body["type"].string == "error" {
      throw KernelError.message(packet.response.body["message"].string)
    }
    return packet
  }
  private func perform(
    allowCancelledResponse: Bool = false, _ operation: @escaping @Sendable () -> OmBuffer
  ) async throws -> KernelPacket {
    let cancellation = RequestCancellation()
    let queuedAt = ContinuousClock.now
    return try await withTaskCancellationHandler {
      try await withCheckedThrowingContinuation { continuation in
        queue.async { [self] in
          let startedAt = ContinuousClock.now
          do {
            guard cancellation.start() else { throw CancellationError() }
            lock.lock()
            let isClosed = closed
            lock.unlock()
            guard !isClosed else {
              _ = cancellation.finish()
              throw KernelError.message("内核已关闭")
            }
            let buffer = operation()
            let completedAt = ContinuousClock.now
            let cancelled = cancellation.finish()
            if cancelled && !allowCancelledResponse {
              om_ios_buffer_free(buffer)
              throw CancellationError()
            }
            var packet = try decode(buffer)
            let decodedAt = ContinuousClock.now
            packet.transportTiming = KernelTransportTiming(
              queuedMS: KernelTransportTiming.milliseconds(queuedAt.duration(to: startedAt)),
              ffiMS: KernelTransportTiming.milliseconds(startedAt.duration(to: completedAt)),
              decodeMS: KernelTransportTiming.milliseconds(completedAt.duration(to: decodedAt)))
            continuation.resume(returning: packet)
          } catch { continuation.resume(throwing: error) }
        }
      }
    } onCancel: { [self] in
      cancellation.cancel { interrupt() }
    }
  }
  func request(_ body: JSONValue) async throws -> KernelPacket {
    let id = try correlation()
    let bytes = try JSONEncoder().encode(Envelope(id: id, body: body))
    let started = ContinuousClock.now
    var packet = try await perform(allowCancelledResponse: body["type"].string.hasPrefix("llm_")) {
      [handle] in
      bytes.withUnsafeBytes {
        om_ios_request(handle, $0.bindMemory(to: UInt8.self).baseAddress, $0.count)
      }
    }
    let measured = KernelTransportTiming.milliseconds(started.duration(to: .now))
    if var timing = packet.transportTiming {
      timing.resumeMS = max(0, measured - timing.queuedMS - timing.ffiMS - timing.decodeMS)
      packet.transportTiming = timing
    }
    guard packet.response.id == id else { throw KernelError.message("内核响应标识不匹配") }
    return packet
  }
  func feed(id: String, status: UInt16, bytes: Data) async throws -> KernelPacket {
    let correlation = try correlation()
    let name = Data(id.utf8)
    let packet = try await perform(allowCancelledResponse: true) { [handle] in
      name.withUnsafeBytes { request in
        bytes.withUnsafeBytes { data in
          om_ios_http_bytes(
            handle, correlation, request.bindMemory(to: UInt8.self).baseAddress, request.count,
            status, data.bindMemory(to: UInt8.self).baseAddress, data.count)
        }
      }
    }
    guard packet.response.id == correlation else { throw KernelError.message("内核响应标识不匹配") }
    return packet
  }
  func interrupt() { om_ios_interrupt(handle) }
  func cancel(_ id: String) {
    Data(id.utf8).withUnsafeBytes {
      om_ios_cancel(handle, $0.bindMemory(to: UInt8.self).baseAddress, $0.count)
    }
  }
  func close() {
    lock.lock()
    let shouldClose = !closed
    closed = true
    lock.unlock()
    if shouldClose {
      interrupt()
      queue.async { [handle] in om_ios_destroy(handle) }
    }
  }
  deinit {
    om_ios_interrupt(handle)
    let value = handle
    queue.async { om_ios_destroy(value) }
  }
}
