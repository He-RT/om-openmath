import Foundation

func object<T:Encodable>(_ value:T)throws->[String:Any] {try JSONSerialization.jsonObject(with:JSONEncoder().encode(value)) as! [String:Any]}
func frame(_ hello:NativeAgentHostReply,kind:String,sequence:Int,payload:[String:Any],reply:String?=nil)throws->Data {
  var v=try JSONSerialization.jsonObject(with:Data(hello.outbound_json.value!.utf8)) as! [String:Any]
  v["type"]=kind;v["sequence"]=sequence;v["message_id"]="peer-\(sequence)";v["reply_to"]=reply ?? (NSNull() as Any);v["payload"]=payload
  return try JSONSerialization.data(withJSONObject:v,options:[.sortedKeys,.withoutEscapingSlashes])
}
private final class LedgerPause:@unchecked Sendable {
  let arrived=DispatchSemaphore(value:0),resume=DispatchSemaphore(value:0)
  func waitForArrival()->Bool {arrived.wait(timeout:.now()+10) == .success}
  func reach(_ point:String)throws {if point=="session_event_before_commit.user_message" {arrived.signal();resume.wait()}}
}
@main struct AgentProtocolFixtures {
  static func main() async throws {
    let repo=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true),root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true),document="00000000-0000-0000-0000-000000000431"
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"隔离Agent协议",cells:[.init(id:"a",kind:.math,source:"2+2",dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));let info=try await storage.openDocument(document);_ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open();let open=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:open.document_generation.value)
    let session="manual-session",hello=try await service.activate(session:session,grants:[.read_notebook],readonly:true,coreVersion:"1.0.4",adapterVersion:"adapter-1")
    let h=try JSONSerialization.jsonObject(with:Data(hello.outbound_json.value!.utf8)) as! [String:Any]
    _ = try await service.receive(frame(hello,kind:"ready",sequence:1,payload:h["payload"] as! [String:Any],reply:h["message_id"] as? String))
    let scope=hello.scope
    let canonical:[String:Any]=["record_type":"canonical_agent_message","message_version":1,"message_id":"complete-message","role":"assistant","agent_turn_id":"turn","task_id":scope.task_id,"parts":[["kind":"text","part_id":"text","text":"UTF8 中文🙂 é / \""],["kind":"tool_call","part_id":"call","tool_call_id":"model-call","name":"read_notebook","arguments_json":"{}"]]]
    let event=try frame(hello,kind:"transcript_event",sequence:2,payload:["event_id":"complete-event","source_sequence":1,"message":canonical])
    do {_ = try await service.receive(event,faults:.init {point in if point=="session_event_committed" {throw StorageError.system(EIO,"fixture-lost-ACK")}});fatalError("lost commit ACK became success")} catch StorageError.unknownCommit { }
    let committedEvents=try await storage.sessionEvents(session);precondition(committedEvents.count==1)
    let blocked=try frame(hello,kind:"tool_call",sequence:3,payload:["agent_turn_id":"turn","tool_call_id":"model-call","proposal_event_ref":"complete-event"])
    do {_ = try await service.receive(blocked);fatalError("unconfirmed ledger admitted tool")} catch NativeHostClientError.rejected { }
    let ack=try await service.receive(event),replay=try await service.receive(event)
    precondition(ack.disposition == .ack && ack.outbound_json.value==replay.outbound_json.value)
    let actual=try await storage.sessionEvent(session,event:"complete-event")!,stored=try JSONDecoder().decode(SessionCanonicalAssistant.self,from:Data(actual.event.payload_inline_json.value!.utf8))
    let decoded=try JSONDecoder().decode(AgentCanonicalMessage.self,from:Data(stored.canonicalJSON.utf8));precondition(decoded.message_id=="complete-message" && stored.calls[0].callID=="model-call")
    let mapped=try await service.receive(blocked);precondition(mapped.tool_intent.value!.operation_id != "model-call")
    let user=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"input-message",role:.user,agent_turn_id:.init("turn"),task_id:.init(scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"input-part",text:"输入/转向/后续🙂"))])
    for kind in ["user_input","steer","follow_up"] {
      let sent=try await service.input(user,delivery:kind,event:kind),retry=try await service.input(user,delivery:kind,event:kind)
      precondition(sent.disposition == .sent && sent.outbound_json.value==retry.outbound_json.value)
    }
    let pause=LedgerPause()
    let waiting=Task {try await service.input(user,event:"blocked-user",faults:.init {try pause.reach($0)})}
    let blockedWriter=await Task.detached {pause.waitForArrival()}.value;precondition(blockedWriter)
    let stopped=try await service.cancel();precondition(stopped.phase == .cancelling)
    pause.resume.signal()
    do {_ = try await waiting.value;fatalError("cancelled queued input sent")} catch NativeHostClientError.rejected { }
    let preserved=try await storage.sessionEvent(session,event:"blocked-user");precondition(preserved != nil)
    let late=try frame(hello,kind:"prepare_request",sequence:4,payload:["agent_turn_id":"turn","request_id":"late-request"])
    do {_ = try await service.receive(late);fatalError("cancel admitted request")} catch NativeHostClientError.rejected { }
    await service.close()
    let pipeService=AgentSessionService(host:host,storage:storage,document:document,generation:open.document_generation.value)
    // New native service starts at generation 1; an old activation must not be reused.
    do {_ = try await pipeService.activate(session:"pipe-session",grants:[.read_notebook],readonly:true,coreVersion:"1.0.4",adapterVersion:"adapter-1");fatalError("reset generations reused authority")} catch NativeHostClientError.rejected { }
    try await host.close()
    let pipeHost=try await NativeHostClient.open();let pipeOpen=try await pipeHost.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let actualService=AgentSessionService(host:pipeHost,storage:storage,document:document,generation:pipeOpen.document_generation.value),pipeHello=try await actualService.activate(session:"pipe-session",grants:[.read_notebook],readonly:true,coreVersion:"1.0.4",adapterVersion:"adapter-1")
    let output=root.appendingPathComponent("actual-peer-frames.json")
    let pipe=try await AgentPrivatePipe.launch(executable:URL(fileURLWithPath:CommandLine.arguments[3]),arguments:[repo.appendingPathComponent("macos/OpenMathNativeTests/AgentProtocolFixtures/peer.mjs").path,output.path],directory:repo)
    let modelPort=AgentNativeModelPort(prepare:{_ in .init(context:"fixture-context",model:"fixture-config")},stream:{intent in AsyncThrowingStream {$0.yield(.init(agent_turn_id:intent.agent_turn_id,request_id:intent.request_id,byte_offset:try! HostSerial(0),text:"",phase:.failed,canonical_message:.init(nil)));$0.finish()}})
    let report=try await actualService.run(pipe:pipe,hello:pipeHello,modelPort:modelPort)
    precondition(report.frames==7 && report.lastPhase == .awaiting_settlement && report.toolIntents.count==1 && report.requestIntents.count==2 && report.requestIntents[0]==report.requestIntents[1])
    let pipeEvents=try await storage.sessionEvents("pipe-session");precondition(pipeEvents.count==2)
    let orderedOutput=root.appendingPathComponent("ordered-host-frames.json")
    let orderedPipe=try await AgentPrivatePipe.launch(executable:URL(fileURLWithPath:CommandLine.arguments[3]),arguments:[repo.appendingPathComponent("macos/OpenMathNativeTests/AgentProtocolFixtures/peer.mjs").path,orderedOutput.path,"collect"],directory:repo)
    var second=try JSONSerialization.jsonObject(with:Data(pipeHello.outbound_json.value!.utf8)) as! [String:Any]
    second["sequence"]=2;second["message_id"]="outbound-second";second["type"]="user_input";second["payload"]=["event_ref":"synthetic-ordering-input","message":try object(user)]
    let secondBytes=try JSONSerialization.data(withJSONObject:second,options:[.sortedKeys])+Data([10])
    try await orderedPipe.write(secondBytes);try await orderedPipe.write(Data((pipeHello.outbound_json.value!+"\n").utf8))
    while try await orderedPipe.read() != nil { }
    await orderedPipe.close()
    let observed=try JSONSerialization.jsonObject(with:Data(contentsOf:orderedOutput)) as! [[String:Any]]
    precondition((observed[0]["frame"] as! [String:Any])["sequence"] as! Int==1 && (observed[1]["frame"] as! [String:Any])["sequence"] as! Int==2)
    let modes=["invalid_utf8","partial_eof","sequence_gap","stale_scope","wrong_handshake","duplicate_field","oversize","empty_eof"]
    for mode in modes {
      let badHello=try await actualService.activate(session:"bad-"+mode,grants:[.read_notebook],readonly:true,coreVersion:"1.0.4",adapterVersion:"adapter-1")
      let bad=try await AgentPrivatePipe.launch(executable:URL(fileURLWithPath:CommandLine.arguments[3]),arguments:[repo.appendingPathComponent("macos/OpenMathNativeTests/AgentProtocolFixtures/peer.mjs").path,root.appendingPathComponent(mode+".json").path,mode],directory:repo)
      do {_ = try await actualService.run(pipe:bad,hello:badHello);fatalError("invalid private peer accepted: \(mode)")} catch AgentSessionError.invalidFrame { } catch NativeHostClientError.rejected { }
      let events=try await storage.sessionEvents("bad-"+mode);precondition(events.isEmpty)
    }

    let largeHello=try await actualService.activate(session:"large-session",grants:[.read_notebook],readonly:true,coreVersion:"1.0.4",adapterVersion:"adapter-1")
    let lh=try JSONSerialization.jsonObject(with:Data(largeHello.outbound_json.value!.utf8)) as! [String:Any]
    _ = try await actualService.receive(frame(largeHello,kind:"ready",sequence:1,payload:lh["payload"] as! [String:Any],reply:lh["message_id"] as? String))
    let largeParts=(0..<4).map {i in ["kind":"text","part_id":"large-part-\(i)","text":String(repeating:"x",count:250_000)]}
    let largeMessage:[String:Any]=["record_type":"canonical_agent_message","message_version":1,"message_id":"large-message","role":"assistant","agent_turn_id":"large-turn","task_id":largeHello.scope.task_id,"parts":largeParts]
    let largeFrame=try frame(largeHello,kind:"transcript_event",sequence:2,payload:["event_id":"large-event","source_sequence":1,"message":largeMessage])
    precondition(largeFrame.count<AgentFrameBytes.maximum);try largeFrame.write(to:root.appendingPathComponent("large-valid-canonical-frame.json"))
    do {_ = try await actualService.receive(largeFrame);fatalError("oversized ledger envelope ACKed")} catch StorageError.corruptIdentity { }
    let largeEvents=try await storage.sessionEvents("large-session");precondition(largeEvents.isEmpty)
    let peerEvidence=try JSONSerialization.jsonObject(with:Data(contentsOf:output)) as! [String:Any]
    let environmentKeys=Set(peerEvidence["environment_keys"] as! [String])
    precondition(environmentKeys.contains("PATH") && environmentKeys.isSubset(of:Set(["PATH","__CF_USER_TEXT_ENCODING"])))
    let unchanged=try await storage.committedSource(document)!;precondition(unchanged.snapshot_hash==snapshot.snapshot_hash)
    // One-byte framing, truncated EOF and poison are tested on original UTF8 bytes.
    var decoder=AgentFrameBytes();var decodedFrames:[Data]=[]
    for byte in event+Data([10]) {decodedFrames+=try decoder.push(Data([byte]))};try decoder.finish();precondition(decodedFrames==[event])
    var broken=AgentFrameBytes();_ = try broken.push(Data("{\"x\":".utf8));do {try broken.finish();fatalError("partial EOF accepted")} catch AgentSessionError.invalidFrame { }
    await actualService.close();try await pipeHost.close();try await storage.close()
    let (restored,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));_ = try await restored.openDocument(document)
    let original=try await restored.sessionEvent(session,event:"complete-event")!
    precondition(original.receipt.recordHash==actual.receipt.recordHash)
    let recoveredHost=try await NativeHostClient.open();let recoveredOpen=try await recoveredHost.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let recovery=AgentSessionService(host:recoveredHost,storage:restored,document:document,generation:recoveredOpen.document_generation.value)
    let recoveryHello=try await recovery.activate(session:session,grants:[],readonly:true,coreVersion:"1.0.4",adapterVersion:"adapter-1")
    do {_ = try await recovery.receive(event);fatalError("old runtime recovered authority")} catch NativeHostClientError.rejected { }
    let rh=try JSONSerialization.jsonObject(with:Data(recoveryHello.outbound_json.value!.utf8)) as! [String:Any]
    _ = try await recovery.receive(frame(recoveryHello,kind:"ready",sequence:1,payload:rh["payload"] as! [String:Any],reply:rh["message_id"] as? String))
    do {_ = try await recovery.receive(frame(recoveryHello,kind:"tool_call",sequence:2,payload:["agent_turn_id":"turn","tool_call_id":"model-call","proposal_event_ref":"complete-event"]));fatalError("recovery reinstated proposal/grant")} catch NativeHostClientError.rejected { }
    var ledgerEvidence:[[String:Any]]=[]
    for selected in [session,"pipe-session","large-session"]+modes.map({"bad-"+$0}) {
      let events=try await restored.sessionEvents(selected,limit:32)
      ledgerEvidence.append(["session":selected,"events":try events.map {stored in ["event":try object(stored.event),"receipt":try object(stored.receipt)]}])
    }
    try JSONSerialization.data(withJSONObject:ledgerEvidence,options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-ledger-events.json"))
    await recovery.close();try await recoveredHost.close();try await restored.close()
    let proof:[String:Any]=["fixture":true,"private_process_frames":report.frames,"negative_private_process_cases":modes,"last_phase":report.lastPhase.rawValue,"model_requests_dispatched":0,"tool_effects_executed":0,"canonical_events":pipeEvents.count,"valid_frame_exceeding_ledger_budget_denied_without_ACK":largeFrame.count,"source_unchanged":true,"lost_ack_reconciled":true,"cancel_did_not_wait_for_SQLite":true,"outbound_sequence_buffer_verified":true,"recovery_did_not_restore_authority":true,"peer_frames_file":output.path]
    print(String(decoding:try JSONSerialization.data(withJSONObject:proof,options:[.sortedKeys]),as:UTF8.self))
  }
}
