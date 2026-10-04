import Security
import XCTest

@testable import OpenMath

final class SettingsTests: XCTestCase {
  func testSecretsAreIsolatedAndFailedPersistenceRollsBack() async throws {
    let client = try KernelClient()
    defer { client.close() }
    var config = try await client.request(.object(["type": .string("get_config")])).response.body[
      "config"]
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: dir) }
    let url = dir.appendingPathComponent("settings.json")
    let service = "org.openmath.test." + UUID().uuidString
    let store = SettingsStore(service: service, fileURL: url)
    var profiles = config["llm"]["profiles"].array
    profiles[0]["api_key"] = .string("synthetic-keychain-secret")
    profiles[0]["extra_headers"] = .object(["X-Test-Secret": .string("synthetic-header")])
    config["llm"]["profiles"] = .array(profiles)
    let saved = try store.persist(config, previous: .null)
    let metadata = try String(contentsOf: url, encoding: .utf8)
    XCTAssertFalse(metadata.contains("synthetic-keychain-secret"))
    XCTAssertFalse(metadata.contains("synthetic-header"))
    XCTAssertEqual(
      try store.load()?["llm"]["profiles"][0]["api_key"].string, "synthetic-keychain-secret")
    var changed = config
    profiles[0]["api_key"] = .string("synthetic-replacement")
    changed["llm"]["profiles"] = .array(profiles)
    let failing = SettingsStore(service: service, fileURL: url) { _, _ in
      throw KernelError.message("fixture write failure")
    }
    XCTAssertThrowsError(try failing.persist(changed, previous: saved))
    XCTAssertEqual(
      try store.load()?["llm"]["profiles"][0]["api_key"].string, "synthetic-keychain-secret")
    XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), metadata)
    _ = try store.persist(.object(["llm": .object(["profiles": .array([])])]), previous: saved)
  }

  func testOmittedProviderParametersSurviveKeychainReadbackAndLegacyNulls() async throws {
    let client = try KernelClient()
    defer { client.close() }
    let config = try await client.request(.object(["type": .string("get_config")])).response.body[
      "config"]
    let url = FileManager.default.temporaryDirectory.appendingPathComponent(
      UUID().uuidString + ".json")
    defer { try? FileManager.default.removeItem(at: url) }
    let service = "org.openmath.test." + UUID().uuidString
    let store = SettingsStore(service: service, fileURL: url)
    let saved = try store.persist(config, previous: .null)
    let loaded = try XCTUnwrap(store.load())
    XCTAssertEqual(loaded, saved)
    let initialized = try KernelClient(config: loaded)
    initialized.close()
    let name = saved["llm"]["profiles"][0]["name"].string
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
      kSecAttrAccount as String: name,
    ]
    let legacy = try JSONValue.object([
      "api_key": .null, "extra_headers": .object([:]), "extra_body": .null,
    ]).data()
    XCTAssertEqual(
      SecItemUpdate(query as CFDictionary, [kSecValueData as String: legacy] as CFDictionary),
      errSecSuccess)
    let migrated = try XCTUnwrap(store.load())
    XCTAssertEqual(migrated["llm"]["profiles"][0]["extra_body"], .object([:]))
    let recovered = try KernelClient(config: migrated)
    recovered.close()
    _ = try store.persist(.object(["llm": .object(["profiles": .array([])])]), previous: migrated)
  }
}
