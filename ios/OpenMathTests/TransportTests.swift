import Foundation
import XCTest

@testable import OpenMath

private final class FixtureProtocol: URLProtocol, @unchecked Sendable {
  final class Registry: @unchecked Sendable {
    let lock = NSLock()
    var status = 200
    var body = Data()
    var delay: TimeInterval = 0
    var calls: [URLRequest] = []
    var sequence: [Data] = []
    func configure(status: Int = 200, body: Data, delay: TimeInterval = 0) {
      lock.lock()
      defer { lock.unlock() }
      self.status = status
      self.body = body
      self.delay = delay
      calls = []
      sequence = []
    }
    func take(_ request: URLRequest) -> (Int, Data, TimeInterval) {
      lock.lock()
      defer { lock.unlock() }
      calls.append(request)
      return (
        status, sequence.isEmpty ? body : sequence[min(calls.count - 1, sequence.count - 1)], delay
      )
    }
    func configureSequence(_ values: [Data]) {
      lock.lock()
      defer { lock.unlock() }
      status = 200
      delay = 0
      calls = []
      sequence = values
    }
    func count() -> Int {
      lock.lock()
      defer { lock.unlock() }
      return calls.count
    }
  }
  static let registry = Registry()
  private var work: DispatchWorkItem?
  override class func canInit(with request: URLRequest) -> Bool {
    request.url?.host == "fixture.invalid"
  }
  override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
  override func startLoading() {
    let (status, body, delay) = Self.registry.take(request)
    let item = DispatchWorkItem { [self] in
      let response = HTTPURLResponse(
        url: request.url!, statusCode: status, httpVersion: "HTTP/1.1",
        headerFields: [
          "Content-Type": "text/event-stream", "Location": "https://elsewhere.invalid/never",
        ])!
      client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
      // Byte-sized chunks exercise UTF-8 boundaries before the kernel decodes them.
      for byte in body { client?.urlProtocol(self, didLoad: Data([byte])) }
      client?.urlProtocolDidFinishLoading(self)
    }
    work = item
    DispatchQueue.global().asyncAfter(deadline: .now() + delay, execute: item)
  }
  override func stopLoading() { work?.cancel() }
}

@MainActor final class TransportTests: XCTestCase {
  private func probe(status: Int, timeout: Double = 5000, delay: TimeInterval = 0) async throws
    -> [JSONValue]
  {
    let stream =
      "data: {\"choices\":[{\"delta\":{\"content\":\"中文🙂\"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
    FixtureProtocol.registry.configure(status: status, body: Data(stream.utf8), delay: delay)
    let client = try KernelClient()
    defer { client.close() }
    let before = try await client.request(.object(["type": .string("get_config")])).response.body[
      "config"]
    var profile = before["llm"]["profiles"][0]
    profile["kind"] = .string("openai_chat")
    profile["base_url"] = .string("https://fixture.invalid/v1")
    profile["api_key"] = .string("synthetic-transport-key")
    profile["model"] = .string("fixture")
    profile["timeout_ms"] = .number(timeout)
    let configuration = URLSessionConfiguration.ephemeral
    configuration.protocolClasses = [FixtureProtocol.self]
    var events: [JSONValue] = []
    let done = expectation(description: "actual kernel terminal event")
    let transport = LLMTransport(client: client, configuration: configuration) { packet in
      for event in packet.events.map(\.body) {
        events.append(event)
        if ["llm_done", "llm_error"].contains(event["type"].string) { done.fulfill() }
      }
    }
    let start = try await client.request(
      .object([
        "type": .string("llm_test_profile"), "request_id": .string("probe"),
        "profile": profile["name"], "config": profile,
      ]))
    transport.start(id: "probe", request: start.response.body["http"], timeout: timeout)
    await fulfillment(of: [done], timeout: 10)
    let after = try await client.request(.object(["type": .string("get_config")])).response.body[
      "config"]
    XCTAssertEqual(before, after, "draft probe must not persist")
    XCTAssertEqual(FixtureProtocol.registry.count(), 1, "redirects must not replay credentials")
    return events
  }
  func testStreamingUTF8AndDraftProbe() async throws {
    let events = try await probe(status: 200)
    XCTAssertEqual(
      events.first { $0["type"].string == "llm_profile_test" }?["response"].string, "中文🙂")
    XCTAssertTrue(events.contains { $0["type"].string == "llm_done" })
  }
  func testHTTPErrorAndRedirectAreRealFailures() async throws {
    for status in [401, 429, 500, 302] {
      let events = try await probe(status: status)
      XCTAssertTrue(events.contains { $0["type"].string == "llm_error" }, "HTTP \(status)")
      XCTAssertFalse(events.contains { $0["type"].string == "llm_profile_test" })
      XCTAssertFalse(events.map(\.pretty).joined().contains("synthetic-transport-key"))
    }
  }
  func testWholeRequestTimeout() async throws {
    let events = try await probe(status: 200, timeout: 30, delay: 2)
    XCTAssertTrue(events.contains { $0["type"].string == "llm_error" })
  }

  func testToolsAndProposalReplayRemainReadOnlyUntilExplicitInsert() async throws {
    func frame(_ value: JSONValue) throws -> Data {
      Data(
        ("data: " + String(decoding: try value.data(), as: UTF8.self) + "\n\ndata: [DONE]\n\n").utf8
      )
    }
    let arguments = "{\"code\":\"let q=7\",\"dialect\":\"modern\"}"
    let call = JSONValue.object([
      "index": .number(0), "id": .string("proposal"),
      "function": .object(["name": .string("propose_cell"), "arguments": .string(arguments)]),
    ])
    let first = JSONValue.object([
      "choices": .array([
        .object([
          "index": .number(0), "delta": .object(["tool_calls": .array([call])]),
          "finish_reason": .string("tool_calls"),
        ])
      ])
    ])
    let last = JSONValue.object([
      "choices": .array([
        .object([
          "index": .number(0), "delta": .object(["content": .string("等待插入")]),
          "finish_reason": .string("stop"),
        ])
      ])
    ])
    FixtureProtocol.registry.configureSequence([try frame(first), try frame(last)])
    let controller = NotebookController()
    await controller.start(restoreLastDocument: false)
    var config = controller.config
    var profiles = config["llm"]["profiles"].array
    profiles[0]["base_url"] = .string("https://fixture.invalid/v1")
    profiles[0]["kind"] = .string("openai_chat")
    profiles[0]["api_key"] = .string("synthetic-tool-key")
    profiles[0]["supports_tools"] = .bool(true)
    config["llm"]["profiles"] = .array(profiles)
    config["llm"]["chat"] = profiles[0]["name"]
    config["llm"]["enabled"] = .bool(true)
    _ = try await controller.call(.object(["type": .string("set_config"), "config": config]))
    let client = try XCTUnwrap(controller.kernel)
    let done = expectation(description: "real tool replay done")
    let configuration = URLSessionConfiguration.ephemeral
    configuration.protocolClasses = [FixtureProtocol.self]
    let transport = LLMTransport(client: client, configuration: configuration) { packet in
      // Forward actual packets through the controller's public transport callback.
      controller.receiveHTTP(packet)
      if packet.events.contains(where: { $0.body["type"].string == "llm_done" }) { done.fulfill() }
    }
    controller.transport = transport
    let started = await controller.beginAI(
      feature: "chat",
      body: .object([
        "type": .string("llm_chat"),
        "messages": .array([
          .object([
            "role": .string("user"), "content": .string("propose"), "tool_calls": .array([]),
            "tool_call_id": .null,
          ])
        ]),
      ]))
    let job = try XCTUnwrap(started)
    await fulfillment(of: [done], timeout: 10)
    XCTAssertEqual(FixtureProtocol.registry.count(), 2)
    XCTAssertTrue(controller.cells.isEmpty)
    XCTAssertNotNil(controller.suggestions[job])
    let variables = try await client.request(.object(["type": .string("get_variables")]))
    XCTAssertTrue(variables.response.body["items"].array.isEmpty)
    controller.insertSuggestion(job)
    XCTAssertEqual(controller.cells.first?.status, .Stale)
    if let id = controller.selected { await controller.run(id) }
    let evaluated = try await client.request(
      .object([
        "type": .string("inspect_expression"), "source": .string("q"), "numeric": .bool(false),
      ]))
    XCTAssertEqual(evaluated.response.body["value"]["input_form"].string, "7")
    controller.kernel?.close()
  }
  func testCancellationSuppressesLateChunks() async throws {
    let client = try KernelClient()
    defer { client.close() }
    let configuration = URLSessionConfiguration.ephemeral
    configuration.protocolClasses = [FixtureProtocol.self]
    FixtureProtocol.registry.configure(body: Data("data: [DONE]\n\n".utf8), delay: 1)
    var events: [JSONValue] = []
    let transport = LLMTransport(client: client, configuration: configuration) {
      events += $0.events.map(\.body)
    }
    var profile = try await client.request(.object(["type": .string("get_config")])).response.body[
      "config"]["llm"]["profiles"][0]
    profile["base_url"] = .string("https://fixture.invalid/v1")
    profile["api_key"] = .string("synthetic-cancel-key")
    let start = try await client.request(
      .object([
        "type": .string("llm_test_profile"), "request_id": .string("cancel"),
        "profile": profile["name"], "config": profile,
      ]))
    transport.start(id: "cancel", request: start.response.body["http"], timeout: 5000)
    await Task.yield()
    transport.cancel("cancel")
    _ = try await client.request(
      .object(["type": .string("llm_cancel"), "request_id": .string("cancel")]))
    try await Task.sleep(for: .milliseconds(100))
    XCTAssertTrue(events.isEmpty)
  }
}
