import Foundation
import CryptoKit

enum ResultClientError:Error,Sendable {case failed(String),invalidReply,staleSelection,closing,sourceTooLarge}
struct NativeResultManifest:Sendable {
  let root:String
  let total:UInt32
  let offset:UInt32
  let status:String
  let bindings:[ResultBinding]
  let payload:HostJSONValue
}
/// Trusted fixture observes actual decoded delivery; no model/native request can set a callback.
struct ResultTransportProbes:Sendable {
  var beforeDelivery:(@Sendable (NativeResultHostReply) async->Void)?
}
/// Result transport owns pending readonly requests, never notebook/evaluator or acceptance state.
actor ResultTransport {
  private let host:NativeHostClient
  private let probes:ResultTransportProbes
  private var closing=false
  private var active=Set<String>()
  init(host:NativeHostClient,probes:ResultTransportProbes = .init()) {self.host=host;self.probes=probes}
  private func perform(_ command:NativeResultHostCommand,id:String) async throws->NativeResultHostReply {
    guard !closing,active.count<32 else {throw ResultClientError.closing}
    active.insert(id);defer {active.remove(id)}
    if Task.isCancelled {throw ResultClientError.failed("CANCELLED")}
    var reply=try await host.resultCommand(command)
    while true {
      guard reply.request_id==id else {throw ResultClientError.invalidReply}
      switch reply.phase {
      case .completed:
        if let delivery=probes.beforeDelivery {await delivery(reply)}
        return reply
      case .failed,.cancelled:throw ResultClientError.failed(reply.error_code.value ?? "RESULT_NOT_AVAILABLE")
      default:
        if Task.isCancelled || closing {_ = try? await host.cancel(operation:id)}
        try? await Task.sleep(for:.milliseconds(10))
        reply=try await host.resultCommand(.result_status(.init(type:.result_status,request_id:id)))
      }
    }
  }
  func manifest(_ root:String,offset:UInt32=0,limit:UInt32=16) async throws->NativeResultManifest {
    let id="result-read-"+UUID().uuidString.lowercased()
    let reply=try await perform(.result_manifest(.init(type:.result_manifest,request_id:id,root_result_id:root,offset:offset,limit:limit)),id:id)
    return try await Task.detached {
      let data=try JSONEncoder().encode(reply.payload),raw=try JSONSerialization.jsonObject(with:data) as? [String:Any]
      guard let raw,raw["root_result_id"] as? String==root,let total=raw["total"] as? UInt32,let actualOffset=raw["offset"] as? UInt32,actualOffset==offset,
        let status=raw["status"] as? String,let entries=raw["entries"] as? [[String:Any]],entries.count<=limit else {throw ResultClientError.invalidReply}
      let bindings=try entries.map {entry in
        guard let binding=entry["binding"] else {throw ResultClientError.invalidReply}
        return try JSONDecoder().decode(ResultBinding.self,from:JSONSerialization.data(withJSONObject:binding))
      }
      guard bindings.allSatisfy({$0.document_id==reply.document_id && $0.document_generation.value==reply.document_generation.value && $0.runtime_instance_id==reply.runtime_instance_id}) else {throw ResultClientError.invalidReply}
      return .init(root:root,total:total,offset:actualOffset,status:status,bindings:bindings,payload:reply.payload)
    }.value
  }
  func inspect(_ reference:String,query:NativeResultQuery) async throws->NativeResultHostReply {
    let id="result-read-"+UUID().uuidString.lowercased()
    let reply=try await perform(.result_inspect(.init(type:.result_inspect,request_id:id,result_ref:reference,query:query)),id:id)
    guard reply.binding.value?.result_ref==reference else {throw ResultClientError.invalidReply};return reply
  }
  func revoke(_ reference:String) async throws {
    let id="result-revoke-"+UUID().uuidString.lowercased()
    _ = try await perform(.result_revoke(.init(type:.result_revoke,request_id:id,result_ref:reference)),id:id)
  }
  func currentScope() async throws->NativeKernelHostReply {try await host.kernelCommand(.kernel_state(.init(type:.kernel_state)))}
  func close() async {
    closing=true
    for id in active {_ = try? await host.cancel(operation:id)}
    while !active.isEmpty {try? await Task.sleep(for:.milliseconds(10))}
  }
}
private struct SourceFragment:Decodable,Sendable {
  let byte_offset:UInt64
  let next_byte_offset:UInt64
  let total_utf8_bytes:UInt64
  let source_hash:String
  let text:String
  let complete:Bool
}
/// MainActor selection owner rejects late data; the Rust binding remains the mathematical authority.
@MainActor final class ResultClient {
  private let transport:ResultTransport
  private(set) var selection:ResultBinding?
  private var generation:UInt64=0
  private var closing=false
  init(host:NativeHostClient,probes:ResultTransportProbes = .init()) {transport=ResultTransport(host:host,probes:probes)}
  func select(_ binding:ResultBinding?) throws {
    guard !closing,generation<HostSerial.maximum else {throw ResultClientError.closing}
    generation+=1;selection=binding
  }
  func manifest(_ root:String,offset:UInt32=0,limit:UInt32=16) async throws->NativeResultManifest {
    let expected=generation;let result=try await transport.manifest(root,offset:offset,limit:limit)
    guard !closing,generation==expected else {throw ResultClientError.staleSelection};return result
  }
  private func stillSelected(_ binding:ResultBinding,generation:UInt64)->Bool {
    self.generation==generation && !closing && selection?.result_ref==binding.result_ref && selection?.result_id==binding.result_id
      && selection?.document_id==binding.document_id && selection?.document_generation.value==binding.document_generation.value
      && selection?.cell_id==binding.cell_id && selection?.out_index.value==binding.out_index.value && selection?.view_id.value==binding.view_id.value
  }
  /// Returns nil if the selection changed while a real worker was computing the readonly view.
  func inspect(_ query:NativeResultQuery) async throws->NativeResultHostReply? {
    guard let binding=selection,!closing else {throw ResultClientError.staleSelection}
    let expected=generation
    let reply=try await transport.inspect(binding.result_ref,query:query)
    guard stillSelected(binding,generation:expected),!Task.isCancelled else {return nil}
    let current=try await transport.currentScope()
    guard current.document_id==reply.document_id,current.document_generation.value==reply.document_generation.value,
      current.source_revision.value==reply.document_revision.value,current.execution_epoch.value==reply.execution_epoch.value,
      current.kernel_state_revision.value==reply.kernel_state_revision.value else {return nil}
    guard stillSelected(binding,generation:expected),!Task.isCancelled else {return nil}
    guard let actual=reply.binding.value,actual.result_id==binding.result_id,actual.cell_id==binding.cell_id,
      actual.out_index.value==binding.out_index.value,actual.view_id.value==binding.view_id.value else {throw ResultClientError.invalidReply}
    selection=actual;return reply
  }
  /// Complete source is assembled from exact UTF8 fragments, with total byte/hash identity checked.
  /// A partial/stale/expired source is never returned as a successful complete copy.
  func completeSource(format:InspectResultSourceFormat = .input_form,path:[UInt32]=[]) async throws->String? {
    guard let binding=selection,!closing else {throw ResultClientError.staleSelection}
    let expected=generation
    var bytes=Data(),offset:UInt64=0,total:UInt64?,hash:String?
    while true {
      let reply=try await transport.inspect(binding.result_ref,query:.inspect_result_source(.init(kind:.source,path:path,format:format,byte_offset:try HostSerial(offset),byte_limit:16384)))
      guard stillSelected(binding,generation:expected) else {return nil}
      let fragment=try await Task.detached {try JSONDecoder().decode(SourceFragment.self,from:JSONEncoder().encode(reply.payload))}.value
      guard fragment.byte_offset==offset,fragment.next_byte_offset==offset+UInt64(fragment.text.utf8.count),fragment.next_byte_offset<=fragment.total_utf8_bytes,
        fragment.total_utf8_bytes<=8*1024*1024,fragment.source_hash.utf8.count==64,hash==nil || hash==fragment.source_hash,
        total==nil || total==fragment.total_utf8_bytes,fragment.complete==(fragment.next_byte_offset==fragment.total_utf8_bytes) else {throw ResultClientError.invalidReply}
      hash=fragment.source_hash;total=fragment.total_utf8_bytes;bytes.append(contentsOf:fragment.text.utf8);offset=fragment.next_byte_offset
      if fragment.complete {break}
      guard !fragment.text.isEmpty else {throw ResultClientError.invalidReply}
    }
    let data=bytes
    let expectedHash=hash
    let text=try await Task.detached { () throws->String in
      guard let text=String(data:data,encoding:.utf8) else {throw ResultClientError.invalidReply}
      let actual=SHA256.hash(data:data).map{String(format:"%02x",$0)}.joined()
      guard actual==expectedHash else {throw ResultClientError.invalidReply};return text
    }.value
    guard stillSelected(binding,generation:expected),!Task.isCancelled else {return nil}
    let current=try await transport.currentScope()
    guard current.document_id==binding.document_id,current.document_generation.value==binding.document_generation.value,
      stillSelected(binding,generation:expected),!Task.isCancelled else {return nil};return text
  }
  func revoke(_ reference:String) async throws {try await transport.revoke(reference)}
  func close() async {closing=true;generation+=1;selection=nil;await transport.close()}
}
