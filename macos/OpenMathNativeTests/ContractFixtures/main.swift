import Foundation

let valid = Data("""
{"contract_version":1,"runtime_instance_id":"runtime-1","host_phase":"starting","ui_event_sequence":0,"rust_event_sequence":0,"document":null,"agent_task":null,"pi":{"phase":"stopped","connection_id":null,"connection_generation":0,"protocol_version":1},"operations":[]}
""".utf8)
let decoded = try JSONDecoder().decode(HostState.self, from: valid)
let encoded = try JSONEncoder().encode(decoded)
let a = try JSONSerialization.jsonObject(with: valid) as! NSDictionary
let b = try JSONSerialization.jsonObject(with: encoded) as! NSDictionary
precondition(a == b)
func reject(_ value: [String: Any]) throws {
  let data = try JSONSerialization.data(withJSONObject: value)
  do {
    _ = try JSONDecoder().decode(HostState.self, from: data)
    fatalError("invalid generated contract was accepted")
  } catch is DecodingError { }
  catch is HostContractError { }
}
var missing = a as! [String: Any]
missing.removeValue(forKey: "document")
try reject(missing)
var unknown = a as! [String: Any]
unknown["unexpected"] = true
try reject(unknown)
var invalidCounter = a as! [String: Any]
invalidCounter["ui_event_sequence"] = UInt64(9_007_199_254_740_992)
try reject(invalidCounter)
var wrongNull = a as! [String: Any]
wrongNull["document"] = "not-a-document"
try reject(wrongNull)
let maximum = try JSONDecoder().decode(HostSerial.self, from: Data("9007199254740991".utf8))
precondition(maximum.value == HostSerial.maximum)
print("Swift/Rust-compatible generated host fields: roundtrip + four negative cases passed")

let requestData = Data("""
{"protocol_version":1,"runtime_instance_id":"runtime-1","request_ref":"req-1","operation_id":null,"document_binding":null,"task_binding":null,"body":{"kind":"get_capabilities"}}
""".utf8)
let request = try JSONDecoder().decode(RequestEnvelope.self, from: requestData)
guard case .read_host_capabilities = request.body else { fatalError("Wrong generated union dispatch") }
let requestEncoded = try JSONEncoder().encode(request)
let encodedRequestObject = try JSONSerialization.jsonObject(with: requestEncoded) as! NSDictionary
let sourceRequestObject = try JSONSerialization.jsonObject(with: requestData) as! NSDictionary
precondition(encodedRequestObject == sourceRequestObject)
print("Native request union uses the exact constant kind in Swift and Rust")

if CommandLine.arguments.count == 3 {
  let incoming = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
  let host = try JSONDecoder().decode(HostState.self, from: incoming)
  try JSONEncoder().encode(host).write(to: URL(fileURLWithPath: CommandLine.arguments[2]))
  print("Actual Rust -> Swift host bytes decoded and re-encoded")
}
