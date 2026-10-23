import Foundation

private final class MobileNoRedirect: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
  func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
    newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) { completionHandler(nil) }
}
@MainActor final class MobileModelTransport {
  private let client: KernelClient
  private let session: URLSession
  init(client: KernelClient, configuration: URLSessionConfiguration = .ephemeral) {
    self.client = client
    let config = configuration.copy() as! URLSessionConfiguration
    config.httpCookieStorage = nil; config.urlCache = nil; config.requestCachePolicy = .reloadIgnoringLocalCacheData
    session = URLSession(configuration: config, delegate: MobileNoRedirect(), delegateQueue: nil)
  }
  func close() { session.invalidateAndCancel() }
  func run(_ payload: JSONValue, task: String, profile: JSONValue, delta: @escaping @MainActor (String) -> Void) async throws -> JSONValue {
    let id = UUID().uuidString
    let prepared = try await client.modelCommand(.object(["op": .string("prepare"), "id": .string(id), "request": payload["request"],
      "message": payload["message"], "task": .string(task), "turn": payload["turn"]]))
    do {
      let anthropic = profile["kind"].string == "anthropic"
      let base = profile["base_url"].string.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
      guard let url = URL(string: base + (anthropic ? "/messages" : "/chat/completions")),
        let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
        ["https", "http"].contains(components.scheme ?? ""), components.host != nil,
        components.user == nil, components.password == nil, components.query == nil, components.fragment == nil
      else { throw KernelError.message("模型端点无效") }
      var request = URLRequest(url: url); request.httpMethod = "POST"
      request.timeoutInterval = max(1, min(profile["timeout_ms"].double / 1000, 300))
      request.setValue("application/json", forHTTPHeaderField: "Content-Type")
      let key = profile["api_key"].string
      guard key != "***" else { throw KernelError.message("模型凭据不可用") }
      guard !profile["requires_api_key"].bool || !key.isEmpty else { throw KernelError.message("此模型需要配置 API 密钥") }
      if !key.isEmpty { request.setValue(anthropic ? key : "Bearer " + key, forHTTPHeaderField: anthropic ? "x-api-key" : "Authorization") }
      if anthropic { request.setValue("2023-06-01", forHTTPHeaderField: "anthropic-version") }
      for (name, value) in profile["extra_headers"].object { request.setValue(value.string, forHTTPHeaderField: name) }
      var body = try JSONDecoder().decode(JSONValue.self, from: Data(prepared["body"].string.utf8))
      let reserved: Set<String> = ["model", "messages", "tools", "stream", "temperature", "max_tokens", "max_completion_tokens", "system"]
      for (name, value) in profile["extra_body"].object {
        guard !reserved.contains(name) else { throw KernelError.message("附加参数不能覆盖协议字段") }
        body[name] = value
      }
      request.httpBody = try body.data()
      let (bytes, response) = try await session.bytes(for: request)
      guard let response = response as? HTTPURLResponse else { throw KernelError.message("无效模型响应") }
      _ = try await client.modelCommand(.object(["op": .string("headers"), "id": .string(id), "status": .number(Double(response.statusCode))]))
      var buffer = Data(), count = 0
      for try await byte in bytes {
        try Task.checkCancellation(); count += 1
        guard count <= 8 * 1024 * 1024 else { throw KernelError.message("模型响应超过预算") }
        buffer.append(byte)
        if byte == 10 || buffer.count == 4096 {
          let result = try await client.modelCommand(.object(["op": .string("feed"), "id": .string(id), "bytes": .array(buffer.map { .number(Double($0)) })]))
          if !result["text"].string.isEmpty { delta(result["text"].string) }
          buffer.removeAll(keepingCapacity: true)
        }
      }
      if !buffer.isEmpty {
        let result = try await client.modelCommand(.object(["op": .string("feed"), "id": .string(id), "bytes": .array(buffer.map { .number(Double($0)) })]))
        if !result["text"].string.isEmpty { delta(result["text"].string) }
      }
      return try await client.modelCommand(.object(["op": .string("finish"), "id": .string(id)]))
    } catch {
      _ = try? await client.modelCommand(.object(["op": .string("cancel"), "id": .string(id)]))
      throw error
    }
  }
}
