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
    var responder: (@Sendable (URLRequest, Int) -> Data)?
    func configure(status: Int = 200, body: Data, delay: TimeInterval = 0) {
      lock.lock()
      defer { lock.unlock() }
      self.status = status
      self.body = body
      self.delay = delay
      calls = []
      sequence = []
      responder = nil
    }
    func take(_ request: URLRequest) -> (Int, Data, TimeInterval) {
      lock.lock()
      defer { lock.unlock() }
      calls.append(request)
      return (
        status, responder?(request, calls.count) ?? (sequence.isEmpty ? body : sequence[min(calls.count - 1, sequence.count - 1)]), delay
      )
    }
    func configureSequence(_ values: [Data]) {
      lock.lock()
      defer { lock.unlock() }
      status = 200
      delay = 0
      calls = []
      sequence = values
      responder = nil
    }
    func configureResponder(_ responder: @escaping @Sendable (URLRequest, Int) -> Data) {
      lock.lock(); defer { lock.unlock() }
      status = 200; delay = 0; calls = []; sequence = []; self.responder = responder
    }
    func count() -> Int {
      lock.lock()
      defer { lock.unlock() }
      return calls.count
    }
  }
  final class Fixtures: @unchecked Sendable {
    private let lock = NSLock()
    private var entries: [String: Registry] = [:]
    func install(_ registry: Registry) -> String {
      let host = UUID().uuidString.lowercased() + ".fixture.invalid"
      lock.lock()
      defer { lock.unlock() }
      entries[host] = registry
      return host
    }
    func registry(for request: URLRequest) -> Registry? {
      lock.lock()
      defer { lock.unlock() }
      return entries[request.url?.host ?? ""]
    }
  }
  static let fixtures = Fixtures()
  private var work: DispatchWorkItem?
  override class func canInit(with request: URLRequest) -> Bool {
    request.url?.host?.hasSuffix(".fixture.invalid") == true
  }
  override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
  override func startLoading() {
    guard let registry = Self.fixtures.registry(for: request) else {
      client?.urlProtocol(self, didFailWithError: URLError(.badURL))
      return
    }
    let (status, body, delay) = registry.take(request)
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

private enum MobileLoopbackReply {
  static func response(_ request: URLRequest, round: Int) -> Data {
    do {
      var bytes = request.httpBody ?? Data()
      if bytes.isEmpty, let stream = request.httpBodyStream {
        stream.open(); defer { stream.close() }
        var buffer = [UInt8](repeating: 0, count: 4096)
        while stream.hasBytesAvailable {
          let count = stream.read(&buffer, maxLength: buffer.count)
          guard count >= 0 else { throw URLError(.cannotDecodeContentData) }
          if count == 0 { break }; bytes.append(contentsOf: buffer.prefix(count))
        }
      }
      let wire = try JSONDecoder().decode(JSONValue.self, from: bytes)
      let tool = wire["messages"].array.last { $0["role"].string == "tool" }
      let reply = try tool.map { try JSONDecoder().decode(JSONValue.self, from: Data($0["content"].string.utf8)) } ?? .null
      let name: String, args: JSONValue
      switch round {
      case 1: name = "read_notebook"; args = .object(["view": .string("outline"), "limit": .number(1)])
      case 2:
        guard reply["ok"].bool else { throw URLError(.cannotParseResponse) }
        name = "preview_source"; args = .object(["snapshot_ref": reply["data"]["snapshot_ref"], "input": .object(["kind": .string("patch"), "operations": .array([.object([
          "type": .string("insert_cell"), "client_key": .string("sum"), "after": .null, "kind": .string("Math"), "dialect": .string("Modern"), "source": .string("2+2")])])])])
      case 3:
        guard reply["ok"].bool, reply["data"]["valid"].bool else { throw URLError(.cannotParseResponse) }
        name = "apply_notebook_patch"; args = .object(["preview_ref": reply["data"]["preview_ref"]])
      case 4:
        guard reply["ok"].bool else { throw URLError(.cannotParseResponse) }
        name = "run_cells"; args = .object(["snapshot_ref": reply["data"]["snapshot_ref"], "cell_ids": reply["data"]["changed_cells"], "scope": .string("required")])
      case 5:
        guard reply["ok"].bool else { throw URLError(.cannotParseResponse) }
        name = "inspect_result"; args = .object(["result_ref": reply["data"]["cells"][0]["result_ref"], "part": .string("value")])
      default:
        guard round == 6, reply["ok"].bool, reply["data"]["items"][0]["input_form"].string == "4" else { throw URLError(.cannotParseResponse) }
        return try frame(.object(["content": .string("已写入并计算：4")]), finish: "stop")
      }
      let call = JSONValue.object(["index": .number(0), "id": .string("mobile-call-\(round)"), "type": .string("function"), "function": .object(["name": .string(name), "arguments": .string(String(decoding: try args.data(), as: UTF8.self))])])
      return try frame(.object(["tool_calls": .array([call])]), finish: "tool_calls")
    } catch {
      return (try? frame(.object(["content": .string("Local fixture rejected the actual request or tool result")]), finish: "stop")) ?? Data()
    }
  }
  private static func frame(_ delta: JSONValue, finish: String) throws -> Data {
    let value = JSONValue.object(["id": .string("mobile-fixture-response"), "model": .string("synthetic-no-network"), "choices": .array([.object(["index": .number(0), "delta": delta, "finish_reason": .string(finish)])])])
    return Data(("data: " + String(decoding: try value.data(), as: UTF8.self) + "\n\ndata: [DONE]\n\n").utf8)
  }
}

@MainActor final class TransportTests: XCTestCase {
  func testMobilePiRunsActualNotebookToolsAndUndoWithSyntheticTransport() async throws {
    let fixture = FixtureProtocol.Registry()
    fixture.configureResponder { request, round in MobileLoopbackReply.response(request, round: round) }
    let host = FixtureProtocol.fixtures.install(fixture)
    let configuration = URLSessionConfiguration.ephemeral
    configuration.protocolClasses = [FixtureProtocol.self]
    let agent = MobileNotebookAgent { MobileModelTransport(client: $0, configuration: configuration) }
    let defaults = try XCTUnwrap(UserDefaults(suiteName: "org.openmath.mobile-fixture." + UUID().uuidString))
    let controller = NotebookController(defaults: defaults, agent: agent)
    await controller.start(restoreLastDocument: false)
    XCTAssertNil(controller.error)
    let stored = try SettingsStore.load(), original = stored ?? controller.config
    var config = controller.config, profile = config["llm"]["profiles"][0]
    profile["kind"] = .string("openai_chat"); profile["base_url"] = .string("https://\(host)/v1")
    profile["model"] = .string("synthetic-no-network"); profile["api_key"] = .string("")
    profile["requires_api_key"] = .bool(false); profile["supports_tools"] = .bool(true)
    profile["extra_headers"] = .object([:]); profile["extra_body"] = .object([:])
    config["llm"]["profiles"] = .array([profile]); config["llm"]["enabled"] = .bool(true)
    for feature in ["chat", "fix", "translate"] { config["llm"][feature] = profile["name"] }
    defer {
      agent.stop(); controller.kernel?.close()
      _ = try? SettingsStore.persist(original, previous: config)
      if stored == nil {
        let url = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("OpenMath-pre-alpha.5/settings.json")
        try? FileManager.default.removeItem(at: url)
      }
    }
    try await controller.applySettings(config)
    agent.newConversation(); agent.selectedProfile = profile["name"].string
    agent.execute = true; agent.allowDestination(); agent.draft = "新增 2+2，计算、检查结果并回复。"
    await agent.send()
    for _ in 0..<1000 { if !agent.busy && !agent.settling { break }; try await Task.sleep(for: .milliseconds(10)) }
    XCTAssertFalse(agent.busy); XCTAssertFalse(agent.settling); XCTAssertNil(agent.error, agent.error ?? "")
    XCTAssertEqual(fixture.count(), 6, agent.outcome)
    XCTAssertEqual(controller.cells.count, 1)
    XCTAssertEqual(controller.cells.first?.input.source, "2+2")
    XCTAssertEqual(controller.cells.first?.output["items"][0]["input_form"].string, "4")
    XCTAssertEqual(agent.state.records.last?["canonical"]["parts"][0]["text"].string, "已写入并计算：4")
    XCTAssertTrue(agent.usage["input"].isNull)
    let file = try XCTUnwrap(controller.fileURL)
    XCTAssertEqual(try JSONDecoder().decode(NotebookFile.self, from: Data(contentsOf: file)).cells.first?.source, "2+2")
    let tools = MobileNotebookTools(controller: controller)
    let read = JSONValue.object(["view": .string("outline")])
    let first = await tools.invoke("read_notebook", args: read, execute: false, agent: agent)
    let second = await tools.invoke("read_notebook", args: read, execute: false, agent: agent)
    XCTAssertEqual(first["data"]["snapshot_ref"], second["data"]["snapshot_ref"])
    let transaction = try XCTUnwrap(agent.state.transactions.first { $0.state == "applied" })
    await agent.undo(transaction.id)
    XCTAssertTrue(controller.cells.isEmpty)
    XCTAssertEqual(agent.state.transactions.first?.state, "undone")
    XCTAssertTrue(try JSONDecoder().decode(NotebookFile.self, from: Data(contentsOf: file)).cells.isEmpty)
    await agent.stopAndWait()
  }
  private func probe(status: Int, timeout: Double = 5000, delay: TimeInterval = 0) async throws
    -> [JSONValue]
  {
    let stream =
      "data: {\"choices\":[{\"delta\":{\"content\":\"中文🙂\"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
    // A cancelled URLSession may enter URLProtocol after the next test starts.
    // Give each request its own immutable endpoint and counters.
    let fixture = FixtureProtocol.Registry()
    fixture.configure(status: status, body: Data(stream.utf8), delay: delay)
    let host = FixtureProtocol.fixtures.install(fixture)
    let client = try KernelClient()
    defer { client.close() }
    let before = try await client.request(.object(["type": .string("get_config")])).response.body[
      "config"]
    var profile = before["llm"]["profiles"][0]
    profile["kind"] = .string("openai_chat")
    profile["base_url"] = .string("https://\(host)/v1")
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
    XCTAssertEqual(fixture.count(), 1, "redirects must not replay credentials")
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
    let fixture = FixtureProtocol.Registry()
    fixture.configureSequence([try frame(first), try frame(last)])
    let host = FixtureProtocol.fixtures.install(fixture)
    let controller = NotebookController()
    await controller.start(restoreLastDocument: false)
    var config = controller.config
    var profiles = config["llm"]["profiles"].array
    profiles[0]["base_url"] = .string("https://\(host)/v1")
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
    XCTAssertEqual(fixture.count(), 2)
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
    let fixture = FixtureProtocol.Registry()
    fixture.configure(body: Data("data: [DONE]\n\n".utf8), delay: 1)
    let host = FixtureProtocol.fixtures.install(fixture)
    var events: [JSONValue] = []
    let transport = LLMTransport(client: client, configuration: configuration) {
      events += $0.events.map(\.body)
    }
    var profile = try await client.request(.object(["type": .string("get_config")])).response.body[
      "config"]["llm"]["profiles"][0]
    profile["base_url"] = .string("https://\(host)/v1")
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
