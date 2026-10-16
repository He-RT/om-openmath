import Foundation
import SwiftUI

struct AssistantLine:Identifiable {
  let id:String,role:String
  var text:String
}
@MainActor final class NativeAssistant:ObservableObject {
  @Published var draft=""
  @Published var baseURL=""
  @Published var model="gpt-6.1-sol"
  @Published var apiKey=""
  @Published var allowsChanges=false
  @Published var showSettings=false
  @Published private(set) var configured=false
  @Published private(set) var busy=false
  @Published private(set) var stopping=false
  @Published private(set) var status="配置模型后，可以在这里提问。"
  @Published private(set) var lines:[AssistantLine]=[]
  @Published private(set) var trace:[String]=[]
  @Published private(set) var outcomes:[TurnOutcome]=[]
  var prepare:(@MainActor ()async throws->Void)?
  var settled:(@MainActor ()async->Void)?
  var savedState:(@MainActor ()async throws->Bool)?
  private let host:NativeHostClient,storage:StorageService,agent:AgentSessionService
  private let settings:AssistantSettingsStore?
  private var configuration:AssistantConfiguration?
  private var session="conversation-"+UUID().uuidString.lowercased()
  private var work:Task<Void,Never>?,adapter:ProviderAgentModelAdapter?,pipe:AgentPrivatePipe?
  private var closed=false
  private var pendingText:[String:String]=[:],pendingTurns:[String]=[]
  private var textFlush:Task<Void,Never>?
  init(host:NativeHostClient,storage:StorageService,agent:AgentSessionService) {
    self.host=host;self.storage=storage;self.agent=agent;settings=try? AssistantSettingsStore()
  }
  func load()async {
    guard !busy,configuration==nil else {return}
    do {if let value=try await settings?.load() {configuration=value;baseURL=value.baseURL;model=value.model;configured=true;status="就绪"}}
    catch {status="模型配置无法读取，请重新配置。"}
  }
  func saveSettings()async {
    guard !busy,let settings else {return}
    do {
      let value=try await settings.save(baseURL:baseURL,model:model,key:apiKey)
      configuration=value;apiKey="";baseURL=value.baseURL;model=value.model;configured=true;showSettings=false;status="模型配置已保存"
    } catch {status="未保存。请检查HTTPS地址、模型名称和API Key；密钥需能写入已解锁的钥匙串。"}
  }
  func newConversation() {guard !busy else {return};session="conversation-"+UUID().uuidString.lowercased();lines=[];trace=[];outcomes=[];status=configured ? "新对话":"请先配置模型"}
  func send() {
    guard !closed,!busy else {return}
    let text=draft.trimmingCharacters(in:.whitespacesAndNewlines)
    guard !text.isEmpty,text.utf8.count<=16384 else {status="请输入问题（最多16KiB）。";return}
    guard let configuration else {showSettings=true;return}
    busy=true;stopping=false;status="正在准备";trace=[]
    lines.append(.init(id:UUID().uuidString,role:"你",text:text));draft=""
    let edits=allowsChanges
    work=Task {await execute(text,configuration:configuration,edits:edits)}
  }
  private func execute(_ text:String,configuration:AssistantConfiguration,edits:Bool)async {
    let started=ContinuousClock.now
    var taskID:String?,reason:TurnEndReason = .failed("请求尚未开始")
    do {
      try await prepare?()
      guard !stopping,!closed else {throw CancellationError()}
      let process=try await AgentProcess.bundled()
      let read:[AgentToolName]=[.read_notebook,.search_functions,.get_function_docs,.inspect_result,.evaluate_scratch,.get_operation_status]
      let tools=edits ? read+[.preview_source,.apply_notebook_patch,.run_cells,.undo_transaction]:read
      let hello=try await agent.activate(session:session,grants:tools,readonly:!edits,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1")
      guard !stopping,!closed else {throw CancellationError()}
      let scope=hello.scope
      taskID=scope.task_id
      let context=AssistantContext(host:host,storage:storage,session:session,task:scope.task_id,configuration:configuration,mode:edits ? .execute:.discuss,tools:tools,schema:process.spec.root.appendingPathComponent("Agent/docs/design/agent-tools.schema.json"),changed:{[weak self] values in await self?.updateTrace(values)})
      let transport=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:SecretStore()),factory:{try await context.prepare($0)})
      adapter=transport
      let underlying=transport.port()
      let visible=AgentNativeModelPort(prepare:underlying.prepare,stream:{[weak self] intent in
        let original=try await underlying.stream(intent)
        return AsyncThrowingStream {continuation in
          let forward=Task {
            do {for try await event in original {await self?.receive(event);continuation.yield(event)};continuation.finish()}
            catch {continuation.finish(throwing:error)}
          }
          continuation.onTermination={_ in forward.cancel()}
        }
      })
      let actual=try await process.launch();pipe=actual
      guard !stopping,!closed else {throw CancellationError()}
      status="正在回复"
      let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"user-"+UUID().uuidString.lowercased(),role:.user,agent_turn_id:.init(nil),task_id:.init(scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"question",text:text))])
      let report=try await agent.run(pipe:actual,hello:hello,modelPort:visible,input:input)
      reason=await agent.endReason(report.loopReason,stopped:stopping || closed)
    } catch {
      reason=await agent.endReason(nil,stopped:stopping || closed)
    }
    flushText()
    await adapter?.close();adapter=nil
    if let pipe {await pipe.close();self.pipe=nil}
    await agent.resetTask();await settled?()
    if edits,let taskID {
      let elapsed=started.duration(to:.now).components
      let duration=Int(elapsed.seconds*1000+elapsed.attoseconds/1_000_000_000_000_000)
      do {
        let facts=try await agent.outcomeFacts(session:session,task:taskID)
        let saved=try await savedState?() ?? false
        outcomes.append(.assemble(task:taskID,reason:reason,facts:facts,saved:saved,durationMs:duration))
      } catch {
        // Missing evidence is explicit. Do not replace unreadable facts with a claim of no edits.
        var outcome=TurnOutcome.assemble(task:taskID,reason:reason,facts:[],saved:false,durationMs:duration)
        outcome.factsUnavailable=true;outcomes.append(outcome)
      }
    }
    status=UIStrings.end(reason)
    if outcomes.last?.id==taskID,outcomes.last?.factsUnavailable==true {status+="；本轮操作事实暂时无法读取，请核对笔记本"}
    busy=false;stopping=false;work=nil
  }
  private func updateTrace(_ values:[String]) {trace=values;status="正在执行任务"}
  private func receive(_ event:AgentModelEventPayload) {
    guard !closed,!event.text.isEmpty else {return}
    let id=event.agent_turn_id
    if pendingText[id]==nil {pendingTurns.append(id)}
    pendingText[id,default:""]+=event.text
    if textFlush==nil {textFlush=Task { [weak self] in
      do {try await Task.sleep(for:.milliseconds(75))} catch {return}
      self?.flushText()
    }}
  }
  private func flushText() {
    textFlush?.cancel();textFlush=nil
    guard !pendingTurns.isEmpty else {return}
    var next=lines
    for id in pendingTurns {
      guard let text=pendingText[id] else {continue}
      if let index=next.firstIndex(where:{$0.id==id}) {next[index].text+=text}
      else {next.append(.init(id:id,role:"OpenMath",text:text))}
    }
    pendingTurns=[];pendingText=[:];lines=next
  }
  func stop()async {
    guard busy,!stopping else {return};stopping=true;status="正在停止"
    _ = try? await agent.cancel()
    await adapter?.close()
    if let pipe {await pipe.close()}
  }
  func close()async {closed=true;await stop();await work?.value}
}
