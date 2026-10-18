import Foundation
import Security

enum AssistantSetupError:Error {case invalid,notConfigured,contextLimit}
struct AssistantConfiguration:Codable,Sendable {
  let baseURL:String,model:String,credentialReference:String
  var endpoint:URL {
    var url=URLComponents(string:baseURL)!
    // Local HTTP uses a literal loopback address; it cannot follow a DNS change to a remote host.
    if url.scheme=="http",url.host?.lowercased()=="localhost" {url.host="127.0.0.1"}
    return url.url!.appendingPathComponent("chat/completions")
  }
}

/// A small configuration for the first native assistant. Secrets remain in the exact new
/// Keychain namespace; the configuration file contains only a fresh opaque reference.
actor AssistantSettingsStore {
  private let file:URL
  init() throws {file=try NativeStoragePaths.system(.preview).root.appendingPathComponent("assistant.json")}
  func load() throws->AssistantConfiguration? {
    guard FileManager.default.fileExists(atPath:file.path) else {return nil}
    let data=try Data(contentsOf:file)
    guard data.count<=16384 else {throw AssistantSetupError.invalid}
    let value=try JSONDecoder().decode(AssistantConfiguration.self,from:data)
    try Self.validate(value.baseURL,model:value.model)
    return value
  }
  func save(baseURL:String,model:String,key:String)throws->AssistantConfiguration {
    let base=baseURL.trimmingCharacters(in:.whitespacesAndNewlines).trimmingCharacters(in:CharacterSet(charactersIn:"/"))
    let model=model.trimmingCharacters(in:.whitespacesAndNewlines)
    try Self.validate(base,model:model)
    let old=try load(),bytes=Data(key.utf8)
    if bytes.isEmpty {
      guard let old else {throw AssistantSetupError.notConfigured}
      let value=AssistantConfiguration(baseURL:base,model:model,credentialReference:old.credentialReference)
      try write(value);return value
    }
    _ = try ProviderSecret(bytes)
    let reference="assistant-"+UUID().uuidString.lowercased()
    try SecretStore.insert(reference:reference,bytes:bytes)
    let value=AssistantConfiguration(baseURL:base,model:model,credentialReference:reference)
    do {try write(value)} catch {Self.remove(reference);throw error}
    if let old {Self.remove(old.credentialReference)}
    return value
  }
  private func write(_ value:AssistantConfiguration)throws {
    try FileManager.default.createDirectory(at:file.deletingLastPathComponent(),withIntermediateDirectories:true,attributes:[.posixPermissions:0o700])
    try JSONEncoder().encode(value).write(to:file,options:[.atomic,.completeFileProtection])
    try FileManager.default.setAttributes([.posixPermissions:0o600],ofItemAtPath:file.path)
  }
  private static func remove(_ reference:String) {
    SecretStore.remove(reference:reference)
  }
  static func validate(_ base:String,model:String)throws {
    guard let u=URLComponents(string:base),u.host?.isEmpty==false,
      u.scheme=="https" || u.scheme=="http" && ["localhost","127.0.0.1","[::1]","::1"].contains(u.host?.lowercased() ?? ""),
      u.user==nil,u.password==nil,u.query==nil,u.fragment==nil,u.url != nil,u.port.map({(1...65535).contains($0)}) ?? true,
      !model.isEmpty,model.utf8.count<=256,!model.unicodeScalars.contains(where:CharacterSet.controlCharacters.contains) else {throw AssistantSetupError.invalid}
  }
}
