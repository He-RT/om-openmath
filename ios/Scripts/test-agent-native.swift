import Foundation
import JavaScriptCore

@main @MainActor struct MobileAgentNativeFixture {
  static func main() async throws {
    let context = JSContext()!
    MobileJSHost.install(in: context)
    var done = false, rounds = 0, tools = 0, records = 0
    var error: String?
    context.exceptionHandler = { _, value in error = value?.toString() }
    let uuid: @convention(block) () -> String = { UUID().uuidString }
    context.setObject(uuid, forKeyedSubscript: "nativeUUID" as NSString)
    let post: @convention(block) (String) -> Void = { source in
      Task { @MainActor in
        let message = try! JSONSerialization.jsonObject(with: Data(source.utf8)) as! [String: Any]
        let id = message["id"] as! String, op = message["op"] as! String, payload = message["payload"] as! [String: Any]
        var result: [String: Any] = ["ok": true]
        if op == "record" { records += 1 }
        if op == "tool" { tools += 1; result = ["ok": true, "status": "completed", "operation_ref": "operation-1", "request_ref": "result-1", "data": ["title": "中文笔记"]] }
        if op == "model" {
          rounds += 1
          let parts: [[String: Any]] = rounds == 1 ? [["kind": "tool_call", "part_id": "tool-part", "tool_call_id": "call-1", "name": "read_notebook", "arguments_json": "{}"]] : [["kind": "text", "part_id": "answer", "text": "中文🙂"]]
          result = ["message": ["record_type": "canonical_agent_message", "message_version": 1, "message_id": payload["message"]!, "role": "assistant", "task_id": "task-1", "agent_turn_id": payload["turn"]!, "parts": parts], "replay": NSNull(), "usage": ["input": NSNull(), "output": NSNull()], "finish": rounds == 1 ? "tool_calls" : "complete"]
        }
        if op == "rejected" { error = String(describing: payload); done = true }
        if op == "settled" { if payload["reason"] as? String != "ended" { error = String(describing: payload) }; done = true }
        let data = try! JSONSerialization.data(withJSONObject: result)
        context.objectForKeyedSubscript("mobileResolve")?.call(withArguments: [id, String(decoding: data, as: UTF8.self), NSNull()])
      }
    }
    context.setObject(post, forKeyedSubscript: "nativePost" as NSString)
    context.evaluateScript(try String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8))
    precondition(error == nil, error ?? "")
    let task: [String: Any] = ["id": "task-1", "text": "读取笔记本", "parts": [["kind": "text", "part_id": "input", "text": "读取笔记本"]], "history": [], "results": [], "images": [], "binding": [:], "model": "synthetic-no-network", "contextWindow": 0, "maxTokens": 1024, "tools": [["name": "read_notebook", "description": "Read notebook", "parameters": ["type": "object", "properties": [:], "additionalProperties": false]]]]
    context.objectForKeyedSubscript("mobileStart")?.call(withArguments: [String(decoding: try JSONSerialization.data(withJSONObject: task), as: UTF8.self)])
    for _ in 0..<500 { if done { break }; try await Task.sleep(for: .milliseconds(10)) }
    precondition(done && error == nil && rounds == 2 && tools == 1 && records == 4, "Native JavaScriptCore Pi failed: \(error ?? "timeout") rounds=\(rounds) tools=\(tools) records=\(records)")
    print("Native JavaScriptCore Pi scheduler: two rounds, one acknowledged tool, four durable records; no simulator or network")
  }
}
