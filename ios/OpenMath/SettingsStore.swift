import Foundation
import Security

struct SettingsStore {
  private let service: String
  private let fileURL: URL?
  private let writeMetadata: (Data, URL) throws -> Void
  init(
    service: String = "org.openmath.mobile.provider", fileURL: URL? = nil,
    writeMetadata: @escaping (Data, URL) throws -> Void = { data, url in
      try data.write(to: url, options: [.atomic, .completeFileProtection])
    }
  ) {
    self.service = service
    self.fileURL = fileURL
    self.writeMetadata = writeMetadata
  }
  private var file: URL {
    get throws {
      if let fileURL { return fileURL }
      let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("OpenMath", isDirectory: true)
      try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
      return dir.appendingPathComponent("settings.json")
    }
  }
  private func read(_ name: String) throws -> Data? {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
      kSecAttrAccount as String: name, kSecReturnData as String: true,
      kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    var data: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &data)
    if status == errSecItemNotFound { return nil }
    guard status == errSecSuccess else { throw KernelError.message("系统凭据库读取失败") }
    return data as? Data
  }
  private func write(_ name: String, _ data: Data?) throws {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
      kSecAttrAccount as String: name,
    ]
    if let data {
      let values: [String: Any] = [
        kSecValueData as String: data,
        kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
      ]
      let status = SecItemUpdate(query as CFDictionary, values as CFDictionary)
      if status == errSecItemNotFound {
        guard
          SecItemAdd(query.merging(values) { _, new in new } as CFDictionary, nil) == errSecSuccess
        else { throw KernelError.message("系统凭据库写入失败") }
      } else if status != errSecSuccess {
        throw KernelError.message("系统凭据库写入失败")
      }
    } else {
      let status = SecItemDelete(query as CFDictionary)
      guard status == errSecSuccess || status == errSecItemNotFound else {
        throw KernelError.message("系统凭据库清除失败")
      }
    }
  }
  func load() throws -> JSONValue? {
    let url = try file
    guard FileManager.default.fileExists(atPath: url.path) else { return nil }
    var config = try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: url))
    var profiles = config["llm"]["profiles"].array
    for i in profiles.indices {
      if let data = try read(profiles[i]["name"].string) {
        let secret = try JSONDecoder().decode(JSONValue.self, from: data)
        profiles[i]["api_key"] = secret["api_key"]
        profiles[i]["extra_headers"] =
          secret["extra_headers"].isNull ? .object([:]) : secret["extra_headers"]
        profiles[i]["extra_body"] =
          secret["extra_body"].isNull ? .object([:]) : secret["extra_body"]
      }
    }
    config["llm"]["profiles"] = .array(profiles)
    return config
  }
  func persist(_ draft: JSONValue, previous: JSONValue) throws -> JSONValue {
    var config = draft
    var profiles = config["llm"]["profiles"].array
    let names = profiles.map { $0["name"].string }
    guard names.allSatisfy({ !$0.isEmpty }), Set(names).count == names.count else {
      throw KernelError.message("配置名称必须非空且唯一")
    }
    let all = Set(names + previous["llm"]["profiles"].array.map { $0["name"].string })
    var backup: [String: Data] = [:]
    for name in all { if let value = try read(name) { backup[name] = value } }
    let url = try file
    let oldFile = try? Data(contentsOf: url)
    do {
      for i in profiles.indices {
        let name = profiles[i]["name"].string
        let old = previous["llm"]["profiles"].array.first { $0["name"].string == name } ?? .null
        if profiles[i]["api_key"].string == "***" { profiles[i]["api_key"] = old["api_key"] }
        guard profiles[i]["api_key"].string != "***" else { throw KernelError.message("实际密钥不可用") }
        if profiles[i]["extra_headers"].isNull { profiles[i]["extra_headers"] = .object([:]) }
        if profiles[i]["extra_body"].isNull { profiles[i]["extra_body"] = .object([:]) }
        profiles[i]["api_key_env"] = .null
        let secret = JSONValue.object([
          "api_key": profiles[i]["api_key"], "extra_headers": profiles[i]["extra_headers"],
          "extra_body": profiles[i]["extra_body"],
        ])
        try write(name, secret.data())
      }
      for name in all.subtracting(names) { try write(name, nil) }
      var metadata = config
      var safe = profiles
      for i in safe.indices {
        safe[i]["api_key"] = .null
        safe[i]["extra_headers"] = .object([:])
        safe[i]["extra_body"] = .object([:])
        safe[i]["api_key_env"] = .null
      }
      metadata["llm"]["profiles"] = .array(safe)
      try writeMetadata(metadata.data(), url)
      config["llm"]["profiles"] = .array(profiles)
      return config
    } catch {
      var failed = false
      for name in all { do { try write(name, backup[name]) } catch { failed = true } }
      if let oldFile {
        do { try oldFile.write(to: url, options: .atomic) } catch { failed = true }
      } else {
        try? FileManager.default.removeItem(at: url)
      }
      if failed { throw KernelError.message("设置保存失败且回滚未完成") }
      throw error
    }
  }
  static func load() throws -> JSONValue? { try SettingsStore().load() }
  static func persist(_ draft: JSONValue, previous: JSONValue) throws -> JSONValue {
    try SettingsStore().persist(draft, previous: previous)
  }
}
