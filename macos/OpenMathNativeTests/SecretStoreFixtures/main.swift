import Foundation
import Security
import LocalAuthentication

private final class QueryCapture: @unchecked Sendable {
  private let lock = NSLock()
  private var count = 0
  func check(_ query: CFDictionary, account: String) {
    let object = query as NSDictionary
    precondition(object[kSecAttrService] as? String == SecretStore.service)
    precondition(object[kSecAttrAccount] as? String == account)
    precondition(object[kSecAttrAccessible] as? String == kSecAttrAccessibleWhenUnlockedThisDeviceOnly as String)
    precondition(object[kSecMatchLimit] as? String == kSecMatchLimitOne as String)
    precondition((object[kSecUseAuthenticationContext] as? LAContext)?.interactionNotAllowed == true)
    precondition(object[kSecAttrSynchronizable] as? Bool == false)
    precondition(object[kSecUseDataProtectionKeychain] as? Bool == false)
    lock.lock(); count += 1; lock.unlock()
  }
  var calls: Int { lock.lock(); defer { lock.unlock() }; return count }
}

@main struct SecretStoreFixtures {
  static func main() async throws {
    for base in ["http://localhost:10100/v1","http://127.0.0.1:10100/v1","http://[::1]:10100/v1","https://api.example.com/v1"] {
      try AssistantSettingsStore.validate(base,model:"configured-model")
    }
    for base in ["http://example.com/v1","http://localhost.example.com/v1","http://127.0.0.2/v1","https://user:pass@example.com/v1","http://localhost:10100/v1?key=secret"] {
      do {try AssistantSettingsStore.validate(base,model:"configured-model");fatalError("non-loopback insecure or credential-bearing route accepted")}
      catch AssistantSetupError.invalid { }
    }
    let local=AssistantConfiguration(baseURL:"http://localhost:10100/v1",model:"configured-model",credentialReference:"fixture")
    precondition(local.endpoint.absoluteString=="http://127.0.0.1:10100/v1/chat/completions")
    let captured = QueryCapture()
    let store = SecretStore(testReader: { query in
      captured.check(query, account: "fixture-reference")
      return (errSecSuccess, Data("synthetic-credential".utf8))
    })
    let secret = try await store.read(reference: "fixture-reference")
    precondition(secret.description == "ProviderSecret(redacted)")
    precondition(secret.debugDescription == "ProviderSecret(redacted)")
    precondition(secret.authenticationHeader(.bearer)["Authorization"] == "Bearer synthetic-credential")
    precondition(secret.authenticationHeader(.anthropicAPIKey)["x-api-key"] == "synthetic-credential")
    for status in [errSecInteractionNotAllowed, errSecAuthFailed, errSecNotAvailable] {
      let denied = SecretStore(testReader: { query in
        captured.check(query, account: "fixture-reference")
        return (status, Data("must-not-be-used".utf8))
      })
      do { _ = try await denied.read(reference: "fixture-reference"); fatalError("locked status returned a secret") }
      catch ProviderSecretError.locked { }
    }
    let missing = SecretStore(testReader: { query in
      captured.check(query, account: "fixture-reference")
      return (errSecItemNotFound, nil)
    })
    do { _ = try await missing.read(reference: "fixture-reference"); fatalError("missing secret fallback") }
    catch ProviderSecretError.missing { }
    for bad in ["", "../old-profile", "contains space", "x\nheader", String(repeating: "a", count: 257)] {
      do { _ = try await store.read(reference: bad); fatalError("invalid reference queried") }
      catch ProviderSecretError.invalidReference { }
    }
    precondition(captured.calls == 5)
    for bytes in [Data(), Data("key\r\nInjected: yes".utf8), Data([0xff]), Data(repeating: 65, count: 8193)] {
      do { _ = try ProviderSecret(bytes); fatalError("invalid HTTP secret") }
      catch ProviderSecretError.invalidValue { }
    }
    let echoed=Data("prefix synthetic-credential suffix".utf8),normal=Data("data: 中文🙂\n\ndata: [DONE]\n\n".utf8)
    for cut in 0...echoed.count {
      var gate=secret.responseGuard(),published=Data()
      do {published.append(try gate.feed(echoed.prefix(cut)));published.append(try gate.feed(echoed.dropFirst(cut)));published.append(try gate.feed(Data(),final:true));fatalError("split credential echo escaped")}
      catch ProviderSecretError.echoed {precondition(published.range(of:Data("synthetic-credential".utf8))==nil)}
    }
    for cut in 0...normal.count {
      var gate=secret.responseGuard(),published=Data();published.append(try gate.feed(normal.prefix(cut)));published.append(try gate.feed(normal.dropFirst(cut)));published.append(try gate.feed(Data(),final:true));precondition(published==normal)
      precondition(gate.debugDescription=="ProviderSecretResponseGuard(redacted)")
    }
    // A real system read for an explicitly new random reference cannot enumerate legacy data.
    let actual = SecretStore(), unused = "fixture-missing-" + UUID().uuidString.lowercased()
    do { _ = try await actual.read(reference: unused); fatalError("random missing reference existed") }
    catch ProviderSecretError.missing { }
    catch ProviderSecretError.locked { }
    catch ProviderSecretError.unavailable { }
    // Exercise the production writer and reader using only a fresh, nonsecret test account.
    let own="fixture-roundtrip-"+UUID().uuidString.lowercased()
    try SecretStore.insert(reference:own,bytes:Data("synthetic-roundtrip".utf8))
    defer {SecretStore.remove(reference:own)}
    let roundtrip=try await actual.read(reference:own)
    precondition(roundtrip.authenticationHeader(.bearer)["Authorization"]=="Bearer synthetic-roundtrip")
    print("Actual encrypted macOS login-keychain writer/read roundtrip passed; fresh fixture account removed on exit.")
    print("New namespace/exact account, no UI/legacy fallback; three locked/auth statuses denied; header injection rejected; actual system missing-reference lookup completed. Physical lock screen not exercised.")
  }
}
