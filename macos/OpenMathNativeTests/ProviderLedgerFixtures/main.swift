import Foundation
import Darwin

enum ProviderLedgerFixtureError:Error {case fixture}
private func assertNoSecret(root:URL,secret:Data)throws {
  guard let files=FileManager.default.enumerator(at:root,includingPropertiesForKeys:[.isRegularFileKey]) else {throw ProviderLedgerFixtureError.fixture}
  for case let file as URL in files {
    if try file.resourceValues(forKeys:[.isRegularFileKey]).isRegularFile==true {let bytes=try Data(contentsOf:file);precondition(bytes.range(of:secret)==nil)}
  }
}
private func originalBlob(_ storage:StorageService,owner:String,hash:String)async throws->Data {
  let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:owner),hash:hash)
  let data=try await storage.readBlob(lease,offset:0,count:Int(lease.descriptor.byteLength));try await storage.closeBlob(lease);return data
}
private func request(_ host:NativeHostClient)async throws->ProviderModelSession {
  let message:[String:Any] = ["record_type":"canonical_agent_message","message_version":1,"message_id":"user","role":"user","task_id":"task","agent_turn_id":"turn","parts":[["kind":"text","part_id":"text","text":"2+2"]]]
  let value:[String:Any] = ["binding":["provider_ref":"provider","route_ref":"route","model_ref":"model","adapter_version":"single-turn-1","protocol":"chat_completions"],"api_model":"synthetic","messages":[["canonical":message,"replay":NSNull()]],"results":[],"tools":[],"max_output_tokens":64,"temperature":NSNull(),"stream":true]
  return try await ProviderModelSession.prepare(host:host,requestJSON:String(decoding:JSONSerialization.data(withJSONObject:value),as:UTF8.self),request:"request",context:"context",task:"task",turn:"turn",message:"assistant")
}
@main struct ProviderLedgerFixtures {
  static let session="provider-durable"
  static func main()async throws {
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true),mode=CommandLine.arguments[2]
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    if mode=="inspect" {
      let expected=CommandLine.arguments[3],before=try await storage.sessionEvents(session),page=try await storage.sessionRecovery(session)
      precondition(page.requests.count==1 && page.requests[0].phase.rawValue==expected)
      let again=try await storage.sessionEvents(session);precondition(before.count==again.count)
      try JSONEncoder().encode(again.map(\.event)).write(to:root.appendingPathComponent("recovered-events.json"))
      print("Recovered \(expected) from \(again.count) original events; no transport/host started, no event appended.")
      try await storage.close();return
    }
    _ = try await storage.createAgentSession(session)
    let host=try await NativeHostClient.open(),model=try await request(host),ledger=SessionLedgerPort(storage:storage,session:session)
    let stop=CommandLine.arguments.count>3 ? CommandLine.arguments[3]:""
    let faults=StorageFaults(reach:{stage in
      if stage==stop {try Data(stage.utf8).write(to:root.appendingPathComponent("stopped"));raise(SIGSTOP)}
    })
    let owner=ProviderRequestLedger(storage:storage,ledger:ledger,model:model,faults:faults)
    let secret="synthetic-provider-secret",text=mode=="echo" ? secret:"4"
    let response=Data("data: {\"id\":\"provider-id\",\"model\":\"served-model\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\(text)\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n".utf8)
    let server=try DelayedLoopbackServer(response:response,delay:mode=="timeout" ? 0.3:0.01)
    let route=try ProviderRoute(binding:model.binding,endpoint:URL(string:"http://127.0.0.1:\(server.port)/v1/chat/completions")!,authentication:mode=="echo" ? .bearer:.none,credentialReference:mode=="echo" ? "fixture-secret":nil,timeouts:try ProviderTimeouts(firstResponse:mode=="timeout" ? 0.05:5,idle:3,total:8),allowHTTPOnLiteralLoopback:true)
    let durable=owner.journal()
    let checked=ProviderTransportJournal(record:{fact in
      do {try await durable.record(fact)} catch {try FileHandle.standardError.write(contentsOf:Data("Fixture durable stage \(fact.phase.rawValue) rejected: \(String(reflecting:error))\n".utf8));throw error}
      if fact.phase == .dispatchStarted {
        let page=try await storage.sessionRecovery(session)
        precondition(page.requests.first?.phase == .dispatch_outcome_unknown && !server.receivedBody)
      }
    },responseBytes:durable.responseBytes)
    let secrets=SecretStore(testReader:{_ in (0,Data(secret.utf8))}),transport=ProviderTransport(secrets:secrets)
    if mode=="echo" || mode=="timeout" {
      do {_ = try await transport.send(model,route:route,journal:checked,onText:{_ in fatalError("negative request emitted text")});fatalError("negative request succeeded")}
      catch let failure as ProviderTransportFailure {precondition(failure.reason==(mode=="echo" ? .credentialEcho:.firstResponseTimeout))}
      let page=try await storage.sessionRecovery(session)
      precondition(page.requests.first?.phase==(mode=="echo" ? .failed:.dispatch_outcome_unknown))
      try await host.close();try await storage.close()
      if mode=="echo" {try assertNoSecret(root:root,secret:Data(secret.utf8))}
      print(mode=="echo" ? "Actual server credential echo rejected before model/Blob/event; entire closed store exact-value scan empty.":"Actual sent request timeout persisted interrupted/dispatch_outcome_unknown; no retry.")
      return
    }
    let result=try await transport.send(model,route:route,journal:checked,onText:{_ in})
    try await owner.complete(result)
    let events=try await storage.sessionEvents(session),page=try await storage.sessionRecovery(session)
    precondition(page.requests.count==1 && page.requests[0].phase == .finished && events.count==5)
    let lifecycle=try events.filter{$0.event.event_kind == .task_transition}.map{try JSONDecoder().decode(SessionRequestLifecycle.self,from:Data($0.event.payload_inline_json.value!.utf8))}
    precondition(lifecycle.compactMap(\.transition?.phase)==[.dispatch_started,.response_started,.finished])
    precondition(server.receivedBody && result.finish=="complete")
    let wire=try await originalBlob(storage,owner:"request-"+session+"-context",hash:lifecycle[0].plan!.wireBlobHash)
    precondition(wire==model.body)
    let original=try await originalBlob(storage,owner:"response-"+session+"-context",hash:lifecycle.last!.transition!.responseBlobHash!)
    precondition(original==response)
    let reference=try JSONSerialization.jsonObject(with:Data(events.last!.event.payload_inline_json.value!.utf8)) as! [String:Any]
    let base="provider-"+KernelHashes.data(Data((session+"\u{0}context\u{0}request").utf8))
    let saved=try await originalBlob(storage,owner:base+".model",hash:reference["blobHash"] as! String),savedJSON=try JSONSerialization.jsonObject(with:saved) as! [String:Any]
    let replay=try JSONSerialization.jsonObject(with:Data((savedJSON["replayJSON"] as! String).utf8)) as! [String:Any]
    precondition(replay["response_id"] as? String=="provider-id" && replay["provider_model"] as? String=="served-model")
    try saved.write(to:root.appendingPathComponent("actual-model-facts.json"))
    try JSONEncoder().encode(events.map(\.event)).write(to:root.appendingPathComponent("actual-events.json"))
    try model.body.write(to:root.appendingPathComponent("actual-request-body.json"));try response.write(to:root.appendingPathComponent("actual-response.sse"))
    try await host.close();try await storage.close()
    print("Actual Library4 request body/plan/dispatch before socket, original response/finish/model facts committed: 5 events; real HTTP/Rust codec result4, no credentials.")
  }
}
