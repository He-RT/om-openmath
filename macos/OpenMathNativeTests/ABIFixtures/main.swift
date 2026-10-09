import Foundation
import OpenMathHost

func bytes(_ buffer: om_host_buffer) throws -> Data {
  defer { om_host_buffer_free(buffer) }
  guard let ptr = buffer.ptr, buffer.len > 0 else {
    throw NSError(domain: "OpenMathABI", code: 1)
  }
  return Data(bytes: ptr, count: buffer.len)
}
func decode<T: Decodable>(_ buffer: om_host_buffer, _ type: T.Type) throws -> T {
  try JSONDecoder().decode(type, from: bytes(buffer))
}
func create() throws -> OpaquePointer {
  let data = try JSONEncoder().encode(HostInit(protocol_version: 1,
    runtime_instance_id: "swift-ffi", max_pending_operations: 8, event_capacity: 32))
  let result = data.withUnsafeBytes { raw in
    om_host_create(raw.bindMemory(to: UInt8.self).baseAddress, raw.count)
  }
  if let handle = result.handle {
    precondition(result.error.ptr == nil && result.error.len == 0)
    return handle
  }
  let failure = try decode(result.error, HostFailure.self)
  throw NSError(domain: "OpenMathABI", code: 2,
    userInfo: [NSLocalizedDescriptionKey: failure.error.message])
}
func submit(_ host: OpaquePointer, _ id: String, _ body: HostRequestBody) throws {
  let request = RequestEnvelope(protocol_version: 1, runtime_instance_id: "swift-ffi",
    request_ref: id, operation_id: .init(id), document_binding: .init(nil),
    task_binding: .init(nil), body: body)
  let data = try JSONEncoder().encode(request)
  let buffer = data.withUnsafeBytes { raw in
    om_host_submit(host, raw.bindMemory(to: UInt8.self).baseAddress, raw.count)
  }
  let admission = try decode(buffer, AdmissionReceipt.self)
  precondition(admission.accepted && admission.request_ref == id)
}
func finished(_ host: OpaquePointer, _ id: String) throws -> [String:Any] {
  let deadline = Date().addingTimeInterval(10)
  while Date() < deadline {
    let data = try bytes(om_host_next_events(host, 100, 512 * 1024))
    let batch = try JSONDecoder().decode(EventBatch.self, from: data)
    for event in batch.events where event.operation_ref.value == id
      && event.event_kind == .operation_finished {
      let payload = try JSONEncoder().encode(event.payload)
      return try JSONSerialization.jsonObject(with: payload) as! [String:Any]
    }
  }
  throw NSError(domain:"OpenMathABI",code:3)
}
func scratch(_ source:String)->HostRequestBody {
  .evaluate_scratch(EvaluateScratch(kind:.evaluate_scratch,source:source,dialect:.modern,
    definition_snapshot_ref:.init(nil),use_notebook_definitions:false,timeout_ms:10000))
}
let invalid = try decode(om_host_create(nil,1).error,HostFailure.self)
precondition(invalid.error.code == .invalid_argument)
let host = try create()
try submit(host,"sum",scratch("2+2"))
let result = try finished(host,"sum")
let value = result["result"] as! [String:Any]
let response = value["response"] as! [String:Any]
let output = response["output"] as! [String:Any]
let items = output["items"] as! [[String:Any]]
precondition(items[0]["input_form"] as! String == "4")
precondition(value["run_outcome"] as! String == "completed")
try submit(host,"bad",scratch("let f(x)=;"))
let bad = try finished(host,"bad")["result"] as! [String:Any]
precondition(bad["run_outcome"] as! String == "failed")
try submit(host,"cap",.read_host_capabilities(.init(kind:.get_capabilities)))
let cap = try finished(host,"cap")["result"] as! [String:Any]
let kernel = cap["kernel"] as! [String:Any]
precondition(kernel["platform"] as! String == "desktop")
precondition(cap["native_renderers_ready"] as! Bool == false)
try submit(host,"long",scratch("map(fn(k)=>sin(k),range(1,100000))"))
let stop = Data("long".utf8).withUnsafeBytes { raw in
  om_host_cancel(host,raw.bindMemory(to:UInt8.self).baseAddress,raw.count)
}
precondition(stop == 1)
let cancelled = try finished(host,"long")
precondition(cancelled["phase"] as! String == "cancelled")
_ = try bytes(om_host_close_begin(host))
precondition(om_host_close_finish(host) == 0)
precondition(om_host_close_finish(host) == -2)
let stale = try decode(om_host_next_events(host,0,512*1024),HostFailure.self)
precondition(stale.error.code == .invalid_reference)
print("Swift -> native C ABI -> real Rust CAS: exact 4, math failure, Desktop capability, direct cancel, owned decode/free, consumed handle passed")
