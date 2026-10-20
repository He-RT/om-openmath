import Foundation
import Security

private struct Script:Sendable {
  var status=200, chunks:[Data]=[], delay:Double=0.005, noHeaders=false, noFinish=false, heartbeat=false
  var redirect:URL?
  var beforeFinish:(@Sendable ()async->Void)?
}
private final class FixtureState:@unchecked Sendable {
  let lock=NSLock();var script=Script(),requests:[URLRequest]=[],stops=0
  private var history:[[String:Any]]=[]
  private func snapshot()->[String:Any] {
    ["script":["status":script.status,"response_chunks_base64":script.chunks.map{$0.base64EncodedString()},"delay":script.delay,"no_headers":script.noHeaders,"no_finish":script.noFinish,"heartbeat":script.heartbeat,"redirect":script.redirect?.absoluteString as Any? ?? NSNull()],"requests":requests.map{["url":$0.url!.absoluteString,"method":$0.httpMethod ?? "","headers":$0.allHTTPHeaderFields ?? [:],"body_base64":$0.httpBody?.base64EncodedString() ?? ""]},"stop_calls":stops]
  }
  func reset(_ script:Script){lock.lock();history.append(snapshot());self.script=script;requests=[];stops=0;lock.unlock()}
  func started(_ request:URLRequest)->Script {
    var actual=request
    if actual.httpBody==nil,let stream=actual.httpBodyStream {
      stream.open();defer {stream.close()};var bytes=Data(),buffer=[UInt8](repeating:0,count:4096)
      while true {let count=stream.read(&buffer,maxLength:buffer.count);precondition(count>=0);if count==0{break};bytes.append(contentsOf:buffer.prefix(count));precondition(bytes.count<=2*1024*1024)}
      actual.httpBody=bytes
    }
    lock.lock();defer {lock.unlock()};requests.append(actual);return script
  }
  func stopped(){lock.lock();stops+=1;lock.unlock()}
  var count:Int {lock.lock();defer {lock.unlock()};return requests.count}
  var captured:[URLRequest]{lock.lock();defer {lock.unlock()};return requests}
  func save(_ url:URL)throws {lock.lock();let value=history+[snapshot()];lock.unlock();try JSONSerialization.data(withJSONObject:value,options:[.sortedKeys]).write(to:url)}
}
private actor Evidence {
  static let shared=Evidence()
  var facts:[[String:Any]]=[],frames:[String]=[],sockets:[[String:Any]]=[]
  func record(_ fact:ProviderJournalFact){facts.append(["phase":fact.phase.rawValue,"request":fact.request,"context":fact.context,"status":fact.status.map{Int($0)} as Any? ?? NSNull(),"outcome":fact.outcome.rawValue])}
  func frame(_ event:AgentModelEventPayload)throws {frames.append(String(decoding:try JSONEncoder().encode(event),as:UTF8.self))}
  func socket(request:Data,response:Data,elapsed:Double){sockets.append(["request_raw_base64":request.base64EncodedString(),"response_sse_base64":response.base64EncodedString(),"connection_budget_seconds":1,"configured_response_delay_seconds":1.3,"elapsed_seconds":elapsed])}
  func save(_ root:URL)throws {try JSONSerialization.data(withJSONObject:["scope":"synthetic URLProtocol and credential; actual URLSession/Rust C ABI/Agent port; owned actual loopback socket","journal_callbacks":facts,"agent_frames_json":frames,"socket_cases":sockets],options:[.sortedKeys]).write(to:root.appendingPathComponent("actual-native-events.json"))}
}
private final class FixtureProtocol:URLProtocol,@unchecked Sendable {
  static let state=FixtureState()
  private var work:Task<Void,Never>?
  private let workLock=NSLock()
  private var stopped=false
  override class func canInit(with request:URLRequest)->Bool {request.url?.host=="unit.fixture.invalid"}
  override class func canonicalRequest(for request:URLRequest)->URLRequest {request}
  override func startLoading() {
    let script=Self.state.started(request)
    let worker=Task.detached { @Sendable [self] in
      do {
        try await Task.sleep(for:.seconds(script.delay))
        if let target=script.redirect {
          let response=HTTPURLResponse(url:request.url!,statusCode:302,httpVersion:"HTTP/1.1",headerFields:["Location":target.absoluteString])!
          client?.urlProtocol(self,wasRedirectedTo:URLRequest(url:target),redirectResponse:response);return
        }
        if !script.noHeaders {
          let response=HTTPURLResponse(url:request.url!,statusCode:script.status,httpVersion:"HTTP/1.1",headerFields:["Content-Type":"text/event-stream"])!
          client?.urlProtocol(self,didReceive:response,cacheStoragePolicy:.notAllowed)
        }
        for chunk in script.chunks {
          try Task.checkCancellation();client?.urlProtocol(self,didLoad:chunk);try await Task.sleep(for:.seconds(script.delay))
        }
        if script.heartbeat {while !Task.isCancelled {client?.urlProtocol(self,didLoad:Data(": ping\n\n".utf8));try await Task.sleep(for:.milliseconds(5))}}
        if !script.noFinish {await script.beforeFinish?();try Task.checkCancellation();client?.urlProtocolDidFinishLoading(self)}
      } catch { }
    }
    workLock.withLock {if stopped {worker.cancel()} else {work=worker}}
  }
  override func stopLoading(){workLock.withLock {stopped=true;work?.cancel()};Self.state.stopped()}
}
private actor Trace {
  var phases:[ProviderJournalFact]=[],text=""
  var results:[ProviderModelResult]=[]
  var hold=false,arrived=false
  private var paused:CheckedContinuation<Void,Never>?
  func record(_ fact:ProviderJournalFact)async throws {
    phases.append(fact)
    await Evidence.shared.record(fact)
    if fact.phase == .dispatchStarted {arrived=true;if hold {await withCheckedContinuation {paused=$0}}}
  }
  func append(_ value:String){text+=value}
  func complete(_ result:ProviderModelResult){results.append(result)}
  func setHold(){hold=true}
  func release(){hold=false;paused?.resume();paused=nil}
  func wait()async throws {for _ in 0..<500 {if arrived{return};try await Task.sleep(for:.milliseconds(5))};throw ProviderTransportReason.journal}
  func waitResponse()async throws {for _ in 0..<500 {if phases.contains(where:{$0.phase == .responseStarted}) {return};try await Task.sleep(for:.milliseconds(5))};throw ProviderTransportReason.journal}
  func journal()->ProviderTransportJournal {.init(record:{[self] fact in try await record(fact)},responseBytes:{_ in})}
}
private actor FixtureGate {
  var open=false
  private var waiters:[CheckedContinuation<Void,Never>]=[]
  func wait()async {if !open {await withCheckedContinuation {waiters.append($0)}}}
  func release(){open=true;let pending=waiters;waiters.removeAll();for waiter in pending {waiter.resume()}}
}
func json(_ object:Any)throws->String {String(decoding:try JSONSerialization.data(withJSONObject:object,options:[.sortedKeys]),as:UTF8.self)}
func chat(_ text:String,reason:String="stop")throws->Data {
  Data(("data: "+(try json(["id":"provider-original","model":"provider-served-alias","choices":[["index":0,"delta":["role":"assistant","content":text],"finish_reason":reason]]]))+"\n\ndata: [DONE]\n\n").utf8)
}
private func prepare(_ host:NativeHostClient,_ suffix:String,wireProtocol:String="chat_completions")async throws->ProviderModelSession {
  let input:[String:Any] = ["record_type":"canonical_agent_message","message_version":1,"message_id":"user-"+suffix,"role":"user","task_id":"task","agent_turn_id":"turn-"+suffix,"parts":[["kind":"text","part_id":"text","text":"中文🙂"]]]
  let request:[String:Any] = ["binding":["provider_ref":"provider","route_ref":"route","model_ref":"model","adapter_version":"single-turn-1","protocol":wireProtocol],"api_model":"configured-alias","messages":[["canonical":input,"replay":NSNull()]],"results":[],"tools":[],"max_output_tokens":64,"temperature":NSNull(),"stream":true]
  return try await ProviderModelSession.prepare(host:host,requestJSON:json(request),request:"request-"+suffix,context:"context-"+suffix,task:"task",turn:"turn-"+suffix,message:"native-"+suffix)
}
private func route(_ model:ProviderModelSession,_ timeouts:ProviderTimeouts) throws->ProviderRoute {
  try ProviderRoute(binding:model.binding,endpoint:URL(string:"https://unit.fixture.invalid/v1/chat/completions")!,authentication:.bearer,credentialReference:"fixture-secret",timeouts:timeouts)
}
private func failure(_ body:@Sendable ()async throws->ProviderModelResult)async throws->ProviderTransportFailure {
  do {_ = try await body();fatalError("request unexpectedly succeeded")}catch let error as ProviderTransportFailure{return error}
}
@main struct ProviderTransportFixtures {
  static func main()async throws {
    let host=try await NativeHostClient.open(),secrets=SecretStore(testReader:{_ in (0,Data("synthetic-secret".utf8))})
    let transport=ProviderTransport(secrets:secrets,testConfiguration:{let config=URLSessionConfiguration.ephemeral;config.protocolClasses=[FixtureProtocol.self];return config})
    let normal=try ProviderTimeouts(firstResponse:3,idle:3,total:5)
    let trace=Trace(),model=try await prepare(host,"success"),wire=try chat("中文🙂完整")
    // Split in the middle of a real UTF8 scalar; the C ABI receives unchanged bytes.
    let cut=wire.firstIndex(of:0xf0)!+1
    FixtureProtocol.state.reset(Script(chunks:[wire.prefix(cut),wire.dropFirst(cut)]))
    let result=try await transport.send(model,route:route(model,normal),journal:await trace.journal(),onText:{await trace.append($0)})
    precondition(result.finish=="complete" && result.message.message_id=="native-success")
    let observedText=await trace.text;precondition(observedText=="中文🙂完整")
    let captured=FixtureProtocol.state.captured;precondition(captured.count==1)
    precondition(captured[0].value(forHTTPHeaderField:"Authorization")=="Bearer synthetic-secret")
    let body=try JSONSerialization.jsonObject(with:captured[0].httpBody!) as! [String:Any];precondition(body["model"] as? String=="configured-alias" && body["api_key"]==nil)
    let phases=await trace.phases.map(\.phase);precondition(phases==[.dispatchStarted,.responseStarted,.finished])
    let anthropic=try await prepare(host,"anthropic",wireProtocol:"anthropic_messages"),anthropicTrace=Trace()
    let events:[[String:Any]] = [
      ["type":"message_start","message":["id":"anthropic-original","model":"served-model","role":"assistant","content":[],"usage":["input_tokens":7,"output_tokens":1]]],
      ["type":"content_block_start","index":0,"content_block":["type":"text","text":""]],
      ["type":"content_block_delta","index":0,"delta":["type":"text_delta","text":"中文🙂"]],
      ["type":"content_block_stop","index":0],
      ["type":"message_delta","delta":["stop_reason":"end_turn"],"usage":["output_tokens":9]],
      ["type":"message_stop"],
    ]
    let sse=try events.map {"event: "+($0["type"] as! String)+"\ndata: "+(try json($0))+"\n\n"}.joined()
    FixtureProtocol.state.reset(Script(chunks:[Data(sse.utf8)]))
    let anthRoute=try ProviderRoute(binding:anthropic.binding,endpoint:URL(string:"https://unit.fixture.invalid/v1/messages")!,authentication:.anthropicAPIKey,credentialReference:"fixture-secret",timeouts:normal)
    let anthResult=try await transport.send(anthropic,route:anthRoute,journal:await anthropicTrace.journal(),onText:{_ in})
    precondition(anthResult.finish=="complete" && anthResult.providerReason=="end_turn")
    let anthRequest=FixtureProtocol.state.captured[0];precondition(anthRequest.value(forHTTPHeaderField:"x-api-key")=="synthetic-secret" && anthRequest.value(forHTTPHeaderField:"Authorization")==nil && anthRequest.value(forHTTPHeaderField:"anthropic-version")=="2023-06-01")
    let anthBody=try JSONSerialization.jsonObject(with:anthRequest.httpBody!) as! [String:Any];precondition(anthBody["max_tokens"] as? Int==64 && anthBody["max_completion_tokens"]==nil)
    FixtureProtocol.state.reset(Script(chunks:[wire]))
    let repeatFailure=try await failure {try await transport.send(model,route:route(model,normal),journal:await trace.journal(),onText:{_ in})};precondition(repeatFailure.reason == .stale && FixtureProtocol.state.count==0)
    let locked=try await prepare(host,"locked"),lockedTrace=Trace()
    let lockedTransport=ProviderTransport(secrets:SecretStore(testReader:{_ in (errSecInteractionNotAllowed,Data("must-not-use".utf8))}),testConfiguration:{let c=URLSessionConfiguration.ephemeral;c.protocolClasses=[FixtureProtocol.self];return c})
    let denied=try await failure {try await lockedTransport.send(locked,route:route(locked,normal),journal:await lockedTrace.journal(),onText:{_ in})}
    precondition(denied.reason == .secret && denied.outcome == .notDispatched && FixtureProtocol.state.count==0)
    for status in [401,429] {
      let model=try await prepare(host,"http-\(status)"),trace=Trace()
      FixtureProtocol.state.reset(Script(status:status,chunks:[Data("synthetic-secret provider error echo".utf8)]))
      let error=try await failure {try await transport.send(model,route:route(model,normal),journal:await trace.journal(),onText:{_ in fatalError("error body emitted text")})}
      precondition(error.reason == .http && error.httpStatus==UInt16(status) && error.outcome == .responseObserved && FixtureProtocol.state.count==1)
      precondition(!String(describing:error).contains("synthetic-secret"))
    }
    let redirected=try await prepare(host,"redirect"),redirectTrace=Trace()
    FixtureProtocol.state.reset(Script(redirect:URL(string:"https://other.invalid/credential-sink")!))
    let redirectError=try await failure {try await transport.send(redirected,route:route(redirected,normal),journal:await redirectTrace.journal(),onText:{_ in})}
    precondition(redirectError.reason == .redirect && redirectError.httpStatus==302 && FixtureProtocol.state.count==1)
    for (name,script,timeouts,reason) in [
      ("connect",Script(delay:0.2,noHeaders:true,noFinish:true),try ProviderTimeouts(connection:0.03,firstResponse:1,idle:1,total:1),ProviderTransportReason.connectionTimeout),
      ("first",Script(delay:0.2,noHeaders:true,noFinish:true),try ProviderTimeouts(firstResponse:0.03,idle:1,total:1),ProviderTransportReason.firstResponseTimeout),
      ("idle",Script(noFinish:true),try ProviderTimeouts(firstResponse:1,idle:0.03,total:1),.idleTimeout),
      // The total case holds competing timer callbacks below. Header delivery can legitimately
      // exceed 30ms on CI; the independent first/idle negatives keep their original budgets.
      ("total",Script(noFinish:true,heartbeat:true),try ProviderTimeouts(firstResponse:0.03,idle:1,total:0.1),.totalTimeout),
      // Force the observed CI ordering: the first timer wakes before any headers can arrive.
      ("total-delayed-header",Script(delay:0.2,noFinish:true),try ProviderTimeouts(firstResponse:0.03,idle:1,total:0.1),.totalTimeout),
    ] {
      let model=try await prepare(host,name),trace=Trace();FixtureProtocol.state.reset(script)
      let competingTimers=FixtureGate(),competingTimerWoke=FixtureGate(),isTotal=name.hasPrefix("total")
      // Exercise the actual total timer without racing an unrelated first-response/idle timer.
      // The existing trusted wake probe pauses only those competing callbacks; production
      // deadlines and the total timer itself are unchanged. Release even if the assertion fails.
      let timeoutTransport=isTotal ? ProviderTransport(secrets:secrets,testConfiguration:{let c=URLSessionConfiguration.ephemeral;c.protocolClasses=[FixtureProtocol.self];return c},testTimeoutProbe:.init(woke:{reason in if reason != .totalTimeout {await competingTimerWoke.release();await competingTimers.wait()}},settled:{_ in})):transport
      let error:ProviderTransportFailure
      do {error=try await failure {try await timeoutTransport.send(model,route:route(model,timeouts),journal:await trace.journal(),onText:{_ in})}}
      catch {await competingTimers.release();throw error}
      await competingTimers.release()
      let actuallyHeld=await competingTimerWoke.open
      if name == "total-delayed-header" {precondition(actuallyHeld && error.outcome == .dispatchOutcomeUnknown)}
      if CommandLine.arguments.count>1 {
        let observation:[String:Any] = ["case":name,"expected_reason":reason.rawValue,"actual_reason":error.reason.rawValue,"outcome":error.outcome.rawValue,"requests":FixtureProtocol.state.count,"competing_timer_callbacks_held":isTotal,"competing_timer_actually_woke":actuallyHeld]
        try JSONSerialization.data(withJSONObject:observation,options:[.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true).appendingPathComponent("timeout-"+name+".json"))
      }
      precondition(error.reason==reason && error.outcome != .notDispatched && FixtureProtocol.state.count==1)
    }
    // Wake the real first-response timer, then accept actual headers while that callback is held.
    // Opening EOF only AFTER the old callback settles distinguishes a stale timeout from success.
    let timerWoke=FixtureGate(),timerRelease=FixtureGate(),timerSettled=FixtureGate(),finish=FixtureGate()
    let staleTransport=ProviderTransport(secrets:secrets,testConfiguration:{let c=URLSessionConfiguration.ephemeral;c.protocolClasses=[FixtureProtocol.self];return c},testTimeoutProbe:.init(woke:{reason in if reason == .firstResponseTimeout {await timerWoke.release();await timerRelease.wait()}},settled:{reason in if reason == .firstResponseTimeout {await timerSettled.release()}}))
    let staleModel=try await prepare(host,"stale-timer"),staleTrace=Trace()
    FixtureProtocol.state.reset(Script(chunks:[wire],delay:0.1,beforeFinish:{await finish.wait()}))
    let staleTask=Task {try await staleTransport.send(staleModel,route:route(staleModel,try ProviderTimeouts(firstResponse:0.03,idle:3,total:5)),journal:await staleTrace.journal(),onText:{_ in})}
    await timerWoke.wait();try await staleTrace.waitResponse();await timerRelease.release();await timerSettled.wait()
    if CommandLine.arguments.count>1 {
      let observation:[String:Any] = ["timer_woke":await timerWoke.open,"response_observed":true,"old_callback_settled":await timerSettled.open,"eof_still_held":!(await finish.open),"requests":FixtureProtocol.state.count]
      try JSONSerialization.data(withJSONObject:observation,options:[.sortedKeys]).write(to:URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true).appendingPathComponent("stale-timer-before-eof.json"))
    }
    await finish.release()
    let staleResult=try await staleTask.value;precondition(staleResult.finish=="complete" && FixtureProtocol.state.count==1)
    let held=try await prepare(host,"held"),heldTrace=Trace();await heldTrace.setHold();FixtureProtocol.state.reset(Script())
    let heldTask=Task {try await transport.send(held,route:route(held,normal),journal:await heldTrace.journal(),onText:{_ in})}
    try await heldTrace.wait();precondition(FixtureProtocol.state.count==0);heldTask.cancel();await heldTrace.release()
    do {_ = try await heldTask.value;fatalError("cancelled pre-resume succeeded")}catch let e as ProviderTransportFailure {precondition(e.reason == .cancelled && e.outcome == .dispatchOutcomeUnknown)}
    precondition(FixtureProtocol.state.count==0)
    let delayedWire=try chat("actual delayed headers🙂"),delayedServer=try DelayedLoopbackServer(response:delayedWire,delay:1.3)
    let delayedModel=try await prepare(host,"delayed-headers"),delayedTrace=Trace()
    let delayedRoute=try ProviderRoute(binding:delayedModel.binding,endpoint:URL(string:"http://127.0.0.1:\(delayedServer.port)/v1/chat/completions")!,authentication:.none,credentialReference:nil,timeouts:try ProviderTimeouts(connection:1,firstResponse:4,idle:3,total:5),allowHTTPOnLiteralLoopback:true)
    let started=ContinuousClock.now
    let delayed=try await transport.send(delayedModel,route:delayedRoute,journal:await delayedTrace.journal(),onText:{_ in})
    let duration=started.duration(to:.now),elapsed=Double(duration.components.seconds)+Double(duration.components.attoseconds)/1e18
    precondition(delayedServer.receivedBody && delayed.finish=="complete")
    precondition(elapsed>=1.3)
    await Evidence.shared.socket(request:delayedServer.requestBytes,response:delayedWire,elapsed:elapsed)
    let active=try await prepare(host,"active"),activeTrace=Trace();FixtureProtocol.state.reset(Script(noFinish:true,heartbeat:true))
    let task=Task {try await transport.send(active,route:route(active,normal),journal:await activeTrace.journal(),onText:{_ in})}
    for _ in 0..<200 {if FixtureProtocol.state.count>0{break};try await Task.sleep(for:.milliseconds(5))}
    // Real CAS request progresses while model transport is still running.
    let request=RequestEnvelope(protocol_version:1,runtime_instance_id:host.runtimeInstanceID,request_ref:"parallel-cas",operation_id:.init("parallel-cas"),document_binding:.init(nil),task_binding:.init(nil),body:.evaluate_scratch(.init(kind:.evaluate_scratch,source:"2+2",dialect:.modern,definition_snapshot_ref:.init(nil),use_notebook_definitions:false,timeout_ms:1000)))
    _ = try await host.submit(request)
    var computed=false
    for _ in 0..<100 {
      let batch=try await host.nextEvents()
      if let event=batch.events.first(where:{$0.operation_ref.value=="parallel-cas" && $0.event_kind == .operation_finished}) {
        let payload=try JSONSerialization.jsonObject(with:JSONEncoder().encode(event.payload)) as! [String:Any]
        let value=payload["result"] as! [String:Any],response=value["response"] as! [String:Any],output=response["output"] as! [String:Any],items=output["items"] as! [[String:Any]]
        precondition(items[0]["input_form"] as? String=="4");computed=true;break
      }
    }
    precondition(computed);task.cancel()
    do {_ = try await task.value;fatalError("cancelled active request succeeded")}catch let e as ProviderTransportFailure {precondition(e.reason == .cancelled)}
    let malformed=try await prepare(host,"malformed"),malTrace=Trace();FixtureProtocol.state.reset(Script(chunks:[Data("data: {\"error\":{\"message\":\"synthetic-secret\"}}\n\n".utf8)]))
    let mal=try await failure {try await transport.send(malformed,route:route(malformed,normal),journal:await malTrace.journal(),onText:{_ in})};precondition(mal.reason == .credentialEcho && !String(describing:mal).contains("synthetic-secret"))
    for url in ["http://outside.invalid/v1","https://user:secret@unit.fixture.invalid/v1","https://unit.fixture.invalid/v1?key=secret","file:///tmp/model","http://localhost/v1"] {
      do {_ = try ProviderRoute(binding:model.binding,endpoint:URL(string:url)!,authentication:.none,credentialReference:nil,timeouts:normal,allowHTTPOnLiteralLoopback:true);fatalError("unsafe route accepted")}catch ProviderTransportReason.invalidRoute { }
    }
    _ = try ProviderRoute(binding:model.binding,endpoint:URL(string:"http://127.0.0.1:12345/v1")!,authentication:.none,credentialReference:nil,timeouts:normal,allowHTTPOnLiteralLoopback:true)
    for (name,reason) in [("adapter","stop"),("truncated","length")] {
      let trace=Trace(),original=String(repeating:"中🙂",count:6000)
      let adapter=ProviderAgentModelAdapter(transport:transport,factory:{_ in
        let model=try await prepare(host,name)
        return ProviderPreparedTurn(model:model,route:try route(model,normal),journal:await trace.journal(),complete:{await trace.complete($0)})
      })
      let port=adapter.port(),intent=NativeAgentRequestIntent(operation_id:"operation-"+name,agent_turn_id:"turn-"+name,request_id:"request-"+name)
      FixtureProtocol.state.reset(Script(chunks:[try chat(original,reason:reason)]))
      let prepared=try await port.prepare(intent);precondition(prepared.context=="context-"+name && prepared.model=="model")
      var offset:UInt64=0,text="",terminal:AgentModelEventPayload?
      for try await event in try await port.stream(intent) {
        try await Evidence.shared.frame(event)
        precondition(event.byte_offset.value==offset && event.request_id==intent.request_id && event.agent_turn_id==intent.agent_turn_id)
        if event.phase == .delta {precondition(event.text.utf8.count<=32768);offset+=UInt64(event.text.utf8.count);text+=event.text} else {terminal=event}
      }
      precondition(text==original && terminal?.phase==(reason=="stop" ? .finished:.failed))
      precondition((terminal?.canonical_message.value != nil)==(reason=="stop"))
      let observed=await trace.results;precondition(observed.count==1 && observed[0].finish==(reason=="stop" ? "complete":"incomplete"))
      await adapter.close()
    }
    // Fast token fragments must survive a slower native message/ledger consumer.
    // The byte limit, exact offsets and terminal canonical message still apply.
    do {
      let trace=Trace(),name="adapter-burst",parts=(0..<160).map{"\($0)中🙂"}
      let adapter=ProviderAgentModelAdapter(transport:transport,factory:{_ in
        let model=try await prepare(host,name)
        return ProviderPreparedTurn(model:model,route:try route(model,normal),journal:await trace.journal(),complete:{await trace.complete($0)})
      })
      let port=adapter.port(),intent=NativeAgentRequestIntent(operation_id:"operation-"+name,agent_turn_id:"turn-"+name,request_id:"request-"+name)
      var bytes=Data()
      for part in parts {bytes.append(try chat(part).dropLast("data: [DONE]\n\n".utf8.count))}
      // Only the final fragment ends this one response.
      var events=String(decoding:bytes,as:UTF8.self).replacingOccurrences(of:"\"finish_reason\":\"stop\"",with:"\"finish_reason\":null")
      events+=String(decoding:try chat("",reason:"stop"),as:UTF8.self)
      FixtureProtocol.state.reset(Script(chunks:[Data(events.utf8)]))
      _ = try await port.prepare(intent)
      var text="",offset:UInt64=0,terminal:AgentModelEventPayload?
      for try await event in try await port.stream(intent) {
        precondition(event.byte_offset.value==offset)
        if event.phase == .delta {text+=event.text;offset+=UInt64(event.text.utf8.count)} else {terminal=event}
        try await Task.sleep(for:.milliseconds(20))
      }
      precondition(text==parts.joined() && terminal?.phase == .finished && terminal?.canonical_message.value != nil)
      await adapter.close()
      let fragmented=try await prepare(host,"transport-fragment-burst"),fragmentTrace=Trace()
      let fragments=events.components(separatedBy:"\n\n").filter{!$0.isEmpty}.map{Data(($0+"\n\n").utf8)}
      FixtureProtocol.state.reset(Script(chunks:fragments))
      let result=try await transport.send(fragmented,route:route(fragmented,normal),journal:await fragmentTrace.journal(),onText:{
        await fragmentTrace.append($0);try await Task.sleep(for:.milliseconds(20))
      })
      let received=await fragmentTrace.text
      precondition(result.finish=="complete" && received==parts.joined())
    }
    do {
      let trace=Trace(),name="adapter-cancel"
      let adapter=ProviderAgentModelAdapter(transport:transport,factory:{_ in
        let model=try await prepare(host,name)
        return ProviderPreparedTurn(model:model,route:try route(model,normal),journal:await trace.journal(),complete:{await trace.complete($0)})
      })
      let port=adapter.port(),intent=NativeAgentRequestIntent(operation_id:"operation-"+name,agent_turn_id:"turn-"+name,request_id:"request-"+name)
      FixtureProtocol.state.reset(Script(noFinish:true,heartbeat:true))
      _ = try await port.prepare(intent)
      let stream=try await port.stream(intent)
      let consumer=Task {for try await _ in stream {}}
      for _ in 0..<200 {if FixtureProtocol.state.count>0 {break};try await Task.sleep(for:.milliseconds(5))}
      consumer.cancel()
      do {try await consumer.value;fatalError("cancelled native stream kept waiting")} catch is CancellationError {} catch let error as ProviderTransportFailure {precondition(error.reason == .cancelled)}
      await adapter.close()
    }
    let closingHost=try await NativeHostClient.open(),closingModel=try await prepare(closingHost,"closing"),closingTrace=Trace()
    FixtureProtocol.state.reset(Script(noFinish:true,heartbeat:true))
    let closing=Task {try await transport.send(closingModel,route:route(closingModel,normal),journal:await closingTrace.journal(),onText:{_ in})}
    for _ in 0..<200 {if FixtureProtocol.state.count>0{break};try await Task.sleep(for:.milliseconds(5))}
    try await closingHost.close()
    do {_ = try await closing.value;fatalError("host close left HTTP running")}catch let error as ProviderTransportFailure {precondition(error.reason == .cancelled)}
    try await host.close()
    if CommandLine.arguments.count>1 {
      let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
      try FixtureProtocol.state.save(root.appendingPathComponent("actual-http-cases.json"));try await Evidence.shared.save(root)
    }
    print("Actual Swift URLSession -> original raw bytes -> Rust C ABI codec: UTF8, actual body/final auth, durable-before-resume, 401/429/no retry/error echo, redirect, connection/first/idle/total timeout, real delayed-header loopback beyond connection budget, pre-resume/active cancel, concurrent real CAS, route boundaries passed. Keychain physical lock-screen pending.")
  }
}
