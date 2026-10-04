import Foundation

indirect enum JSONValue: Codable, Sendable, Equatable {
  case object([String: JSONValue])
  case array([JSONValue])
  case string(String)
  case number(Double)
  case bool(Bool)
  case null
  init(from decoder: Decoder) throws {
    let c = try decoder.singleValueContainer()
    if c.decodeNil() {
      self = .null
    } else if let v = try? c.decode(Bool.self) {
      self = .bool(v)
    } else if let v = try? c.decode(Double.self) {
      self = .number(v)
    } else if let v = try? c.decode(String.self) {
      self = .string(v)
    } else if let v = try? c.decode([JSONValue].self) {
      self = .array(v)
    } else {
      self = .object(try c.decode([String: JSONValue].self))
    }
  }
  func encode(to encoder: Encoder) throws {
    var c = encoder.singleValueContainer()
    switch self {
    case .object(let v): try c.encode(v)
    case .array(let v): try c.encode(v)
    case .string(let v): try c.encode(v)
    case .number(let v): try c.encode(v)
    case .bool(let v): try c.encode(v)
    case .null: try c.encodeNil()
    }
  }
  subscript(_ key: String) -> JSONValue {
    get { object[key] ?? .null }
    set {
      var v = object
      v[key] = newValue
      self = .object(v)
    }
  }
  subscript(_ index: Int) -> JSONValue { array.indices.contains(index) ? array[index] : .null }
  var object: [String: JSONValue] {
    if case .object(let v) = self { return v }
    return [:]
  }
  var array: [JSONValue] {
    if case .array(let v) = self { return v }
    return []
  }
  var string: String {
    if case .string(let v) = self { return v }
    return ""
  }
  var double: Double {
    if case .number(let v) = self { return v }
    return 0
  }
  var bool: Bool {
    if case .bool(let v) = self { return v }
    return false
  }
  var isNull: Bool { self == .null }
  static func text(_ v: String) -> Self { .string(v) }
  func data() throws -> Data { try JSONEncoder().encode(self) }
  var pretty: String {
    let e = JSONEncoder()
    e.outputFormatting = [.prettyPrinted, .sortedKeys]
    return (try? String(decoding: e.encode(self), as: UTF8.self)) ?? ""
  }
}
struct Envelope: Codable, Sendable {
  let id: UInt64
  var body: JSONValue
}
struct KernelPacket: Codable, Sendable {
  var response: Envelope
  var events: [Envelope]
}
enum CellKind: String, Codable, Sendable, CaseIterable { case Math, Text, Ask }
enum Dialect: String, Codable, Sendable, CaseIterable { case Modern, Wolfram, Auto }
enum CellStatus: String, Codable, Sendable { case Queued, Running, Done, Error, Stale }
struct CellInput: Codable, Sendable, Identifiable, Equatable {
  var id: String = UUID().uuidString
  var kind: CellKind = .Math
  var source = ""
  var dialect: Dialect = .Modern
  var json: JSONValue {
    .object([
      "id": .string(id), "kind": .string(kind.rawValue), "source": .string(source),
      "dialect": .string(dialect.rawValue),
    ])
  }
}
struct NotebookFile: Codable, Sendable, Equatable {
  var version = 1
  var title = "未命名笔记本"
  var cells: [CellInput] = []
  func validate() throws {
    guard version == 1, Set(cells.map(\.id)).count == cells.count,
      cells.allSatisfy({ !$0.id.isEmpty })
    else { throw KernelError.message("笔记本格式无效") }
  }
}
enum KernelError: Error, LocalizedError, Sendable {
  case message(String)
  var errorDescription: String? {
    if case .message(let v) = self { return v }
    return nil
  }
}
struct NotebookCell: Identifiable {
  var input: CellInput
  var revision = 0
  var status: CellStatus = .Stale
  var output: JSONValue = .null
  var id: String { input.id }
}
