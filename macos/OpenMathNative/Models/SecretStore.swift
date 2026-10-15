import Foundation
import Security
import LocalAuthentication

enum ProviderSecretError: Error, Sendable { case invalidReference, missing, locked, unavailable, invalidValue, echoed }
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
  func responseGuard()->ProviderSecretResponseGuard {ProviderSecretResponseGuard(needle:bytes)}
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

/// Hold a possible key suffix across chunks so a literal auth echo never reaches model/ledger.
struct ProviderSecretResponseGuard:Sendable,CustomStringConvertible,CustomDebugStringConvertible {
  private let needle:Data
  private var pending=Data()
  fileprivate init(needle:Data){self.needle=needle}
  var description:String {"ProviderSecretResponseGuard(redacted)"}
  var debugDescription:String {description}
  mutating func feed(_ bytes:Data,final:Bool=false)throws->Data {
    pending.append(bytes)
    guard pending.range(of:needle)==nil else {pending.removeAll();throw ProviderSecretError.echoed}
    let count=final ? pending.count:max(0,pending.count-(needle.count-1))
    let ready=Data(pending.prefix(count));pending.removeFirst(count);return ready
  }
}

/// Reads exact references from the new Mac namespace. No enumeration, legacy service or fallback.
/// Registry candidate creation/replacement is a separate R4.4.01 transaction.
actor SecretStore {
  static let service = "org.openmath.NativeMacPreview.provider-secrets.r4.v1"
  // The native macOS app uses the encrypted login keychain. Data Protection keychain
  // requires an application entitlement unavailable to experimental ad hoc builds.
  static let useDataProtectionKeychain=false
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

  static func insert(reference:String,bytes:Data)throws {
    guard validReference(reference) else {throw ProviderSecretError.invalidReference}
    _ = try ProviderSecret(bytes)
    let query:[String:Any]=[kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:service,kSecAttrAccount as String:reference,kSecValueData as String:bytes,kSecAttrAccessible as String:kSecAttrAccessibleWhenUnlockedThisDeviceOnly,kSecAttrSynchronizable as String:false,kSecUseDataProtectionKeychain as String:useDataProtectionKeychain]
    guard SecItemAdd(query as CFDictionary,nil)==errSecSuccess else {throw ProviderSecretError.unavailable}
  }
  static func remove(reference:String) {
    guard validReference(reference) else {return}
    SecItemDelete([kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:service,kSecAttrAccount as String:reference,kSecAttrSynchronizable as String:false,kSecUseDataProtectionKeychain as String:useDataProtectionKeychain] as CFDictionary)
  }
  private static func validReference(_ reference:String)->Bool {
    !reference.isEmpty && reference.utf8.count<=256 && reference.utf8.enumerated().allSatisfy {index,byte in
      (65...90).contains(byte) || (97...122).contains(byte) || (48...57).contains(byte) || (index>0 && [46,95,58,45].contains(byte))
    }
  }

  func read(reference: String) throws -> ProviderSecret {
    guard Self.validReference(reference) else {throw ProviderSecretError.invalidReference}
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
      kSecUseDataProtectionKeychain as String: Self.useDataProtectionKeychain,
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
