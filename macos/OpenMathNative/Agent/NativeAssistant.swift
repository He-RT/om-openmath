import Foundation
import SwiftUI

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
  @Published private(set) var messages:[AssistantMessage]=[]
  @Published private(set) var transcriptError:String?
  @Published private(set) var outcomes:[TurnOutcome]=[]
  @Published private(set) var undoneTurns=Set<String>()
  @Published private(set) var undoing=false
  @Published private(set) var undoError:String?
  var prepare:(@MainActor ()async throws->Void)?
  var settled:(@MainActor ()async->Void)?
  var savedState:(@MainActor ()async throws->Bool)?
  var outcomeReady:(@MainActor (TurnOutcome)async throws->Void)?
  var undoOutcome:(@MainActor (String)async throws->Void)?
  var insertCode:(@MainActor (String,String)async throws->Void)?
  func insertCodeBlock(_ code:String,language:String) async {
    guard !busy,let insertCode else {return}
    do {try await insertCode(code,language);status="已插入数学单元格，尚未运行"}
    catch {status="未插入代码，原文已保留；请核对源码或当前笔记本状态。"}
  }
  private let host:NativeHostClient,storage:StorageService,agent:AgentSessionService
  private let settings:AssistantSettingsStore?
  private var configuration:AssistantConfiguration?
  private var secrets=SecretStore()
  private var session="conversation-"+UUID().uuidString.lowercased()
  private var work:Task<Void,Never>?,adapter:ProviderAgentModelAdapter?,pipe:AgentPrivatePipe?
  private var closed=false
  private var pendingText:[String:String]=[:],pendingTurns:[String]=[]
  private var textFlush:Task<Void,Never>?
  private var durableMessages:[AssistantMessage]=[],streamedText:[String:String]=[:],streamedOrder:[String]=[]
  private var pendingInput:AssistantMessage?,activePresentation:AssistantMessageContext?,activeTask=""
  private var streamedContexts:[String:AssistantMessageContext]=[:],streamedTasks:[String:String]=[:]
  init(host:NativeHostClient,storage:StorageService,agent:AgentSessionService) {
    self.host=host;self.storage=storage;self.agent=agent;settings=try? AssistantSettingsStore()
  }
  func load()async {
    guard !busy,configuration==nil else {return}
    do {if let value=try await settings?.load(),configuration==nil {configuration=value;baseURL=value.baseURL;model=value.model;configured=true;status="就绪"}}
    catch {status="模型配置无法读取，请重新配置。"}
  }
  /// Trusted host selection (also used by isolated live fixtures); secrets are never serialized.
  func configure(_ value:AssistantConfiguration,secrets:SecretStore?=nil)throws {
    guard !busy,!undoing,!closed else {throw AgentSessionError.busy}
    try AssistantSettingsStore.validate(value.baseURL,model:value.model)
    configuration=value;baseURL=value.baseURL;model=value.model;configured=true
    if let secrets {self.secrets=secrets}
    status="模型已就绪"
  }
  func undoTurn(_ id:String)async {
    guard !busy,!undoing,!undoneTurns.contains(id),let undoOutcome else {return}
    undoing=true;undoError=nil;defer {undoing=false}
    do {try await undoOutcome(id);undoneTurns.insert(id);status="已撤销本轮修改"}
    catch CommitPortError.unknownOutcome {undoError="撤销结果待核对，未重复提交";status=undoError!}
    catch {undoError="撤销未提交，请核对后续编辑或当前输入";status=undoError!}
  }
  func saveSettings()async {
    guard !busy,!undoing,let settings else {return}
    do {
      let value=try await settings.save(baseURL:baseURL,model:model,key:apiKey)
      configuration=value;apiKey="";baseURL=value.baseURL;model=value.model;configured=true;showSettings=false;status="模型配置已保存"
    } catch {status="未保存。请检查服务地址（远程HTTPS或本机HTTP）、模型名称和API Key；密钥需能写入已解锁的钥匙串。"}
  }
  func newConversation() {
    guard !busy,!undoing else {return};session="conversation-"+UUID().uuidString.lowercased()
    messages=[];durableMessages=[];streamedText=[:];streamedOrder=[];pendingText=[:];pendingTurns=[];pendingInput=nil
    streamedContexts=[:];streamedTasks=[:];outcomes=[];undoneTurns=[];undoError=nil;transcriptError=nil;status=configured ? "新对话":"请先配置模型"
  }
  func send() {
    guard !closed,!busy,!undoing else {return}
    let text=draft.trimmingCharacters(in:.whitespacesAndNewlines)
    guard !text.isEmpty,text.utf8.count<=16384 else {status="请输入问题（最多16KiB）。";return}
    guard let configuration else {showSettings=true;return}
    busy=true;stopping=false;status="正在准备";transcriptError=nil
    let edits=allowsChanges
    let id="user-"+UUID().uuidString.lowercased(),presentation=AssistantMessageContext(model:configuration.model,mode:edits ? "execute":"discuss")
    activePresentation=presentation
    pendingInput = .init(id:id,taskID:"",role:"user",context:presentation,parts:[.init(id:"question",value:.text(text))])
    publishMessages();draft=""
    work=Task {await execute(text,id:id,configuration:configuration,edits:edits,presentation:presentation)}
  }
  private func execute(_ text:String,id:String,configuration:AssistantConfiguration,edits:Bool,presentation:AssistantMessageContext)async {
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
      taskID=scope.task_id;activeTask=scope.task_id
      await agent.observeTranscript { [weak self] in await self?.refreshMessages() }
      let context=AssistantContext(host:host,storage:storage,session:session,task:scope.task_id,configuration:configuration,mode:edits ? .execute:.discuss,tools:tools,schema:process.spec.root.appendingPathComponent("Agent/docs/design/agent-tools.schema.json"))
      let transport=ProviderAgentModelAdapter(transport:ProviderTransport(secrets:secrets),factory:{try await context.prepare($0)})
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
      let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:id,role:.user,agent_turn_id:.init(nil),task_id:.init(scope.task_id),parts:[.agent_text_part(.init(kind:.text,part_id:"question",text:text))])
      let report=try await agent.run(pipe:actual,hello:hello,modelPort:visible,input:input,inputPresentation:presentation)
      reason=await agent.endReason(report.loopReason,stopped:stopping || closed)
    } catch {
      if taskID==nil,!stopping,!closed {reason = .failed("无法准备当前笔记本或助手运行环境")}
      else {reason=await agent.endReason(nil,stopped:stopping || closed)}
    }
    flushText()
    await adapter?.close();adapter=nil
    if let pipe {await pipe.close();self.pipe=nil}
    await agent.resetTask();await settled?()
    await refreshMessages()
    if edits,let taskID {
      let elapsed=started.duration(to:.now).components
      let duration=Int(elapsed.seconds*1000+elapsed.attoseconds/1_000_000_000_000_000)
      do {
        let facts=try await agent.outcomeFacts(session:session,task:taskID)
        let saved=try await savedState?() ?? false
        let outcome=TurnOutcome.assemble(task:taskID,reason:reason,facts:facts,saved:saved,durationMs:duration)
        outcomes.append(outcome)
        do {try await outcomeReady?(outcome)} catch {undoError="本轮撤销记录暂无法读取，请保留当前笔记本"}
      } catch {
        // Missing evidence is explicit. Do not replace unreadable facts with a claim of no edits.
        var outcome=TurnOutcome.assemble(task:taskID,reason:reason,facts:[],saved:false,durationMs:duration)
        outcome.factsUnavailable=true;outcomes.append(outcome)
      }
    }
    status=UIStrings.end(reason)
    if case .failed=reason,draft.isEmpty {draft=text}
    if outcomes.last?.id==taskID,outcomes.last?.factsUnavailable==true {status+="；本轮操作事实暂时无法读取，请核对笔记本"}
    busy=false;stopping=false;work=nil
  }
  private func refreshMessages()async {
    let current=session
    do {
      let next=try await agent.transcript(session:current)
      guard !closed,current==session else {return}
      durableMessages=next;transcriptError=nil
      let ids=Set(next.map(\.id))
      for id in ids {pendingText.removeValue(forKey:id);streamedText.removeValue(forKey:id);streamedContexts.removeValue(forKey:id);streamedTasks.removeValue(forKey:id)}
      pendingTurns.removeAll{ids.contains($0)};streamedOrder.removeAll{ids.contains($0)}
      if let pendingInput,ids.contains(pendingInput.id) {self.pendingInput=nil}
      publishMessages()
      if let action=next.last?.actions.last,action.status == .running {status=action.verb+" "+action.object}
    } catch {if current==session {transcriptError="操作记录暂时无法读取，已有内容保留"}}
  }
  private func receive(_ event:AgentModelEventPayload) {
    guard !closed,!event.text.isEmpty else {return}
    let id=event.agent_turn_id
    guard !durableMessages.contains(where:{$0.id==id}) else {return}
    if pendingText[id]==nil {pendingTurns.append(id)}
    if streamedTasks[id]==nil {streamedTasks[id]=activeTask;streamedContexts[id]=activePresentation}
    pendingText[id,default:""]+=event.text
    if textFlush==nil {textFlush=Task { [weak self] in
      do {try await Task.sleep(for:.milliseconds(75))} catch {return}
      self?.flushText()
    }}
  }
  private func flushText() {
    textFlush?.cancel();textFlush=nil
    guard !pendingTurns.isEmpty else {return}
    for id in pendingTurns {
      guard let text=pendingText[id] else {continue}
      if streamedText[id]==nil {streamedOrder.append(id)}
      streamedText[id,default:""]+=text
    }
    pendingTurns=[];pendingText=[:];publishMessages()
  }
  private func publishMessages() {
    var partial:[AssistantMessage]=[]
    for id in streamedOrder {
      guard let text=streamedText[id] else {continue}
      partial.append(.init(id:id,taskID:streamedTasks[id] ?? activeTask,role:"assistant",context:streamedContexts[id],parts:[.init(id:"stream-text",value:.text(text))]))
    }
    messages=AssistantMessageProjection.merge(durable:durableMessages,partial:partial,pendingInput:pendingInput)
  }
  func stop()async {
    guard busy,!stopping else {return};stopping=true;status="正在停止"
    _ = try? await agent.cancel()
    await adapter?.close()
    if let pipe {await pipe.close()}
  }
  func close()async {closed=true;await stop();await work?.value}
}
