import Foundation
import Security
import LocalAuthentication

enum ProviderSecretError: Error, Sendable { case invalidReference, missing, locked, unavailable, invalidValue }
enum ProviderAuthentication: Sendable { case none, bearer, anthropicAPIKey }

/// Ephemeral credential bytes. They are not Codable and never have a printable value.
struct ProviderSecret: Sendable, CustomStringConvertible, CustomDebugStringConvertible {
  private let bytes: Data
  init(_ bytes: Data) throws {
    guard !bytes.isEmpty, bytes.count <= 8192,
      bytes.allSatisfy({ (0x21...0x7e).contains($0) }) else { throw ProviderSecretError.invalidValue }
    self.bytes = bytes
  }
  var description: String { "ProviderSecret(redacted)" }
  var debugDescription: String { description }
  /// Called only while constructing the final native URLRequest, after route/body validation.
  func authenticationHeader(_ method: ProviderAuthentication) -> [String: String] {
    let value = String(decoding: bytes, as: UTF8.self)
    switch method {
    case .none: return [:]
    case .bearer: return ["Authorization": "Bearer " + value]
    case .anthropicAPIKey: return ["x-api-key": value]
    }
  }
}

/// Reads exact references from the new Mac namespace. No enumeration, legacy service or fallback.
/// Registry candidate creation/replacement is a separate R4.4.01 transaction.
actor SecretStore {
  static let service = "org.openmath.NativeMacPreview.provider-secrets.r4.v1"
  typealias Reader = @Sendable (CFDictionary) -> (OSStatus, Data?)
  private let reader: Reader

  init() {
    reader = { query in
      var value: CFTypeRef?
      let status = SecItemCopyMatching(query, &value)
      return (status, value as? Data)
    }
  }
  /// Test-only status injection exercises the exact production query and failure policy.
  init(testReader: @escaping Reader) { reader = testReader }

  func read(reference: String) throws -> ProviderSecret {
    guard !reference.isEmpty, reference.utf8.count <= 256,
      reference.utf8.enumerated().allSatisfy({ index, byte in
        (65...90).contains(byte) || (97...122).contains(byte) || (48...57).contains(byte)
          || (index > 0 && [46, 95, 58, 45].contains(byte))
      }) else { throw ProviderSecretError.invalidReference }
    let authentication = LAContext()
    authentication.interactionNotAllowed = true
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: Self.service,
      kSecAttrAccount as String: reference,
      kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
      kSecMatchLimit as String: kSecMatchLimitOne,
      kSecReturnData as String: true,
      kSecAttrSynchronizable as String: false,
      kSecUseDataProtectionKeychain as String: true,
      kSecUseAuthenticationContext as String: authentication,
    ]
    let (status, bytes) = reader(query as CFDictionary)
    switch status {
    case errSecSuccess:
      guard let bytes else { throw ProviderSecretError.unavailable }
      return try ProviderSecret(bytes)
    case errSecItemNotFound: throw ProviderSecretError.missing
    case errSecInteractionNotAllowed, errSecAuthFailed, errSecNotAvailable:
      throw ProviderSecretError.locked
    default: throw ProviderSecretError.unavailable
    }
  }
}
