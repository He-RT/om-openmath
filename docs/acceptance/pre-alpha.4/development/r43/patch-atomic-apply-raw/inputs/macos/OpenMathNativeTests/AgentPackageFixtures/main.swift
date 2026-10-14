import Foundation
import CryptoKit

func encoded<T:Encodable>(_ value:T)throws->Data {try JSONEncoder().encode(value)}
/// This three-turn fixture exceeds the storage API's default 16-event page. Read its bounded
/// original pages instead of mistaking the first page for a complete conversation.
private func fixtureEvents(_ storage:StorageService,_ session:String)async throws->[StoredSessionEvent] {
  var after:UInt64=0
  var all:[StoredSessionEvent]=[]
  while true {
    let page=try await storage.sessionEvents(session,after:after,limit:16)
    guard let last=page.last else {return all}
    precondition(last.event.event_sequence>after && all.count+page.count<=64)
    all+=page;after=last.event.event_sequence
  }
}
private actor WireCapture {
  var host=AgentFrameBytes(),helper=AgentFrameBytes(),frames:[[String:String]]=[]
  func append(_ bytes:Data,toHelper:Bool) {
    do {let actual=try toHelper ? host.push(bytes):helper.push(bytes);for frame in actual {frames.append(["direction":toHelper ? "host":"helper","frame_json":String(decoding:frame,as:UTF8.self)])}}
    catch {frames.append(["capture_error":"invalid fixture frame"])}
  }
  func save(_ url:URL)throws {try JSONSerialization.data(withJSONObject:frames,options:[.sortedKeys]).write(to:url)}
}
private actor ModelHold {
  var arrived=false
  var continuation:AsyncThrowingStream<AgentModelEventPayload,any Error>.Continuation?
  func stream()->AsyncThrowingStream<AgentModelEventPayload,any Error> {arrived=true;return AsyncThrowingStream {continuation=$0}}
  func wait() async throws {for _ in 0..<500 {if arrived {return};try await Task.sleep(for:.milliseconds(10))};throw AgentSessionError.inactive}
  func late(_ event:AgentModelEventPayload) {continuation?.yield(event);continuation?.finish()}
}

/// The actual native transport/C ABI/Library producer; only provider wire and route are synthetic.
private func nativeProviderTurn(host:NativeHostClient,storage:StorageService,root:URL,input:AgentCanonicalMessage,intent:NativeAgentRequestIntent,task:String,session:String="production-task",preview:Bool=false,apply:Bool=false)async throws->ProviderPreparedTurn {
  let stored=try await fixtureEvents(storage,session)
  let readonly=stored.compactMap {event->AgentReadonlyRecord? in guard let raw=event.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(raw.utf8)),record.kind=="native_readonly_result" else {return nil};return record}
  let effects=stored.compactMap {event->AgentEffectRecord? in guard let raw=event.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(raw.utf8)),record.kind=="native_effect_result" else {return nil};return record}
  let count=readonly.count+effects.count,second = count>0,finishing = second && (!preview || readonly.count==2) && (!apply || effects.count==1)
  var messages:[[String:Any]] = [["canonical":try JSONSerialization.jsonObject(with:encoded(input)),"replay":NSNull()]],results:[[String:Any]]=[]
  // Preserve the actual durable event order: every assistant call is followed by its tool result
  // before the next assistant. The native codec rejects a reordered/dangling tool conversation.
  for event in stored {
    if event.event.event_kind == .assistant_end {
      let original=try JSONDecoder().decode(SessionCanonicalAssistant.self,from:Data(event.event.payload_inline_json.value!.utf8))
      messages.append(["canonical":try JSONSerialization.jsonObject(with:Data(original.canonicalJSON.utf8)),"replay":NSNull()])
    }
    guard let text=event.event.payload_inline_json.value else {continue}
    let result:AgentReadonlyRecord
    if let read=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(text.utf8)),read.kind=="native_readonly_result" {result=read}
    else if let effect=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(text.utf8)) {result=AgentReadonlyRecord(codecVersion:effect.codecVersion,kind:effect.kind,intent:effect.intent,resultRef:effect.resultRef,blobHash:effect.blobHash,byteLength:effect.byteLength,canonical:effect.canonical)}
    else {continue}
    let owner=BlobOwner(kind:.context,id:result.canonical.message_id),lease=try await storage.openReferencedBlob(owner:owner,hash:result.blobHash)
    let bytes=try await storage.readBlob(lease,offset:0,count:Int(result.byteLength));try await storage.closeBlob(lease)
    guard KernelHashes.data(bytes)==result.blobHash else {throw AgentSessionError.ledgerMismatch}
    let data=try JSONSerialization.jsonObject(with:bytes) as! [String:Any]
    precondition(data["ok"] as? Bool==true)
    messages.append(["canonical":try JSONSerialization.jsonObject(with:encoded(result.canonical)),"replay":NSNull()])
    results.append(["message_id":result.canonical.message_id,"tool_call_id":result.intent.tool_call_id,"text":String(decoding:bytes,as:UTF8.self),"is_error":false])
  }
  var tool="read_notebook",arguments=preview ? "{ \"view\" : \"cells\", \"cell_ids\" : [\"a\"] }":"{ \"view\" : \"outline\", \"include_outputs\" : true }"
  if preview && readonly.count==1 {
    let data=try JSONSerialization.jsonObject(with:Data((results[0]["text"] as! String).utf8)) as! [String:Any],read=data["data"] as! [String:Any],cell=(read["cells"] as! [[String:Any]])[0]
    precondition(cell["source_complete"] as? Bool==true && cell["source"] as? String=="2+2")
    tool="preview_source"
    arguments=String(decoding:try JSONSerialization.data(withJSONObject:["snapshot_ref":read["snapshot_ref"]!,"input":["kind":"patch","operations":[["type":"update_cell","target":["cell_id":"a"],"expected_source_hash":cell["source_hash"]!,"source":"3+3"]]]],options:[.sortedKeys]),as:UTF8.self)
  }
  if apply && readonly.count==2 && effects.isEmpty {
    let envelope=try JSONSerialization.jsonObject(with:Data((results[1]["text"] as! String).utf8)) as! [String:Any],data=envelope["data"] as! [String:Any]
    precondition(data["valid"] as? Bool==true);tool="apply_notebook_patch"
    arguments=String(decoding:try JSONSerialization.data(withJSONObject:["preview_ref":data["preview_ref"]!],options:[.sortedKeys]),as:UTF8.self)
  }
  let delta:[String:Any] = finishing ? ["role":"assistant","content":"已读取真实笔记本🙂"]:["role":"assistant","content":"中文🙂","tool_calls":[["index":0,"id":count==2 ? "apply-call":second ? "preview-call":"provider-call","type":"function","function":["name":tool,"arguments":arguments]]]]
  let chunk:[String:Any] = ["id":"provider-response-\(count+1)","model":"synthetic-served-alias","choices":[["index":0,"delta":delta,"finish_reason":finishing ? "stop":"tool_calls"]]]
  let sse=Data(("data: "+String(decoding:try JSONSerialization.data(withJSONObject:chunk),as:UTF8.self)+"\n\ndata: [DONE]\n\n").utf8)
  var tools:[[String:Any]] = [["name":"read_notebook","description":"Read actual confirmed source","parameters":["type":"object"]]]
  if preview {tools.append(["name":"preview_source","description":"Inspect actual frozen source plan","parameters":["type":"object"]])}
  if apply {tools.append(["name":"apply_notebook_patch","description":"Commit the original frozen source plan","parameters":["type":"object"]])}
  let request:[String:Any] = ["binding":["provider_ref":"fixture-provider","route_ref":"fixture-route","model_ref":"fixture-model","adapter_version":"single-turn-1","protocol":"chat_completions"],"api_model":"synthetic-configured-alias","messages":messages,"results":results,"tools":tools,"max_output_tokens":128,"temperature":NSNull(),"stream":true]
  let bytes=try JSONSerialization.data(withJSONObject:request,options:[.sortedKeys]),context="fixture-context-"+UUID().uuidString.lowercased()
  let model=try await ProviderModelSession.prepare(host:host,requestJSON:String(decoding:bytes,as:UTF8.self),request:intent.request_id,context:context,task:task,turn:intent.agent_turn_id,message:"provider-canonical-"+intent.agent_turn_id)
  let server=try DelayedLoopbackServer(response:sse,delay:0.01)
  let route=try ProviderRoute(binding:model.binding,endpoint:URL(string:"http://127.0.0.1:\(server.port)/v1/chat/completions")!,authentication:.none,credentialReference:nil,timeouts:try ProviderTimeouts(firstResponse:5,idle:3,total:8),allowHTTPOnLiteralLoopback:true)
  let ledger=SessionLedgerPort(storage:storage,session:session),owner=ProviderRequestLedger(storage:storage,ledger:ledger,model:model)
  let suffix=second ? "-\(count+1)":""
  try bytes.write(to:root.appendingPathComponent("actual-native-model-input"+suffix+".json"));try model.body.write(to:root.appendingPathComponent("actual-native-http-body"+suffix+".json"));try sse.write(to:root.appendingPathComponent("actual-native-response"+suffix+".sse"))
  return owner.turn(route:route)
}
/// A separate actual three-turn Pi task. The model can freeze a patch from the real returned read
/// token; neither this task nor storage-only recovery commits/runs/saves that proposed patch.
private func nativePreviewRoundTrip(package:URL,signatureRoot:URL?,root:URL)async throws {
  let document="00000000-0000-0000-0000-000000000436",session="preview-task"
  var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"真实Agent冻结预览",cells:[.init(id:"a",kind:.math,source:"2+2",dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
  snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
  let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview)),info=try await storage.openDocument(document)
  _ = try await storage.initializeSource(snapshot)
  let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
  let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value)
  let hello=try await service.activate(session:session,grants:[.read_notebook,.preview_source],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
  let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"preview-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"读取实际源码后仅预览整个修改"))])
  let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:root,input:input,intent:intent,task:hello.scope.task_id,session:session,preview:true)})
  let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
  await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
  let report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)
  try await wire.save(root.appendingPathComponent("actual-preview-wire.json"))
  try JSONSerialization.data(withJSONObject:["helper_frames":report.frames,"tool_operation_ids":report.toolIntents,"last_phase":report.lastPhase.rawValue],options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-preview-report.json"))
  precondition(report.frames==13 && report.toolIntents.count==2 && report.lastPhase == .awaiting_settlement)
  let events=try await fixtureEvents(storage,session);precondition(events.count==21)
  let records=events.compactMap {entry->AgentReadonlyRecord? in guard let text=entry.event.payload_inline_json.value else {return nil};return try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(text.utf8))}
  precondition(records.count==2 && records.map(\.intent.name)==[.read_notebook,.preview_source])
  var previewText=""
  for record in records {
    let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:record.canonical.message_id),hash:record.blobHash)
    let bytes=try await storage.readBlob(lease,offset:0,count:Int(record.byteLength));try await storage.closeBlob(lease)
    precondition(KernelHashes.data(bytes)==record.blobHash)
    try bytes.write(to:root.appendingPathComponent(record.intent.name == .read_notebook ? "actual-preview-read.json":"actual-preview-result.json"))
    if record.intent.name == .preview_source {
      let envelope=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],data=envelope["data"] as! [String:Any]
      precondition(envelope["ok"] as? Bool==true && data["valid"] as? Bool==true && (data["preview_ref"] as? String)?.hasPrefix("preview-")==true)
      previewText=String(decoding:bytes,as:UTF8.self)
    }
  }
  let third=try JSONSerialization.jsonObject(with:Data(contentsOf:root.appendingPathComponent("actual-native-http-body-3.json"))) as! [String:Any],messages=third["messages"] as! [[String:Any]]
  precondition(messages.filter{$0["role"] as? String=="tool"}.count==2 && messages.contains{$0["tool_call_id"] as? String=="preview-call" && $0["content"] as? String==previewText})
  let (head,history)=try await storage.readNotebookHistory(document:document)
  precondition(head.snapshot_hash==snapshot.snapshot_hash && head.revision.value==0 && history.isEmpty)
  try await wire.save(root.appendingPathComponent("actual-preview-wire.json"))
  try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-preview-ledger.json"))
  await adapter.close();await service.close();try await host.close();try await storage.close()
  let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview)),recovery=try await reopened.sessionRecovery(session,limit:32)
  precondition(recovery.complete && recovery.lastEventSequence==21 && recovery.tools.count==2 && recovery.tools.allSatisfy{$0.phase == .readonly_completed && $0.target==nil && $0.admissionEvent==nil})
  let recoveredData:[String:Any] = ["tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovery.tools)),"requests":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovery.requests)),"last_event_sequence":recovery.lastEventSequence,"complete":recovery.complete,"host_or_helper_created":false]
  try JSONSerialization.data(withJSONObject:recoveredData,options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-preview-recovery.json"))
  try await reopened.close()
}
/// Actual four-turn write path: canonical ACK -> real frozen plan -> SessionToolPort admission
/// and start -> DocCommitPort physical fence/SQLite -> verified original effect -> exact tool data.
private func nativeApplyRoundTrip(package:URL,signatureRoot:URL?,root:URL)async throws {
  let document="00000000-0000-0000-0000-000000000437",session="apply-task"
  var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"真实Agent原子提交",cells:[.init(id:"a",kind:.math,source:"2+2",dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
  snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
  let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview)),info=try await storage.openDocument(document)
  _ = try await storage.initializeSource(snapshot)
  let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
  let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
  let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
  let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock)
  let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value,sourcePort:source)
  let hello=try await service.activate(session:session,grants:[.read_notebook,.preview_source,.apply_notebook_patch],readonly:false,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
  let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"apply-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"读取实际源码，预览后提交原冻结修改，不运行不保存"))])
  let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:root,input:input,intent:intent,task:hello.scope.task_id,session:session,preview:true,apply:true)})
  let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
  await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
  let report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)
  try await wire.save(root.appendingPathComponent("actual-apply-wire.json"))
  try JSONSerialization.data(withJSONObject:["helper_frames":report.frames,"tool_operation_ids":report.toolIntents,"last_phase":report.lastPhase.rawValue],options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-apply-report.json"))
  precondition(report.frames==17 && report.toolIntents.count==3 && report.lastPhase == .awaiting_settlement)
  let events=try await fixtureEvents(storage,session);precondition(events.count==31)
  let records=events.compactMap {entry->AgentEffectRecord? in guard let text=entry.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(text.utf8)),record.kind=="native_effect_result" else {return nil};return record}
  precondition(records.count==1)
  let record=records[0],target=record.target!
  let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:record.canonical.message_id),hash:record.blobHash)
  let bytes=try await storage.readBlob(lease,offset:0,count:Int(record.byteLength));try await storage.closeBlob(lease)
  precondition(KernelHashes.data(bytes)==record.blobHash)
  let result=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],data=result["data"] as! [String:Any]
  precondition(result["ok"] as? Bool==true && result["status"] as? String=="completed" && result["operation_ref"] as? String==target.operationID)
  precondition(data["saved"] as? Bool==false && data["changed_cell_ids"] as? [String]==["a"] && (data["invalidated_cell_ids"] as! [String]).contains("a"))
  let receipt=try await storage.sourceReceipt(document:document,operation:target.operationID)!
  precondition(receipt.receipt.transaction_id.value==data["transaction_ref"] as? String && receipt.receipt.request_hash==target.expectedHash)
  let (head,history)=try await storage.readNotebookHistory(document:document)
  precondition(head.revision.value==1 && head.file.cells[0].source=="3+3" && history.count==1)
  let noRun=try await storage.acceptedKernelHead(document:document);precondition(noRun==nil)
  let fourth=try JSONSerialization.jsonObject(with:Data(contentsOf:root.appendingPathComponent("actual-native-http-body-4.json"))) as! [String:Any],messages=fourth["messages"] as! [[String:Any]]
  precondition(messages.contains{$0["tool_call_id"] as? String=="apply-call" && $0["content"] as? String==String(decoding:bytes,as:UTF8.self)})
  try bytes.write(to:root.appendingPathComponent("actual-apply-result.json"))
  try encoded(receipt).write(to:root.appendingPathComponent("actual-apply-receipt.json"))
  try encoded(head).write(to:root.appendingPathComponent("actual-apply-head.json"))
  try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-apply-ledger.json"))
  await adapter.close();await service.close();await source.close();try await host.close();try await storage.close()
  let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
  let recovery=try await reopened.sessionRecovery(session,limit:32)
  precondition(recovery.complete && recovery.lastEventSequence==31 && recovery.tools.count==3 && recovery.tools.filter{$0.phase == .completed}.count==1)
  let recoveredData:[String:Any] = ["tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovery.tools)),"requests":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovery.requests)),"last_event_sequence":recovery.lastEventSequence,"complete":recovery.complete,"host_or_helper_created":false,"source_committed_not_run":true,"saved":false]
  try JSONSerialization.data(withJSONObject:recoveredData,options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-apply-recovery.json"))
  _ = try await reopened.openDocument(document)
  let actual=try await reopened.sourceReceipt(document:document,operation:target.operationID)
  precondition(actual?.receipt.transaction_id.value==receipt.receipt.transaction_id.value)
  try await reopened.close()
}

private final class PatchBoundaryPause:@unchecked Sendable {
  let reached=DispatchSemaphore(value:0),release=DispatchSemaphore(value:0)
  func stop() {reached.signal();release.wait()}
  func wait()async->Bool {await withCheckedContinuation { c in DispatchQueue.global().async {c.resume(returning:self.reached.wait(timeout:.now()+5) == .success)} }}
}
/// Synthetic private proposals still cross actual C ABI and actual SQLite ACKs. No model/helper or
/// OS input method is involved here; the marked case exercises the real trusted DraftStore fence.
private func nativeApplyBoundaries(root:URL)async throws {
  var reports:[[String:Any]]=[]
  for mode in ["duplicate","marked","source_changed","lost_source_ack","cancel_before_commit"] {
    let path=root.appendingPathComponent(mode),document="00000000-0000-0000-0000-000000000438",session="boundary-"+mode
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"原子提交边界",cells:[.init(id:"a",kind:.math,source:"2+2",dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let (storage,_)=try await StorageService.open(paths:.init(root:path,channel:.preview)),info=try await storage.openDocument(document)
    _ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
    let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock)
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value,sourcePort:source)
    let activated=try await service.activate(session:session,grants:[.read_notebook,.preview_source,.apply_notebook_patch],readonly:false,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
    let hello=try JSONSerialization.jsonObject(with:Data(activated.outbound_json.value!.utf8)) as! [String:Any],scope=activated.scope
    func peer(_ kind:String,_ sequence:UInt64,_ payload:[String:Any],reply:String?=nil)throws->Data {
      var frame=hello;frame["type"]=kind;frame["sequence"]=sequence;frame["message_id"]="boundary-peer-\(sequence)";frame["reply_to"]=reply as Any? ?? NSNull();frame["payload"]=payload
      return try JSONSerialization.data(withJSONObject:frame,options:[.sortedKeys])
    }
    _ = try await service.receive(peer("ready",1,hello["payload"] as! [String:Any],reply:hello["message_id"] as? String))
    var sequence:UInt64=2
    func call(_ name:String,_ args:[String:Any])async throws->NativeAgentToolIntent {
      let n=sequence,event="boundary-proposal-\(n)",turn="boundary-turn-\(n)",id="boundary-call-\(n)"
      let raw=String(decoding:try JSONSerialization.data(withJSONObject:args,options:[.sortedKeys]),as:UTF8.self)
      let canonical:[String:Any] = ["record_type":"canonical_agent_message","message_version":1,"message_id":event,"role":"assistant","agent_turn_id":turn,"task_id":scope.task_id,"parts":[["kind":"tool_call","part_id":id,"tool_call_id":id,"name":name,"arguments_json":raw]]]
      _ = try await service.receive(peer("transcript_event",n,["event_id":event,"source_sequence":n/2,"message":canonical]))
      let mapped=try await service.receive(peer("tool_call",n+1,["agent_turn_id":turn,"tool_call_id":id,"proposal_event_ref":event]));sequence+=2
      return mapped.tool_intent.value!
    }
    let read=try await call("read_notebook",["view":"cells","cell_ids":["a"]])
    let readReply=try await host.agentCommand(.native_agent_read_tool(.init(type:.agent_read_tool,scope:scope,operation_id:read.operation_id)))
    let readData=(try JSONSerialization.jsonObject(with:Data(readReply.readonly_result.value!.result_json.utf8)) as! [String:Any])["data"] as! [String:Any]
    let cell=(readData["cells"] as! [[String:Any]])[0]
    let preview=try await call("preview_source",["snapshot_ref":readData["snapshot_ref"]!,"input":["kind":"patch","operations":[["type":"update_cell","target":["cell_id":"a"],"expected_source_hash":cell["source_hash"]!,"source":"3+3"]]]])
    let previewReply=try await host.agentCommand(.native_agent_read_tool(.init(type:.agent_read_tool,scope:scope,operation_id:preview.operation_id)))
    let previewData=(try JSONSerialization.jsonObject(with:Data(previewReply.readonly_result.value!.result_json.utf8)) as! [String:Any])["data"] as! [String:Any]
    let intent=try await call("apply_notebook_patch",["preview_ref":previewData["preview_ref"]!])
    if mode=="source_changed" {
      try await MainActor.run {try drafts.attachEditor(cell:"a",generation:1);try drafts.edited(cell:"a",generation:1,source:"5+5",hasMarkedText:false)}
      _ = try await source.synchronizeDrafts()
    }
    let prepared=try await host.sourceCommand(.source_begin_agent(.init(type:.source_begin_agent,operation_id:intent.operation_id)))
    let ledger=SessionLedgerPort(storage:storage,session:session),tool=SessionToolPort(storage:storage,ledger:ledger)
    var target:SessionOperationTarget?,admitted:String?,outcome:String?,failure:NativeAgentEffectToolFailureCode?
    var unknownBeforeReadback=false
    if let plan=prepared.plan.value {
      let actual=SessionOperationTarget(documentID:document,storeID:info.identity.storeID,operationID:plan.commit.operation_id,kind:.source,expectedHash:plan.commit.request_hash)
      let admissionID="admit-"+intent.operation_id,resultID="result-"+intent.operation_id
      target=actual;admitted=admissionID
      let admission=SessionToolAdmission(codecVersion:1,callID:intent.tool_call_id,proposalEvent:intent.proposal_event_ref,toolName:intent.name.rawValue,argumentsHash:intent.arguments_hash,target:actual)
      if mode=="marked" {try await MainActor.run {try drafts.attachEditor(cell:"a",generation:1);try drafts.edited(cell:"a",generation:1,source:"2+2",hasMarkedText:true)}}
      let pause=PatchBoundaryPause()
      let faults=StorageFaults { point in
        if mode=="lost_source_ack" && point=="source_committed" {throw StorageError.system(EIO,"actual-agent-source-ack-lost")}
        if mode=="cancel_before_commit" && point=="before_source_commit" {pause.stop()}
      }
      let execution=Task {try await tool.perform(admission:admission,admissionID:admissionID,startedID:"started-"+intent.operation_id,resultID:resultID,turn:intent.agent_turn_id,task:scope.task_id,execute:{
        let state=try await source.applyPrepared(prepared,faults:faults)
        guard state.phase == .completed else {throw CommitPortError.cancelled}
      })}
      if mode=="cancel_before_commit" {
        let reached=await pause.wait();precondition(reached)
        _ = try await host.cancel(operation:plan.commit.operation_id)
        pause.release.signal()
      }
      do {_ = try await execution.value;outcome=resultID}
      catch {
        if mode=="duplicate" {throw error}
        if mode=="marked" {guard case CommitPortError.editingBusy=error else {throw error};failure = .editing_busy}
        else if mode=="lost_source_ack" {guard case CommitPortError.unknownOutcome=error else {throw error};unknownBeforeReadback=true}
        else if mode=="cancel_before_commit" {failure = .preview_mismatch}
      }
    } else {precondition(mode=="source_changed")}
    let native=try await host.agentCommand(.native_agent_effect_tool(.init(type:.agent_effect_tool,scope:scope,operation_id:intent.operation_id,failure_code:.init(failure))))
    let result=native.effect_result.value!,bytes=Data(result.result_json.utf8),event="effect-"+intent.operation_id,owner=BlobOwner(kind:.context,id:event)
    let partOutcome=try JSONDecoder().decode(AgentToolResultPartOutcome.self,from:JSONEncoder().encode(result.outcome))
    let canonical=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:event,role:.tool,agent_turn_id:.init(intent.agent_turn_id),task_id:.init(scope.task_id),parts:[.agent_tool_result_part(.init(kind:.tool_result,part_id:event,tool_call_id:intent.tool_call_id,operation_ref:result.operation_ref,result_ref:.init(result.result_ref),outcome:partOutcome))])
    let record=AgentEffectRecord(codecVersion:1,kind:"native_effect_result",intent:intent,resultRef:result.result_ref,blobHash:result.result_hash,byteLength:UInt64(bytes.count),canonical:canonical,target:target,admissionEvent:admitted,outcomeEvent:outcome)
    if mode=="duplicate" {
      let absent=AgentEffectRecord(codecVersion:1,kind:record.kind,intent:intent,resultRef:record.resultRef,blobHash:String(repeating:"a",count:64),byteLength:record.byteLength,canonical:canonical,target:target,admissionEvent:admitted,outcomeEvent:outcome)
      do {_ = try await ledger.record(absent,id:event,kind:.recovery,parent:intent.proposal_event_ref,turn:intent.agent_turn_id,task:scope.task_id,target:target);fatalError("effect record accepted missing referenced Blob")}catch BlobError.missing { }
      var forged=intent;forged.arguments_hash=String(repeating:"0",count:64)
      let wrong=AgentEffectRecord(codecVersion:1,kind:record.kind,intent:forged,resultRef:record.resultRef,blobHash:record.blobHash,byteLength:record.byteLength,canonical:canonical,target:target,admissionEvent:admitted,outcomeEvent:outcome)
      do {_ = try await ledger.record(wrong,id:event,kind:.recovery,parent:intent.proposal_event_ref,turn:intent.agent_turn_id,task:scope.task_id,target:target);fatalError("effect record accepted changed original argument hash")}catch SessionLedgerError.invalidTransition { }
      let noReceipt=AgentEffectRecord(codecVersion:1,kind:record.kind,intent:intent,resultRef:record.resultRef,blobHash:record.blobHash,byteLength:record.byteLength,canonical:canonical,target:target,admissionEvent:admitted,outcomeEvent:nil)
      do {_ = try await ledger.record(noReceipt,id:event,kind:.recovery,parent:intent.proposal_event_ref,turn:intent.agent_turn_id,task:scope.task_id,target:target);fatalError("effect success accepted without actual operation outcome event")}catch SessionLedgerError.invalidTransition { }
    }
    let blob=try await storage.publishBlob(data:bytes)
    _ = try await ledger.record(record,id:event,kind:.recovery,parent:intent.proposal_event_ref,turn:intent.agent_turn_id,task:scope.task_id,target:target,resource:blob,resourceOwner:owner);try await storage.releaseBlob(blob)
    let stored=try await storage.sessionEvent(session,event:event)!
    do {_ = try await host.agentCommand(.native_agent_confirm_tool(.init(type:.agent_confirm_tool,scope:scope,operation_id:intent.operation_id,result_ref:result.result_ref,result_hash:String(repeating:"0",count:64),event_id:event,event_sequence:try HostSerial(stored.receipt.eventSequence),receipt_hash:stored.receipt.recordHash)));fatalError("native effect confirmation accepted changed result hash")}catch NativeHostClientError.rejected { }
    _ = try await host.agentCommand(.native_agent_confirm_tool(.init(type:.agent_confirm_tool,scope:scope,operation_id:intent.operation_id,result_ref:result.result_ref,result_hash:result.result_hash,event_id:event,event_sequence:try HostSerial(stored.receipt.eventSequence),receipt_hash:stored.receipt.recordHash)))
    let body=try JSONSerialization.jsonObject(with:bytes) as! [String:Any]
    if mode=="marked" {precondition(body["status"] as? String=="failed" && (body["error"] as! [String:Any])["code"] as? String=="EDITING_BUSY")}
    if mode=="source_changed" {precondition(body["status"] as? String=="failed" && (body["error"] as! [String:Any])["code"] as? String=="STALE_SNAPSHOT")}
    if mode=="cancel_before_commit" {precondition(body["status"] as? String=="cancelled")}
    if mode=="lost_source_ack" {
      precondition(unknownBeforeReadback && body["status"] as? String=="unknown")
      let fixed=try await source.reconcile(target!.operationID);precondition(fixed.phase == .completed)
      let plan=prepared.plan.value!,actual=target!
      let admission=SessionToolAdmission(codecVersion:1,callID:intent.tool_call_id,proposalEvent:intent.proposal_event_ref,toolName:intent.name.rawValue,argumentsHash:intent.arguments_hash,target:actual)
      let confirmed=try await tool.reconcile(admission:admission,admissionID:admitted!,resultID:"result-"+intent.operation_id,turn:intent.agent_turn_id,task:scope.task_id)
      precondition(confirmed != nil && fixed.receipt.value!.receipt.transaction_id.value==plan.commit.transaction_id)
    }
    if mode=="duplicate" || mode=="lost_source_ack" {
      let again=try await source.applyPrepared(prepared);precondition(again.phase == .completed && again.operation_id==target!.operationID)
    }
    let (head,history)=try await storage.readNotebookHistory(document:document),kernel=try await storage.acceptedKernelHead(document:document)
    precondition(kernel==nil)
    if mode=="marked" || mode=="cancel_before_commit" {precondition(head.revision.value==0 && head.file.cells[0].source=="2+2" && history.isEmpty)}
    else {precondition(head.revision.value==1 && history.count==1 && head.file.cells[0].source==(mode=="source_changed" ? "5+5":"3+3"))}
    try bytes.write(to:path.appendingPathComponent("actual-boundary-result.json"))
    try encoded(head).write(to:path.appendingPathComponent("actual-boundary-head.json"))
    let events=try await fixtureEvents(storage,session)
    try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-boundary-ledger.json"))
    await service.close();await source.close();try await host.close();try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:path,channel:.preview)),recovery=try await reopened.sessionRecovery(session,limit:32)
    let final=recovery.tools.first{$0.callID==intent.tool_call_id}!
    let expected:RecoveredToolPhase=mode=="marked" || mode=="source_changed" ? .failed : mode=="cancel_before_commit" ? .cancelled : .completed
    precondition(recovery.complete && final.phase==expected)
    let fact:[String:Any] = ["mode":mode,"status":body["status"]!,"recovered_phase":final.phase.rawValue,"confirmed_revision":head.revision.value,"source":head.file.cells[0].source,"transaction_count":history.count,"no_kernel_checkpoint":true,"unknown_then_original_receipt_only":unknownBeforeReadback,"storage_only_recovery":true,"native_wrong_result_hash_rejected":true,"missing_blob_changed_args_absent_actual_effect_rejected":mode=="duplicate"]
    reports.append(fact);try JSONSerialization.data(withJSONObject:fact,options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-boundary-recovery.json"))
    try await reopened.close()
  }
  try JSONSerialization.data(withJSONObject:reports,options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-boundaries.json"))
}
@main struct AgentPackageFixtures {
  static func main() async throws {
    let package=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true),root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    let document="00000000-0000-0000-0000-000000000432"
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"包内Pi真实任务",cells:[.init(id:"a",kind:.math,source:"2+2",dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));let info=try await storage.openDocument(document);_ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open();let opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
    let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
    let rename=try await source.preview(.preview_patch_input(.init(kind:.patch,operations:[.preview_rename_notebook(.init(type:.rename_notebook,title:"持久事务与真实输出"))])))
    let renamed=try await source.apply(rename.preview_ref.value!)
    precondition(renamed.phase == .completed)
    let actualHistoryTransaction=renamed.receipt.value!.receipt.transaction_id.value!
    _ = try await kernel.runCell("a",operation:"actual-output-before-read")
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value)
    let verified=try await AgentProcess.verified(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil)
    let hello=try await service.activate(session:"production-task",grants:[.read_notebook],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
    let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"user-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"真实Pi，合成模型，读取实际笔记本🙂"))])
    let task=hello.scope.task_id
    let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:root,input:input,intent:intent,task:task)})
    let port=adapter.port()
    let pipe=try await verified.launch(),wire=WireCapture();await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)};let report=try await service.run(pipe:pipe,hello:hello,modelPort:port,input:input)
    try await wire.save(root.appendingPathComponent("actual-production-wire.json"))
    precondition(report.frames==9 && report.lastPhase == .awaiting_settlement && report.toolIntents.count==1)
    let events=try await storage.sessionEvents("production-task");precondition(events.count==14 && events.filter{$0.event.event_kind == .task_transition}.count==8 && events.filter{$0.event.event_kind == .assistant_end}.count==2)
    let canonical=try JSONDecoder().decode(SessionCanonicalAssistant.self,from:Data(events.first{$0.event.event_kind == .assistant_end}!.event.payload_inline_json.value!.utf8))
    precondition(canonical.calls[0].arguments==Data("{ \"view\" : \"outline\", \"include_outputs\" : true }".utf8))
    let after=try await kernel.runCell("a",operation:"manual-after-helper-task");precondition(after.result_id.value != nil)
    let client=await MainActor.run {ResultClient(host:host)};let manifest=try await client.manifest(after.result_id.value!)
    try client.select(manifest.bindings.last!)
    let full=try await client.completeSource(format:.input_form,path:[])!;precondition(full=="4")
    // An actual SIGKILL during the next model preparation must leave the notebook/kernel live.
    let crashHello=try await service.activate(session:"production-crash",grants:[],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1"),crashPipe=try await verified.launch()
    var crashInput=input;crashInput.task_id = .init(crashHello.scope.task_id)
    let crashPort=AgentNativeModelPort(prepare:{_ in await crashPipe.terminateImmediately();throw AgentSessionError.inactive},stream:{_ in AsyncThrowingStream {$0.finish()}})
    do {_ = try await service.run(pipe:crashPipe,hello:crashHello,modelPort:crashPort,input:crashInput);fatalError("killed helper reported settled")} catch { }
    let intact=try await kernel.runCell("a",operation:"manual-after-sigkill");precondition(intact.result_id.value != nil)
    let intactManifest=try await client.manifest(intact.result_id.value!);try client.select(intactManifest.bindings.last!);let again=try await client.completeSource(format:.input_form,path:[])!;precondition(again=="4")
    do {try await pipe.write(Data("{\"sequence\":99}\n".utf8));fatalError("late stdin revived process")} catch { }
    let cancelHello=try await service.activate(session:"production-cancel",grants:[],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1"),cancelPipe=try await verified.launch(),heldModel=ModelHold()
    var cancelInput=input;cancelInput.task_id = .init(cancelHello.scope.task_id)
    let heldPort=AgentNativeModelPort(prepare:{_ in .init(context:"held-context",model:"held-model")},stream:{_ in await heldModel.stream()})
    let cancellation=Task {try await service.run(pipe:cancelPipe,hello:cancelHello,modelPort:heldPort,input:cancelInput)}
    try await heldModel.wait();let stopped=try await service.cancel();precondition(stopped.phase == .cancelling)
    let cancellationReport=try await cancellation.value;precondition(cancellationReport.lastPhase == .awaiting_settlement)
    await heldModel.late(.init(agent_turn_id:"old-turn",request_id:"old-request",byte_offset:try HostSerial(0),text:"late",phase:.delta,canonical_message:.init(nil)))
    let cancelEvents=try await storage.sessionEvents("production-cancel");precondition(cancelEvents.count==1 && cancelEvents[0].event.event_kind == .user_message)
    let absentHello=try await service.activate(session:"production-no-model",grants:[],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1"),absentPipe=try await verified.launch()
    var absentInput=input;absentInput.task_id = .init(absentHello.scope.task_id)
    let absent=try await service.run(pipe:absentPipe,hello:absentHello,input:absentInput)
    precondition(absent.lastPhase == .awaiting_settlement && absent.loopReason == .failed)
    let absentEvents=try await storage.sessionEvents("production-no-model");precondition(absentEvents.count==1 && absentEvents[0].event.event_kind == .user_message)
    // Move a required real resource and restore it; closure verification must fail before launch.
    let schema=package.appendingPathComponent("Agent/docs/design/native-agent.schema.json"),held=schema.appendingPathExtension("held")
    try FileManager.default.moveItem(at:schema,to:held)
    do {_ = try await AgentProcess.verified(package:package);fatalError("missing runtime resource accepted")} catch { }
    try FileManager.default.moveItem(at:held,to:schema)
    let manifestURL=package.appendingPathComponent("HelperManifest.json"),original=try Data(contentsOf:manifestURL)
    var wrong=try JSONSerialization.jsonObject(with:original) as! [String:Any];wrong["architecture"]="x86_64"
    try JSONSerialization.data(withJSONObject:wrong).write(to:manifestURL)
    do {_ = try await AgentProcess.verified(package:package);fatalError("wrong architecture accepted")} catch AgentProcessError.architecture { }
    try original.write(to:manifestURL)
    // Wrong real Mach-O bytes with a matching manifest hash still fail architecture admission.
    let node=package.appendingPathComponent("Node/node"),backup=root.appendingPathComponent("saved-node")
    try FileManager.default.copyItem(at:node,to:backup)
    let x86=Data([0xcf,0xfa,0xed,0xfe,7,0,0,1]);try x86.write(to:node)
    var actualWrong=try JSONSerialization.jsonObject(with:original) as! [String:Any],entries=actualWrong["files"] as! [[String:Any]]
    let index=entries.firstIndex(where:{$0["path"] as? String=="Node/node"})!
    entries[index]["bytes"]=8;entries[index]["sha256"]=SHA256.hash(data:x86).map{String(format:"%02x",$0)}.joined();actualWrong["files"]=entries
    try JSONSerialization.data(withJSONObject:actualWrong).write(to:manifestURL)
    do {_ = try await AgentProcess.verified(package:package);fatalError("wrong real Mach-O architecture accepted")} catch AgentProcessError.architecture { }
    try FileManager.default.removeItem(at:node);try FileManager.default.moveItem(at:backup,to:node);try original.write(to:manifestURL)
    let actualEvents=try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))}
    try JSONSerialization.data(withJSONObject:actualEvents,options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-production-ledger.json"))
    await adapter.close();await service.close();await source.close();try await host.close();try await storage.close()
    // Reopen only storage. Historical data/ref retention never recreates a host/task/helper grant.
    let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let recoveredEvents=try await reopened.sessionEvents("production-task")
    precondition(recoveredEvents.count==events.count)
    let recovered=try await reopened.sessionRecovery("production-task")
    precondition(recovered.tools.count==1 && recovered.tools[0].phase == .readonly_completed && recovered.tools[0].target==nil && recovered.tools[0].admissionEvent==nil)
    let recovery:[String:Any] = ["tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovered.tools)),"requests":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovered.requests)),"last_event_sequence":recovered.lastEventSequence,"complete":recovered.complete,"host_or_helper_created":false]
    try JSONSerialization.data(withJSONObject:recovery,options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-readonly-recovery.json"))
    let recoveredReads=recoveredEvents.compactMap {entry->AgentReadonlyRecord? in guard let text=entry.event.payload_inline_json.value else {return nil};return try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(text.utf8))}
    precondition(recoveredReads.count==1)
    for record in recoveredReads {
      let reader=try await reopened.openReferencedBlob(owner:.init(kind:.context,id:record.canonical.message_id),hash:record.blobHash)
      let bytes=try await reopened.readBlob(reader,offset:0,count:Int(record.byteLength));try await reopened.closeBlob(reader)
      precondition(KernelHashes.data(bytes)==record.blobHash)
      let envelope=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],data=envelope["data"] as! [String:Any]
      precondition((data["cells"] as! [[String:Any]])[0]["cell_id"] as? String=="a")
      let originalCells=data["cells"] as! [[String:Any]]
      precondition((originalCells[0]["result_ref"] as? String)?.hasPrefix("result-")==true)
      precondition((data["recent_transaction_refs"] as! [String]).contains(actualHistoryTransaction))
      try bytes.write(to:root.appendingPathComponent("actual-readonly-result.json"))
      let negativeSession="readonly-negative";_ = try await reopened.createAgentSession(negativeSession)
      let negativeLedger=SessionLedgerPort(storage:reopened,session:negativeSession)
      let assistant=try JSONDecoder().decode(SessionCanonicalAssistant.self,from:Data(events.first{$0.event.event_kind == .assistant_end}!.event.payload_inline_json.value!.utf8))
      _ = try await negativeLedger.record(assistant,id:record.intent.proposal_event_ref,kind:.assistant_end,turn:record.intent.agent_turn_id,task:record.canonical.task_id.value)
      let missing=AgentReadonlyRecord(codecVersion:1,kind:record.kind,intent:record.intent,resultRef:record.resultRef,blobHash:String(repeating:"a",count:64),byteLength:record.byteLength,canonical:record.canonical)
      do {_ = try await negativeLedger.record(missing,id:record.canonical.message_id,kind:.recovery,parent:record.intent.proposal_event_ref,turn:record.intent.agent_turn_id,task:record.canonical.task_id.value);fatalError("readonly record accepted absent Blob reference")}catch BlobError.missing { }
      var changed=record.intent;changed.arguments_hash=String(repeating:"0",count:64)
      let wrong=AgentReadonlyRecord(codecVersion:1,kind:record.kind,intent:changed,resultRef:record.resultRef,blobHash:record.blobHash,byteLength:record.byteLength,canonical:record.canonical)
      do {_ = try await negativeLedger.record(wrong,id:record.canonical.message_id,kind:.recovery,parent:record.intent.proposal_event_ref,turn:record.intent.agent_turn_id,task:record.canonical.task_id.value);fatalError("readonly record accepted changed proposal arguments")}catch SessionLedgerError.invalidTransition { }
      let rejected=try await reopened.sessionEvents(negativeSession);precondition(rejected.count==1)
    }
    let unchanged=try await reopened.sessionEvents("production-task");precondition(unchanged.count==recoveredEvents.count)
    _ = try await reopened.openDocument(document)
    let (historyHead,historyEntries)=try await reopened.readNotebookHistory(document:document)
    let newHost=try await NativeHostClient.open()
    let newOpen=try await newHost.sourceCommand(.source_open(.init(type:.source_open,snapshot:historyHead,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let newService=AgentSessionService(host:newHost,storage:reopened,document:document,generation:newOpen.document_generation.value)
    let activation=try await newService.activate(session:"history-after-restart",grants:[.read_notebook],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
    let originalHello=try JSONSerialization.jsonObject(with:Data(activation.outbound_json.value!.utf8)) as! [String:Any]
    func peer(_ kind:String,_ sequence:UInt64,_ payload:[String:Any],reply:String?=nil)throws->Data {
      var frame=originalHello;frame["type"]=kind;frame["sequence"]=sequence;frame["message_id"]="restart-peer-\(sequence)";frame["reply_to"]=reply as Any? ?? NSNull();frame["payload"]=payload
      return try JSONSerialization.data(withJSONObject:frame,options:[.sortedKeys])
    }
    _ = try await newService.receive(peer("ready",1,originalHello["payload"] as! [String:Any],reply:originalHello["message_id"] as? String))
    let historyCanonical:[String:Any] = ["record_type":"canonical_agent_message","message_version":1,"message_id":"history-proposal","role":"assistant","agent_turn_id":"history-turn","task_id":activation.scope.task_id,"parts":[["kind":"tool_call","part_id":"history-part","tool_call_id":"history-call","name":"read_notebook","arguments_json":"{}"]]]
    _ = try await newService.receive(peer("transcript_event",2,["event_id":"history-event","source_sequence":1,"message":historyCanonical]))
    let mapped=try await newService.receive(peer("tool_call",3,["agent_turn_id":"history-turn","tool_call_id":"history-call","proposal_event_ref":"history-event"]))
    do {_ = try await newHost.agentCommand(.native_agent_bind_history(.init(type:.agent_bind_history,scope:activation.scope,expected_snapshot_hash:String(repeating:"0",count:64),transaction_refs:historyEntries.map(\.transaction))));fatalError("stale history head accepted")}catch NativeHostClientError.rejected { }
    _ = try await newHost.agentCommand(.native_agent_bind_history(.init(type:.agent_bind_history,scope:activation.scope,expected_snapshot_hash:historyHead.snapshot_hash,transaction_refs:historyEntries.map(\.transaction))))
    let fresh=try await newHost.agentCommand(.native_agent_read_tool(.init(type:.agent_read_tool,scope:activation.scope,operation_id:mapped.tool_intent.value!.operation_id)))
    let freshJSON=fresh.readonly_result.value!.result_json,freshObject=try JSONSerialization.jsonObject(with:Data(freshJSON.utf8)) as! [String:Any]
    precondition(((freshObject["data"] as! [String:Any])["recent_transaction_refs"] as! [String]).contains(actualHistoryTransaction))
    try Data(freshJSON.utf8).write(to:root.appendingPathComponent("actual-new-host-persisted-history.json"))
    await newService.close();try await newHost.close()
    try await reopened.close()
    try await nativePreviewRoundTrip(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("preview-task"))
    try await nativeApplyRoundTrip(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("apply-task"))
    try await nativeApplyBoundaries(root:root.appendingPathComponent("apply-boundaries"))
    print(String(decoding:try JSONSerialization.data(withJSONObject:["production_core":"1.0.4","bundled_node":"26.11.1","actual_private_frames":report.frames,"canonical_events":events.count,"model_transport":"actual_urlsession_rust_cabi_durable_request_ledger","tool_outcome":"completed_actual_readonly_no_mutation","manual_after_sigkill":"4","system_node_fallback":false,"PATH":"empty","wrong_architecture":"rejected","missing_resource":"rejected","late_stdin":"rejected","unconfigured_model":"no_fake_message_or_effect","model_cancel_and_late_delta":"rejected","actual_macho_architecture":"rejected","evidence_root":root.path],options:[.sortedKeys]),as:UTF8.self))
  }
}
