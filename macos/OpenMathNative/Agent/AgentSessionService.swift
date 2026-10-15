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
  private let sourcePort:DocCommitPort?
  private let kernelPort:KernelCommitPort?
  private let runFaults:StorageFaults
  private let toolFailure:(@Sendable(AgentToolName,any Error)->Void)?
  private var allowedRunCells:[String]?
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
  init(host:NativeHostClient,storage:StorageService,document:String,generation:UInt64,sourcePort:DocCommitPort?=nil,kernelPort:KernelCommitPort?=nil,runFaults:StorageFaults = .init(),toolFailure:(@Sendable(AgentToolName,any Error)->Void)?=nil) {
    self.runFaults=runFaults;self.toolFailure=toolFailure
    self.host=host;self.storage=storage;self.document=document;self.generation=generation;self.sourcePort=sourcePort;self.kernelPort=kernelPort ?? sourcePort.map {KernelCommitPort(client:host,storage:storage,source:$0)}
  }
  /// Fresh explicit activation. Persisted messages are data and never recreate grants.
  func activate(session:String,grants:[AgentToolName],readonly:Bool,coreVersion:String,adapterVersion:String,allowedRunCells:[String]?=nil) async throws->NativeAgentHostReply {
    try await acquire();defer {release()}
    guard scope==nil else {throw AgentSessionError.busy}
    guard taskGeneration<HostSerial.maximum,connectionGeneration<HostSerial.maximum else {throw AgentSessionError.stale}
    // Reserve activation before any suspension, including an unavailable Library.
    _ = try await storage.createAgentSession(session)
    guard !closed else {throw AgentSessionError.inactive}
    let s=NativeAgentControlScope(agent_session_id:session,task_id:"task-"+UUID().uuidString.lowercased(),task_generation:try HostSerial(taskGeneration+1),connection_id:"connection-"+UUID().uuidString.lowercased(),connection_generation:try HostSerial(connectionGeneration+1),document_generation:try HostSerial(generation))
    let reply=try await host.agentCommand(.native_agent_activate(.init(type:.agent_activate,scope:s,hello_id:"hello-"+UUID().uuidString.lowercased(),core_version:coreVersion,adapter_version:adapterVersion,grants:grants,readonly:readonly)))
    guard !closed,reply.document_id==document else {_ = try? await host.agentCommand(.native_agent_close(.init(type:.agent_close,scope:s)));throw AgentSessionError.stale}
    self.allowedRunCells=allowedRunCells
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
  /// Readonly source/catalog/preview producers require actual native results and durable readback.
  private func dispatchTool(_ intent:NativeAgentToolIntent) async {
    if intent.name == .apply_notebook_patch {await dispatchPatch(intent);return}
    if intent.name == .run_cells {await dispatchRun(intent);return}
    guard [.read_notebook,.search_functions,.get_function_docs,.preview_source,.inspect_result,.evaluate_scratch,.get_operation_status].contains(intent.name) else {await unavailableTool(intent);return}
    do {
      guard let s=scope,let ledger else {throw AgentSessionError.inactive}
      if intent.name == .read_notebook {
        let (head,entries)=try await storage.readNotebookHistory(document:document)
        _ = try await host.agentCommand(.native_agent_bind_history(.init(type:.agent_bind_history,scope:s,expected_snapshot_hash:head.snapshot_hash,transaction_refs:entries.map(\.transaction))))
      }
      var reply:NativeAgentHostReply
      if intent.name == .get_operation_status {
        reply=try await host.agentCommand(.native_agent_query_operation(.init(type:.agent_query_operation,scope:s,operation_id:intent.operation_id,observation:.init(nil),failure_code:.init(nil))))
        if let lookup=reply.operation_query.value {
          guard lookup.operation_id==intent.operation_id,lookup.arguments_hash==intent.arguments_hash,lookup.document_id==document else {throw AgentSessionError.stale}
          let observation:NativeOperationObservation?
          do {observation=try await OperationStatusReader(storage:storage,kernel:kernelPort).read(lookup)} catch {
            observation=nil
            reply=try await host.agentCommand(.native_agent_query_operation(.init(type:.agent_query_operation,scope:s,operation_id:intent.operation_id,observation:.init(nil),failure_code:.init(.not_available))))
          }
          if let observation {reply=try await host.agentCommand(.native_agent_query_operation(.init(type:.agent_query_operation,scope:s,operation_id:intent.operation_id,observation:.init(observation),failure_code:.init(nil))))}
        }
      } else {reply=try await host.agentCommand(.native_agent_read_tool(.init(type:.agent_read_tool,scope:s,operation_id:intent.operation_id)))}
      let deadline=ContinuousClock.now.advanced(by:.seconds(10))
      while reply.disposition == .pending {
        try Task.checkCancellation();guard ContinuousClock.now<deadline else {throw AgentSessionError.busy}
        try await Task.sleep(for:.milliseconds(10))
        if intent.name == .get_operation_status {reply=try await host.agentCommand(.native_agent_query_operation(.init(type:.agent_query_operation,scope:s,operation_id:intent.operation_id,observation:.init(nil),failure_code:.init(nil))))}
        else {reply=try await host.agentCommand(.native_agent_read_tool(.init(type:.agent_read_tool,scope:s,operation_id:intent.operation_id)))}
      }
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
    } catch {toolFailure?(intent.name,error);if !Task.isCancelled {_ = try? await send("error",payload:JSONEncoder().encode(AgentErrorPayload(code:"NATIVE_READONLY_RESULT_UNAVAILABLE",recoverable:false)))}}
  }
  private func dispatchPatch(_ intent:NativeAgentToolIntent)async {
    do {
      guard let s=scope,let ledger,let sourcePort else {await unavailableTool(intent);return}
      let prepared=try await host.sourceCommand(.source_begin_agent(.init(type:.source_begin_agent,operation_id:intent.operation_id)))
      var target:SessionOperationTarget?,admissionID:String?,outcomeID:String?,failure:NativeAgentEffectToolFailureCode?
      if let plan=prepared.plan.value {
        let info=try await storage.documentIdentity(document)
        let actual=SessionOperationTarget(documentID:plan.commit.document_id,storeID:info.storeID,operationID:plan.commit.operation_id,kind:.source,expectedHash:plan.commit.request_hash)
        let admitted="admit-"+intent.operation_id,result="operation-result-"+intent.operation_id
        target=actual;admissionID=admitted
        let admission=SessionToolAdmission(codecVersion:1,callID:intent.tool_call_id,proposalEvent:intent.proposal_event_ref,toolName:intent.name.rawValue,argumentsHash:intent.arguments_hash,target:actual)
        let tool=SessionToolPort(storage:storage,ledger:ledger)
        do {
          _ = try await tool.perform(admission:admission,admissionID:admitted,startedID:"started-"+intent.operation_id,resultID:result,turn:intent.agent_turn_id,task:s.task_id,execute:{
            let state=try await sourcePort.applyPrepared(prepared)
            guard state.phase == .completed else {throw state.phase == .cancelled ? CommitPortError.cancelled:CommitPortError.sourceConflict}
          })
          outcomeID=result
        } catch {
          if Task.isCancelled {throw CancellationError()}
          if let error=error as? CommitPortError {
            switch error {case .editingBusy,.invalidFence:failure = .editing_busy;default:failure = .preview_mismatch}
          } else {failure = .preview_mismatch}
          // Read original receipt only. An ACK loss never invokes a second source operation.
          _ = try? await sourcePort.reconcile(plan.commit.operation_id)
          if try await tool.reconcile(admission:admission,admissionID:admitted,resultID:result,turn:intent.agent_turn_id,task:s.task_id) != nil {outcomeID=result;failure=nil}
        }
      }
      try Task.checkCancellation()
      let reply=try await host.agentCommand(.native_agent_effect_tool(.init(type:.agent_effect_tool,scope:s,operation_id:intent.operation_id,failure_code:.init(failure))))
      try await deliverEffect(intent,reply:reply,target:target,admissionID:admissionID,outcomeID:outcomeID)
    } catch {if !Task.isCancelled {_ = try? await send("error",payload:JSONEncoder().encode(AgentErrorPayload(code:"NATIVE_PATCH_RESULT_UNAVAILABLE",recoverable:false)))}}
  }
  private func deliverEffect(_ intent:NativeAgentToolIntent,reply:NativeAgentHostReply,target:SessionOperationTarget?,admissionID:String?,outcomeID:String?)async throws {
    guard let s=scope,let ledger else {throw AgentSessionError.inactive}
      guard let result=reply.effect_result.value,result.operation_id==intent.operation_id,result.tool_call_id==intent.tool_call_id,
        result.agent_turn_id==intent.agent_turn_id,result.name == intent.name,
        result.operation_ref.value==target?.operationID else {throw AgentSessionError.stale}
      let bytes=Data(result.result_json.utf8)
      guard bytes.count<=262144,KernelHashes.data(bytes)==result.result_hash else {throw AgentSessionError.invalidFrame}
      let outcome=try JSONDecoder().decode(AgentToolResultPayloadOutcome.self,from:JSONEncoder().encode(result.outcome))
      let canonicalOutcome=try JSONDecoder().decode(AgentToolResultPartOutcome.self,from:JSONEncoder().encode(result.outcome))
      let event="effect-"+intent.operation_id,owner=BlobOwner(kind:.context,id:event)
      let canonical=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:event,role:.tool,agent_turn_id:.init(intent.agent_turn_id),task_id:.init(s.task_id),parts:[.agent_tool_result_part(.init(kind:.tool_result,part_id:event,tool_call_id:intent.tool_call_id,operation_ref:result.operation_ref,result_ref:.init(result.result_ref),outcome:canonicalOutcome))])
      let record=AgentEffectRecord(codecVersion:1,kind:"native_effect_result",intent:intent,resultRef:result.result_ref,blobHash:result.result_hash,byteLength:UInt64(bytes.count),canonical:canonical,target:target,admissionEvent:admissionID,outcomeEvent:outcomeID)
      let publication=try await storage.publishBlob(data:bytes,expectedHash:result.result_hash)
      do {
        _ = try await ledger.record(record,id:event,kind:.recovery,parent:intent.proposal_event_ref,turn:intent.agent_turn_id,task:s.task_id,target:target,resource:publication,resourceOwner:owner)
        try await storage.releaseBlob(publication)
      } catch {try? await storage.releaseBlob(publication);throw error}
      guard let stored=try await storage.sessionEvent(s.agent_session_id,event:event),stored.event.payload_inline_json.value.map({Data($0.utf8)}) == (try RecoveryValidation.encode(record)) else {throw AgentSessionError.ledgerMismatch}
      let lease=try await storage.openReferencedBlob(owner:owner,hash:result.result_hash)
      let original:Data
      do {original=try await storage.readBlob(lease,offset:0,count:bytes.count);try await storage.closeBlob(lease)}catch {try? await storage.closeBlob(lease);throw error}
      guard original==bytes else {throw AgentSessionError.ledgerMismatch}
      try Task.checkCancellation()
      _ = try await host.agentCommand(.native_agent_confirm_tool(.init(type:.agent_confirm_tool,scope:s,operation_id:intent.operation_id,result_ref:result.result_ref,result_hash:result.result_hash,event_id:event,event_sequence:try HostSerial(stored.receipt.eventSequence),receipt_hash:stored.receipt.recordHash)))
      let payload=AgentToolResultPayload(agent_turn_id:intent.agent_turn_id,tool_call_id:intent.tool_call_id,operation_ref:result.operation_ref,result_ref:.init(result.result_ref),outcome:outcome,result_json:.init(String(decoding:original,as:UTF8.self)))
      toolFrames[intent.operation_id]=try await send("tool_result",payload:JSONEncoder().encode(payload))
  }
  private func dispatchRun(_ intent:NativeAgentToolIntent)async {
    var runPort:SessionRunPort?,admissionID:String?,target:SessionOperationTarget?
    do {
      guard let s=scope,let ledger,let kernelPort else {await unavailableTool(intent);return}
      let port=SessionRunPort(storage:storage,ledger:ledger);runPort=port
      var reply=try await host.agentCommand(.native_agent_prepare_run(.init(type:.agent_prepare_run,scope:s,operation_id:intent.operation_id,allowed_cell_ids:.init(allowedRunCells))))
      let planningDeadline=ContinuousClock.now.advanced(by:.seconds(10))
      while reply.effect_result.value==nil && reply.run_progress.value?.descriptor.value==nil {
        try Task.checkCancellation();guard ContinuousClock.now<planningDeadline else {throw AgentSessionError.busy}
        try await Task.sleep(for:.milliseconds(10))
        reply=try await host.agentCommand(.native_agent_poll_run(.init(type:.agent_poll_run,scope:s,operation_id:intent.operation_id)))
      }
      var outcomeID:String?
      if let descriptor=reply.run_progress.value?.descriptor.value {
        guard descriptor.operation_id==intent.operation_id,descriptor.arguments_hash==intent.arguments_hash,
          descriptor.document_generation.value==generation else {throw AgentSessionError.stale}
        let info=try await storage.documentIdentity(document)
        if descriptor.initial_checkpoint_id.value==nil,try await storage.acceptedKernelHead(document:document) != nil {throw KernelPortError.recoveryRequired}
        let actual=SessionOperationTarget(documentID:document,storeID:info.storeID,operationID:intent.operation_id,kind:.kernel,expectedHash:descriptor.source_snapshot_hash)
        target=actual
        let plan=SessionRunPlan(codecVersion:1,scope:descriptor.run_scope.rawValue,requestedCellIDs:descriptor.requested_cell_ids,
          children:descriptor.children.map {.init(operationID:$0.operation_id,cellID:$0.cell_id.value,cellSourceHash:$0.cell_source_hash.value)},
          blockedCells:descriptor.blocked_cells.map {.init(cellID:$0.cell_id,code:$0.code)},continueOnError:descriptor.continue_on_error,
          documentGeneration:descriptor.document_generation.value,sourceRevision:descriptor.source_revision.value,executionEpoch:descriptor.execution_epoch.value,configRevision:descriptor.config_revision.value,
          kernelBuild:descriptor.kernel_build,generalHash:descriptor.general_hash,initialCheckpointID:descriptor.initial_checkpoint_id.value,initialCheckpointHash:descriptor.initial_checkpoint_hash.value,initialKernelRevision:descriptor.initial_kernel_revision.value?.value)
        let admitted="admit-"+intent.operation_id;admissionID=admitted
        _ = try await port.begin(.init(codecVersion:1,callID:intent.tool_call_id,proposalEvent:intent.proposal_event_ref,toolName:intent.name.rawValue,argumentsHash:intent.arguments_hash,target:actual,run:plan),id:admitted,turn:intent.agent_turn_id,task:s.task_id,faults:runFaults)
        while reply.effect_result.value==nil {
          try Task.checkCancellation()
          guard let progress=reply.run_progress.value,progress.operation_id==intent.operation_id else {throw AgentSessionError.stale}
          if let childIndex=progress.next_child_index.value,let childID=progress.next_operation_id.value {
            let index=Int(childIndex.value)
            guard plan.children.indices.contains(index),plan.children[index].operationID==childID else {throw AgentSessionError.stale}
            do {
              _ = try await port.performChild(index:index,faults:runFaults) { [host,storage,runFaults] child in
                guard child.operationID==childID else {throw AgentSessionError.stale}
                func ack(_ event:String)async throws->NativeRunEventAck {
                  guard let original=try await storage.sessionEvent(s.agent_session_id,event:event) else {throw AgentSessionError.ledgerMismatch}
                  return .init(event_id:event,event_sequence:try HostSerial(original.receipt.eventSequence),receipt_hash:original.receipt.recordHash)
                }
                _ = try await host.agentCommand(.native_agent_acknowledge_run_child(.init(type:.agent_acknowledge_run_child,scope:s,operation_id:intent.operation_id,child_operation_id:childID,
                  group_admission:try await ack(admitted),group_started:try await ack("run-started-"+intent.operation_id),child_admission:try await ack("run-admit-"+childID),child_started:try await ack("run-start-"+childID))))
                do {_ = try await kernelPort.finishExecution(childID,faults:runFaults);return .accepted}
                catch {
                  let original=try await kernelPort.status(childID)
                  if original.phase == .failed || original.phase == .discarded {return .stopped(original.error_code.value ?? "NOT_ACCEPTED")}
                  throw error
                }
              }
            } catch {
              // The original child is the only reconciliation target. No new ID/callback is used.
              if Task.isCancelled {throw error}
              let actual=try await storage.sessionRunSnapshot(s.agent_session_id,admission:admitted)
              if actual.activeChild != nil {
                _ = try? await port.finish(.unknown,code:"UNKNOWN_OUTCOME")
              }
              _ = try? await host.cancel(operation:intent.operation_id)
            }
          } else {try await Task.sleep(for:.milliseconds(10))}
          reply=try await host.agentCommand(.native_agent_poll_run(.init(type:.agent_poll_run,scope:s,operation_id:intent.operation_id)))
        }
        guard let result=reply.effect_result.value else {throw AgentSessionError.invalidFrame}
        let snapshot=try await storage.sessionRunSnapshot(s.agent_session_id,admission:admitted)
        let finish:SessionRunSnapshot
        if snapshot.transition?.phase != nil {
          finish=snapshot // Preserve the original observed unknown; explicit status recovery may resolve it later.
        } else {
          let phase:SessionRunPhase
          switch result.outcome {case .completed:phase = .completed;case .partial:phase = .partial;case .failed:phase = .failed;case .cancelled:phase = .cancelled;case .unknown:phase = .unknown}
          finish=try await port.finish(phase,code:phase == .completed ? nil:phase == .unknown ? "UNKNOWN_OUTCOME":phase == .cancelled ? "CANCELLED":"EXECUTION_FAILED")
        }
        outcomeID=finish.latest.event_id
      }
      try await deliverEffect(intent,reply:reply,target:target,admissionID:admissionID,outcomeID:outcomeID)
    } catch {
      toolFailure?(intent.name,error)
      _ = try? await host.cancel(operation:intent.operation_id)
      if let runPort,let admissionID { _ = try? await runPort.reconcile(admission:admissionID) }
      if !Task.isCancelled {_ = try? await send("error",payload:JSONEncoder().encode(AgentErrorPayload(code:"NATIVE_RUN_RESULT_UNAVAILABLE",recoverable:false)))}
    }
  }
  private func stopDispatches() {for task in dispatches.values {task.cancel()};dispatches.removeAll()}
  func close() async {
    closed=true;stopDispatches()
    if let s=scope { _ = try? await host.agentCommand(.native_agent_close(.init(type:.agent_close,scope:s))) }
    scope=nil;ledger=nil
    if let pipe=privatePipe {await pipe.close();privatePipe=nil}
  }
}
