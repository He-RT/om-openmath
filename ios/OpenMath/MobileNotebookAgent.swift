import JavaScriptCore
import Observation
import PDFKit
import SwiftUI
import UniformTypeIdentifiers

@MainActor @Observable final class MobileNotebookAgent {
  private(set) var state = MobileAgentState()
  var draft: String { get { state.draft } set { state.draft = newValue; persistDraft() } }
  var selectedProfile: String { get { state.profile } set { state.profile = newValue; persistDraft() } }
  var execute: Bool { get { state.execute } set { state.execute = newValue; persistDraft() } }
  private var imageCapabilityKey: String { selectedProfile + "|" + profile["base_url"].string + "|" + profile["model"].string }
  var imageInput: Bool {
    get { state.imageProfiles.contains(imageCapabilityKey) }
    set { state.imageProfiles.removeAll { $0 == imageCapabilityKey }; if newValue { state.imageProfiles.append(imageCapabilityKey) }; persistDraft() }
  }
  private(set) var busy = false
  var settling: Bool { !busy && !jobs.isEmpty }
  private(set) var streaming = ""
  private(set) var outcome = ""
  private(set) var usage: JSONValue = .null
  var error: String?
  var fullInput: String?
  private var failedStore = false
  private weak var controller: NotebookController?
  private var context: JSContext?
  private var store: MobileAgentStore?
  private var toolHost: MobileNotebookTools?
  private var transport: MobileModelTransport?
  private var jobs: [String: Task<Void, Never>] = [:]
  private var generation = UUID()
  private var taskID = ""
  private var activeProfile: JSONValue = .null
  private var draftSave: Task<Void, Never>?
  private var timer: Task<Void, Never>?
  private var authorizedDestinations = Set<String>()
  private var backupDraft = ""
  var profiles: [JSONValue] {
    guard controller?.config["llm"]["enabled"].bool == true else { return [] }
    return controller?.config["llm"]["profiles"].array.filter { ["openai_chat", "anthropic"].contains($0["kind"].string) && !$0["base_url"].string.isEmpty && (!$0["requires_api_key"].bool || !$0["api_key"].string.isEmpty) } ?? []
  }
  var profile: JSONValue { profiles.first { $0["name"].string == selectedProfile } ?? .null }
  var destination: String { profile["base_url"].string }
  var authorized: Bool { authorizedDestinations.contains(selectedProfile + "|" + destination) }
  func allowDestination() { authorizedDestinations.insert(selectedProfile + "|" + destination) }
  func attach(_ controller: NotebookController, document: String) {
    stop(); generation = UUID(); self.controller = controller; toolHost = MobileNotebookTools(controller: controller)
    state = MobileAgentState(); outcome = ""; streaming = ""; usage = .null; fullInput = nil; error = nil
    do {
      let store = try MobileAgentStore(document: document); self.store = store; state = try store.load(); failedStore = false
      if state.profile.isEmpty { state.profile = controller.config["llm"]["chat"].string }
      for i in state.transactions.indices where state.transactions[i].state == "pending" {
        state.transactions[i].state = state.transactions[i].after == controller.notebook ? "applied" : "unknown"
      }
      try persist()
    } catch { failedStore = true; store = nil; self.error = error.localizedDescription }
  }
  private func persistDraft() {
    draftSave?.cancel()
    draftSave = Task { [weak self] in
      try? await Task.sleep(for: .milliseconds(300)); guard !Task.isCancelled else { return }
      do { try self?.persist() } catch { self?.error = error.localizedDescription }
    }
  }
  func relocate(document: String) throws {
    guard !failedStore else { throw KernelError.message("会话存储读取失败；原文件保留，请先处理会话错误") }
    let destination = try MobileAgentStore(document: document)
    try destination.save(state); store = destination
  }
  func persist() throws { guard let store else { throw KernelError.message("会话存储尚未就绪") }; try store.save(state) }
  func appendTransaction(_ value: MobileSourceTransaction) throws { state.transactions.append(value); try persist() }
  func finishTransaction(_ id: String, state value: String) throws {
    guard let i = state.transactions.firstIndex(where: { $0.id == id }) else { throw KernelError.message("找不到事务记录") }
    state.transactions[i].state = value; try persist()
  }
  func recordOperation(_ id: String, name: String, reply: JSONValue) throws {
    var value = reply; value["name"] = .string(name); state.operations[id] = value; try persist()
  }
  func newConversation() {
    guard !busy, jobs.isEmpty, controller?.agentCommitting != true else { return }
    do { try store?.archive(state); let profile = state.profile, images = state.imageProfiles; state = MobileAgentState(); state.profile = profile; state.imageProfiles = images; try persist(); outcome = ""; streaming = ""; toolHost?.reset() }
    catch { self.error = error.localizedDescription }
  }
  func history() -> [String] { (try? store?.history().map(\.lastPathComponent)) ?? [] }
  func restoreConversation(_ name: String) {
    guard !busy, jobs.isEmpty, controller?.agentCommitting != true else { return }
    do {
      guard let store else { return }
      let restored = try store.restore(name)
      try store.archive(state); state = restored; try persist(); toolHost?.reset(); outcome = ""; streaming = ""
    } catch { self.error = error.localizedDescription }
  }
  func stop() {
    generation = UUID()
    context?.objectForKeyedSubscript("mobileStop")?.call(withArguments: [])
    for task in jobs.values { task.cancel() }
    timer?.cancel(); timer = nil; transport?.close(); transport = nil
    if busy {
      if state.draft.isEmpty { state.draft = backupDraft }
      outcome = "已停止，已提交的文稿修改保留，可从操作记录撤销。"
    }
    busy = false; context = nil
    do { if store != nil { try persist() } } catch { self.error = error.localizedDescription }
  }
  func stopAndWait() async {
    stop()
    for job in Array(jobs.values) { await job.value }
  }
  func send() async {
    guard !busy, jobs.isEmpty, let controller, !profile.isNull, authorized, !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
      let kernel = controller.kernel else { return }
    guard controller.composingCells.isEmpty, !controller.agentCommitting else { error = "请先完成当前编辑"; return }
    if execute && !profile["supports_tools"].bool { error = "此模型未声明工具能力，不能执行文稿任务"; return }
    generation = UUID()
    let token = generation
    do {
      let saved = try SettingsStore.load() ?? controller.config
      guard let profile = saved["llm"]["profiles"].array.first(where: { $0["name"].string == selectedProfile }) else { throw KernelError.message("请选择已配置模型") }
      guard ["openai_chat", "anthropic"].contains(profile["kind"].string), profile["model"].string == self.profile["model"].string, profile["base_url"].string == destination else { throw KernelError.message("模型配置已改变，请重新确认目标") }
      activeProfile = profile; taskID = "task-" + UUID().uuidString.lowercased()
      backupDraft = draft; streaming = ""; outcome = ""; error = nil
      let binding = JSONValue.object(["provider_ref": .string("provider-" + MobileAgentStore.hash(profile["name"].string)), "route_ref": .string("route-" + MobileAgentStore.hash(profile["base_url"].string)),
        "model_ref": .string("model-" + MobileAgentStore.hash(profile["model"].string)), "adapter_version": .string("single-turn-v1"), "protocol": .string(profile["kind"].string == "anthropic" ? "anthropic_messages" : "chat_completions")])
      let prompt = try resourceJSON("mobile-prompt")
      let system = prompt["template"].string.replacingOccurrences(of: "{{version}}", with: "0.1.0-pre-alpha.5")
        .replacingOccurrences(of: "{{mode_line}}", with: prompt["modes"][execute ? "execute" : "discuss"].string)
        + "\n当前宿主是 iOS/iPadOS 原生工作台。只能依据工具的实际结果陈述运行与修改；几何数据不是截图。"
      let systemRecord = JSONValue.object(["canonical": .object(["record_type": .string("canonical_agent_message"), "message_version": .number(1), "message_id": .string(UUID().uuidString), "role": .string("system"), "task_id": .string(taskID), "agent_turn_id": .null, "parts": .array([.object(["kind": .string("text"), "part_id": .string(UUID().uuidString), "text": .string(system)])])]), "replay": .null])
      var history = MobileAgentReplay.history(state.records, binding: binding)
      let readOnly: Set<String> = ["read_notebook", "search_functions", "get_function_docs", "preview_source", "inspect_result", "evaluate_scratch", "read_attachment", "prepare_attachment", "get_operation_status"]
      let declarations = profile["supports_tools"].bool ? try resourceJSON("mobile-tools").array.filter { execute || readOnly.contains($0["name"].string) } : []
      var parts: [JSONValue] = [.object(["kind": .string("text"), "part_id": .string(UUID().uuidString), "text": .string(draft)])]
      for attachment in state.attachments { parts.append(.object(["kind": .string("text"), "part_id": .string(UUID().uuidString), "text": .string("Attachment \(attachment.name), media_ref=\(attachment.id), \(attachment.bytes) bytes. Use read_attachment to inspect.")])) }
      var images: [JSONValue] = []
      if imageInput {
        for attachment in state.attachments where attachment.image != nil {
          let part = UUID().uuidString.lowercased()
          parts.append(.object(["kind": .string("media"), "part_id": .string(part), "media_ref": .string(attachment.id), "submission_kind": .string("image")]))
          images.append(.object(["message_id": .string("current"), "part_id": .string(part), "media_ref": .string(attachment.id), "media_type": .string(attachment.mediaType ?? "image/jpeg"), "data": .string(attachment.image!)]))
        }
      }
      for i in history.indices {
        let message = history[i]["canonical"]
        history[i]["canonical"]["parts"] = .array(message["parts"].array.map { part in
          guard part["kind"].string == "media" else { return part }
          if imageInput, let attachment = state.attachments.first(where: { $0.id == part["media_ref"].string }), let data = attachment.image {
            images.append(.object(["message_id": message["message_id"], "part_id": part["part_id"], "media_ref": part["media_ref"], "media_type": .string(attachment.mediaType ?? "image/jpeg"), "data": .string(data)])); return part
          }
          return .object(["kind": .string("text"), "part_id": part["part_id"], "text": .string("[Previous image omitted; original conversation retained]")])
        })
      }
      let task = JSONValue.object(["id": .string(taskID), "text": .string(draft), "parts": .array(parts), "history": .array([systemRecord] + history), "results": .array(state.results), "tools": .array(declarations), "binding": binding,
        "images": .array(images), "model": profile["model"], "maxTokens": profile["max_tokens"], "contextWindow": .number(0)])
      let bytes = try task.data(); guard bytes.count <= 6 * 1024 * 1024 else { throw KernelError.message("会话上下文过长，请新建会话；旧记录会保留") }
      guard token == generation else { return }
      let js = JSContext()!
      MobileJSHost.install(in: js)
      js.exceptionHandler = { [weak self] _, _ in self?.error = "助手解释器运行失败，文稿已保留"; self?.stop() }
      let post: @convention(block) (String) -> Void = { [weak self] source in self?.receive(source, generation: token) }
      let uuid: @convention(block) () -> String = { UUID().uuidString.lowercased() }
      js.setObject(post, forKeyedSubscript: "nativePost" as NSString); js.setObject(uuid, forKeyedSubscript: "nativeUUID" as NSString)
      guard let source = Bundle.main.url(forResource: "mobile-agent", withExtension: "js") else { throw KernelError.message("缺少 Pi 运行资源") }
      js.evaluateScript(try String(contentsOf: source, encoding: .utf8))
      guard js.exception == nil else { throw KernelError.message("Pi 运行资源无法初始化") }
      context = js; transport = MobileModelTransport(client: kernel); busy = true
      timer = Task { [weak self] in try? await Task.sleep(for: .seconds(300)); guard !Task.isCancelled else { return }; self?.stop(); self?.outcome = "已达到本轮时间预算" }
      js.objectForKeyedSubscript("mobileStart")?.call(withArguments: [String(decoding: bytes, as: UTF8.self)])
    } catch { self.error = error.localizedDescription; stop() }
  }
  private func receive(_ source: String, generation token: UUID) {
    guard token == generation, let bytes = source.data(using: .utf8), let value = try? JSONDecoder().decode(JSONValue.self, from: bytes) else { return }
    let id = value["id"].string
    let key = token.uuidString + ":" + id
    guard jobs[key] == nil else { return }
    jobs[key] = Task { [weak self] in
      guard let self else { return }
      defer { jobs.removeValue(forKey: key) }
      guard token == generation else { return }
      do {
        let result = try await dispatch(value["op"].string, payload: value["payload"])
        guard token == generation else { return }
        context?.objectForKeyedSubscript("mobileResolve")?.call(withArguments: [id, String(decoding: try result.data(), as: UTF8.self), NSNull()])
      } catch {
        if token == generation { context?.objectForKeyedSubscript("mobileResolve")?.call(withArguments: [id, "null", error.localizedDescription]) }
      }
    }
  }
  private func dispatch(_ op: String, payload: JSONValue) async throws -> JSONValue {
    switch op {
    case "record":
      state.records.append(payload["record"])
      if !payload["result"].isNull { state.results.append(payload["result"]) }
      if payload["record"]["canonical"]["role"].string == "user" { if state.draft == backupDraft { state.draft = "" } }
      if !payload["usage"].isNull { usage = payload["usage"]; streaming = "" }
      try persist(); return .object(["ok": .bool(true)])
    case "model":
      try Task.checkCancellation()
      guard let kernel = controller?.kernel else { throw CancellationError() }
      transport?.close()
      let transport = MobileModelTransport(client: kernel)
      self.transport = transport
      fullInput = payload["request"].pretty
      return try await transport.run(payload, task: taskID, profile: activeProfile) { [weak self] text in self?.streaming += text }
    case "tool":
      try Task.checkCancellation()
      guard let toolHost else { throw KernelError.message("文稿工具尚未就绪") }
      return await toolHost.invoke(payload["name"].string, args: payload["args"], execute: execute, agent: self)
    case "rejected":
      let operation = "operation-" + UUID().uuidString.lowercased()
      let reply = JSONValue.object(["ok": .bool(false), "status": .string("failed"), "request_ref": .string("request-" + UUID().uuidString.lowercased()), "operation_ref": .string(operation), "document": .null, "data": .null,
        "error": .object(["code": .string("INVALID_ARGUMENT"), "message": payload["message"], "recovery": .string("none"), "details": .object([:])]), "pagination": .object(["truncated": .bool(false), "next_cursor": .null])])
      try recordOperation(operation, name: payload["name"].string, reply: reply)
      return reply
    case "settled":
      busy = false; timer?.cancel(); streaming = ""; transport?.close(); transport = nil
      outcome = payload["reason"].string == "ended" ? "本轮结束。实际修改、运行与失败见操作记录。" : payload["reason"].string == "stopped" ? "已停止" : "本轮未完成：" + payload["error"].string
      if payload["reason"].string != "ended", state.draft.isEmpty { state.draft = backupDraft }
      try persist(); return .object(["ok": .bool(true)])
    default: throw KernelError.message("未注册助手通道")
    }
  }
  private func resourceJSON(_ name: String) throws -> JSONValue {
    guard let url = Bundle.main.url(forResource: name, withExtension: "json") else { throw KernelError.message("缺少助手资源") }
    return try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: url))
  }
  func importFile(_ url: URL) async {
    guard !busy, jobs.isEmpty, controller?.agentCommitting != true else { return }
    let scoped = url.startAccessingSecurityScopedResource(); defer { if scoped { url.stopAccessingSecurityScopedResource() } }
    do {
      guard state.attachments.count < 12 else { throw KernelError.message("每个会话最多保留 12 个附件") }
      let info = try url.resourceValues(forKeys: [.fileSizeKey, .isRegularFileKey, .contentTypeKey])
      guard info.isRegularFile == true, let count = info.fileSize, count <= 10 * 1024 * 1024 else { throw KernelError.message("附件必须是 10 MiB 以内的普通文件") }
      let bytes = try Data(contentsOf: url), type = info.contentType
      var item = MobileAttachment(id: "media-" + UUID().uuidString.lowercased(), name: url.lastPathComponent, kind: "file", bytes: bytes.count)
      guard let store else { throw KernelError.message("附件存储尚未就绪") }
      item.contentHash = try store.retainAttachment(bytes)
      if type?.conforms(to: .image) == true, let image = UIImage(data: bytes) {
        let scale = min(1, 1280 / max(image.size.width, image.size.height)), size = CGSize(width: image.size.width * scale, height: image.size.height * scale)
        let format = UIGraphicsImageRendererFormat(); format.scale = 1
        let prepared = UIGraphicsImageRenderer(size: size, format: format).image { _ in image.draw(in: CGRect(origin: .zero, size: size)) }
        guard let jpeg = prepared.jpegData(compressionQuality: 0.78), jpeg.count <= 1024 * 1024 else { throw KernelError.message("图片准备超过预算，请使用较小图片") }
        item.kind = "image"; item.image = jpeg.base64EncodedString(); item.mediaType = "image/jpeg"
      } else if type?.conforms(to: .pdf) == true {
        guard let document = PDFDocument(data: bytes) else { throw KernelError.message("无法读取 PDF") }
        var text = ""
        for i in 0..<min(document.pageCount, 100) { text += "\n[Page \(i+1)]\n" + (document.page(at: i)?.string ?? ""); guard text.utf8.count <= 512 * 1024 else { throw KernelError.message("PDF 文字超过预算") } }
        item.kind = "pdf"; item.text = text
      } else if let text = String(data: bytes, encoding: .utf8), text.utf8.count <= 512 * 1024 { item.kind = "text"; item.text = text }
      state.attachments.append(item); try persist()
    } catch { self.error = error.localizedDescription }
  }
  func removeAttachment(_ id: String) {
    guard !busy, jobs.isEmpty, controller?.agentCommitting != true else { return }
    guard !state.records.contains(where: { $0["canonical"]["parts"].array.contains { $0["media_ref"].string == id } }) else { error = "该图片属于已有对话；新建对话可清空当前附件，旧对话会保留"; return }
    state.attachments.removeAll { $0.id == id }; do { try persist() } catch { self.error = error.localizedDescription }
  }
  func undo(_ id: String) async {
    guard !busy, jobs.isEmpty, let controller, let transaction = state.transactions.first(where: { $0.id == id && $0.state == "applied" }) else { return }
    do {
      try await controller.commitAgentNotebook(transaction.before, expected: transaction.after)
      try finishTransaction(id, state: "undone")
    } catch { self.error = error.localizedDescription }
  }
  func attachment(_ args: JSONValue, prepare: Bool) throws -> JSONValue {
    guard let item = state.attachments.first(where: { $0.id == args["media_ref"].string }) else { throw KernelError.message("附件引用无效") }
    let view = prepare ? args["method"].string : args["view"].string
    if view == "text" || view == "document_text" || view == "auto" {
      guard let text = item.text else { throw KernelError.message("此附件没有可提取文字") }
      let offset = Int(args["cursor"].string) ?? 0, limit = min(32, max(1, args["limit"].isNull ? 8 : Int(args["limit"].double))) * 1024
      guard offset >= 0, offset <= text.count else { throw KernelError.message("无效附件分页") }
      let page = String(text.dropFirst(offset).prefix(limit))
      return .object(["name": .string(item.name), "text": .string(page), "next_cursor": offset + page.count < text.count ? .string(String(offset + page.count)) : .null])
    }
    if ["image", "image_normalize"].contains(view) { throw KernelError.message("请在输入框启用模型图片能力并重新发送附件；图片不作为工具文字返回") }
    return .object(["media_ref": .string(item.id), "name": .string(item.name), "kind": .string(item.kind), "bytes": .number(Double(item.bytes)), "text_available": .bool(item.text != nil), "image_available": .bool(item.image != nil)])
  }
}
