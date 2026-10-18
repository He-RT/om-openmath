import Foundation

extension AssistantMode {
  var tools:[AgentToolName] {
    let read:[AgentToolName]=[.read_notebook,.search_functions,.get_function_docs,.inspect_result,.evaluate_scratch,.get_operation_status]
    return self == .execute ? read+[.preview_source,.apply_notebook_patch,.run_cells,.undo_transaction]:read
  }
}

/// Reconstruct each turn from durable canonical messages and exact referenced tool bytes.
/// The helper supplies neither the provider route nor a replacement conversation history.
actor AssistantContext {
  private let host:NativeHostClient,storage:StorageService,session:String,task:String
  private let configuration:AssistantConfiguration,mode:AssistantMode,tools:[AgentToolName],schema:URL
  private let observed:(@Sendable(AssistantInputSnapshot)async->Void)?,completed:(@Sendable(Data)async->Void)?
  init(host:NativeHostClient,storage:StorageService,session:String,task:String,configuration:AssistantConfiguration,mode:AssistantMode,tools:[AgentToolName],schema:URL,observed:(@Sendable(AssistantInputSnapshot)async->Void)?=nil,completed:(@Sendable(Data)async->Void)?=nil) {
    self.host=host;self.storage=storage;self.session=session;self.task=task;self.configuration=configuration;self.mode=mode;self.tools=tools;self.schema=schema
    self.observed=observed;self.completed=completed
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
    let data=try await requestData()
    let model=try await ProviderModelSession.prepare(host:host,requestJSON:String(decoding:data,as:UTF8.self),request:intent.request_id,context:"context-"+UUID().uuidString.lowercased(),task:task,turn:intent.agent_turn_id,message:"assistant-"+intent.agent_turn_id)
    let snapshot=try inputSnapshot(data,fullInput:String(decoding:model.body,as:UTF8.self),actual:true)
    await observed?(snapshot)
    let route=try ProviderRoute(binding:model.binding,endpoint:configuration.endpoint,authentication:.bearer,credentialReference:configuration.credentialReference,timeouts:try ProviderTimeouts(),allowHTTPOnLiteralLoopback:configuration.endpoint.scheme=="http")
    let original=ProviderRequestLedger(storage:storage,ledger:SessionLedgerPort(storage:storage,session:session),model:model).turn(route:route)
    let completed=self.completed
    return .init(model:original.model,route:original.route,journal:original.journal,complete:{result in
      try await original.complete(result);await completed?(result.usageJSON)
    })
  }
  func preview(_ input:AgentCanonicalMessage)async throws->AssistantInputSnapshot {
    let data=try await requestData(pending:input)
    return try inputSnapshot(data,fullInput:String(decoding:data,as:UTF8.self),actual:false)
  }
  private func inputSnapshot(_ data:Data,fullInput:String,actual:Bool)throws->AssistantInputSnapshot {
    guard let root=try JSONSerialization.jsonObject(with:data) as? [String:Any],let messages=root["messages"] as? [Any] else {throw AssistantSetupError.invalid}
    func count(_ value:Any)throws->Int {String(decoding:try JSONSerialization.data(withJSONObject:value,options:[.sortedKeys]),as:UTF8.self).count}
    return .init(systemCharacters:try count(Array(messages.prefix(1)))+count(root["tools"] ?? []),conversationCharacters:try count(Array(messages.dropFirst()))+count(root["results"] ?? []),attachmentCharacters:0,fullInput:fullInput,actual:actual)
  }
  private func requestData(pending:AgentCanonicalMessage?=nil)async throws->Data {
    let system=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"assistant-instructions-v1",role:.system,agent_turn_id:.init(nil),task_id:.init(nil),parts:[.agent_text_part(.init(kind:.text,part_id:"instructions",text:SystemPrompt.render(mode:mode,version:SystemPrompt.releaseVersion)))])
    var messages:[[String:Any]]=[["canonical":try object(system),"replay":NSNull()]],results:[[String:Any]]=[]
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
      } else if stored.event.task_id.value==task,let result=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:bytes),result.kind=="native_readonly_result" {
        try await append(result.canonical,intent:result.intent,hash:result.blobHash,length:result.byteLength,messages:&messages,results:&results)
      } else if stored.event.task_id.value==task,let result=try? JSONDecoder().decode(AgentEffectRecord.self,from:bytes) {
        try await append(result.canonical,intent:result.intent,hash:result.blobHash,length:result.byteLength,messages:&messages,results:&results)
      }
    }
    if let pending {messages.append(["canonical":try object(pending),"replay":NSNull()])}
    let catalog=try JSONSerialization.jsonObject(with:Data(contentsOf:schema)) as! [String:Any],definitions=catalog["$defs"] as! [String:Any]
    guard let catalogTools=(catalog["x-openmath-catalog"] as? [String:Any])?["model_tools"] as? [[String:Any]] else {throw AssistantSetupError.invalid}
    var declared:[[String:Any]]=[]
    for tool in tools {
      guard let args=definitions[tool.rawValue+"_args"],
        let entry=catalogTools.first(where:{$0["name"] as? String==tool.rawValue}),
        let description=entry["description"] as? String else {throw AssistantSetupError.invalid}
      var help=description
      if let example=entry["example_arguments"] {
        help+="\n参数示例（引用须换成本次读取取得的值）："+String(decoding:try JSONSerialization.data(withJSONObject:example,options:[.sortedKeys,.withoutEscapingSlashes]),as:UTF8.self)
      }
      declared.append(["name":tool.rawValue,"description":help,"parameters":try expand(args,definitions:definitions,depth:0)])
    }
    let request:[String:Any]=["binding":["provider_ref":"native-assistant","route_ref":"native-assistant-chat","model_ref":"native-assistant-model","adapter_version":"single-turn-1","protocol":"chat_completions"],"api_model":configuration.model,"messages":messages,"results":results,"tools":declared,"max_output_tokens":2048,"temperature":NSNull(),"stream":true]
    let data=try JSONSerialization.data(withJSONObject:request,options:[.sortedKeys,.withoutEscapingSlashes])
    guard data.count<=1024*1024 else {throw AssistantSetupError.contextLimit}
    return data
  }
  private func object<T:Encodable>(_ value:T)throws->Any {try JSONSerialization.jsonObject(with:JSONEncoder().encode(value))}
  private func append(_ canonical:AgentCanonicalMessage,intent:NativeAgentToolIntent,hash:String,length:UInt64,messages:inout [[String:Any]],results:inout [[String:Any]])async throws {
    let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:canonical.message_id),hash:hash)
    let bytes:Data
    do {bytes=try await storage.readBlob(lease,offset:0,count:Int(length))} catch {try? await storage.closeBlob(lease);throw error}
    try await storage.closeBlob(lease)
    guard KernelHashes.data(bytes)==hash,let value=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],let ok=value["ok"] as? Bool else {throw AgentSessionError.ledgerMismatch}
    messages.append(["canonical":try object(canonical),"replay":NSNull()])
    results.append(["message_id":canonical.message_id,"tool_call_id":intent.tool_call_id,"text":String(decoding:bytes,as:UTF8.self),"is_error":!ok])
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
