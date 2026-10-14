import Foundation

enum ProviderTransportReason: String, Error, Sendable { case invalidRoute, secret, credentialEcho, stale, codec, http, redirect, connectionTimeout, firstResponseTimeout, idleTimeout, totalTimeout, cancelled, network, limit, journal }
enum ProviderDispatchOutcome: String, Sendable { case notDispatched, dispatchOutcomeUnknown, responseObserved }
struct ProviderTransportFailure: Error, Sendable {
  let reason: ProviderTransportReason
  let outcome: ProviderDispatchOutcome
  let httpStatus: UInt16?
}
struct ProviderBinding: Equatable, Sendable {
  let provider: String, route: String, model: String, adapter: String, wireProtocol: String
  init(json: [String: Any]) throws {
    guard let provider=json["provider_ref"] as? String,let route=json["route_ref"] as? String,
      let model=json["model_ref"] as? String,let adapter=json["adapter_version"] as? String,
      let wireProtocol=json["protocol"] as? String else {throw ProviderTransportReason.stale}
    self.provider=provider;self.route=route;self.model=model;self.adapter=adapter;self.wireProtocol=wireProtocol
  }
}
struct ProviderTimeouts: Sendable {
  let connection: Double, firstResponse: Double, idle: Double, total: Double
  init(connection:Double=10,firstResponse:Double=60,idle:Double=30,total:Double=180)throws {
    guard [connection,firstResponse,idle,total].allSatisfy({$0.isFinite && $0>=0.01 && $0<=600}),total>=firstResponse else {throw ProviderTransportReason.invalidRoute}
    self.connection=connection;self.firstResponse=firstResponse;self.idle=idle;self.total=total
  }
}
/// Created by the native config/context preparer. It is never decoded from helper/model frames.
struct ProviderRoute: Sendable {
  let binding:ProviderBinding, endpoint:URL, authentication:ProviderAuthentication, credentialReference:String?
  let timeouts:ProviderTimeouts
  init(binding:ProviderBinding,endpoint:URL,authentication:ProviderAuthentication,credentialReference:String?,timeouts:ProviderTimeouts,allowHTTPOnLiteralLoopback:Bool=false)throws {
    guard let components=URLComponents(url:endpoint,resolvingAgainstBaseURL:false),
      let host=components.host,!host.isEmpty,components.user==nil,components.password==nil,
      components.query==nil,components.fragment==nil,
      components.port.map({(1...65535).contains($0)}) ?? true,
      components.scheme=="https" || (allowHTTPOnLiteralLoopback && components.scheme=="http" && ["127.0.0.1","[::1]","::1"].contains(host)),
      ["chat_completions","anthropic_messages"].contains(binding.wireProtocol) else {throw ProviderTransportReason.invalidRoute}
    switch authentication {
    case .none:guard credentialReference==nil else {throw ProviderTransportReason.invalidRoute}
    case .bearer,.anthropicAPIKey:guard let credentialReference,!credentialReference.isEmpty else {throw ProviderTransportReason.invalidRoute}
    }
    self.binding=binding;self.endpoint=endpoint;self.authentication=authentication;self.credentialReference=credentialReference;self.timeouts=timeouts
  }
}
struct ProviderModelResult:Sendable {
  let message:AgentCanonicalMessage, finish:String, providerReason:String
  let usageJSON:Data, replayJSON:Data?, partialCallsJSON:Data
}
/// A single frozen Rust channel. It is never an executable provider/helper/plugin loader.
actor ProviderModelSession {
  let binding:ProviderBinding,prepared:NativeModelCodecReply,body:Data
  private let host:NativeHostClient
  let lifetime=NativeModelLifetime()
  private var dispatchClaimed=false
  private init(host:NativeHostClient,binding:ProviderBinding,prepared:NativeModelCodecReply,body:Data){self.host=host;self.binding=binding;self.prepared=prepared;self.body=body}
  static func prepare(host:NativeHostClient,requestJSON:String,request:String,context:String,task:String,turn:String,message:String)async throws->ProviderModelSession {
    guard let data=requestJSON.data(using:.utf8),let json=try JSONSerialization.jsonObject(with:data) as? [String:Any],let source=json["binding"] as? [String:Any] else {throw ProviderTransportReason.stale}
    let binding=try ProviderBinding(json:source)
    let prepared=try await host.modelCodecCommand(.native_model_codec_prepare(.init(type:.model_codec_prepare,runtime_instance_id:host.runtimeInstanceID,request_ref:request,context_ref:context,task_id:task,agent_turn_id:turn,message_id:message,request_json:requestJSON)))
    guard prepared.phase == .prepared,prepared.request_ref==request,prepared.context_ref==context,prepared.task_id==task,prepared.agent_turn_id==turn,prepared.message_id==message,let body=prepared.body_json.value?.data(using:.utf8) else {throw ProviderTransportReason.codec}
    let model=ProviderModelSession(host:host,binding:binding,prepared:prepared,body:body)
    try await host.registerModelLifetime(prepared.codec_ref,model.lifetime)
    return model
  }
  func claim()throws {guard !dispatchClaimed else {throw ProviderTransportReason.stale};guard !lifetime.isClosed else {throw ProviderTransportReason.cancelled};dispatchClaimed=true}
  private func checked(_ reply:NativeModelCodecReply)throws->NativeModelCodecReply {
    guard reply.codec_ref==prepared.codec_ref,reply.request_ref==prepared.request_ref,reply.context_ref==prepared.context_ref,reply.task_id==prepared.task_id,reply.agent_turn_id==prepared.agent_turn_id,reply.message_id==prepared.message_id else {throw ProviderTransportReason.stale}
    guard reply.error_code.value==nil else {throw reply.error_code.value == .http ? ProviderTransportReason.http:ProviderTransportReason.codec}
    return reply
  }
  func headers(_ status:UInt16)async throws {
    _ = try checked(await host.modelCodecCommand(.native_model_codec_headers(.init(type:.model_codec_headers,runtime_instance_id:host.runtimeInstanceID,codec_ref:prepared.codec_ref,status:UInt32(status)))))
  }
  func feed(_ data:Data)async throws->[String] {
    var text:[String]=[]
    for start in stride(from:0,to:data.count,by:8192) {
      let part=data.subdata(in:start..<min(start+8192,data.count))
      let reply=try checked(await host.modelCodecCommand(.native_model_codec_feed(.init(type:.model_codec_feed,runtime_instance_id:host.runtimeInstanceID,codec_ref:prepared.codec_ref,bytes:part.map(UInt32.init)))))
      text.append(contentsOf:reply.text_deltas)
    }
    return text
  }
  func finish()async throws->ProviderModelResult {
    let reply=try checked(await host.modelCodecCommand(.native_model_codec_finish(.init(type:.model_codec_finish,runtime_instance_id:host.runtimeInstanceID,codec_ref:prepared.codec_ref))))
    guard let data=reply.result_json.value?.data(using:.utf8),let object=try JSONSerialization.jsonObject(with:data) as? [String:Any],let canonical=object["message"],let finish=object["finish"] as? String,["complete","tool_calls","incomplete"].contains(finish),let reason=object["provider_reason"] as? String,let usage=object["usage"] as? [String:Any],let partial=object["partial_calls"] as? [Any],let replay=object["replay"] else {throw ProviderTransportReason.codec}
    let message=try JSONDecoder().decode(AgentCanonicalMessage.self,from:JSONSerialization.data(withJSONObject:canonical))
    guard message.message_id==prepared.message_id,message.task_id.value==prepared.task_id,message.agent_turn_id.value==prepared.agent_turn_id else {throw ProviderTransportReason.stale}
    let result=ProviderModelResult(message:message,finish:finish,providerReason:reason,usageJSON:try JSONSerialization.data(withJSONObject:usage),replayJSON:replay is NSNull ? nil:try JSONSerialization.data(withJSONObject:replay),partialCallsJSON:try JSONSerialization.data(withJSONObject:partial))
    await host.removeModelLifetime(prepared.codec_ref);return result
  }
  func cancel()async {lifetime.close();_ = try? await host.modelCodecCommand(.native_model_codec_cancel(.init(type:.model_codec_cancel,runtime_instance_id:host.runtimeInstanceID,codec_ref:prepared.codec_ref)));await host.removeModelLifetime(prepared.codec_ref)}
}
enum ProviderJournalPhase:String,Sendable {case dispatchStarted,responseStarted,finished,failed,interrupted}
struct ProviderJournalFact:Sendable {
  let phase:ProviderJournalPhase,request:String,context:String,status:UInt16?,outcome:ProviderDispatchOutcome
}
struct ProviderTransportJournal:Sendable {
  /// The caller supplies actual durable request-lifecycle persistence. No automatic no-op default.
  let record:@Sendable (ProviderJournalFact)async throws->Void
  /// Original response bytes after the native auth-echo guard; no auth header is provided.
  let responseBytes:@Sendable (Data)async throws->Void
}
private enum ProviderHTTPEvent:Sendable {case headers(UInt16),bytes(Data)}
private final class ProviderHTTPPump:NSObject,URLSessionDataDelegate,@unchecked Sendable {
  private let lock=NSLock()
  private let continuation:AsyncThrowingStream<ProviderHTTPEvent,any Error>.Continuation
  private let timeouts:ProviderTimeouts
  private var closed=false,totalBytes=0,status:UInt16?,task:URLSessionDataTask?
  private var connection:Task<Void,Never>?,stage:Task<Void,Never>?,total:Task<Void,Never>?
  init(continuation:AsyncThrowingStream<ProviderHTTPEvent,any Error>.Continuation,timeouts:ProviderTimeouts){self.continuation=continuation;self.timeouts=timeouts}
  private func timer(_ seconds:Double,_ reason:ProviderTransportReason)->Task<Void,Never> {
    Task { [weak self] in do {try await Task.sleep(for:.seconds(seconds));self?.stop(reason)} catch { } }
  }
  func start(_ task:URLSessionDataTask) {
    lock.lock();guard !closed else {lock.unlock();task.cancel();return}
    self.task=task;connection=timer(timeouts.connection,.connectionTimeout);stage=timer(timeouts.firstResponse,.firstResponseTimeout);total=timer(timeouts.total,.totalTimeout);lock.unlock();task.resume()
  }
  var observedStatus:UInt16? {lock.lock();defer {lock.unlock()};return status}
  func stop(_ reason:ProviderTransportReason?) {
    lock.lock();guard !closed else {lock.unlock();return};closed=true
    let connection=connection,stage=stage,total=total,task=task;self.connection=nil;self.stage=nil;self.total=nil;lock.unlock()
    connection?.cancel();stage?.cancel();total?.cancel();task?.cancel()
    if let reason {continuation.finish(throwing:reason)} else {continuation.finish()}
  }
  private func yield(_ event:ProviderHTTPEvent) {
    if case .dropped=continuation.yield(event) {stop(.limit)}
  }
  func urlSession(_ session:URLSession,dataTask:URLSessionDataTask,didReceive response:URLResponse,completionHandler:@escaping @Sendable (URLSession.ResponseDisposition)->Void) {
    guard let response=response as? HTTPURLResponse,let status=UInt16(exactly:response.statusCode),(100...599).contains(status) else {completionHandler(.cancel);stop(.network);return}
    lock.lock();guard !closed,self.status==nil else {lock.unlock();completionHandler(.cancel);return};self.status=status;connection?.cancel();connection=nil;stage?.cancel();stage=timer(timeouts.idle,.idleTimeout);lock.unlock()
    yield(.headers(status));completionHandler(.allow)
  }
  func urlSession(_ session:URLSession,dataTask:URLSessionDataTask,didReceive data:Data) {
    lock.lock();guard !closed else {lock.unlock();return};totalBytes+=data.count
    let over=totalBytes>8*1024*1024;stage?.cancel();stage=timer(timeouts.idle,.idleTimeout);lock.unlock()
    if over {stop(.limit)} else {yield(.bytes(data))}
  }
  func urlSession(_ session:URLSession,task:URLSessionTask,didCompleteWithError error:(any Error)?) {
    stop(error==nil ? nil:((error as? URLError)?.code == .cancelled ? .cancelled:.network))
  }
  /// Public URLSession upload progress confirms request-body bytes sent to the server.
  /// Our codec always produces a nonempty POST body; a slow model response is then separate.
  func urlSession(_ session:URLSession,task:URLSessionTask,didSendBodyData bytesSent:Int64,totalBytesSent:Int64,totalBytesExpectedToSend:Int64) {
    guard totalBytesSent>0 else {return}
    lock.lock();connection?.cancel();connection=nil;lock.unlock()
  }
  func urlSession(_ session:URLSession,task:URLSessionTask,willPerformHTTPRedirection response:HTTPURLResponse,newRequest request:URLRequest,completionHandler:@escaping @Sendable (URLRequest?)->Void) {
    lock.lock();status=UInt16(exactly:response.statusCode);lock.unlock();completionHandler(nil);stop(.redirect)
  }
  func urlSession(_ session:URLSession,task:URLSessionTask,didReceive challenge:URLAuthenticationChallenge,completionHandler:@escaping @Sendable (URLSession.AuthChallengeDisposition,URLCredential?)->Void) {
    if challenge.protectionSpace.authenticationMethod==NSURLAuthenticationMethodServerTrust {completionHandler(.performDefaultHandling,nil)} else {completionHandler(.cancelAuthenticationChallenge,nil)}
  }
}
/// One explicit request; no paid retries or legacy chat loop. It never runs on the CAS queue.
struct ProviderTransport:Sendable {
  let secrets:SecretStore
  private let makeConfiguration:@Sendable ()->URLSessionConfiguration
  init(secrets:SecretStore){self.secrets=secrets;makeConfiguration={.ephemeral}}
  init(secrets:SecretStore,testConfiguration:@escaping @Sendable ()->URLSessionConfiguration){self.secrets=secrets;makeConfiguration=testConfiguration}
  func send(_ model:ProviderModelSession,route:ProviderRoute,journal:ProviderTransportJournal,onText:@escaping @Sendable (String)async throws->Void)async throws->ProviderModelResult {
    var dispatched=false,pump:ProviderHTTPPump?,responseGuard:ProviderSecretResponseGuard?
    let prepared=model.prepared
    func outcome()->ProviderDispatchOutcome {pump?.observedStatus != nil ? .responseObserved:dispatched ? .dispatchOutcomeUnknown:.notDispatched}
    do {
      guard model.binding==route.binding else {throw ProviderTransportReason.stale}
      try await model.claim();try Task.checkCancellation()
      var request=URLRequest(url:route.endpoint);request.httpMethod="POST";request.httpBody=model.body;request.httpShouldHandleCookies=false
      request.setValue("application/json",forHTTPHeaderField:"Content-Type");request.setValue(prepared.stream ? "text/event-stream":"application/json",forHTTPHeaderField:"Accept")
      if route.binding.wireProtocol=="anthropic_messages" {request.setValue("2023-06-01",forHTTPHeaderField:"anthropic-version")}
      if let reference=route.credentialReference {
        let secret=try await secrets.read(reference:reference)
        responseGuard=secret.responseGuard()
        for (key,value) in secret.authenticationHeader(route.authentication){request.setValue(value,forHTTPHeaderField:key)}
      }
      try Task.checkCancellation()
      let configuration=makeConfiguration();configuration.httpCookieStorage=nil;configuration.urlCache=nil;configuration.urlCredentialStorage=nil;configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
      configuration.timeoutIntervalForRequest=max(route.timeouts.firstResponse,route.timeouts.idle);configuration.timeoutIntervalForResource=route.timeouts.total
      let pair=AsyncThrowingStream<ProviderHTTPEvent,any Error>.makeStream(bufferingPolicy:.bufferingOldest(32))
      let actualPump=ProviderHTTPPump(continuation:pair.continuation,timeouts:route.timeouts);pump=actualPump
      model.lifetime.bind {actualPump.stop(.cancelled)}
      let session=URLSession(configuration:configuration,delegate:actualPump,delegateQueue:nil)
      defer {session.invalidateAndCancel()}
      let task=session.dataTask(with:request)
      // Durable callback precedes resume; interruption after this receipt remains unknown.
      try await journal.record(.init(phase:.dispatchStarted,request:prepared.request_ref,context:prepared.context_ref,status:nil,outcome:.dispatchOutcomeUnknown))
      dispatched=true;try Task.checkCancellation()
      let result=try await withTaskCancellationHandler {
        actualPump.start(task)
        for try await event in pair.stream {
          try Task.checkCancellation()
          switch event {
          case .headers(let status):
            try await journal.record(.init(phase:.responseStarted,request:prepared.request_ref,context:prepared.context_ref,status:status,outcome:.responseObserved))
            try await model.headers(status)
          case .bytes(let bytes):
            let ready=try responseGuard?.feed(bytes) ?? bytes
            if !ready.isEmpty {try await journal.responseBytes(ready);for text in try await model.feed(ready) {try await onText(text)}}
          }
        }
        if let final=try responseGuard?.feed(Data(),final:true),!final.isEmpty {try await journal.responseBytes(final);for text in try await model.feed(final){try await onText(text)}}
        try Task.checkCancellation();return try await model.finish()
      } onCancel:{actualPump.stop(.cancelled)}
      try await journal.record(.init(phase:.finished,request:prepared.request_ref,context:prepared.context_ref,status:actualPump.observedStatus,outcome:.responseObserved))
      return result
    } catch {
      let reason:ProviderTransportReason
      if error is CancellationError {reason = .cancelled}
      else if case ProviderSecretError.echoed=error {reason = pump?.observedStatus.map({!(200..<300).contains($0)}) == true ? .http:.credentialEcho}
      else {reason = error as? ProviderTransportReason ?? (error is ProviderSecretError ? .secret:.journal)}
      pump?.stop(reason);await model.cancel()
      let observed=outcome(),status=pump?.observedStatus
      // A failing terminal persistence callback cannot turn an unknown request into safe retry.
      try? await journal.record(.init(phase:reason == .cancelled ? .interrupted:.failed,request:prepared.request_ref,context:prepared.context_ref,status:status,outcome:observed))
      throw ProviderTransportFailure(reason:reason,outcome:observed,httpStatus:status)
    }
  }
}
