import Foundation
import CryptoKit

func encoded<T:Encodable>(_ value:T)throws->Data {try JSONEncoder().encode(value)}
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

/// Test-only bridge through the actual Rust library, not a replacement model/provider adapter.
private func codecMessage(executable:URL,root:URL,input:AgentCanonicalMessage,intent:NativeAgentRequestIntent,task:String)throws->AgentCanonicalMessage {
  let chunk:[String:Any] = ["id":"provider-response","model":"synthetic-served-alias","choices":[["index":0,"delta":["role":"assistant","content":"中文🙂","tool_calls":[["index":0,"id":"provider-call","type":"function","function":["name":"read_notebook","arguments":"{ \"view\" : \"outline\" }"]]]],"finish_reason":"tool_calls"]]]
  let sse="data: "+String(decoding:try JSONSerialization.data(withJSONObject:chunk),as:UTF8.self)+"\n\ndata: [DONE]\n\n"
  let binding:[String:Any] = ["provider_ref":"fixture-provider","route_ref":"fixture-route","model_ref":"fixture-model","adapter_version":"single-turn-1","protocol":"chat_completions"]
  let frame:[String:Any] = ["request":["binding":binding,"api_model":"synthetic-configured-alias","messages":[["canonical":try JSONSerialization.jsonObject(with:encoded(input)),"replay":NSNull()]],"results":[],"tools":[["name":"read_notebook","description":"Test-only unavailable native handler","parameters":["type":"object"]]],"max_output_tokens":128,"temperature":NSNull(),"stream":true],"message_id":"provider-canonical","task_id":task,"turn_id":intent.agent_turn_id,"status":200,"response":sse]
  let bytes=try JSONSerialization.data(withJSONObject:frame,options:[.sortedKeys]),process=Process(),stdin=Pipe(),stdout=Pipe()
  process.executableURL=executable;process.arguments=[];process.standardInput=stdin;process.standardOutput=stdout;process.standardError=FileHandle.nullDevice
  try process.run();try stdin.fileHandleForWriting.write(contentsOf:bytes);try stdin.fileHandleForWriting.close()
  let actual=stdout.fileHandleForReading.readDataToEndOfFile();process.waitUntilExit()
  guard process.terminationStatus==0,actual.count<=1024*1024,let object=try JSONSerialization.jsonObject(with:actual) as? [String:Any],let result=object["result"] as? [String:Any],result["finish"] as? String=="tool_calls",let message=result["message"] else {throw AgentSessionError.invalidFrame}
  try bytes.write(to:root.appendingPathComponent("actual-codec-input.json"));try actual.write(to:root.appendingPathComponent("actual-codec-output.json"))
  return try JSONDecoder().decode(AgentCanonicalMessage.self,from:JSONSerialization.data(withJSONObject:message))
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
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value)
    let verified=try await AgentProcess.verified(package:package,signatureRoot:CommandLine.arguments.count>3 && !CommandLine.arguments[3].isEmpty ? URL(fileURLWithPath:CommandLine.arguments[3]):nil)
    let codecExecutable=URL(fileURLWithPath:CommandLine.arguments[4])
    let hello=try await service.activate(session:"production-task",grants:[.read_notebook],readonly:true,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
    let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"user-input",role:.user,agent_turn_id:.init("input-turn"),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"text",text:"真实Pi，合成模型，宿主工具暂不可用🙂"))])
    let task=hello.scope.task_id
    let port=AgentNativeModelPort(prepare:{_ in .init(context:"fixture-context",model:"fixture-model")},stream:{intent in
      let message=try codecMessage(executable:codecExecutable,root:root,input:input,intent:intent,task:task)
      return AsyncThrowingStream {continuation in
        let delta=message.parts.compactMap {part->String? in if case .agent_text_part(let text)=part {return text.text};return nil}.joined()
        continuation.yield(.init(agent_turn_id:intent.agent_turn_id,request_id:intent.request_id,byte_offset:try! HostSerial(0),text:delta,phase:.delta,canonical_message:.init(nil)))
        continuation.yield(.init(agent_turn_id:intent.agent_turn_id,request_id:intent.request_id,byte_offset:try! HostSerial(UInt64(delta.utf8.count)),text:"",phase:.finished,canonical_message:.init(message)));continuation.finish()
      }
    })
    let pipe=try await verified.launch(),wire=WireCapture();await pipe.setCapture {bytes,direction in await wire.append(bytes,toHelper:direction)};let report=try await service.run(pipe:pipe,hello:hello,modelPort:port,input:input)
    try await wire.save(root.appendingPathComponent("actual-production-wire.json"))
    precondition(report.frames==6 && report.lastPhase == .awaiting_settlement && report.toolIntents.count==1)
    let events=try await storage.sessionEvents("production-task");precondition(events.count==3 && events.map(\.event.event_kind)==[.user_message,.assistant_end,.recovery])
    let canonical=try JSONDecoder().decode(SessionCanonicalAssistant.self,from:Data(events[1].event.payload_inline_json.value!.utf8))
    precondition(canonical.calls[0].arguments==Data("{ \"view\" : \"outline\" }".utf8))
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
    await service.close();await source.close();try await host.close();try await storage.close()
    print(String(decoding:try JSONSerialization.data(withJSONObject:["production_core":"1.0.4","bundled_node":"26.11.1","actual_private_frames":report.frames,"canonical_events":events.count,"model_transport":"actual_rust_codec_synthetic_provider_wire","tool_outcome":"unavailable_no_effect","manual_after_sigkill":"4","system_node_fallback":false,"PATH":"empty","wrong_architecture":"rejected","missing_resource":"rejected","late_stdin":"rejected","unconfigured_model":"no_fake_message_or_effect","model_cancel_and_late_delta":"rejected","actual_macho_architecture":"rejected","evidence_root":root.path],options:[.sortedKeys]),as:UTF8.self))
  }
}
