import Foundation

private final class RejectRedirectDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
  func urlSession(
    _ session: URLSession, task: URLSessionTask,
    willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest,
    completionHandler: @escaping @Sendable (URLRequest?) -> Void
  ) { completionHandler(nil) }
}
@MainActor final class LLMTransport {
  private final class Job {
    var next: JSONValue?
    var task: Task<Void, Never>?
    var timedOut = false
    let timeout: Double
    init(_ next: JSONValue, _ timeout: Double) {
      self.next = next
      self.timeout = timeout
    }
  }
  private let client: KernelClient
  private let consume: @MainActor (KernelPacket) async -> Void
  private let session: URLSession
  private var jobs: [String: Job] = [:]
  init(
    client: KernelClient, configuration: URLSessionConfiguration = .ephemeral,
    consume: @escaping @MainActor (KernelPacket) async -> Void
  ) {
    self.client = client
    self.consume = consume
    configuration.httpCookieStorage = nil
    configuration.urlCache = nil
    configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
    session = URLSession(
      configuration: configuration, delegate: RejectRedirectDelegate(), delegateQueue: nil)
  }
  func start(id: String, request: JSONValue, timeout: Double) {
    guard jobs[id] == nil else { return }
    let job = Job(request, max(1, timeout))
    jobs[id] = job
    job.task = Task { [weak self] in await self?.drive(id, job: job) }
  }
  func next(id: String, request: JSONValue) { jobs[id]?.next = request }
  func cancel(_ id: String) {
    let job = jobs.removeValue(forKey: id)
    job?.task?.cancel()
    client.cancel(id)
  }
  func complete(_ id: String) {
    let job = jobs.removeValue(forKey: id)
    job?.task?.cancel()
  }
  private func drive(_ id: String, job: Job) async {
    while jobs[id] === job, let http = job.next, !Task.isCancelled {
      job.next = nil
      job.timedOut = false
      var status: UInt16 = 0
      let deadline = Task { [weak job] in
        try? await Task.sleep(for: .milliseconds(Int(job?.timeout ?? 30000)))
        guard !Task.isCancelled else { return }
        job?.timedOut = true
        client.cancel(id)
        job?.task?.cancel()
      }
      defer { deadline.cancel() }
      do {
        guard let url = URL(string: http["url"].string),
          ["https", "http"].contains(url.scheme?.lowercased() ?? "")
        else { throw KernelError.message("模型 URL 无效") }
        var request = URLRequest(url: url)
        request.httpMethod = http["method"].string
        request.httpBody = Data(http["body"].string.utf8)
        request.timeoutInterval = job.timeout / 1000
        for header in http["headers"].array {
          request.setValue(header[1].string, forHTTPHeaderField: header[0].string)
        }
        let (stream, response) = try await session.bytes(for: request)
        guard let response = response as? HTTPURLResponse else {
          throw KernelError.message("无效 HTTP 响应")
        }
        status = UInt16(response.statusCode)
        var buffer = Data()
        var count = 0
        for try await byte in stream {
          try Task.checkCancellation()
          guard jobs[id] === job else { return }
          count += 1
          guard count <= 1_048_576 else { throw KernelError.message("模型响应超过限制") }
          buffer.append(byte)
          if byte == 10 || buffer.count >= 4096 {
            await consume(try await client.feed(id: id, status: status, bytes: buffer))
            buffer.removeAll(keepingCapacity: true)
          }
        }
        if !buffer.isEmpty {
          await consume(try await client.feed(id: id, status: status, bytes: buffer))
        }
        if jobs[id] === job {
          await consume(
            try await client.request(
              .object([
                "type": .string("llm_http_end"), "request_id": .string(id),
                "status": .number(Double(status)), "error": .null,
              ])))
        }
      } catch {
        if jobs[id] === job {
          let message =
            job.timedOut
            ? "HTTP request timed out" : "HTTP request failed; check network and profile settings"
          let ending = JSONValue.object([
            "type": .string("llm_http_end"), "request_id": .string(id),
            "status": .number(Double(status)), "error": .string(message),
          ])
          // A cancelled transport still owes the Rust job its terminal feedback.
          let completion = Task.detached { [client] in try await client.request(ending) }
          if let packet = try? await completion.value {
            await consume(packet)
          }
        }
      }
      deadline.cancel()
      if job.next == nil { break }
    }
    if jobs[id] === job { jobs.removeValue(forKey: id) }
  }
}
