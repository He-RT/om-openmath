import Foundation
import Security

/// Opt-in smoke against an explicitly supplied private API file. No route/key discovery,
/// request dumps or synthetic model responses; this uses the UI's actual context builder.
@main struct LiveAssistantFixtures {
  @MainActor static func main() async {
    do {try await run()} catch {
      print("Live assistant failed: "+String(reflecting:type(of:error)))
      exit(1)
    }
  }
  @MainActor static func run() async throws {
    guard CommandLine.arguments.count==5 else {throw AssistantSetupError.invalid}
    let api=URL(fileURLWithPath:CommandLine.arguments[1]),app=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    let root=URL(fileURLWithPath:CommandLine.arguments[3],isDirectory:true),model=CommandLine.arguments[4]
    let config=try JSONSerialization.jsonObject(with:Data(contentsOf:api)) as! [String:Any]
    guard let base=config["base_url"] as? String,let key=config["api_key"] as? String,
      let models=config["models"] as? [String],models.contains(model),base=="https://4Router.net/v1" else {throw AssistantSetupError.invalid}
    let secret=Data(key.utf8),document=UUID().uuidString.lowercased(),session="live-assistant"
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let info=try await storage.openDocument(document)
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"实际模型验收",cells:[.init(id:"a",kind:.math,source:"2+2",dialect:.modern)]),cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
    snapshot.snapshot_hash=SourceHashes.snapshot(snapshot);_ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open()
    let opened=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
    let drafts=DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
    let service=AgentSessionService(host:host,storage:storage,document:document,generation:opened.document_generation.value,sourcePort:source,kernelPort:kernel)
    let tools:[AgentToolName]=[.read_notebook,.search_functions,.get_function_docs,.preview_source,.apply_notebook_patch,.run_cells,.inspect_result,.get_operation_status]
    let hello=try await service.activate(session:session,grants:tools,readonly:false,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
    let process=try await AgentProcess.verified(package:app.appendingPathComponent("Contents/Resources/AgentRuntime"),signatureRoot:app)
    let context=AssistantContext(host:host,storage:storage,session:session,task:hello.scope.task_id,configuration:.init(baseURL:base,model:model,credentialReference:"authorized-live-api"),mode:.execute,tools:tools,schema:process.spec.root.appendingPathComponent("Agent/docs/design/agent-tools.schema.json"))
    let transport=ProviderTransport(secrets:SecretStore(testReader:{_ in (errSecSuccess,secret)}))
    let adapter=ProviderAgentModelAdapter(transport:transport,factory:{try await context.prepare($0)})
    let pipe=try await process.launch()
    let watchdog=Task {
      try? await Task.sleep(for:.seconds(180));guard !Task.isCancelled else {return}
      _ = try? await service.cancel();await adapter.close();await pipe.close()
    }
    do {
      let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"live-question",role:.user,agent_turn_id:.init(nil),task_id:.init(hello.scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"question",text:"请读取当前笔记本，把数学单元格 a 改成 solve(x^2 - 5x + 6 = 0, x)，运行此单元格，然后用 inspect_result 核对两个精确解。不新增单元格。只简短回答实际核对的结果。"))])
      let report=try await service.run(pipe:pipe,hello:hello,modelPort:adapter.port(),input:input)
      let actual=drafts.confirmed
      guard actual.file.cells.count==1,actual.file.cells[0].source=="solve(x^2 - 5x + 6 = 0, x)" else {throw AgentSessionError.ledgerMismatch}
      let state=try await kernel.state()
      guard state.definitions_current,state.active_checkpoint_id.value != nil,report.loopReason == .loop_stopped else {throw AgentSessionError.ledgerMismatch}
      var after:UInt64=0,names:[String]=[],results:[String]=[]
      while true {
        let page=try await storage.sessionEvents(session,after:after,limit:16)
        for stored in page {
          guard let text=stored.event.payload_inline_json.value else {continue}
          if let record=try? JSONDecoder().decode(AgentReadonlyRecord.self,from:Data(text.utf8)),record.kind=="native_readonly_result" {names.append(record.intent.name.rawValue);results.append(record.blobHash)}
          else if let record=try? JSONDecoder().decode(AgentEffectRecord.self,from:Data(text.utf8)) {names.append(record.intent.name.rawValue);results.append(record.blobHash)}
        }
        guard let last=page.last else {break};after=last.event.event_sequence
        if page.count<16 {break}
      }
      guard Set(["read_notebook","preview_source","apply_notebook_patch","run_cells","inspect_result"]).isSubset(of:Set(names)) else {throw AgentSessionError.ledgerMismatch}
      let result:[String:Any]=["model":model,"loop_reason":report.loopReason?.rawValue ?? "","tools":names,"result_blob_hashes":results,"source_hash":actual.snapshot_hash,"definitions_current":state.definitions_current,"frames":report.frames]
      let data=try JSONSerialization.data(withJSONObject:result,options:[.sortedKeys,.prettyPrinted])
      try data.write(to:root.appendingPathComponent("summary.json"));print(String(decoding:data,as:UTF8.self))
      watchdog.cancel();await adapter.close();await pipe.close();await service.close();await source.close();try await host.close();try await storage.closeDocument(document);try await storage.close()
    } catch {
      watchdog.cancel();await adapter.close();await pipe.close();await service.close();await source.close();try? await host.close();try? await storage.closeDocument(document);try? await storage.close();throw error
    }
  }
}
