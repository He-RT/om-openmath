import Foundation

/// Reconstruct each turn from durable canonical messages and exact referenced tool bytes.
/// The helper supplies neither the provider route nor a replacement conversation history.
actor AssistantContext {
  private let host:NativeHostClient,storage:StorageService,session:String,task:String
  private let configuration:AssistantConfiguration,tools:[AgentToolName],schema:URL
  private let changed:@Sendable ([String])async->Void
  init(host:NativeHostClient,storage:StorageService,session:String,task:String,configuration:AssistantConfiguration,tools:[AgentToolName],schema:URL,changed:@escaping @Sendable ([String])async->Void) {
    self.host=host;self.storage=storage;self.session=session;self.task=task;self.configuration=configuration;self.tools=tools;self.schema=schema;self.changed=changed
  }
  private func events()async throws->[StoredSessionEvent] {
    var all:[StoredSessionEvent]=[],after:UInt64=0
    while true {
      let page=try await storage.sessionEvents(session,after:after,limit:16)
      all+=page
      guard all.count<=512 else {throw AssistantSetupError.contextLimit}
      guard let last=page.last else {return all}
      after=last.event.event_sequence
      if page.count<16 {return all}
    }
  }
  func prepare(_ intent:NativeAgentRequestIntent)async throws->ProviderPreparedTurn {
    let system=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"assistant-instructions-v1",role:.system,agent_turn_id:.init(nil),task_id:.init(nil),parts:[.agent_text_part(.init(kind:.text,part_id:"instructions",text:"你是OpenMath笔记本助手。用中文清楚作答。数学必须用实际内核核对。先read_notebook，只有cells视图明确给出的完整源码才能修改；修改先preview_source再apply_notebook_patch。apply_notebook_patch成功后旧snapshot_ref立即失效，必须重新read_notebook获取新的snapshot_ref，再用run_cells以required范围运行依赖，之后inspect_result核对结果。INVALID_REFERENCE表示引用过期时，只读重新获取引用后继续；不要重放已完成的修改。引用只在当前任务有效，旧任务的引用必须重新读取。尊重只读模式；停止或未知提交后不重放。不要声称未执行或未完成的计算、修改、保存已成功。"))])
    var messages:[[String:Any]]=[["canonical":try object(system),"replay":NSNull()]],results:[[String:Any]]=[],trace:[String]=[]
    for stored in try await events() {
      guard let text=stored.event.payload_inline_json.value else {continue}
      let bytes=Data(text.utf8)
      if stored.event.event_kind == .user_message {
        let input=try JSONDecoder().decode(SessionCanonicalInput.self,from:bytes)
        messages.append(["canonical":try object(input.message),"replay":NSNull()])
      } else if stored.event.event_kind == .assistant_end {
        let original=try JSONDecoder().decode(SessionCanonicalAssistant.self,from:bytes)
        var message=try JSONDecoder().decode(AgentCanonicalMessage.self,from:Data(original.canonicalJSON.utf8))
        if stored.event.task_id.value != task {message.parts=message.parts.filter{if case .agent_text_part = $0 {return true};return false}}
        if !message.parts.isEmpty {messages.append(["canonical":try object(message),"replay":NSNull()])}
        if stored.event.task_id.value==task {trace+=original.calls.map{"调用 "+$0.toolName}}
      } else if stored.event.task_id.value==task,let result=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:bytes),result.kind=="native_readonly_result" {
        try await append(result.canonical,intent:result.intent,hash:result.blobHash,length:result.byteLength,messages:&messages,results:&results,trace:&trace)
      } else if stored.event.task_id.value==task,let result=try? JSONDecoder().decode(AgentEffectRecord.self,from:bytes) {
        try await append(result.canonical,intent:result.intent,hash:result.blobHash,length:result.byteLength,messages:&messages,results:&results,trace:&trace)
      }
    }
    await changed(trace)
    let catalog=try JSONSerialization.jsonObject(with:Data(contentsOf:schema)) as! [String:Any],definitions=catalog["$defs"] as! [String:Any]
    let declared=try tools.map {tool->[String:Any] in
      guard let args=definitions[tool.rawValue+"_args"] else {throw AssistantSetupError.invalid}
      return ["name":tool.rawValue,"description":"OpenMath "+tool.rawValue,"parameters":try expand(args,definitions:definitions,depth:0)]
    }
    let request:[String:Any]=["binding":["provider_ref":"native-assistant","route_ref":"native-assistant-chat","model_ref":"native-assistant-model","adapter_version":"single-turn-1","protocol":"chat_completions"],"api_model":configuration.model,"messages":messages,"results":results,"tools":declared,"max_output_tokens":2048,"temperature":NSNull(),"stream":true]
    let data=try JSONSerialization.data(withJSONObject:request,options:[.sortedKeys,.withoutEscapingSlashes])
    guard data.count<=1024*1024 else {throw AssistantSetupError.contextLimit}
    let model=try await ProviderModelSession.prepare(host:host,requestJSON:String(decoding:data,as:UTF8.self),request:intent.request_id,context:"context-"+UUID().uuidString.lowercased(),task:task,turn:intent.agent_turn_id,message:"assistant-"+intent.agent_turn_id)
    let route=try ProviderRoute(binding:model.binding,endpoint:configuration.endpoint,authentication:.bearer,credentialReference:configuration.credentialReference,timeouts:try ProviderTimeouts())
    return ProviderRequestLedger(storage:storage,ledger:SessionLedgerPort(storage:storage,session:session),model:model).turn(route:route)
  }
  private func object<T:Encodable>(_ value:T)throws->Any {try JSONSerialization.jsonObject(with:JSONEncoder().encode(value))}
  private func append(_ canonical:AgentCanonicalMessage,intent:NativeAgentToolIntent,hash:String,length:UInt64,messages:inout [[String:Any]],results:inout [[String:Any]],trace:inout [String])async throws {
    let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:canonical.message_id),hash:hash)
    let bytes:Data
    do {bytes=try await storage.readBlob(lease,offset:0,count:Int(length))} catch {try? await storage.closeBlob(lease);throw error}
    try await storage.closeBlob(lease)
    guard KernelHashes.data(bytes)==hash,let value=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],let ok=value["ok"] as? Bool else {throw AgentSessionError.ledgerMismatch}
    messages.append(["canonical":try object(canonical),"replay":NSNull()])
    results.append(["message_id":canonical.message_id,"tool_call_id":intent.tool_call_id,"text":String(decoding:bytes,as:UTF8.self),"is_error":!ok])
    trace.append(intent.name.rawValue+(ok ? " · 已完成":" · 未完成"))
  }
  private func expand(_ value:Any,definitions:[String:Any],depth:Int)throws->Any {
    guard depth<=64 else {throw AssistantSetupError.invalid}
    if let object=value as? [String:Any] {
      if let ref=object["$ref"] as? String,ref.hasPrefix("#/$defs/"),let target=definitions[String(ref.dropFirst(8))] {return try expand(target,definitions:definitions,depth:depth+1)}
      return try object.mapValues{try expand($0,definitions:definitions,depth:depth+1)}
    }
    if let array=value as? [Any] {return try array.map{try expand($0,definitions:definitions,depth:depth+1)}}
    return value
  }
}
