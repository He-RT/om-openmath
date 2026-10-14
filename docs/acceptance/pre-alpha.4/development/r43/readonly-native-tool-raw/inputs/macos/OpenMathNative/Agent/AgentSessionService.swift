import Foundation

enum AgentSessionError:Error,Sendable {case inactive,busy,invalidFrame,stale,ledgerMismatch}
/// Rich canonical bytes and the existing ledger projection travel together. Encoding this
/// envelope never re-encodes the canonical JSON used by Rust's confirmation hash.
struct SessionCanonicalAssistant:Codable,Sendable {
  let codecVersion:Int,messageID:String,textUTF8:Data,calls:[SessionToolCall],canonicalJSON:String
  let peerMessageID:String,connectionID:String,sourceSequence:UInt64
}
struct AgentPipeReport:Sendable {var frames=0;var loopReason:AgentSettledPayloadReason?;var lastPhase:NativeAgentHostReplyPhase = .starting;var toolIntents:[String]=[];var requestIntents:[String]=[]}
struct SessionCanonicalInput:Codable,Sendable {let codecVersion:Int,delivery:String,message:AgentCanonicalMessage}

struct AgentToolUnavailable:Codable,Sendable {let codecVersion:Int,kind:String,intent:NativeAgentToolIntent}

/// One document-owned conversation port. Construction/recovery starts no model, tool or helper.
/// Actor suspension cannot grant authority: every resumed callback is fenced again by Rust.
actor AgentSessionService {
  private let host:NativeHostClient,storage:StorageService,document:String,generation:UInt64
  private var scope:NativeAgentControlScope?
  private var ledger:SessionLedgerPort?
  private var taskGeneration:UInt64=0,connectionGeneration:UInt64=0,outgoingSequence:UInt64=0
  private var receiving=false,closed=false
  private var waiters:[CheckedContinuation<Void,Never>]=[]
  private var inputFrames:[String:String]=[:]
  private var privatePipe:AgentPrivatePipe?
  private var dispatches:[String:Task<Void,Never>]=[:]
  private var readyConnection:String?
  private var toolFrames:[String:String]=[:]
  init(host:NativeHostClient,storage:StorageService,document:String,generation:UInt64) {
    self.host=host;self.storage=storage;self.document=document;self.generation=generation
  }
  /// Fresh explicit activation. Persisted messages are data and never recreate grants.
  func activate(session:String,grants:[AgentToolName],readonly:Bool,coreVersion:String,adapterVersion:String) async throws->NativeAgentHostReply {
    try await acquire();defer {release()}
    guard scope==nil else {throw AgentSessionError.busy}
    guard taskGeneration<HostSerial.maximum,connectionGeneration<HostSerial.maximum else {throw AgentSessionError.stale}
    // Reserve activation before any suspension, including an unavailable Library.
    _ = try await storage.createAgentSession(session)
    guard !closed else {throw AgentSessionError.inactive}
    let s=NativeAgentControlScope(agent_session_id:session,task_id:"task-"+UUID().uuidString.lowercased(),task_generation:try HostSerial(taskGeneration+1),connection_id:"connection-"+UUID().uuidString.lowercased(),connection_generation:try HostSerial(connectionGeneration+1),document_generation:try HostSerial(generation))
    let reply=try await host.agentCommand(.native_agent_activate(.init(type:.agent_activate,scope:s,hello_id:"hello-"+UUID().uuidString.lowercased(),core_version:coreVersion,adapter_version:adapterVersion,grants:grants,readonly:readonly)))
    guard !closed,reply.document_id==document else {_ = try? await host.agentCommand(.native_agent_close(.init(type:.agent_close,scope:s)));throw AgentSessionError.stale}
    scope=s;taskGeneration=s.task_generation.value;connectionGeneration=s.connection_generation.value
    ledger=SessionLedgerPort(storage:storage,session:session);outgoingSequence=1;inputFrames.removeAll();toolFrames.removeAll()
    return reply
  }
  /// Complete raw UTF8 frame bytes reach the stricter Rust decoder before any Swift JSON parsing.
  /// Unknown SQLite commit has no ACK; an exact frame retry reconciles its original event.
  func receive(_ bytes:Data,faults:StorageFaults = .init()) async throws->NativeAgentHostReply {
    try await acquire();defer {release()}
    guard let s=scope,let ledger else {throw AgentSessionError.inactive}
    guard bytes.count<=AgentFrameBytes.maximum,let raw=String(data:bytes,encoding:.utf8) else {throw AgentSessionError.invalidFrame}
    let reply=try await host.agentCommand(.native_agent_receive(.init(type:.agent_receive,scope:s,frame_json:raw)))
    guard reply.document_id==document else {throw AgentSessionError.stale}
    guard reply.disposition == .transcript else {if reply.disposition == .ready {readyConnection=s.connection_id};updateSequence(reply);return reply}
    guard let canonical=reply.canonical_json.value,let event=reply.event_id.value,let peer=reply.peer_message_id.value,let sourceSequence=reply.source_sequence.value else {throw AgentSessionError.invalidFrame}
    let message=try JSONDecoder().decode(AgentCanonicalMessage.self,from:Data(canonical.utf8))
    guard message.role == .assistant,message.task_id.value==s.task_id,let turn=message.agent_turn_id.value else {throw AgentSessionError.stale}
    var text=Data(),calls:[SessionToolCall]=[]
    for part in message.parts {
      switch part {
      case .agent_text_part(let p):text.append(contentsOf:p.text.utf8)
      case .agent_tool_proposal_part(let p):calls.append(.init(callID:p.tool_call_id,toolName:p.name.rawValue,arguments:Data(p.arguments_json.utf8)))
      default:break
      }
    }
    let envelope=SessionCanonicalAssistant(codecVersion:1,messageID:message.message_id,textUTF8:text,calls:calls,canonicalJSON:canonical,peerMessageID:peer,connectionID:s.connection_id,sourceSequence:sourceSequence.value)
    let receipt=try await ledger.record(envelope,id:event,kind:.assistant_end,turn:turn,task:s.task_id,faults:faults)
    guard let actual=try await storage.sessionEvent(s.agent_session_id,event:event),
      actual.receipt.recordHash==receipt.stored.receipt.recordHash,
      actual.event.payload_hash==KernelHashes.data(try RecoveryValidation.encode(envelope)),
      let payload=actual.event.payload_inline_json.value,
      try JSONDecoder().decode(SessionCanonicalAssistant.self,from:Data(payload.utf8)).canonicalJSON==canonical else {throw AgentSessionError.ledgerMismatch}
    // No ACK can be emitted from a mere append callback or helper's claimed persistence.
    let ack=try await host.agentCommand(.native_agent_confirm(.init(type:.agent_confirm,scope:s,message_id:peer,event_id:event,canonical_hash:KernelHashes.data(Data(canonical.utf8)),event_sequence:try HostSerial(actual.receipt.eventSequence),receipt_hash:actual.receipt.recordHash)))
    updateSequence(ack);return ack
  }
  /// User/steer/follow-up input is durable before the helper receives its event reference.
  func input(_ message:AgentCanonicalMessage,delivery:String="user_input",event:String="input-"+UUID().uuidString.lowercased(),faults:StorageFaults = .init()) async throws->NativeAgentHostReply {
    try await acquire();defer {release()}
    guard let s=scope,let ledger else {throw AgentSessionError.inactive}
    guard ["user_input","steer","follow_up"].contains(delivery),message.role == .user,message.task_id.value==s.task_id else {throw AgentSessionError.invalidFrame}
    try Self.validateInput(message)
    _ = try await ledger.record(SessionCanonicalInput(codecVersion:1,delivery:delivery,message:message),id:event,kind:.user_message,turn:message.agent_turn_id.value,task:s.task_id,faults:faults)
    let value:[String:Any]=["protocol_version":1,"runtime_instance_id":host.runtimeInstanceID,"document_id":document,"document_generation":generation,"agent_session_id":s.agent_session_id,"task_id":s.task_id,"task_generation":s.task_generation.value,"connection_id":s.connection_id,"connection_generation":s.connection_generation.value,"message_id":"input-frame-"+UUID().uuidString.lowercased(),"sequence":outgoingSequence+1,"reply_to":NSNull(),"type":delivery,"payload":["event_ref":event,"message":try JSONSerialization.jsonObject(with:JSONEncoder().encode(message))]]
    let bytes=try JSONSerialization.data(withJSONObject:value,options:[.sortedKeys,.withoutEscapingSlashes])
    let raw=inputFrames[event] ?? String(decoding:bytes,as:UTF8.self);inputFrames[event]=raw
    let reply=try await host.agentCommand(.native_agent_send(.init(type:.agent_send,scope:s,frame_json:raw)))
    updateSequence(reply)
    if let pipe=privatePipe,let outbound=reply.outbound_json.value {try await pipe.write(Data((outbound+"\n").utf8))}
    return reply
  }
  private func acquire() async throws {
    guard !closed else {throw AgentSessionError.inactive}
    if receiving {
      guard waiters.count<32 else {throw AgentSessionError.busy}
      await withCheckedContinuation {waiters.append($0)}
    } else {receiving=true}
    guard !closed else {release();throw AgentSessionError.inactive}
  }
  private func release() {
    if waiters.isEmpty {receiving=false} else {waiters.removeFirst().resume()}
  }
  private nonisolated static func validateInput(_ message:AgentCanonicalMessage)throws {
    guard message.message_version==1,SourceValidation.identity(message.message_id),
      message.agent_turn_id.value.map(SourceValidation.identity) ?? true,
      (1...128).contains(message.parts.count),try JSONEncoder().encode(message).count<=AgentFrameBytes.maximum else {throw AgentSessionError.invalidFrame}
    var ids=Set<String>()
    for part in message.parts {
      let id:String
      switch part {
      case .agent_text_part(let p):id=p.part_id;guard p.text.unicodeScalars.count<=262144 else {throw AgentSessionError.invalidFrame}
      case .agent_media_part(let p):id=p.part_id;guard SourceValidation.identity(p.media_ref) else {throw AgentSessionError.invalidFrame}
      case .agent_step_link_part(let p):id=p.part_id;guard SourceValidation.identity(p.result_ref),SourceValidation.identity(p.step_id) else {throw AgentSessionError.invalidFrame}
      default:throw AgentSessionError.invalidFrame
      }
      guard SourceValidation.identity(id),ids.insert(id).inserted else {throw AgentSessionError.invalidFrame}
    }
  }
  private func updateSequence(_ reply:NativeAgentHostReply) {
    if let json=reply.outbound_json.value,let object=try? JSONSerialization.jsonObject(with:Data(json.utf8)) as? [String:Any],let n=object["sequence"] as? UInt64 {outgoingSequence=max(outgoingSequence,n)}
  }
  func cancel(reason:AgentCancelPayloadReason = .user_stop) async throws->NativeAgentHostReply {
    guard let s=scope else {throw AgentSessionError.inactive}
    let reply=try await host.agentCommand(.native_agent_cancel(.init(type:.agent_cancel,scope:s,reason:reason)));updateSequence(reply)
    if let pipe=privatePipe,let bytes=reply.outbound_json.value {Task {try? await pipe.write(Data((bytes+"\n").utf8))}}
    return reply
  }
  /// Actual private subprocess/pipe. Production Pi packaging is a separate adapter task.
  func run(pipe:AgentPrivatePipe,hello:NativeAgentHostReply,modelPort:AgentNativeModelPort?=nil,input:AgentCanonicalMessage?=nil) async throws->AgentPipeReport {
    guard !closed,scope != nil,privatePipe==nil,let initial=hello.outbound_json.value else {throw AgentSessionError.inactive}
    privatePipe=pipe
    let connection=scope!.connection_id
    let handshake=Task {try? await Task.sleep(for:.seconds(5));if !Task.isCancelled {await self.handshakeTimeout(pipe,connection:connection)}}
    defer {handshake.cancel()}
    var report=AgentPipeReport();var delivered=false
    do {
      try await pipe.write(Data((initial+"\n").utf8))
      var decoder=AgentFrameBytes()
      while let chunk=try await pipe.read() {
        for frame in try decoder.push(chunk) {
          let reply=try await receive(frame)
          report.frames+=1;report.lastPhase=reply.phase
          if let intent=reply.tool_intent.value {report.toolIntents.append(intent.operation_id)}
          if let intent=reply.request_intent.value {report.requestIntents.append(intent.operation_id)}
          if let outbound=reply.outbound_json.value {try await pipe.write(Data((outbound+"\n").utf8))}
          if reply.disposition == .ready,let input,!delivered {delivered=true;_ = try await self.input(input)}
          let decoded=try JSONDecoder().decode(AgentIPCFrame.self,from:frame)
          if case .agent_settled_frame(let settled)=decoded {report.loopReason=settled.payload.reason}
          if reply.disposition == .request_intent,let intent=reply.request_intent.value {
            let preparing:Bool
            switch decoded {case .agent_prepare_request_frame:preparing=true;case .agent_model_request_frame:preparing=false;default:throw AgentSessionError.invalidFrame}
            let key=(preparing ? "prepare-":"model-")+intent.operation_id
            if dispatches[key]==nil {dispatches[key]=Task {await self.dispatchModel(intent,preparing:preparing,port:modelPort)}}
          }
          if reply.disposition == .tool_intent,let intent=reply.tool_intent.value {
            let key="tool-"+intent.operation_id
            if dispatches[key]==nil {dispatches[key]=Task {await self.dispatchTool(intent)}}
          }

        }
      }
      try decoder.finish()
      guard report.lastPhase == .awaiting_settlement else {throw AgentSessionError.invalidFrame}
      stopDispatches();await pipe.close();privatePipe=nil
      if let s=scope {_ = try? await host.agentCommand(.native_agent_close(.init(type:.agent_close,scope:s)))}
      scope=nil;ledger=nil;return report
    } catch {
      stopDispatches();await pipe.close();privatePipe=nil
      if let s=scope { _ = try? await host.agentCommand(.native_agent_close(.init(type:.agent_close,scope:s))) }
      scope=nil;ledger=nil;throw error
    }
  }
  private func handshakeTimeout(_ pipe:AgentPrivatePipe,connection:String) async {
    if scope?.connection_id==connection,readyConnection != connection {await pipe.close()}
  }
  private func send(_ kind:String,payload:Data) async throws->String {
    try await acquire();defer {release()}
    guard let s=scope,let pipe=privatePipe else {throw AgentSessionError.inactive}
    let value:[String:Any]=["protocol_version":1,"runtime_instance_id":host.runtimeInstanceID,"document_id":document,"document_generation":generation,"agent_session_id":s.agent_session_id,"task_id":s.task_id,"task_generation":s.task_generation.value,"connection_id":s.connection_id,"connection_generation":s.connection_generation.value,"message_id":"native-frame-"+UUID().uuidString.lowercased(),"sequence":outgoingSequence+1,"reply_to":NSNull(),"type":kind,"payload":try JSONSerialization.jsonObject(with:payload)]
    let bytes=try JSONSerialization.data(withJSONObject:value,options:[.sortedKeys,.withoutEscapingSlashes]),raw=String(decoding:bytes,as:UTF8.self)
    let reply=try await host.agentCommand(.native_agent_send(.init(type:.agent_send,scope:s,frame_json:raw)))
    updateSequence(reply);try await pipe.write(Data((raw+"\n").utf8));return raw
  }
  private func dispatchModel(_ intent:NativeAgentRequestIntent,preparing:Bool,port:AgentNativeModelPort?) async {
    do {
      guard let port else {throw AgentSessionError.inactive}
      if preparing {
        let prepared=try await port.prepare(intent);try Task.checkCancellation()
        _ = try await send("model_request",payload:JSONEncoder().encode(AgentModelRequestPayload(agent_turn_id:intent.agent_turn_id,request_id:intent.request_id,context_ref:prepared.context,model_config_ref:prepared.model)))
      } else {
        let stream=try await port.stream(intent)
        for try await event in stream {try Task.checkCancellation();_ = try await send("model_event",payload:JSONEncoder().encode(event))}
      }
    } catch {
      if !Task.isCancelled {_ = try? await send("error",payload:JSONEncoder().encode(AgentErrorPayload(code:"NATIVE_MODEL_UNAVAILABLE",recoverable:false)))}
    }
  }
  private func unavailableTool(_ intent:NativeAgentToolIntent) async {
    do {
      guard let s=scope,let ledger else {throw AgentSessionError.inactive}
      _ = try await ledger.record(AgentToolUnavailable(codecVersion:1,kind:"native_tool_unavailable",intent:intent),id:"unavailable-"+intent.operation_id,kind:.recovery,turn:intent.agent_turn_id,task:s.task_id)
      try Task.checkCancellation()
      if let original=toolFrames[intent.operation_id],let pipe=privatePipe {
        let reply=try await host.agentCommand(.native_agent_send(.init(type:.agent_send,scope:s,frame_json:original)));updateSequence(reply);try await pipe.write(Data((original+"\n").utf8))
      } else {
        let result=AgentToolResultPayload(agent_turn_id:intent.agent_turn_id,tool_call_id:intent.tool_call_id,operation_ref:.init(nil),result_ref:.init(nil),outcome:.unavailable,result_json:.init(nil))
        toolFrames[intent.operation_id]=try await send("tool_result",payload:JSONEncoder().encode(result))
      }
    } catch {if !Task.isCancelled {_ = try? await send("error",payload:JSONEncoder().encode(AgentErrorPayload(code:"NATIVE_TOOL_UNAVAILABLE",recoverable:false)))}}
  }
  /// Only three actual readonly producers are available. Other tools keep their unavailable path.
  private func dispatchTool(_ intent:NativeAgentToolIntent) async {
    guard [.read_notebook,.search_functions,.get_function_docs].contains(intent.name) else {await unavailableTool(intent);return}
    do {
      guard let s=scope,let ledger else {throw AgentSessionError.inactive}
      let reply=try await host.agentCommand(.native_agent_read_tool(.init(type:.agent_read_tool,scope:s,operation_id:intent.operation_id)))
      guard let result=reply.readonly_result.value,result.operation_id==intent.operation_id,result.tool_call_id==intent.tool_call_id,
        result.agent_turn_id==intent.agent_turn_id,result.name==intent.name else {throw AgentSessionError.stale}
      let bytes=Data(result.result_json.utf8)
      guard bytes.count<=262144,KernelHashes.data(bytes)==result.result_hash else {throw AgentSessionError.invalidFrame}
      let event="readonly-"+intent.operation_id,owner=BlobOwner(kind:.context,id:event)
      let canonical=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:event,role:.tool,agent_turn_id:.init(intent.agent_turn_id),task_id:.init(s.task_id),parts:[.agent_tool_result_part(.init(kind:.tool_result,part_id:event,tool_call_id:intent.tool_call_id,operation_ref:.init(nil),result_ref:.init(result.result_ref),outcome:result.outcome == .completed ? .completed:.failed))])
      let record=AgentReadonlyRecord(codecVersion:1,kind:"native_readonly_result",intent:intent,resultRef:result.result_ref,blobHash:result.result_hash,byteLength:UInt64(bytes.count),canonical:canonical)
      let publication=try await storage.publishBlob(data:bytes,expectedHash:result.result_hash)
      do {
        _ = try await ledger.record(record,id:event,kind:.recovery,parent:intent.proposal_event_ref,turn:intent.agent_turn_id,task:s.task_id,resource:publication,resourceOwner:owner)
        try await storage.releaseBlob(publication)
      } catch {try? await storage.releaseBlob(publication);throw error}
      // Independent SQLite event and referenced Blob readback precede the native confirmation.
      guard let stored=try await storage.sessionEvent(s.agent_session_id,event:event),stored.event.payload_hash==KernelHashes.data(try RecoveryValidation.encode(record)),
        stored.event.payload_inline_json.value.map({Data($0.utf8)}) == (try RecoveryValidation.encode(record)) else {throw AgentSessionError.ledgerMismatch}
      let lease=try await storage.openReferencedBlob(owner:owner,hash:record.blobHash)
      let original:Data
      do {original=try await storage.readBlob(lease,offset:0,count:bytes.count);try await storage.closeBlob(lease)}
      catch {try? await storage.closeBlob(lease);throw error}
      guard original==bytes else {throw AgentSessionError.ledgerMismatch}
      try Task.checkCancellation()
      _ = try await host.agentCommand(.native_agent_confirm_tool(.init(type:.agent_confirm_tool,scope:s,operation_id:intent.operation_id,result_ref:result.result_ref,result_hash:result.result_hash,event_id:event,event_sequence:try HostSerial(stored.receipt.eventSequence),receipt_hash:stored.receipt.recordHash)))
      let payload=AgentToolResultPayload(agent_turn_id:intent.agent_turn_id,tool_call_id:intent.tool_call_id,operation_ref:.init(nil),result_ref:.init(result.result_ref),outcome:result.outcome == .completed ? .completed:.failed,result_json:.init(String(decoding:original,as:UTF8.self)))
      toolFrames[intent.operation_id]=try await send("tool_result",payload:JSONEncoder().encode(payload))
    } catch {if !Task.isCancelled {_ = try? await send("error",payload:JSONEncoder().encode(AgentErrorPayload(code:"NATIVE_READONLY_RESULT_UNAVAILABLE",recoverable:false)))}}
  }
  private func stopDispatches() {for task in dispatches.values {task.cancel()};dispatches.removeAll()}
  func close() async {
    closed=true;stopDispatches()
    if let s=scope { _ = try? await host.agentCommand(.native_agent_close(.init(type:.agent_close,scope:s))) }
    scope=nil;ledger=nil
    if let pipe=privatePipe {await pipe.close();privatePipe=nil}
  }
}
