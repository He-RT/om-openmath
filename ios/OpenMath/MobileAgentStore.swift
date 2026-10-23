import CryptoKit
import Foundation

struct MobileSourceTransaction: Codable, Identifiable {
  var id: String
  var before: NotebookFile
  var after: NotebookFile
  var state: String
}
struct MobileAttachment: Codable, Identifiable {
  var id: String
  var name: String
  var kind: String
  var bytes: Int
  var text: String?
  var image: String?
  var mediaType: String?
  var contentHash: String?
}
struct MobileAgentState: Codable {
  var version = 1
  var draft = ""
  var profile = ""
  var execute = false
  var imageProfiles: [String] = []
  var records: [JSONValue] = []
  var results: [JSONValue] = []
  var operations: [String: JSONValue] = [:]
  var transactions: [MobileSourceTransaction] = []
  var attachments: [MobileAttachment] = []
}
enum MobileAgentReplay {
  /// A stopped turn can contain both finished and unstarted calls. Retain the
  /// completed pairs while removing only proposals with no durable result.
  static func history(_ records: [JSONValue], binding: JSONValue) -> [JSONValue] {
    let paired = Set(records.filter { $0["canonical"]["role"].string == "tool" }
      .flatMap { $0["canonical"]["parts"].array.map { $0["tool_call_id"].string } })
    return records.compactMap { record in
      var value = record
      let parts = record["canonical"]["parts"].array
      let retained = parts.filter { $0["kind"].string != "tool_call" || paired.contains($0["tool_call_id"].string) }
      guard !retained.isEmpty else { return nil }
      if retained != parts { value["canonical"]["parts"] = .array(retained); value["replay"] = .null }
      if value["replay"]["binding"] != binding { value["replay"] = .null }
      return value
    }
  }
}
struct MobileAgentStore {
  let file: URL
  init(document: String, root: URL? = nil) throws {
    let directory = try root ?? FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
      .appendingPathComponent("OpenMath-pre-alpha.5/Conversations", isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication])
    file = directory.appendingPathComponent(Self.hash(document) + ".json")
  }
  static func hash(_ source: String) -> String { SHA256.hash(data: Data(source.utf8)).map { String(format: "%02x", $0) }.joined() }
  func load() throws -> MobileAgentState {
    guard FileManager.default.fileExists(atPath: file.path) else { return MobileAgentState() }
    let data = try Data(contentsOf: file)
    guard data.count <= 32 * 1024 * 1024 else { throw KernelError.message("会话超过存储预算，原文件已保留") }
    let state = try JSONDecoder().decode(MobileAgentState.self, from: data)
    guard state.version == 1 else { throw KernelError.message("不支持此会话版本，原文件已保留") }
    return state
  }
  func save(_ state: MobileAgentState) throws {
    let data = try JSONEncoder().encode(state)
    guard data.count <= 32 * 1024 * 1024 else { throw KernelError.message("会话超过存储预算") }
    try data.write(to: file, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
  }
  func archive(_ state: MobileAgentState) throws {
    let data = try JSONEncoder().encode(state)
    try data.write(to: file.deletingPathExtension().appendingPathExtension(ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "-") + "-" + UUID().uuidString + ".json"), options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
  }
  func retainAttachment(_ data: Data) throws -> String {
    guard data.count <= 10 * 1024 * 1024 else { throw KernelError.message("附件超过存储预算") }
    let hash = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    let directory = file.deletingLastPathComponent().appendingPathComponent("Attachments", isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    let destination = directory.appendingPathComponent(hash)
    if FileManager.default.fileExists(atPath: destination.path) {
      guard try Data(contentsOf: destination) == data else { throw KernelError.message("附件散列存储冲突，原文件保留") }
    } else { try data.write(to: destination, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication]) }
    return hash
  }
  func history() throws -> [URL] {
    let prefix = file.deletingPathExtension().lastPathComponent + "."
    return try FileManager.default.contentsOfDirectory(at: file.deletingLastPathComponent(), includingPropertiesForKeys: [.contentModificationDateKey])
      .filter { $0 != file && $0.lastPathComponent.hasPrefix(prefix) && $0.pathExtension == "json" }
      .sorted { $0.lastPathComponent > $1.lastPathComponent }
  }
  func restore(_ name: String) throws -> MobileAgentState {
    guard let url = try history().first(where: { $0.lastPathComponent == name }) else { throw KernelError.message("找不到会话") }
    let data = try Data(contentsOf: url)
    guard data.count <= 32 * 1024 * 1024 else { throw KernelError.message("会话超过读取预算") }
    let state = try JSONDecoder().decode(MobileAgentState.self, from: data)
    guard state.version == 1 else { throw KernelError.message("不支持此会话版本") }
    return state
  }
}
