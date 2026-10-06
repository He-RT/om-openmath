import Observation
import SwiftUI

@MainActor @Observable final class NotebookController {
  var title = "未命名笔记本"
  var cells: [NotebookCell] = []
  var selected: String?
  var editorSnapshots: [String: EditorSnapshot] = [:]
  var focusCell: String?
  var config: JSONValue = .null
  var loading = true
  var busy = false
  private(set) var composingCells = Set<String>()
  var error: String?
  var notice: String?
  var dirty = false
  var inspector = "docs"
  var showInspector = false
  var stepDetails = false
  var stepExpanded: [String: Bool] = [:]
  var stepJob: String?
  var stepHighlight: String?
  var stepScroll: String?
  var stepCell: String?
  var stepIndex: Int?
  var document: NotebookDocument?
  var fileURL: URL?
  var shareURL: URL?
  var showSettings = false
  var showOpen = false
  var showSaveAs = false
  var showCommands = false
  var aiText: [String: String] = [:]
  var suggestions: [String: JSONValue] = [:]
  var aiStates: [String: String] = [:]
  var tools: [String: [JSONValue]] = [:]
  var variableRows: [JSONValue] = []
  var assistantDraft = ""
  var chatJob: String?
  var chatTurns: [(role: String, content: String)] = []
  var query = ""
  var scheme = "system"
  var language = "auto"
  private(set) var kernel: KernelClient?
  var transport: LLMTransport?
  private var epoch = 0
  private var saveTask: Task<Void, Never>?
  private var sourceWrites: Task<Void, Never>?
  private var writing = false
  private var batchGeneration = 0
  private var securityURL: URL?
  private var activeRequests = Set<String>()
  private var consent = Set<String>()
  private var aiOrder: [String] = []
  private var aiTimeouts: [String: Double] = [:]
  private var aiScopes: [String: (epoch: Int, cell: String?, revision: Int?)] = [:]
  private let defaults: UserDefaults
  init(defaults: UserDefaults = .standard) { self.defaults = defaults }
  var isEnglish: Bool {
    language == "en" || (language == "auto" && !Locale.current.identifier.hasPrefix("zh"))
  }
  func text(_ zh: String, _ en: String) -> String { isEnglish ? en : zh }
  func start(restoreLastDocument: Bool = true) async {
    do {
      let stored = try SettingsStore.load()
      let client = try KernelClient(config: stored)
      kernel = client
      transport = LLMTransport(client: client) { [weak self] packet in self?.consume(packet) }
      let reply = try await client.request(.object(["type": .string("get_config")]))
      config = reply.response.body["config"]
      language = config["general"]["language"].string
      scheme = defaults.string(forKey: "OpenMathTheme") ?? "system"
      _ = try await client.request(
        .object([
          "type": .string("set_system_language"), "language": .string(isEnglish ? "en" : "zh-CN"),
        ]))
      loading = false
      if restoreLastDocument, !ProcessInfo.processInfo.arguments.contains("-OpenMathUITesting") {
        if let bookmark = defaults.data(forKey: "OpenMathLastBookmark") {
          var stale = false
          let url = try URL(
            resolvingBookmarkData: bookmark, options: .withoutUI, bookmarkDataIsStale: &stale)
          await load(url)
        } else if let path = defaults.string(forKey: "OpenMathLastDocument") {
          await load(URL(fileURLWithPath: path))
        }
      }
    } catch {
      self.error = error.localizedDescription
      loading = false
    }
  }
  @discardableResult func call(_ body: JSONValue) async throws -> KernelPacket {
    guard let kernel else { throw KernelError.message("内核尚未就绪") }
    let generation = epoch
    let packet = try await kernel.request(body)
    guard generation == epoch, self.kernel === kernel else { throw KernelError.message("文档已切换") }
    consume(packet)
    return packet
  }
  func receiveHTTP(_ packet: KernelPacket) { consume(packet) }
  private func consume(_ packet: KernelPacket) {
    for event in packet.events.map(\.body) { eventReceived(event) }
    let body = packet.response.body
    if body["type"].string == "llm_started" {
      let id = body["request_id"].string
      if activeRequests.contains(id), !body["http"].isNull {
        transport?.start(id: id, request: body["http"], timeout: profileTimeout(for: id))
      } else {
        kernel?.cancel(id)
        Task {
          _ = try? await self.call(
            .object(["type": .string("llm_cancel"), "request_id": .string(id)]))
        }
      }
    }
  }
  private func profileTimeout(for id: String) -> Double { aiTimeouts[id] ?? 30000 }
  private func eventReceived(_ event: JSONValue) {
    let id = event["request_id"].string
    if !id.isEmpty {
      guard activeRequests.contains(id), let scope = aiScopes[id], scope.epoch == epoch else {
        return
      }
      if let cell = scope.cell, let rev = scope.revision,
        cells.first(where: { $0.id == cell })?.revision != rev
      {
        cancelAI(id)
        return
      }
      switch event["type"].string {
      case "llm_http": transport?.next(id: id, request: event["http"])
      case "llm_delta": aiText[id, default: ""] += event["text"].string
      case "llm_suggestion": suggestions[id] = event["suggestion"]
      case "llm_tool_call": tools[id, default: []].append(event)
      case "llm_profile_test":
        aiText[id] = event["response"].string
        notice = "\(event["response"].string) · \(Int(event["latency_ms"].double)) ms"
      case "llm_done":
        aiStates[id] = "done"
        activeRequests.remove(id)
        transport?.complete(id)
      case "llm_error":
        aiStates[id] = "error"
        error = event["message"].string
        activeRequests.remove(id)
        transport?.complete(id)
      default: break
      }
    }
  }
  func add(_ kind: CellKind = .Math, source: String = "", after: String? = nil) {
    let input = CellInput(
      kind: kind, source: source,
      dialect: config["general"]["dialect"].string == "wolfram" ? .Wolfram : .Modern)
    let index =
      after.flatMap { id in cells.firstIndex { $0.id == id }.map { $0 + 1 } } ?? cells.count
    cells.insert(NotebookCell(input: input, status: kind == .Text ? .Done : .Stale), at: index)
    selected = input.id
    if source.isEmpty { focusCell = input.id }
    markDirty()
    sync(input)
    if index < cells.count - 1 {
      enqueue(
        .object([
          "type": .string("move_cell"), "cell_id": .string(input.id),
          "to_index": .number(Double(index)),
        ]))
    }
  }
  func setComposing(_ id: String, _ composing: Bool) {
    let wasComposing = composingCells.contains(id)
    if composing { composingCells.insert(id) } else { composingCells.remove(id) }
    if wasComposing && !composing, let input = cells.first(where: { $0.id == id })?.input {
      sync(input)
    }
  }
  func edit(_ id: String, source: String) {
    guard let i = cells.firstIndex(where: { $0.id == id }), cells[i].input.source != source else {
      return
    }
    cells[i].input.source = source
    cells[i].revision += 1
    cells[i].status = cells[i].input.kind == .Text ? .Done : .Stale
    for index in cells.indices where cells[index].input.kind == .Math {
      cells[index].status = .Stale
    }
    for (job, scope) in aiScopes where scope.cell == id { cancelAI(job) }
    markDirty()
    if !composingCells.contains(id) { sync(cells[i].input) }
  }
  func update(_ id: String, kind: CellKind? = nil, dialect: Dialect? = nil) {
    guard let i = cells.firstIndex(where: { $0.id == id }) else { return }
    if let kind { cells[i].input.kind = kind }
    if let dialect { cells[i].input.dialect = dialect }
    cells[i].revision += 1
    cells[i].status = .Stale
    cells[i].output = .null
    sync(cells[i].input)
    markDirty()
  }
  private func sync(_ input: CellInput) {
    enqueue(.object(["type": .string("upsert_cell"), "cell": input.json]))
  }
  private func enqueue(_ body: JSONValue) {
    let prior = sourceWrites
    let generation = epoch
    sourceWrites = Task { [weak self] in
      await prior?.value
      guard let self, generation == self.epoch else { return }
      do { _ = try await self.call(body) } catch { self.error = error.localizedDescription }
    }
  }
  func run(_ id: String, next: Bool = false) async {
    guard !busy, composingCells.isEmpty, let input = cells.first(where: { $0.id == id })?.input
    else { return }
    busy = true
    let generation = epoch
    let snapshots = Dictionary(uniqueKeysWithValues: cells.map { ($0.id, $0.revision) })
    if let i = cells.firstIndex(where: { $0.id == id }) { cells[i].status = .Running }
    defer { if generation == epoch { busy = false } }
    do {
      await sourceWrites?.value
      let packet = try await call(
        .object([
          "type": .string("evaluate"), "cell_id": .string(id), "source": .string(input.source),
          "dialect": .string(input.dialect.rawValue),
        ]))
      await sourceWrites?.value
      guard generation == epoch else { return }
      install(packet.response.body["output"], id: id, snapshots: snapshots)
      for event in packet.events.map(\.body) {
        let cellID = event["cell_id"].string
        if event["type"].string == "cell_output" {
          install(event["output"], id: cellID, snapshots: snapshots)
        }
        if event["type"].string == "cell_status",
          let index = cells.firstIndex(where: { $0.id == cellID }),
          cells[index].revision == snapshots[cellID],
          let status = CellStatus(rawValue: event["status"].string)
        {
          cells[index].status = status
        }
      }
      await refreshVariables()
      if next, let i = cells.firstIndex(where: { $0.id == id }) {
        if i + 1 < cells.count { selected = cells[i + 1].id } else { add(after: id) }
        focusCell = selected
      }
    } catch {
      if !(error is CancellationError) { self.error = error.localizedDescription }
      if generation == epoch, let i = cells.firstIndex(where: { $0.id == id }),
        cells[i].revision == snapshots[id]
      {
        cells[i].status = error is CancellationError ? .Stale : .Error
      }
    }
  }
  private func install(_ output: JSONValue, id: String, snapshots: [String: Int]) {
    guard let i = cells.firstIndex(where: { $0.id == id }), snapshots[id] == cells[i].revision
    else { return }
    cells[i].output = output
    cells[i].status =
      output["messages"].array.contains(where: { $0["level"].string == "Error" }) ? .Error : .Done
  }
  func runAll() async {
    guard composingCells.isEmpty else { return }
    let batch = batchGeneration
    let generation = epoch
    let ids = cells.filter { $0.input.kind == .Math }.map(\.id)
    for id in ids {
      guard batch == batchGeneration, generation == epoch else { break }
      await run(id)
    }
  }
  func interrupt() {
    batchGeneration += 1
    kernel?.interrupt()
    for id in Array(activeRequests) { cancelAI(id) }
  }
  func remove(_ id: String) {
    guard let i = cells.firstIndex(where: { $0.id == id }) else { return }
    composingCells.remove(id)
    cells.remove(at: i)
    enqueue(.object(["type": .string("delete_cell"), "cell_id": .string(id)]))
    selected = cells.first?.id
    markDirty()
  }
  func move(_ id: String, delta: Int) {
    guard let from = cells.firstIndex(where: { $0.id == id }) else { return }
    let to = min(cells.count - 1, max(0, from + delta))
    guard to != from else { return }
    let cell = cells.remove(at: from)
    cells.insert(cell, at: to)
    enqueue(
      .object([
        "type": .string("move_cell"), "cell_id": .string(id), "to_index": .number(Double(to)),
      ]))
    markDirty()
  }
  func refreshVariables() async {
    if let p = try? await call(.object(["type": .string("get_variables")])) {
      variableRows = p.response.body["items"].array
    }
  }
  var notebook: NotebookFile { NotebookFile(title: title, cells: cells.map(\.input)) }
  func rename(_ value: String) {
    title = value
    enqueue(.object(["type": .string("rename_notebook"), "title": .string(value)]))
    markDirty()
  }
  func markDirty() {
    dirty = true
    saveTask?.cancel()
    saveTask = Task {
      try? await Task.sleep(for: .seconds(1))
      guard !Task.isCancelled else { return }
      await save()
    }
  }
  func save() async {
    guard !loading else { return }
    while writing {
      if Task.isCancelled { return }
      try? await Task.sleep(for: .milliseconds(20))
    }
    writing = true
    defer { writing = false }
    let snapshot = notebook
    let revisions = cells.map(\.revision)
    let generation = epoch
    do {
      if document == nil {
        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
          .appendingPathComponent("Notebooks", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let created = NotebookDocument(
          fileURL: dir.appendingPathComponent(UUID().uuidString + ".omnb"))
        created.notebook = snapshot
        guard await created.write(operation: .forCreating) else {
          throw KernelError.message("笔记本保存失败")
        }
        document = created
      } else {
        document?.notebook = snapshot
        document?.updateChangeCount(.done)
        guard await document!.write(operation: .forOverwriting) else {
          throw KernelError.message("笔记本保存失败")
        }
      }
      guard generation == epoch else { return }
      fileURL = document?.fileURL
      if let fileURL { remember(fileURL) }
      dirty = snapshot != notebook || revisions != cells.map(\.revision)
      notice = text("笔记本已保存", "Notebook saved")
    } catch {
      dirty = true
      self.error = error.localizedDescription
    }
  }
  func load(_ url: URL) async {
    if dirty {
      await save()
      guard !dirty else { return }
    }
    let scoped = url.startAccessingSecurityScopedResource()
    var retained = false
    defer { if scoped && !retained { url.stopAccessingSecurityScopedResource() } }
    let doc = NotebookDocument(fileURL: url)
    guard await doc.openFile() else {
      error = text("无法打开笔记本", "Cannot open notebook")
      return
    }
    do {
      let file = doc.notebook
      try file.validate()
      interrupt()
      await sourceWrites?.value
      let client = try KernelClient(config: try SettingsStore.load())
      _ = try await client.request(
        .object([
          "type": .string("set_system_language"), "language": .string(isEnglish ? "en" : "zh-CN"),
        ]))
      do {
        _ = try await client.request(
          .object([
            "type": .string("load_notebook"),
            "file": try JSONDecoder().decode(JSONValue.self, from: JSONEncoder().encode(file)),
          ]))
      } catch {
        client.close()
        throw error
      }
      kernel?.close()
      kernel = client
      transport = LLMTransport(client: client) { [weak self] packet in self?.consume(packet) }
      if let securityURL { securityURL.stopAccessingSecurityScopedResource() }
      securityURL = scoped ? url : nil
      retained = true
      epoch += 1
      if let old = document { _ = await old.closeFile() }
      document = doc
      fileURL = url
      remember(url)
      composingCells = []
      editorSnapshots = [:]
      focusCell = nil
      cells = file.cells.map { NotebookCell(input: $0, status: $0.kind == .Text ? .Done : .Stale) }
      title = file.title
      selected = cells.first?.id
      chatTurns = []
      aiText = [:]
      suggestions = [:]
      dirty = false
      busy = false
    } catch {
      self.error = error.localizedDescription
      _ = await doc.closeFile()
    }
  }
  func newNotebook() async {
    if dirty {
      await save()
      guard !dirty else { return }
    }
    interrupt()
    await sourceWrites?.value
    do {
      let client = try KernelClient(config: try SettingsStore.load())
      _ = try await client.request(
        .object([
          "type": .string("set_system_language"), "language": .string(isEnglish ? "en" : "zh-CN"),
        ]))
      kernel?.close()
      kernel = client
      transport = LLMTransport(client: client) { [weak self] packet in self?.consume(packet) }
      if let securityURL { securityURL.stopAccessingSecurityScopedResource() }
      securityURL = nil
      defaults.removeObject(forKey: "OpenMathLastDocument")
      defaults.removeObject(forKey: "OpenMathLastBookmark")
      epoch += 1
      if let document { _ = await document.closeFile() }
      document = nil
      fileURL = nil
      cells = []
      composingCells = []
      editorSnapshots = [:]
      focusCell = nil
      selected = nil
      title = text("未命名笔记本", "Untitled notebook")
      dirty = false
      busy = false
      chatTurns = []
      aiText = [:]
      suggestions = [:]
    } catch { self.error = error.localizedDescription }
  }
  func export(_ format: String) async {
    do {
      let value = NotebookExporter.render(title: title, cells: cells, format: format)
      let url = FileManager.default.temporaryDirectory.appendingPathComponent(
        "OpenMath-" + UUID().uuidString + (format == "latex" ? ".tex" : ".md"))
      try value.write(to: url, atomically: true, encoding: .utf8)
      shareURL = url
    } catch { self.error = error.localizedDescription }
  }
  func shareArtifact(_ artifact: JSONValue, name: String) throws {
    let ext = artifact["extension"].string
    let types = ["svg": "image/svg+xml", "png": "image/png", "csv": "text/csv;charset=utf-8", "json": "application/json;charset=utf-8"]
    guard types[ext] == artifact["mime"].string,
      artifact["byte_len"].double <= Double(16*1024*1024),
      let bytes = Data(base64Encoded: artifact["base64"].string),
      Double(bytes.count) == artifact["byte_len"].double else { throw KernelError.message("导出字节格式或长度无效") }
    let safe = name.replacingOccurrences(of: #"[/\\:*?"<>|\x00-\x1f]"#, with: "-", options: .regularExpression)
    let url = FileManager.default.temporaryDirectory.appendingPathComponent(safe + "-" + UUID().uuidString + "." + ext)
    try bytes.write(to: url, options: .atomic)
    guard try Data(contentsOf: url) == bytes else { throw KernelError.message("导出文件回读校验失败") }
    shareURL = url
  }
  private func remember(_ url: URL) {
    defaults.set(url.path, forKey: "OpenMathLastDocument")
    if let bookmark = try? url.bookmarkData(
      options: .minimalBookmark, includingResourceValuesForKeys: nil, relativeTo: nil)
    {
      defaults.set(bookmark, forKey: "OpenMathLastBookmark")
    } else {
      defaults.removeObject(forKey: "OpenMathLastBookmark")
    }
  }
  func background() async {
    interrupt()
    let activity = UIApplication.shared.beginBackgroundTask(withName: "Save notebook draft")
    await save()
    if activity != .invalid { UIApplication.shared.endBackgroundTask(activity) }
  }
  func example(_ value: String) {
    add(source: value)
    if let selected { Task { await run(selected) } }
  }
  func cancelAI(_ id: String) {
    kernel?.cancel(id)
    transport?.cancel(id)
    activeRequests.remove(id)
    aiStates[id] = "cancelled"
    suggestions.removeValue(forKey: id)
    Task {
      _ = try? await call(.object(["type": .string("llm_cancel"), "request_id": .string(id)]))
    }
  }
  func beginAI(feature: String, body: JSONValue, cell: String? = nil) async -> String? {
    if let cell, composingCells.contains(cell) { return nil }
    let id = UUID().uuidString
    var request = body
    request["request_id"] = .string(id)
    let profile =
      body["type"].string == "llm_test_profile"
      ? body["config"]
      : (config["llm"]["profiles"].array.first { $0["name"] == config["llm"][feature] } ?? .null)
    aiTimeouts[id] = profile["timeout_ms"].isNull ? 30000 : profile["timeout_ms"].double
    aiOrder.append(id)
    while aiOrder.count > 256, let oldest = aiOrder.first, !activeRequests.contains(oldest) {
      aiOrder.removeFirst()
      aiScopes.removeValue(forKey: oldest)
      aiTimeouts.removeValue(forKey: oldest)
      aiText.removeValue(forKey: oldest)
      suggestions.removeValue(forKey: oldest)
      aiStates.removeValue(forKey: oldest)
      tools.removeValue(forKey: oldest)
    }
    activeRequests.insert(id)
    aiStates[id] = "pending"
    aiText[id] = ""
    aiScopes[id] = (epoch, cell, cell.flatMap { value in cells.first { $0.id == value }?.revision })
    do {
      _ = try await call(request)
      return id
    } catch {
      activeRequests.remove(id)
      aiStates[id] = "error"
      self.error = error.localizedDescription
      return nil
    }
  }
  func authorized(_ feature: String) -> Bool { consent.contains(consentKey(feature)) }
  private func consentKey(_ feature: String) -> String {
    let name = config["llm"][feature].string
    let profile = config["llm"]["profiles"].array.first { $0["name"].string == name } ?? .null
    return feature + "|" + profile["base_url"].string + "|" + name + "|"
      + String(config["llm"]["send_context"].bool)
  }
  func allow(_ feature: String) { consent.insert(consentKey(feature)) }
  func destination(_ feature: String) -> String {
    let name = config["llm"][feature].string
    return config["llm"]["profiles"].array.first { $0["name"].string == name }?["base_url"].string
      ?? ""
  }
  func insertSuggestion(_ id: String, replacing cell: String? = nil, run: Bool = false) {
    guard let scope = aiScopes[id], scope.epoch == epoch else { return }
    if let sourceCell = scope.cell, let revision = scope.revision,
      cells.first(where: { $0.id == sourceCell })?.revision != revision
    {
      return
    }
    let suggestion = suggestions[id] ?? .null
    guard !suggestion["wolfram"].string.isEmpty else { return }
    let source = suggestion["modern"].string
    if let cell, let i = cells.firstIndex(where: { $0.id == cell }) {
      update(cell, kind: .Math, dialect: .Modern)
      edit(cell, source: source)
      if run { Task { await self.run(cells[i].id) } }
    } else {
      add(source: source)
      if run, let selected { Task { await self.run(selected) } }
    }
  }
  func applySettings(_ draft: JSONValue) async throws {
    let old = try SettingsStore.load() ?? config
    let validator = try KernelClient()
    defer { validator.close() }
    _ = try await validator.request(.object(["type": .string("set_config"), "config": draft]))
    let saved = try SettingsStore.persist(draft, previous: old)
    for id in Array(activeRequests) { cancelAI(id) }
    do {
      _ = try await call(.object(["type": .string("set_config"), "config": saved]))
      let p = try await call(.object(["type": .string("get_config")]))
      guard try SettingsStore.load() == saved else { throw KernelError.message("设置回读不一致") }
      config = p.response.body["config"]
      language = config["general"]["language"].string
      _ = try await call(
        .object([
          "type": .string("set_system_language"), "language": .string(isEnglish ? "en" : "zh-CN"),
        ]))
    } catch {
      _ = try? SettingsStore.persist(old, previous: saved)
      _ = try? await call(.object(["type": .string("set_config"), "config": old]))
      throw error
    }
  }
}
