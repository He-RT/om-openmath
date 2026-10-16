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
private actor ActionObservation {
  var running=Set<String>(),returned=Set<String>()
  func accept(_ messages:[AssistantMessage]) {
    for action in messages.flatMap(\.actions) {
      if action.status == .running {running.insert(action.callID)} else {returned.insert(action.callID)}
    }
  }
  func paired(_ ids:Set<String>)->Bool {ids.isSubset(of:running) && ids.isSubset(of:returned)}
}

/// The actual native transport/C ABI/Library producer; only provider wire and route are synthetic.
private func nativeProviderTurn(host:NativeHostClient,storage:StorageService,root:URL,input:AgentCanonicalMessage,intent:NativeAgentRequestIntent,task:String,session:String="production-task",preview:Bool=false,apply:Bool=false,inspect:String?=nil,scratchSource:String?=nil,scratchDialect:Dialect = .wolfram,runCells:[String]?=nil,runScope:String="required",runContinue:Bool=false,runInspectCell:String?=nil,operationRef:String?=nil,undoTransactions:[String]?=nil)async throws->ProviderPreparedTurn {
  let stored=try await fixtureEvents(storage,session)
  let readonly=stored.compactMap {event->AgentReadonlyRecord? in guard let raw=event.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(raw.utf8)),record.kind=="native_readonly_result" else {return nil};return record}
  let effects=stored.compactMap {event->AgentEffectRecord? in guard let raw=event.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(raw.utf8)),record.kind=="native_effect_result" else {return nil};return record}
  let count=readonly.count+effects.count,second = count>0,finishing = undoTransactions != nil ? count>=2:operationRef != nil ? count>=(inspect==nil ? 2:3):runCells != nil ? count>=(inspect==nil ? 2:3):second && (!preview || readonly.count==2) && (!apply || effects.count==1) && (inspect==nil || readonly.count==(scratchSource==nil ? 2:3))
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
    precondition(data["ok"] as? Bool != nil)
    messages.append(["canonical":try JSONSerialization.jsonObject(with:encoded(result.canonical)),"replay":NSNull()])
    results.append(["message_id":result.canonical.message_id,"tool_call_id":result.intent.tool_call_id,"text":String(decoding:bytes,as:UTF8.self),"is_error":data["ok"] as? Bool==false])
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
  if let inspect,readonly.count==1,scratchSource==nil,runCells==nil,operationRef==nil {
    let envelope=try JSONSerialization.jsonObject(with:Data((results[0]["text"] as! String).utf8)) as! [String:Any],data=envelope["data"] as! [String:Any],cell=(data["cells"] as! [[String:Any]])[0]
    precondition(cell["status"] as? String=="Done");tool="inspect_result"
    arguments=String(decoding:try JSONSerialization.data(withJSONObject:["result_ref":cell["result_ref"]!,"part":inspect,"limit":20],options:[.sortedKeys]),as:UTF8.self)
  }
  if let scratchSource,readonly.count==1 {
    let envelope=try JSONSerialization.jsonObject(with:Data((results[0]["text"] as! String).utf8)) as! [String:Any],data=envelope["data"] as! [String:Any]
    tool="evaluate_scratch";arguments=String(decoding:try JSONSerialization.data(withJSONObject:["snapshot_ref":data["snapshot_ref"]!,"source":scratchSource,"dialect":scratchDialect.rawValue,"use_notebook_definitions":true],options:[.sortedKeys]),as:UTF8.self)
  }
  if scratchSource != nil,readonly.count==2 {
    let envelope=try JSONSerialization.jsonObject(with:Data((results[1]["text"] as! String).utf8)) as! [String:Any],data=envelope["data"] as! [String:Any]
    precondition(data["notebook_modified"] as? Bool==false && ["done","error"].contains(data["evaluation_status"] as? String ?? ""))
    tool="inspect_result";arguments=String(decoding:try JSONSerialization.data(withJSONObject:["result_ref":data["result_ref"]!,"part":inspect!,"limit":20],options:[.sortedKeys]),as:UTF8.self)
  }
  if let undoTransactions,count==1 {
    let envelope=try JSONSerialization.jsonObject(with:Data((results[0]["text"] as! String).utf8)) as! [String:Any],data=envelope["data"] as! [String:Any]
    tool="undo_transaction";arguments=String(decoding:try JSONSerialization.data(withJSONObject:["snapshot_ref":data["snapshot_ref"]!,"transaction_refs":undoTransactions],options:[.sortedKeys]),as:UTF8.self)
  }
  if operationRef != nil,count==2,let inspect {
    let envelope=try JSONSerialization.jsonObject(with:Data((results[1]["text"] as! String).utf8)) as! [String:Any],data=envelope["data"] as! [String:Any],reference=(data["result_refs"] as! [String]).first!
    tool="inspect_result";arguments=String(decoding:try JSONSerialization.data(withJSONObject:["result_ref":reference,"part":inspect,"limit":20],options:[.sortedKeys]),as:UTF8.self)
  }
  if let operationRef,count==1 {tool="get_operation_status";arguments=String(decoding:try JSONSerialization.data(withJSONObject:["operation_ref":operationRef],options:[.sortedKeys]),as:UTF8.self)}
  if let runCells,count==1 {
    let envelope=try JSONSerialization.jsonObject(with:Data((results[0]["text"] as! String).utf8)) as! [String:Any],data=envelope["data"] as! [String:Any]
    tool="run_cells";arguments=String(decoding:try JSONSerialization.data(withJSONObject:["snapshot_ref":data["snapshot_ref"]!,"cell_ids":runCells,"scope":runScope,"continue_on_error":runContinue],options:[.sortedKeys]),as:UTF8.self)
  }
  if let runCells,count==2,inspect != nil {
    let envelope=try JSONSerialization.jsonObject(with:Data((results[1]["text"] as! String).utf8)) as! [String:Any],data=envelope["data"] as! [String:Any],cells=data["cells"] as! [[String:Any]]
    let cell=cells.first {$0["cell_id"] as? String==(runInspectCell ?? runCells.last!)}!
    precondition(cell["result_ref"] as? String != nil,"actual run result reference missing")
    tool="inspect_result";arguments=String(decoding:try JSONSerialization.data(withJSONObject:["result_ref":cell["result_ref"]!,"part":inspect ?? "value","limit":20],options:[.sortedKeys]),as:UTF8.self)
  }
  let callID=runCells != nil ? (count==2 ? "inspect-run-call":second ? "run-call":"provider-call"):scratchSource != nil ? (count==2 ? "inspect-scratch-call":second ? "scratch-call":"provider-call"):(count==2 ? "apply-call":second ? "preview-call":"provider-call")
  let delta:[String:Any] = finishing ? ["role":"assistant","content":"已读取真实笔记本🙂"]:["role":"assistant","content":"中文🙂","tool_calls":[["index":0,"id":callID,"type":"function","function":["name":tool,"arguments":arguments]]]]
  let chunk:[String:Any] = ["id":"provider-response-\(count+1)","model":"synthetic-served-alias","choices":[["index":0,"delta":delta,"finish_reason":finishing ? "stop":"tool_calls"]]]
  let sse=Data(("data: "+String(decoding:try JSONSerialization.data(withJSONObject:chunk),as:UTF8.self)+"\n\ndata: [DONE]\n\n").utf8)
  var tools:[[String:Any]] = [["name":"read_notebook","description":"Read actual confirmed source","parameters":["type":"object"]]]
  if preview {tools.append(["name":"preview_source","description":"Inspect actual frozen source plan","parameters":["type":"object"]])}
  if apply {tools.append(["name":"apply_notebook_patch","description":"Commit the original frozen source plan","parameters":["type":"object"]])}
  if inspect != nil {tools.append(["name":"inspect_result","description":"Inspect original retained mathematical output","parameters":["type":"object"]])}
  if undoTransactions != nil {tools.append(["name":"undo_transaction","description":"Undo original durable transactions against the current snapshot","parameters":["type":"object"]])}
  if operationRef != nil {tools.append(["name":"get_operation_status","description":"Query the original durable operation without replay","parameters":["type":"object"]])}
  if runCells != nil {tools.append(["name":"run_cells","description":"Run actual selected cells and prerequisites","parameters":["type":"object"]])}
  if scratchSource != nil {tools.append(["name":"evaluate_scratch","description":"Evaluate an independent writable scratch without notebook changes","parameters":["type":"object"]])}
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
/// Real Agent read -> main execution -> optional inspection -> final, then storage-only recovery.
private func nativeRunRoundTrip(package:URL,signatureRoot:URL?,root:URL)async throws {
  let chain:[NativeSourceCell]=[.init(id:"answer",kind:.math,source:"b+1",dialect:.modern),.init(id:"unrelated",kind:.math,source:"seed_random(99);rand()",dialect:.modern),.init(id:"b",kind:.math,source:"let b=a+1;b",dialect:.modern),.init(id:"a",kind:.math,source:"let a=2;a",dialect:.modern)]
  let errorCells:[NativeSourceCell]=[.init(id:"good",kind:.math,source:"2",dialect:.modern),.init(id:"boom",kind:.math,source:"seed=99;\nloop:=loop\nloop\nnever=5",dialect:.wolfram),.init(id:"independent",kind:.math,source:"7",dialect:.modern)]
  let cases:[(name:String,cells:[NativeSourceCell],targets:[String],scope:String,keepGoing:Bool,allowed:[String]?,parent:[String],status:String,executed:[String],inspectCell:String?,part:String?,scalar:String?)]=[
    ("required",chain,["answer"],"required",false,nil,[],"completed",["a","b","answer"],"answer","value","4"),
    ("affected",chain,["a"],"affected",false,nil,["a","b","answer"],"completed",["a","b","answer"],"answer","value","4"),
    ("selected-refused",chain,["answer"],"selected",false,nil,[],"failed",[],nil,nil,nil),
    ("selected-partial",chain,["answer","unrelated"],"selected",true,nil,[],"partial",["unrelated"],"unrelated","summary",nil),
    ("allowed-refused",chain,["answer"],"required",false,["answer"],[],"failed",[],nil,nil,nil),
    ("error-stop",errorCells,["good","boom","independent"],"selected",false,nil,[],"partial",["good","boom"],"boom","summary",nil),
    ("error-continue",errorCells,["good","boom","independent"],"selected",true,nil,[],"partial",["good","boom","independent"],"independent","value","7"),
    ("solve",[.init(id:"solve",kind:.math,source:"solve(x^2-5*x+6==0,x)",dialect:.modern)],["solve"],"selected",false,nil,[],"completed",["solve"],"solve","summary",nil),
    ("ode",[.init(id:"ode",kind:.math,source:"let curve=ode(fn(t,y)=>y,initial:1,t:0..1);curve",dialect:.modern)],["ode"],"selected",false,nil,[],"completed",["ode"],"ode","value",nil),
    ("scene",[.init(id:"scene",kind:.math,source:"plot(x^2-y^2,x:-1..1,y:-1..1,view:\"surface\",mesh_points:8)",dialect:.modern)],["scene"],"selected",false,nil,[],"completed",["scene"],"scene","geometry",nil),
    ("plot",[.init(id:"plot",kind:.math,source:"plot([sin(x),cos(x)],x:-pi..pi)",dialect:.modern)],["plot"],"selected",false,nil,[],"completed",["plot"],"plot","geometry",nil)
  ]
  for spec in cases {
    let path=root.appendingPathComponent(spec.name),document="00000000-0000-0000-0000-000000000441",session="actual-run-task"
    let (storage,_)=try await StorageService.open(paths:.init(root:path.appendingPathComponent("store"),channel:.preview))
    let info=try await storage.openDocument(document)
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"实际运行："+spec.name,cells:spec.cells),cell_revisions:spec.cells.map{.init(cell_id:$0.id,revision:try! HostSerial(0))},snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot);_ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
    let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
    for cell in spec.parent {_ = try await kernel.runCell(cell,operation:"actual-before-agent-"+cell)}
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value,sourcePort:source,kernelPort:kernel,toolFailure:{tool,error in try? Data((tool.rawValue+":"+String(reflecting:error)).utf8).write(to:path.appendingPathComponent("actual-run-dispatch-error.txt"))})
    let hello=try await service.activate(session:session,grants:[.read_notebook,.run_cells,.inspect_result],readonly:false,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1",allowedRunCells:spec.allowed)
    let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"run-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"验证原笔记本运行及真实回执："+spec.name))])
    let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:path,input:input,intent:intent,task:hello.scope.task_id,session:session,inspect:spec.part,runCells:spec.targets,runScope:spec.scope,runContinue:spec.keepGoing,runInspectCell:spec.inspectCell)})
    let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
    await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
    let report:AgentPipeReport
    do {report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)} catch {
      try await wire.save(path.appendingPathComponent("actual-run-failed-wire.json"))
      let events=try await fixtureEvents(storage,session)
      try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-run-failed-ledger.json"))
      throw error
    }
    try await wire.save(path.appendingPathComponent("actual-run-wire.json"))
    let hasInspect=spec.part != nil
    precondition(report.frames==(hasInspect ? 17:13) && report.toolIntents.count==(hasInspect ? 3:2) && report.lastPhase == .awaiting_settlement,spec.name)
    let events=try await fixtureEvents(storage,session)
    let facts=try await service.outcomeFacts(session:session,task:hello.scope.task_id)
    let outcome=TurnOutcome.assemble(task:hello.scope.task_id,reason:.modelFinished,facts:facts,saved:false,durationMs:0)
    precondition(Set(outcome.ranCells.map(\.cellID))==Set(spec.executed),"durable run facts: "+spec.name)
    precondition(outcome.changedCells.isEmpty && outcome.modelRequests==(hasInspect ? 4:3),"request facts: "+spec.name)
    if spec.name=="solve" {precondition(outcome.products.first?.count==2 && outcome.products.first?.verified==true)}
    if spec.name=="scene" {precondition(outcome.products.first?.kind == .scene3d && (outcome.products.first?.count ?? 0)>0)}
    if spec.name=="plot" {precondition(outcome.products.first?.kind == .plot && outcome.products.first?.count==2)}
    let transcript=try await service.transcript(session:session),actions=transcript.flatMap(\.actions)
    precondition(actions.count==(hasInspect ? 3:2) && Set(actions.map(\.id)).count==actions.count)
    precondition(actions.map(\.tool)==(hasInspect ? ["read_notebook","run_cells","inspect_result"]:["read_notebook","run_cells"]))
    precondition(actions.allSatisfy{$0.status != .running})
    try JSONEncoder().encode(transcript).write(to:path.appendingPathComponent("actual-assistant-transcript.json"))
    let effects=events.compactMap {e->AgentEffectRecord? in guard let raw=e.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(raw.utf8)),record.kind=="native_effect_result" else {return nil};return record}
    precondition(effects.count==1 && effects[0].intent.name == .run_cells)
    let effect=effects[0],lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:effect.canonical.message_id),hash:effect.blobHash)
    let bytes=try await storage.readBlob(lease,offset:0,count:Int(effect.byteLength));try await storage.closeBlob(lease)
    try bytes.write(to:path.appendingPathComponent("actual-run-result.json"))
    let envelope=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],data=envelope["data"] as? [String:Any]
    precondition(envelope["status"] as? String==spec.status && (data?["executed_cell_ids"] as? [String] ?? [])==spec.executed,spec.name)
    if let admission=effect.admissionEvent {
      let run=try await storage.sessionRunSnapshot(session,admission:admission)
      precondition(run.progress.acceptedCells==spec.executed.count && run.transition?.phase?.rawValue==spec.status)
      precondition(Set(run.admission.run!.children.map(\.operationID)).count==run.admission.run!.children.count)
      let noGroupReceipt=try await storage.kernelReceipt(document:document,operation:effect.intent.operation_id)
      precondition(noGroupReceipt==nil)
    } else {precondition(spec.name=="allowed-refused" && data==nil)}
    var inspected=""
    for event in events {
      guard let raw=event.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(raw.utf8)),record.kind=="native_readonly_result",record.intent.name == .inspect_result else {continue}
      let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:record.canonical.message_id),hash:record.blobHash)
      let bytes=try await storage.readBlob(lease,offset:0,count:Int(record.byteLength));try await storage.closeBlob(lease)
      try bytes.write(to:path.appendingPathComponent("actual-run-inspect.json"));inspected=String(decoding:bytes,as:UTF8.self)
      let value=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],resultData=value["data"] as! [String:Any],payload=resultData["payload"] as! [String:Any],binding=payload["binding"] as! [String:Any]
      precondition(resultData["origin"] as? String=="notebook" && binding["acceptance"] as? String=="accepted" && binding["cell_id"] as? String==spec.inspectCell)
      if let expected=spec.scalar {precondition(((payload["page"] as! [String:Any])["source"] as! [String:Any])["input_form"] as? String==expected)}
      if spec.name=="solve" {let presentation=(payload["presentation"] as! [String:Any])["presentation"] as! [String:Any],solutions=presentation["solutions"] as! [[String:Any]];precondition(solutions.map{(($0["bindings"] as! [[String:Any]])[0])["input_form"] as! String}==["2","3"])}
      if spec.name=="ode" {let rows=(payload["page"] as! [String:Any])["rows"] as! [[String:Any]],converged=rows.first{$0["label"] as? String=="converged"}!;let source=((converged["cells"] as! [[String:Any]])[0])["source"] as! [String:Any];precondition(source["input_form"] as? String=="True")}
      if spec.name=="scene" {let page=payload["page"] as! [String:Any];precondition((page["total"] as! Int)==384 && (page["values"] as! [[Double]]).first==[-1,-1,0] && resultData["render_state"] as? String=="data_only")}
      if spec.name=="error-stop" {precondition(payload["status"] as? String=="Error" && (payload["statement_count"] as! Int)>0)}
    }
    precondition(!hasInspect || !inspected.isEmpty)
    let finalBody=try JSONSerialization.jsonObject(with:Data(contentsOf:path.appendingPathComponent("actual-native-http-body-\(hasInspect ? 4:3).json"))) as! [String:Any]
    precondition((finalBody["messages"] as! [[String:Any]]).last?["content"] as? String==(hasInspect ? inspected:String(decoding:bytes,as:UTF8.self)))
    let before=try await storage.committedSource(document)!,head=try await storage.acceptedKernelHead(document:document)
    precondition(before.snapshot_hash==snapshot.snapshot_hash && (head?.kernel_state_revision.value ?? 0)==UInt64(spec.parent.count+spec.executed.count))
    try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-run-ledger.json"))
    await adapter.close();await service.close();await kernel.close();await source.close();try await host.close();try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:path.appendingPathComponent("store"),channel:.preview));_ = try await reopened.openDocument(document)
    let recovered=try await RecoveryService(storage:reopened).recoverSession(session,ledger:.init(storage:reopened,session:session))
    precondition(recovered.tools.count==(hasInspect ? 3:2) && recovered.tools[1].phase.rawValue==spec.status)
    if effect.admissionEvent != nil {precondition(recovered.tools[1].run?.acceptedCells==spec.executed.count)}
    try JSONSerialization.data(withJSONObject:["case":spec.name,"tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovered.tools)),"host_or_helper_created":false,"source_unchanged":true,"original_scope":"Agent run grant without source-patch grant"],options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-run-recovery.json"))
    if effect.admissionEvent != nil {
      try await queryRestoredRun(package:package,signatureRoot:signatureRoot,root:path.appendingPathComponent("operation-status-restart"),storage:reopened,document:document,reference:effect.intent.operation_id,state:spec.status,committed:events.contains{$0.event.payload_inline_json.value?.contains("\"kind\":\"run_child_result\"")==true})
    }
    try await reopened.close()
  }
}

private final class RunBoundaryProbe:@unchecked Sendable {
  let mode:String
  private let lock=NSLock(),release=DispatchSemaphore(value:0)
  private var commits=0,arrived=false,blocked=false
  init(_ mode:String) {self.mode=mode}
  func physical(_ point:String)throws {
    let target=mode=="unknown" ? "kernel_committed":"before_kernel_commit"
    guard point==target else {return}
    let stop=lock.withLock {()->Bool in commits+=1;guard commits==2 else {return false};arrived=true;blocked=mode=="unknown";return true}
    guard stop else {return}
    if mode=="unknown" {throw StorageError.system(EIO,"lost first-cell physical receipt ACK")}
    guard release.wait(timeout:.now()+10) == .success else {throw StorageError.system(ETIMEDOUT,"cancel fixture pause")}
  }
  func reconciliation(_ point:String)throws {
    if point=="before_kernel_reconciliation",lock.withLock({blocked}) {throw StorageError.system(EIO,"original readback temporarily unavailable")}
  }
  func wait()async throws {for _ in 0..<500 {if lock.withLock({arrived}) {return};try await Task.sleep(for:.milliseconds(10))};throw AgentSessionError.busy}
  func resume() {lock.withLock {blocked=false};release.signal()}
}
/// Real lost SQLite ACK with withheld readback, and real host cancellation at final SQL barrier.
private func nativeRunBoundaries(package:URL,signatureRoot:URL?,root:URL)async throws {
  for mode in ["unknown","cancel"] {
    let path=root.appendingPathComponent(mode),probe=RunBoundaryProbe(mode),document="00000000-0000-0000-0000-000000000442",session="run-boundary-task"
    let cells:[NativeSourceCell]=[.init(id:"a",kind:.math,source:"let a=2;a",dialect:.modern),.init(id:"b",kind:.math,source:"let b=a+1;b",dialect:.modern),.init(id:"answer",kind:.math,source:"b+1",dialect:.modern)]
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"真实运行边界："+mode,cells:cells),cell_revisions:cells.map{.init(cell_id:$0.id,revision:try! HostSerial(0))},snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let (storage,_)=try await StorageService.open(paths:.init(root:path,channel:.preview)),info=try await storage.openDocument(document)
    _ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
    let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock)
    let kernel=KernelCommitPort(client:host,storage:storage,source:source,reconciliationFaults:.init{try probe.reconciliation($0)})
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value,sourcePort:source,kernelPort:kernel,runFaults:.init{try probe.physical($0)},toolFailure:{tool,error in try? Data((tool.rawValue+":"+String(reflecting:error)).utf8).write(to:path.appendingPathComponent("actual-boundary-dispatch-error.txt"))})
    let hello=try await service.activate(session:session,grants:[.read_notebook,.run_cells],readonly:false,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
    let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"boundary-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"真实运行边界"))])
    let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:path,input:input,intent:intent,task:hello.scope.task_id,session:session,runCells:["answer"])})
    let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
    await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
    let execution=Task {try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)}
    if mode=="cancel" {
      try await probe.wait()
      let events=try await fixtureEvents(storage,session),admit=events.first{$0.event.event_kind == .tool_admitted}!
      _ = try await host.cancel(operation:admit.event.operation_id.value!);probe.resume()
    }
    let report:AgentPipeReport
    do {report=try await execution.value} catch {probe.resume();try await wire.save(path.appendingPathComponent("actual-boundary-failed-wire.json"));throw error}
    try await wire.save(path.appendingPathComponent("actual-boundary-wire.json"))
    precondition(report.frames==10 && report.toolIntents.count==2 && report.lastPhase == .awaiting_settlement)
    precondition(!FileManager.default.fileExists(atPath:path.appendingPathComponent("actual-native-http-body-3.json").path))
    let events=try await fixtureEvents(storage,session),effect=events.compactMap {e->AgentEffectRecord? in guard let text=e.event.payload_inline_json.value,let value=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(text.utf8)),value.kind=="native_effect_result" else {return nil};return value}.first!
    let run=try await storage.sessionRunSnapshot(session,admission:effect.admissionEvent!),active=run.summary.originalActiveOperation
    precondition(run.progress.acceptedCells==0 && run.progress.nextChild==(mode=="unknown" ? 1:2) && run.transition?.phase?.rawValue==(mode=="unknown" ? "unknown":"cancelled"))
    precondition(events.filter{$0.event.payload_inline_json.value?.contains("\"kind\":\"run_child_started\"")==true}.count==2)
    let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:effect.canonical.message_id),hash:effect.blobHash)
    let bytes=try await storage.readBlob(lease,offset:0,count:Int(effect.byteLength));try await storage.closeBlob(lease)
    try bytes.write(to:path.appendingPathComponent("actual-boundary-result.json"))
    let envelope=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],data=envelope["data"] as! [String:Any]
    precondition(envelope["status"] as? String==(mode=="unknown" ? "unknown":"cancelled") && (data["executed_cell_ids"] as! [String]).isEmpty)
    let nativeBefore=try await kernel.state(),physical=try await storage.acceptedKernelHead(document:document)!
    precondition(nativeBefore.kernel_state_revision.value==0 && physical.kernel_state_revision.value==(mode=="unknown" ? 1:0))
    if mode=="unknown" {
      _ = try await nativeQueryRoundTrip(package:package,signatureRoot:signatureRoot,root:path.appendingPathComponent("operation-status-unavailable"),host:host,storage:storage,service:service,reference:effect.intent.operation_id,session:"query-held-original-readback",state:"unknown",committed:nil,queryError:"NOT_AVAILABLE")
      let stillUnknown=try await kernel.state();precondition(stillUnknown.kernel_state_revision.value==0 && stillUnknown.pending_document_write)
    }
    probe.resume()
    if mode=="unknown" {
      _ = try await nativeQueryRoundTrip(package:package,signatureRoot:signatureRoot,root:path.appendingPathComponent("operation-status-live"),host:host,storage:storage,service:service,reference:effect.intent.operation_id,session:"query-current-unknown-run",state:"partial",committed:true,inspect:"value",runRef:effect.intent.operation_id,inspectedOperation:active,inspectedValue:"2")
      let settled=try await kernel.state();precondition(settled.kernel_state_revision.value==1 && settled.active_checkpoint_id.value==physical.checkpoint_id && settled.pending_document_write==false)
      let original=try await kernel.reconcile(active!);precondition(original?.operation_id==active && original?.producer.cell_id.value=="a")
      try encoded(original).write(to:path.appendingPathComponent("actual-original-receipt-readback.json"))
    }
    try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-boundary-ledger.json"))
    await adapter.close();await service.close();await kernel.close();await source.close();try await host.close();try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:path,channel:.preview));_ = try await reopened.openDocument(document)
    let recovered=try await RecoveryService(storage:reopened).recoverSession(session,ledger:.init(storage:reopened,session:session))
    precondition(recovered.tools.count==2 && recovered.tools[1].phase.rawValue==(mode=="unknown" ? "partial":"cancelled") && recovered.tools[1].run?.acceptedCells==(mode=="unknown" ? 1:0))
    try JSONSerialization.data(withJSONObject:["case":mode,"tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovered.tools)),"host_or_helper_created":false,"fresh_id_replay":false],options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-boundary-recovery.json"))
    try await queryRestoredRun(package:package,signatureRoot:signatureRoot,root:path.appendingPathComponent("operation-status-restart"),storage:reopened,document:document,reference:effect.intent.operation_id,state:mode=="unknown" ? "partial":"cancelled",committed:true)
    try await reopened.close()
  }
}

private func nativeScratchRoundTrips(package:URL,signatureRoot:URL?,root:URL)async throws {
  let cases:[(String,String,Dialect,String)]=[("local","a=99; {a,Out[1],RandomUniform[]}",.wolfram,"value"),("solve","solve(x^2-5*x+6==0,x)",.modern,"summary"),("ode","let curve=ode(fn(t,y)=>y,initial:1,t:0..1);curve",.modern,"value"),("scene","plot(x^2-y^2,x:-1..1,y:-1..1,view:\"surface\",mesh_points:8)",.modern,"geometry"),("partial","a=99;\nloop:=loop\nloop\nnever=5",.wolfram,"value")]
  for (name,code,dialect,part) in cases {
    let path=root.appendingPathComponent(name),document="00000000-0000-0000-0000-000000000440",session="scratch-task"
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"真实隔离试算",cells:[.init(id:"a",kind:.math,source:"let a=2;seed_random(42);a",dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let (storage,_)=try await StorageService.open(paths:.init(root:path,channel:.preview)),info=try await storage.openDocument(document)
    _ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
    let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
    let producer=try await kernel.runCell("a",operation:"actual-main-before-scratch"),checkpoint=try await storage.acceptedKernelHead(document:document)!
    precondition(producer.producer.terminal_status == .done)
    try encoded(producer).write(to:path.appendingPathComponent("actual-scratch-parent.json"))
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value)
    let hello=try await service.activate(session:session,grants:[.read_notebook,.evaluate_scratch,.inspect_result],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
    let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"scratch-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"在隔离状态试算并检查实际结果，保留主状态"))])
    let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:path,input:input,intent:intent,task:hello.scope.task_id,session:session,inspect:part,scratchSource:code,scratchDialect:dialect)})
    let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
    await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
    let report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)
    try await wire.save(path.appendingPathComponent("actual-scratch-wire.json"))
    precondition(report.frames==17 && report.toolIntents.count==3 && report.lastPhase == .awaiting_settlement)
    let events=try await fixtureEvents(storage,session);precondition(events.count==28)
    let records=events.compactMap {e->AgentReadonlyRecord? in guard let raw=e.event.payload_inline_json.value,let r=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(raw.utf8)),r.kind=="native_readonly_result" else {return nil};return r}
    precondition(records.map(\.intent.name)==[.read_notebook,.evaluate_scratch,.inspect_result])
    var inspected="",scratchRef=""
    for record in records {
      let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:record.canonical.message_id),hash:record.blobHash)
      let bytes=try await storage.readBlob(lease,offset:0,count:Int(record.byteLength));try await storage.closeBlob(lease);precondition(KernelHashes.data(bytes)==record.blobHash)
      try bytes.write(to:path.appendingPathComponent("actual-"+record.intent.name.rawValue+".json"))
      let envelope=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],data=envelope["data"] as! [String:Any]
      if record.intent.name == .evaluate_scratch {precondition(data["notebook_modified"] as? Bool==false && data["evaluation_status"] as? String==(name=="partial" ? "error":"done"));if name=="partial" {precondition(envelope["ok"] as? Bool==false)};scratchRef=data["result_ref"] as! String}
      if record.intent.name == .inspect_result {
        inspected=String(decoding:bytes,as:UTF8.self);let payload=data["payload"] as! [String:Any],binding=payload["binding"] as! [String:Any]
        precondition(data["origin"] as? String=="scratch" && binding["acceptance"] as? String=="isolated" && binding["result_ref"] as? String==scratchRef && binding["checkpoint_id"]==nil)
        precondition((binding["parent_checkpoint"] as! [String:Any])["checkpoint_id"] as? String==producer.checkpoint_id)
        if name=="local" {let page=payload["page"] as! [String:Any],rows=page["rows"] as! [[String:Any]],a=(rows[0]["cells"] as! [[String:Any]])[0],out=(rows[1]["cells"] as! [[String:Any]])[0];precondition((a["source"] as! [String:Any])["input_form"] as? String=="99" && (out["source"] as! [String:Any])["input_form"] as? String=="2")}
        if name=="scene" {precondition(data["render_state"] as? String=="data_only" && data["media_ref"] is NSNull)}
      }
    }
    let fourth=try JSONSerialization.jsonObject(with:Data(contentsOf:path.appendingPathComponent("actual-native-http-body-4.json"))) as! [String:Any],messages=fourth["messages"] as! [[String:Any]]
    precondition(messages.last?["content"] as? String==inspected)
    let after=try await storage.acceptedKernelHead(document:document)!;precondition(after.checkpoint_id==checkpoint.checkpoint_id && after.checkpoint_blob_hash==checkpoint.checkpoint_blob_hash)
    let (head,history)=try await storage.readNotebookHistory(document:document);precondition(head.snapshot_hash==snapshot.snapshot_hash && history.isEmpty)
    try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-scratch-ledger.json"))
    await adapter.close();await service.close();await kernel.close();await source.close();try await host.close();try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:path,channel:.preview)),recovery=try await reopened.sessionRecovery(session,limit:32)
    precondition(recovery.complete && recovery.lastEventSequence==28 && recovery.tools.count==3 && recovery.tools.allSatisfy{$0.target==nil && $0.admissionEvent==nil})
    precondition(recovery.tools.filter{$0.phase == .readonly_failed}.count==(name=="partial" ? 1:0) && recovery.tools.filter{$0.phase == .readonly_completed}.count==(name=="partial" ? 2:3))
    try JSONSerialization.data(withJSONObject:["case":name,"complete":recovery.complete,"last_event_sequence":recovery.lastEventSequence,"tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovery.tools)),"host_or_helper_created":false,"main_checkpoint_unchanged":true,"scratch_has_no_main_receipt":true],options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-scratch-recovery.json"))
    try await reopened.close()
  }
}
/// Actual accepted math -> Agent read-issued scoped result -> independent result worker ->
/// verified durable readonly Blob -> exact third model body, with storage-only recovery.
private func nativeInspectRoundTrips(package:URL,signatureRoot:URL?,root:URL)async throws {
  let cases=[("rational","1/3","value"),("solve","solve(x^2-5*x+6==0,x)","summary"),("ode","let curve=ode(fn(t,y)=>y,initial:1,t:0..1);curve","value"),("scene","plot(x^2-y^2,x:-1..1,y:-1..1,view:\"surface\",mesh_points:8)","geometry")]
  for (name,expression,part) in cases {
    let path=root.appendingPathComponent(name),document="00000000-0000-0000-0000-000000000439",session="inspect-task"
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"真实数学结果检查",cells:[.init(id:"a",kind:.math,source:expression,dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let (storage,_)=try await StorageService.open(paths:.init(root:path,channel:.preview)),info=try await storage.openDocument(document)
    _ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
    let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
    let producer=try await kernel.runCell("a",operation:"actual-math-before-inspection")
    precondition(producer.producer.terminal_status == .done)
    try encoded(producer).write(to:path.appendingPathComponent("actual-inspect-producer.json"))
    let checkpoint=try await storage.acceptedKernelHead(document:document)!
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value)
    let hello=try await service.activate(session:session,grants:[.read_notebook,.inspect_result],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
    let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"inspect-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"检查原计算结果，不重复计算"))])
    let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:path,input:input,intent:intent,task:hello.scope.task_id,session:session,inspect:part)})
    let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
    await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
    let report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)
    try await wire.save(path.appendingPathComponent("actual-inspect-wire.json"))
    precondition(report.frames==13 && report.toolIntents.count==2 && report.lastPhase == .awaiting_settlement)
    let events=try await fixtureEvents(storage,session);precondition(events.count==21)
    let records=events.compactMap {e->AgentReadonlyRecord? in guard let raw=e.event.payload_inline_json.value,let r=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(raw.utf8)),r.kind=="native_readonly_result" else {return nil};return r}
    precondition(records.map(\.intent.name)==[.read_notebook,.inspect_result])
    var resultText=""
    for record in records {
      let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:record.canonical.message_id),hash:record.blobHash)
      let bytes=try await storage.readBlob(lease,offset:0,count:Int(record.byteLength));try await storage.closeBlob(lease)
      precondition(KernelHashes.data(bytes)==record.blobHash)
      try bytes.write(to:path.appendingPathComponent(record.intent.name == .read_notebook ? "actual-inspect-read.json":"actual-inspect-result.json"))
      if record.intent.name == .inspect_result {
        let envelope=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],data=envelope["data"] as! [String:Any]
        precondition(envelope["ok"] as? Bool==true && data["origin"] as? String=="notebook" && data["is_current"] as? Bool==true)
        let payload=data["payload"] as! [String:Any],binding=payload["binding"] as! [String:Any]
        precondition(binding["operation_id"] as? String==producer.operation_id && binding["kernel_state_revision"] as? UInt64==producer.kernel_state_revision.value)
        if name=="rational" {let page=payload["page"] as! [String:Any];precondition((page["source"] as! [String:Any])["input_form"] as? String=="1/3")}
        if name=="scene" {precondition(data["render_state"] as? String=="data_only" && data["media_ref"] is NSNull)}
        resultText=String(decoding:bytes,as:UTF8.self)
        // A manual task=None request cannot consume the Agent-scoped result reference.
        let resultRef=binding["result_ref"] as! String
        var denied=try await host.resultCommand(.result_inspect(.init(type:.result_inspect,request_id:"manual-cannot-use-agent-result",result_ref:resultRef,query:.inspect_result_summary(.init(kind:.summary)))))
        while denied.phase == .queued || denied.phase == .running {try await Task.sleep(for:.milliseconds(10));denied=try await host.resultCommand(.result_status(.init(type:.result_status,request_id:denied.request_id)))}
        precondition(denied.phase == .failed)
      }
    }
    let body=try JSONSerialization.jsonObject(with:Data(contentsOf:path.appendingPathComponent("actual-native-http-body-3.json"))) as! [String:Any],messages=body["messages"] as! [[String:Any]]
    precondition(messages.last?["role"] as? String=="tool" && messages.last?["content"] as? String==resultText)
    let after=try await storage.acceptedKernelHead(document:document)!
    precondition(after.checkpoint_id==checkpoint.checkpoint_id && after.checkpoint_blob_hash==checkpoint.checkpoint_blob_hash)
    let (head,history)=try await storage.readNotebookHistory(document:document);precondition(head.snapshot_hash==snapshot.snapshot_hash && history.isEmpty)
    try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-inspect-ledger.json"))
    await adapter.close();await service.close();await kernel.close();await source.close();try await host.close();try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:path,channel:.preview)),recovered=try await reopened.sessionRecovery(session,limit:32)
    precondition(recovered.complete && recovered.lastEventSequence==21 && recovered.tools.count==2 && recovered.tools.allSatisfy{$0.phase == .readonly_completed && $0.target==nil && $0.admissionEvent==nil})
    try JSONSerialization.data(withJSONObject:["case":name,"complete":recovered.complete,"last_event_sequence":recovered.lastEventSequence,"tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovered.tools)),"host_or_helper_created":false,"original_checkpoint_unchanged":true,"manual_scope_result_rejected":true],options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-inspect-recovery.json"))
    try await reopened.close()
  }
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
  let observation=ActionObservation()
  await service.observeTranscript {try? await observation.accept(service.transcript(session:session))}
  let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
  await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
  let report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input,inputPresentation:.init(model:"synthetic-model",mode:"execute"))
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
  let facts=try await service.outcomeFacts(session:session,task:hello.scope.task_id)
  let outcome=TurnOutcome.assemble(task:hello.scope.task_id,reason:.modelFinished,facts:facts,saved:false,durationMs:0)
  precondition(outcome.changedCells==[.init(cellID:"a",kind:.edited,transactionRef:history[0].transaction)])
  precondition(outcome.ranCells.isEmpty && outcome.staleAfterEdit==["a"] && outcome.modelRequests==4)
  let transcript=try await service.transcript(session:session),actions=transcript.flatMap(\.actions)
  precondition(actions.map(\.tool)==["read_notebook","preview_source","apply_notebook_patch"] && actions.allSatisfy{$0.status == .ok})
  precondition(actions.last?.transactionRef==history[0].transaction && actions.last?.object=="修改单元格 1")
  precondition(transcript.allSatisfy{$0.context?.model=="synthetic-model" && $0.context?.mode=="execute"})
  let paired=await observation.paired(Set(actions.map(\.callID)));precondition(paired)
  try JSONEncoder().encode(transcript).write(to:root.appendingPathComponent("actual-assistant-transcript.json"))
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
private func nativeQueryRoundTrip(package:URL,signatureRoot:URL?,root:URL,host:NativeHostClient,storage:StorageService,service:AgentSessionService,reference:String,session:String,state:String,committed:Bool?,inspect:String?=nil,transaction:String?=nil,runRef:String?=nil,inspectedOperation:String?=nil,inspectedValue:String="6",queryError:String?=nil)async throws->NativeAgentControlScope {
  try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
  let hello=try await service.activate(session:session,grants:[.read_notebook,.get_operation_status,.inspect_result],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
  let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"query-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"仅核对原操作，不重放："+reference))])
  let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:root,input:input,intent:intent,task:hello.scope.task_id,session:session,inspect:inspect,operationRef:reference)})
  let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
  await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
  let report:AgentPipeReport
  do {report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)} catch {
    try await wire.save(root.appendingPathComponent("actual-query-failed-wire.json"));throw error
  }
  precondition(report.frames==(inspect==nil ? 13:17) && report.toolIntents.count==(inspect==nil ? 2:3) && report.lastPhase == .awaiting_settlement)
  try await wire.save(root.appendingPathComponent("actual-query-wire.json"))
  let events=try await fixtureEvents(storage,session)
  let records=events.compactMap {event->AgentReadonlyRecord? in guard let raw=event.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(raw.utf8)),record.kind=="native_readonly_result" else {return nil};return record}
  precondition(records.map(\.intent.name)==(inspect==nil ? [.read_notebook,.get_operation_status]:[.read_notebook,.get_operation_status,.inspect_result]))
  for record in records where record.intent.name != .read_notebook {
    let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:record.canonical.message_id),hash:record.blobHash)
    let bytes=try await storage.readBlob(lease,offset:0,count:Int(record.byteLength));try await storage.closeBlob(lease)
    precondition(KernelHashes.data(bytes)==record.blobHash)
    try bytes.write(to:root.appendingPathComponent("actual-"+record.intent.name.rawValue+".json"))
    let envelope=try JSONSerialization.jsonObject(with:bytes) as! [String:Any]
    if record.intent.name == .get_operation_status,let queryError {
      precondition(envelope["status"] as? String=="failed" && envelope["data"] is NSNull && (envelope["error"] as! [String:Any])["code"] as? String==queryError)
      continue
    }
    let data=envelope["data"] as! [String:Any]
    precondition(envelope["status"] as? String=="completed")
    if record.intent.name == .get_operation_status {
      precondition(data["operation_state"] as? String==state && data["outcome_known"] as? Bool==(!["queued","running","unknown"].contains(state)))
      if let committed {precondition(data["effect_committed"] as? Bool==committed)} else {precondition(data["effect_committed"] is NSNull)}
      precondition((data["transaction_ref"] as? String)==transaction && (data["run_ref"] as? String)==runRef)
      if inspect != nil {precondition((data["result_refs"] as! [String]).count>0)}
    } else {
      let payload=data["payload"] as! [String:Any],binding=payload["binding"] as! [String:Any]
      precondition(binding["operation_id"] as? String==(inspectedOperation ?? reference) && binding["acceptance"] as? String=="accepted")
      precondition(((payload["page"] as! [String:Any])["source"] as! [String:Any])["input_form"] as? String==inspectedValue)
    }
  }
  let final=try JSONSerialization.jsonObject(with:Data(contentsOf:root.appendingPathComponent("actual-native-http-body-\(inspect==nil ? 3:4).json"))) as! [String:Any]
  let lastRecord=records.last!,lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:lastRecord.canonical.message_id),hash:lastRecord.blobHash)
  let lastBytes=try await storage.readBlob(lease,offset:0,count:Int(lastRecord.byteLength));try await storage.closeBlob(lease)
  precondition((final["messages"] as! [[String:Any]]).last?["content"] as? String==String(decoding:lastBytes,as:UTF8.self))
  let recovery=try await storage.sessionRecovery(session,limit:32)
  let failedCall=queryError==nil ? nil:records.first{$0.intent.name == .get_operation_status}!.intent.tool_call_id
  precondition(recovery.complete && recovery.tools.count==records.count && recovery.tools.allSatisfy{$0.phase == ($0.callID==failedCall ? .readonly_failed:.readonly_completed) && $0.target==nil && $0.admissionEvent==nil})
  try JSONSerialization.data(withJSONObject:try events.map {try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-query-ledger.json"))
  try JSONSerialization.data(withJSONObject:["tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(recovery.tools)),"query_creates_no_effect":true,"original_reference":reference],options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-query-recovery.json"))
  await adapter.close();return hello.scope
}
private func queryRestoredRun(package:URL,signatureRoot:URL?,root:URL,storage:StorageService,document:String,reference:String,state:String,committed:Bool)async throws {
  let source=try await storage.committedSource(document)!,head=try await storage.acceptedKernelHead(document:document),info=try await storage.documentIdentity(document)
  let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:source,store_id:info.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
  let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value,toolFailure:{tool,error in try? Data((tool.rawValue+":"+String(reflecting:error)).utf8).write(to:root.appendingPathComponent("actual-query-dispatch-error.txt"))})
  _ = try await nativeQueryRoundTrip(package:package,signatureRoot:signatureRoot,root:root,host:host,storage:storage,service:service,reference:reference,session:"query-original-run",state:state,committed:committed,runRef:reference)
  let math=try await host.kernelCommand(.kernel_state(.init(type:.kernel_state))),after=try await storage.acceptedKernelHead(document:document),unchanged=try await storage.committedSource(document)!
  precondition(math.active_checkpoint_id.value==nil && math.kernel_state_revision.value==0 && head?.checkpoint_id==after?.checkpoint_id && unchanged.snapshot_hash==source.snapshot_hash)
  await service.close();try await host.close()
}
private func nativeOperationQueries(package:URL,signatureRoot:URL?,root:URL)async throws {
  let document="00000000-0000-0000-0000-000000000443"
  var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"真实原操作查账",cells:[.init(id:"a",kind:.math,source:"2+2",dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
  snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
  let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview)),info=try await storage.openDocument(document)
  _ = try await storage.initializeSource(snapshot)
  let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
  let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
  let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
  let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
  let preview=try await source.preview(.preview_patch_input(.init(kind:.patch,operations:[.preview_update_cell(.init(type:.update_cell,target:.init(cell_id:"a"),expected_source_hash:KernelHashes.data(Data("2+2".utf8)),source:"3+3"))])))
  let applied=try await source.apply(preview.preview_ref.value!),sourceReceipt=applied.receipt.value!
  let kernelReceipt=try await kernel.runCell("a",operation:"actual-kernel-before-query")
  try encoded(sourceReceipt).write(to:root.appendingPathComponent("actual-original-source-receipt.json"));try encoded(kernelReceipt).write(to:root.appendingPathComponent("actual-original-kernel-receipt.json"))
  let before=try await storage.committedSource(document)!,checkpoint=try await storage.acceptedKernelHead(document:document)!
  let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value,toolFailure:{tool,error in try? Data((tool.rawValue+":"+String(reflecting:error)).utf8).write(to:root.appendingPathComponent("actual-query-dispatch-error.txt"))})
  _ = try await nativeQueryRoundTrip(package:package,signatureRoot:signatureRoot,root:root.appendingPathComponent("source-live"),host:host,storage:storage,service:service,reference:sourceReceipt.receipt.operation_id,session:"query-source-live",state:"completed",committed:true,transaction:sourceReceipt.receipt.transaction_id.value)
  _ = try await nativeQueryRoundTrip(package:package,signatureRoot:signatureRoot,root:root.appendingPathComponent("kernel-live"),host:host,storage:storage,service:service,reference:kernelReceipt.operation_id,session:"query-kernel-live",state:"completed",committed:true,inspect:"value")
  let oldScope=try await nativeQueryRoundTrip(package:package,signatureRoot:signatureRoot,root:root.appendingPathComponent("missing-live"),host:host,storage:storage,service:service,reference:"never-issued-operation",session:"query-missing-live",state:"unknown",committed:nil)
  let after=try await storage.committedSource(document)!,head=try await storage.acceptedKernelHead(document:document)!
  precondition(after.snapshot_hash==before.snapshot_hash && head.checkpoint_id==checkpoint.checkpoint_id)
  await service.close();await kernel.close();await source.close();try await host.close();try await storage.close()
  let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));_ = try await reopened.openDocument(document)
  let committed=try await reopened.committedSource(document)!,newHost=try await NativeHostClient.open()
  let newOpen=try await newHost.sourceCommand(.source_open(.init(type:.source_open,snapshot:committed,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
  do {_ = try await newHost.agentCommand(.native_agent_query_operation(.init(type:.agent_query_operation,scope:oldScope,operation_id:"old-query",observation:.init(nil),failure_code:.init(nil))));fatalError("old query scope restored an operation grant")}catch NativeHostClientError.rejected { }
  let fresh=AgentSessionService(host:newHost,storage:reopened,document:document,generation:newOpen.document_generation.value)
  _ = try await nativeQueryRoundTrip(package:package,signatureRoot:signatureRoot,root:root.appendingPathComponent("source-restart"),host:newHost,storage:reopened,service:fresh,reference:sourceReceipt.receipt.operation_id,session:"query-source-restart",state:"completed",committed:true,transaction:sourceReceipt.receipt.transaction_id.value)
  _ = try await nativeQueryRoundTrip(package:package,signatureRoot:signatureRoot,root:root.appendingPathComponent("kernel-restart"),host:newHost,storage:reopened,service:fresh,reference:kernelReceipt.operation_id,session:"query-kernel-restart",state:"completed",committed:true)
  let math=try await newHost.kernelCommand(.kernel_state(.init(type:.kernel_state)))
  precondition(math.active_checkpoint_id.value==nil && math.kernel_state_revision.value==0)
  let unchanged=try await reopened.committedSource(document)!,physical=try await reopened.acceptedKernelHead(document:document)!
  precondition(unchanged.snapshot_hash==before.snapshot_hash && physical.checkpoint_id==checkpoint.checkpoint_id)
  try JSONSerialization.data(withJSONObject:["old_scope":"rejected before fresh activation","source_unchanged":true,"original_kernel_checkpoint_unchanged":true,"native_kernel_not_restored_or_evaluated_by_query":true],options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-query-proof.json"))
  await fresh.close();try await newHost.close();try await reopened.close()
}

private func nativeUndoRoundTrips(package:URL,signatureRoot:URL?,root:URL)async throws {
  for mode in ["normal","group","independent","conflict","restricted","unissued","lost-source-ack"] {
    let path=root.appendingPathComponent(mode),document="00000000-0000-0000-0000-000000000444",session="actual-undo-task"
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"真实Agent撤销："+mode,cells:[.init(id:"a",kind:.math,source:"2+2",dialect:.modern),.init(id:"b",kind:.math,source:"9",dialect:.modern)]),cell_revisions:["a","b"].map{.init(cell_id:$0,revision:try! HostSerial(0))},snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let (storage,_)=try await StorageService.open(paths:.init(root:path,channel:.preview)),info=try await storage.openDocument(document)
    _ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open(),opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
    let drafts=await MainActor.run {DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
    func edit(_ cell:String,_ before:String,_ after:String)async throws->NativeCommitState {
      let p=try await source.preview(.preview_patch_input(.init(kind:.patch,operations:[.preview_update_cell(.init(type:.update_cell,target:.init(cell_id:cell),expected_source_hash:KernelHashes.data(Data(before.utf8)),source:after))])))
      return try await source.apply(p.preview_ref.value!)
    }
    let initial=try await edit("a","2+2","3+3"),transaction=initial.receipt.value!.receipt.transaction_id.value!
    var undoTransactions=[transaction]
    if mode=="group" {let next=try await edit("b","9","11");undoTransactions.append(next.receipt.value!.receipt.transaction_id.value!)}
    if mode=="independent" {_ = try await edit("b","9","11")}
    if mode=="conflict" {_ = try await edit("a","3+3","9+9")}
    let before=try await storage.committedSource(document)!
    let noWrite=["conflict","restricted","unissued"].contains(mode)
    let sourceFaults=StorageFaults {point in if mode=="lost-source-ack" && point=="source_committed" {throw StorageError.system(EIO,"lost original inverse commit ACK")}}
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value,sourcePort:source,kernelPort:kernel,sourceFaults:sourceFaults,toolFailure:{tool,error in try? Data((tool.rawValue+":"+String(reflecting:error)).utf8).write(to:path.appendingPathComponent("actual-undo-dispatch-error.txt"))})
    let targets=mode=="unissued" ? ["unissued-transaction"]:undoTransactions
    let rounds=noWrite ? 1:2
    for round in 0..<rounds {
      let folder=path.appendingPathComponent("round-\(round+1)");try FileManager.default.createDirectory(at:folder,withIntermediateDirectories:true)
      let hello=try await service.activate(session:session+"-\(round)",grants:[.read_notebook,.undo_transaction],readonly:false,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1",allowedUndoTransactions:mode=="restricted" ? []:nil)
      let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"undo-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"撤销指定原事务，保留后续手工内容"))])
      let adapter=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{intent in try await nativeProviderTurn(host:host,storage:storage,root:folder,input:input,intent:intent,task:hello.scope.task_id,session:session+"-\(round)",undoTransactions:targets)})
      let verified=try await AgentProcess.verified(package:package,signatureRoot:signatureRoot),pipe=try await verified.launch(),wire=WireCapture()
      await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)}
      let report:AgentPipeReport
      do {report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)}catch{try await wire.save(folder.appendingPathComponent("actual-undo-failed-wire.json"));throw error}
      precondition(report.frames==13 && report.toolIntents.count==2 && report.lastPhase == .awaiting_settlement)
      try await wire.save(folder.appendingPathComponent("actual-undo-wire.json"))
      let events=try await fixtureEvents(storage,session+"-\(round)"),effects=events.compactMap{e->AgentEffectRecord? in guard let text=e.event.payload_inline_json.value,let record=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(text.utf8)),record.kind=="native_effect_result" else{return nil};return record}
      precondition(effects.count==1 && effects[0].intent.name == .undo_transaction)
      let effect=effects[0],lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:effect.canonical.message_id),hash:effect.blobHash)
      let bytes=try await storage.readBlob(lease,offset:0,count:Int(effect.byteLength));try await storage.closeBlob(lease)
      try bytes.write(to:folder.appendingPathComponent("actual-undo-result.json"))
      let envelope=try JSONSerialization.jsonObject(with:bytes) as! [String:Any],shouldWrite=round==0 && !noWrite
      precondition(envelope["status"] as? String==(shouldWrite ? "completed":"failed"))
      let after=try await storage.committedSource(document)!
      if shouldWrite {
        let data=envelope["data"] as! [String:Any];precondition(data["saved"] as? Bool==false && data["undo_transaction_ref"] as? String != transaction && Set(data["changed_cell_ids"] as! [String])==Set(mode=="group" ? ["a","b"]:["a"]))
        precondition(after.revision.value==before.revision.value+1 && after.file.cells[0].source=="2+2" && after.file.cells[1].source==(mode=="independent" ? "11":"9"))
        let receipt=try await storage.sourceReceipt(document:document,operation:effect.target!.operationID)!,metadata=try await storage.sourceUndoMetadata(document:document,operation:effect.target!.operationID)!
        precondition(receipt.receipt.operation_kind == .undo && Set(metadata.transaction_ids)==Set(undoTransactions) && receipt.receipt.transaction_id.value==data["undo_transaction_ref"] as? String)
        try encoded(receipt).write(to:folder.appendingPathComponent("actual-inverse-receipt.json"));try encoded(metadata).write(to:folder.appendingPathComponent("actual-inverse-metadata.json"))
      } else {
        let code=(envelope["error"] as! [String:Any])["code"] as! String
        precondition(code==(mode=="restricted" || mode=="unissued" ? "PERMISSION_DENIED":"UNDO_CONFLICT"))
        precondition(after.revision.value==before.revision.value+(noWrite ? 0:1))
        precondition(effect.target==nil && effect.admissionEvent==nil)
      }
      let math=try await kernel.state(),physical=try await storage.acceptedKernelHead(document:document)
      precondition(math.active_checkpoint_id.value==nil && physical==nil)
      let final=try JSONSerialization.jsonObject(with:Data(contentsOf:folder.appendingPathComponent("actual-native-http-body-3.json"))) as! [String:Any]
      precondition((final["messages"] as! [[String:Any]]).last?["content"] as? String==String(decoding:bytes,as:UTF8.self))
      try JSONSerialization.data(withJSONObject:try events.map{try JSONSerialization.jsonObject(with:encoded($0.event))},options:[.sortedKeys]).write(to:folder.appendingPathComponent("actual-undo-ledger.json"))
      await adapter.close()
    }
    let final=try await storage.committedSource(document)!
    await service.close();await kernel.close();await source.close();try await host.close();try await storage.close()
    let(reopened,_)=try await StorageService.open(paths:.init(root:path,channel:.preview));_ = try await reopened.openDocument(document)
    var recovered:[[String:Any]]=[]
    for round in 0..<rounds {
      let page=try await RecoveryService(storage:reopened).recoverSession(session+"-\(round)",ledger:.init(storage:reopened,session:session+"-\(round)"))
      precondition(page.complete && page.tools.count==2 && page.tools[1].phase == (round==0 && !noWrite ? .completed:.failed))
      recovered.append(["round":round+1,"tools":try JSONSerialization.jsonObject(with:RecoveryValidation.encode(page.tools))])
    }
    let unchanged=try await reopened.committedSource(document)!
    precondition(unchanged.snapshot_hash==final.snapshot_hash)
    try JSONSerialization.data(withJSONObject:["case":mode,"rounds":recovered,"source_revision":unchanged.revision.value,"host_or_helper_created":false,"computation_started":false,"file_saved":false],options:[.sortedKeys]).write(to:path.appendingPathComponent("actual-undo-recovery.json"))
    try await reopened.close()
  }
}

@main struct AgentPackageFixtures {
  static func main() async throws {
    let package=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true),root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    if ProcessInfo.processInfo.environment["OPENMATH_PACKAGE_FIXTURE_MODE"]=="turn-outcome" {
      let signatureRoot=CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil
      try await nativeApplyRoundTrip(package:package,signatureRoot:signatureRoot,root:root.appendingPathComponent("apply"))
      try await nativeRunRoundTrip(package:package,signatureRoot:signatureRoot,root:root.appendingPathComponent("run"))
      print("Turn outcome durable apply/run/inspection roundtrips passed");return
    }
    for _ in 0..<32 {
      let child=try await AgentPrivatePipe.launch(executable:URL(fileURLWithPath:"/usr/bin/true"),arguments:[],directory:root)
      let end=try await child.read();precondition(end==nil);await child.close()
    }
    if ProcessInfo.processInfo.environment["OPENMATH_PACKAGE_FIXTURE_MODE"]=="undo" {
      try await nativeUndoRoundTrips(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("undo-cases"))
      print("Actual native undo roundtrips passed; evidence_root="+root.path);return
    }
    if ProcessInfo.processInfo.environment["OPENMATH_PACKAGE_FIXTURE_MODE"]=="operations" {
      try await nativeOperationQueries(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("operation-queries"))
      print("Actual native operation queries passed; evidence_root="+root.path);return
    }
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
    // A second actual title-only source transaction after math must keep the real accepted
    // status/definitions fresh; its new whole-document hash is not a Math invalidation.
    let titleOnly=try await source.preview(.preview_patch_input(.init(kind:.patch,operations:[.preview_rename_notebook(.init(type:.rename_notebook,title:"计算后仅改标题，结果仍新鲜"))])))
    let titleCommit=try await source.apply(titleOnly.preview_ref.value!)
    precondition(titleCommit.phase == .completed)
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
      precondition(originalCells[0]["status"] as? String=="Done")
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
    try await nativeInspectRoundTrips(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("inspect-cases"))
    try await nativeRunRoundTrip(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("run-case"))
    try await nativeRunBoundaries(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("run-boundaries"))
    try await nativeScratchRoundTrips(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("scratch-cases"))
    try await nativeOperationQueries(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("operation-queries"))
    try await nativeUndoRoundTrips(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil,root:root.appendingPathComponent("undo-cases"))
    print(String(decoding:try JSONSerialization.data(withJSONObject:["production_core":"1.0.4","bundled_node":"26.11.1","actual_private_frames":report.frames,"canonical_events":events.count,"model_transport":"actual_urlsession_rust_cabi_durable_request_ledger","tool_outcome":"completed_actual_readonly_no_mutation","manual_after_sigkill":"4","system_node_fallback":false,"PATH":"empty","wrong_architecture":"rejected","missing_resource":"rejected","late_stdin":"rejected","unconfigured_model":"no_fake_message_or_effect","model_cancel_and_late_delta":"rejected","actual_macho_architecture":"rejected","evidence_root":root.path],options:[.sortedKeys]),as:UTF8.self))
  }
}
