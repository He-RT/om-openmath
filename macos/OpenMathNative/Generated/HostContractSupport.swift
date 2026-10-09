import Foundation

public struct HostCodingKey: CodingKey {
  public var stringValue: String
  public var intValue: Int?
  public init?(stringValue: String) { self.stringValue = stringValue; self.intValue = nil }
  public init?(intValue: Int) { self.stringValue = String(intValue); self.intValue = intValue }
}

public struct HostNullable<T: Codable & Sendable>: Codable, Sendable {
  public var value: T?
  public init(_ value: T?) { self.value = value }
  public init(from decoder: Decoder) throws {
    let c = try decoder.singleValueContainer()
    value = try c.decodeNil() ? nil : c.decode(T.self)
  }
  public func encode(to encoder: Encoder) throws {
    var c = encoder.singleValueContainer()
    if let value { try c.encode(value) } else { try c.encodeNil() }
  }
}

public struct HostSerial: Codable, Sendable {
  public static let maximum: UInt64 = 9_007_199_254_740_991
  public let value: UInt64
  public init(_ value: UInt64) throws {
    guard value <= Self.maximum else { throw HostContractError.invalidSerial }
    self.value = value
  }
  public init(from decoder: Decoder) throws {
    let c = try decoder.singleValueContainer()
    try self.init(c.decode(UInt64.self))
  }
  public func encode(to encoder: Encoder) throws {
    var c = encoder.singleValueContainer()
    try c.encode(value)
  }
}

public enum HostContractError: Error { case invalidSerial }

public enum HostJSONValue: Codable, Sendable, Equatable {
  case null, bool(Bool), number(Double), string(String), array([Self]), object([String: Self])
  public init(from decoder: Decoder) throws {
    let c = try decoder.singleValueContainer()
    if c.decodeNil() { self = .null }
    else if let v = try? c.decode(Bool.self) { self = .bool(v) }
    else if let v = try? c.decode(String.self) { self = .string(v) }
    else if let v = try? c.decode([Self].self) { self = .array(v) }
    else if let v = try? c.decode([String: Self].self) { self = .object(v) }
    else { self = .number(try c.decode(Double.self)) }
  }
  public func encode(to encoder: Encoder) throws {
    var c = encoder.singleValueContainer()
    switch self {
    case .null: try c.encodeNil()
    case .bool(let v): try c.encode(v)
    case .number(let v): try c.encode(v)
    case .string(let v): try c.encode(v)
    case .array(let v): try c.encode(v)
    case .object(let v): try c.encode(v)
    }
  }
}
