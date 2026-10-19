import Foundation
import SwiftUI

@MainActor final class NativeAssistant:ObservableObject {
  @Published var draft=""
  @Published var allowsChanges=UserDefaults.standard.string(forKey:"assistant.default-mode")=="execute"
  @Published var registry=ModelRegistryDocument()
  @Published private(set) var selectedModelID:String?
  @Published var references:[AssistantCellReference]=[] {didSet {draftSequence+=1;scheduleContextPreview()}}
  @Published private(set) var attachments:[AssistantAttachment]=[]
  @Published private(set) var importingAttachments=0
  @Published private(set) var attachmentError:String?
  @Published var composerMarked=false
  @Published var sendShortcut:AssistantSendShortcut=AssistantSendShortcut(rawValue:UserDefaults.standard.string(forKey:"assistant.send-shortcut") ?? "") ?? .enter {
    didSet {UserDefaults.standard.set(sendShortcut.rawValue,forKey:"assistant.send-shortcut")}
  }
  @Published private(set) var inputPreview:AssistantInputSnapshot?
  @Published private(set) var lastRequestInput:AssistantInputSnapshot?
  @Published private(set) var lastUsage:AssistantUsage?
  @Published private(set) var contextError:String?
  @Published private(set) var modelChangeNote:String?
  var referenceCandidates:(@MainActor ()->[AssistantCellReference])?
  var focusComposer:(@MainActor ()->Void)?
  var addFilesRequested:(@MainActor ()->Void)?
  private var previewTask:Task<Void,Never>?,previewSequence:UInt64=0
  private var draftSequence:UInt64=0
  private var pendingDraftSequence:UInt64=0
  private var acceptedInputSequence:UInt64?
  private var inputWasAccepted=false
  private var removedAttachmentIDs=Set<String>()
  var selectedModel:RegisteredModel? {registry.selection(selectedModelID)?.1}
  var nextTools:[AgentToolName] {selectedModel?.tools==false ? []:(allowsChanges ? AssistantMode.execute:AssistantMode.discuss).tools}
  var canSend:Bool {configured && !busy && !undoing && !historyLoading && !composerMarked && unknownActionUndo==nil && unknownRoundChange==nil &&
    importingAttachments==0 && (!draft.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty || attachments.contains(where:\.sendable)) &&
    !attachments.contains(where:{$0.image != nil && selectedModel?.images != true}) && (!allowsChanges || selectedModel?.tools != false)}
  func removeAttachment(_ id:String) {removedAttachmentIDs.insert(id);attachments.removeAll{$0.id==id};draftSequence+=1;scheduleContextPreview()}
  func addAttachmentFiles(_ urls:[URL])async {
    guard !closed else {return}
    guard importingAttachments==0 else {attachmentError="正在准备附件，请稍候再加入";return}
    let scope=session;attachmentError=nil
    importingAttachments+=1;defer {importingAttachments-=1}
    let document=await agent.presentationDocumentID()
    for url in urls.prefix(16) {
      guard attachments.count<16,!closed else {attachmentError="每条消息最多 16 个附件";break}
      do {let item=try await attachmentStore.add(file:url,session:scope,document:document);if !closed {attachments.append(item);draftSequence+=1}}
      catch {attachmentError="无法加入 \(url.lastPathComponent)，请检查文件是否可读及剩余空间"}
    }
    if urls.count>16 {attachmentError="附件已达上限；其余文件未加入，请分批处理"}
    scheduleContextPreview()
  }
  func addAttachmentData(_ data:Data,name:String)async {
    guard !closed else {return}
    guard importingAttachments==0,attachments.count<16 else {attachmentError="请等待附件准备结束，或移除附件后再加入（最多 16 个）";return}
    importingAttachments+=1;defer {importingAttachments-=1}
    do {
      let item=try await attachmentStore.add(data:data,name:name,session:session,document:agent.presentationDocumentID())
      if !closed {attachments.append(item);draftSequence+=1;attachmentError=nil;scheduleContextPreview()}
    } catch {attachmentError="图片无法加入，原有输入已保留"}
  }
  func addReference(_ id:String) {
    guard let item=referenceCandidates?().first(where:{$0.id==id}),!references.contains(where:{$0.id==id}) else {return}
    references.append(item);focusComposer?()
  }
  func selectModel(_ id:String) {
    guard let (provider,model)=registry.selection(id),!allowsChanges || model.tools else {return}
    selectedModelID=id;configuration = .init(baseURL:provider.baseURL,model:model.apiName,credentialReference:provider.credentialReference)
    configured=true
    modelChangeNote=busy ? model.displayName+" 从下一轮开始使用":nil;scheduleContextPreview()
  }
  func applyRegistry(_ document:ModelRegistryDocument,useDefault:Bool=false) {
    let selection=useDefault ? document.defaultModelID:(document.selection(selectedModelID)==nil ? document.defaultModelID:selectedModelID)
    registry=document;selectedModelID=selection
    if let (provider,model)=document.selection(selection) {
      configuration = .init(baseURL:provider.baseURL,model:model.apiName,credentialReference:provider.credentialReference);configured=true
    } else {configuration=nil;configured=false}
    if busy {modelChangeNote="模型配置从下一轮开始使用"}
    scheduleContextPreview()
  }
  func draftEdited(_ text:String) {draft=text;draftSequence+=1;scheduleContextPreview()}
  func setMode(execute:Bool) {
    allowsChanges=execute;modelChangeNote=busy ? (execute ? "执行":"讨论")+"模式从下一轮开始使用":nil
    scheduleContextPreview()
  }
  func scheduleContextPreview() {
    previewSequence+=1;let serial=previewSequence
    previewTask?.cancel()
    previewTask=Task { [weak self] in
      do {try await Task.sleep(for:.milliseconds(350))} catch {return}
      guard let self,!closed,serial==previewSequence,let configuration else {return}
      let mode:AssistantMode=allowsChanges ? .execute:.discuss
      guard let package=Bundle.main.url(forResource:"AgentRuntime",withExtension:nil) else {contextError="本轮输入预览暂不可用";return}
      let scope=session
      let items=attachments.filter(\.sendable)
      let context=AssistantContext(host:host,storage:storage,session:scope,task:"draft-preview",configuration:configuration,mode:mode,tools:nextTools,schema:package.appendingPathComponent("Agent/docs/design/agent-tools.schema.json"))
      do {
        let canonical=try await frozenInput(text:draft,id:"draft-preview",task:"draft-preview",references:references,attachments:items)
        let preview=try await context.preview(canonical,attachments:items)
        guard !closed,!Task.isCancelled,scope==session,serial==previewSequence else {return}
        inputPreview=preview;contextError=nil
      } catch {if serial==previewSequence,!Task.isCancelled {contextError="本轮输入暂无法完整读取"}}
    }
  }
  private func frozenInput(text:String,id:String,task:String,references:[AssistantCellReference],attachments:[AssistantAttachment]=[])async throws->AgentCanonicalMessage {
    var parts:[AgentMessagePart]=[]
    if !references.isEmpty {parts.append(.agent_text_part(.init(kind:.text,part_id:"notebook-references",text:AssistantComposerRules.referencePart(references))))}
    for item in attachments {
      if item.image != nil {parts.append(.agent_media_part(.init(kind:.media,part_id:item.id,media_ref:item.id,submission_kind:.image)))}
      else if let text=item.text {
        let bytes=try await AssistantAttachmentStore.bytes(storage:storage,owner:item.owner,descriptor:text,maximum:128*1024)
        parts.append(.agent_text_part(.init(kind:.text,part_id:item.id,text:"附件：\(item.name)（PDF 提取文字）\n"+String(decoding:bytes,as:UTF8.self))))
      }
    }
    parts.append(.agent_text_part(.init(kind:.text,part_id:"question",text:text)))
    return .init(record_type:.canonical_agent_message,message_version:1,message_id:id,role:.user,agent_turn_id:.init(nil),task_id:.init(task),parts:parts)
  }
  private func acceptedDraft(sequence:UInt64,text:String,referenceIDs:Set<String>,attachmentIDs:Set<String>) {
    inputWasAccepted=true
    attachments.removeAll{attachmentIDs.contains($0.id)}
    if draftSequence==sequence,draft.utf8.elementsEqual(text.utf8),!composerMarked {
      draft="";references.removeAll{referenceIDs.contains($0.id)};acceptedInputSequence=draftSequence;scheduleContextPreview()
    }
  }
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
  @Published private(set) var roundErrors:[String:String]=[:]
  @Published private(set) var changingRound:String?
  @Published private(set) var unknownRoundChange:String?
  @Published private(set) var unknownRoundRedo=false
  private var roundOperations:[String:(String,Bool)]=[:]
  @Published private(set) var conversations:[SessionConversationSummary]=[]
  @Published private(set) var historyError:String?
  @Published private(set) var historyLoading=false
  @Published private(set) var actionDiffs:[String:SourceTransactionDiff]=[:]
  @Published private(set) var actionErrors:[String:String]=[:]
  @Published private(set) var loadingActions=Set<String>()
  @Published private(set) var knownActionTransactions=Set<String>()
  @Published private(set) var undoneActionTransactions=Set<String>()
  @Published private(set) var unknownActionUndo:String?
  @Published private(set) var actionUndoInProgress:String?
  private var actionOperations:[String:String]=[:],actionFactsSerial:UInt64=0
  var actionTransaction:(@MainActor (AgentAction)async throws->NativeStoredSourceTransaction)?
  var undoActionTransaction:(@MainActor (AgentAction,String)async throws->Void)?
  var actionUndoFacts:(@MainActor (Set<String>)async throws->SourceUndoFacts)?
  var actionCanUndo:(@MainActor (AgentAction)->Bool)?
  var actionCellLabel:(@MainActor (String)->String?)?
  var actionLocate:(@MainActor (String,Bool)->Void)?
  var conversationID:String {session}
  var conversationTitle:String {messages.first(where:{$0.role=="user"}).map{String($0.text.trimmingCharacters(in:.whitespacesAndNewlines).prefix(20)).replacingOccurrences(of:"\n",with:" ")} ?? "新对话"}
  var prepare:(@MainActor ()async throws->Void)?
  var settled:(@MainActor ()async->Void)?
  var savedState:(@MainActor ()async throws->Bool)?
  var outcomeReady:(@MainActor (TurnOutcome)async throws->Void)?
  var changeOutcome:(@MainActor (TurnOutcome,Bool,String)async throws->Void)?
  var insertCode:(@MainActor (String,String)async throws->Void)?
  func insertCodeBlock(_ code:String,language:String) async {
    guard !busy,let insertCode else {return}
    do {try await insertCode(code,language);status="已插入数学单元格，尚未运行"}
    catch {status="未插入代码，原文已保留；请核对源码或当前笔记本状态。"}
  }
  private let host:NativeHostClient,storage:StorageService,agent:AgentSessionService
  private let attachmentStore:AssistantAttachmentStore
  private let modelRegistry:ModelRegistry?
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
    self.host=host;self.storage=storage;self.agent=agent;attachmentStore=AssistantAttachmentStore(storage:storage);modelRegistry=try? ModelRegistry()
  }
  func load()async {
    await refreshHistory()
    guard !busy,configuration==nil else {return}
    do {
      if let modelRegistry,configuration==nil {
        let document=try await modelRegistry.loadInitial();applyRegistry(document,useDefault:true)
        if configured {status="就绪"}
      }
      scheduleContextPreview()
    }
    catch {status="模型配置无法读取，请重新配置。"}
  }
  func refreshHistory()async {
    guard !closed else {return}
    do {let records=try await agent.conversations();guard !closed else {return};conversations=records;historyError=nil}
    catch {if !closed {historyError="历史对话暂无法读取"}}
  }
  func selectConversation(_ id:String)async {
    guard !closed,!busy,!undoing,!historyLoading,unknownActionUndo==nil,unknownRoundChange==nil,conversations.contains(where:{$0.id==id}) else {return}
    historyLoading=true;defer {historyLoading=false}
    do {
      let previous=try await agent.transcript(session:id)
      guard !closed else {return}
      resetConversation(id:id);durableMessages=previous;publishMessages();historyError=nil
      status=configured ? "就绪":"请先配置模型"
      await refreshActionFacts()
      await restoreConclusions(id)
    } catch {if !closed {historyError="这段对话暂无法读取，当前内容保留"}}
  }
  func restoreRecordedConversation(_ id:String)async throws {
    guard !closed,!busy,!undoing,!historyLoading,unknownActionUndo==nil,unknownRoundChange==nil else {throw AgentSessionError.busy}
    let records=try await agent.recordedTranscript(session:id)
    guard !closed else {throw StorageError.closing}
    resetConversation(id:id);durableMessages=records;publishMessages();await refreshActionFacts()
    await restoreConclusions(id)
  }
  /// Trusted host selection (also used by isolated live fixtures); secrets are never serialized.
  func configure(_ value:AssistantConfiguration,secrets:SecretStore?=nil)throws {
    guard !busy,!undoing,!closed else {throw AgentSessionError.busy}
    try AssistantSettingsStore.validate(value.baseURL,model:value.model)
    configuration=value;configured=true
    registry = .current(value);selectedModelID=registry.defaultModelID
    if let secrets {self.secrets=secrets}
    status="模型已就绪"
    scheduleContextPreview()
  }
  func undoTurn(_ id:String)async {
    await changeTurn(id,redo:false)
  }
  func redoTurn(_ id:String)async {await changeTurn(id,redo:true)}
  func canChangeTurn(_ outcome:TurnOutcome,redo:Bool)->Bool {
    guard !busy,!undoing,!historyLoading,unknownActionUndo==nil,!outcome.factsUnavailable,changeOutcome != nil,
      !outcome.transactions.isEmpty else {return false}
    if let unknownRoundChange {return unknownRoundChange==outcome.id && unknownRoundRedo==redo}
    guard Set(outcome.transactions).isSubset(of:knownActionTransactions) else {return false}
    return redo ? undoneTurns.contains(outcome.id):!undoneTurns.contains(outcome.id)
  }
  private func changeTurn(_ id:String,redo:Bool)async {
    guard let outcome=outcomes.first(where:{$0.id==id}),canChangeTurn(outcome,redo:redo),let changeOutcome else {return}
    undoing=true;changingRound=id;roundErrors[id]=nil;defer {undoing=false;changingRound=nil}
    let existing=roundOperations[id]
    let operation=existing?.1==redo ? existing!.0:"round-change-"+UUID().uuidString.lowercased()
    roundOperations[id]=(operation,redo)
    do {
      try await changeOutcome(outcome,redo,operation)
      roundOperations[id]=nil;unknownRoundChange=nil
      status=redo ? "已重做本轮修改":"已撤销本轮修改"
      await refreshActionFacts()
    } catch CommitPortError.unknownOutcome {
      unknownRoundChange=id;unknownRoundRedo=redo
      roundErrors[id]="操作结果待核对；再次点按只核对原操作。"
    } catch {
      if unknownRoundChange==id {roundErrors[id]="原操作暂无法核对，操作身份已保留；可再次核对。"}
      else {roundOperations[id]=nil;roundErrors[id]="操作未提交。请核对后续编辑或当前计算；原内容保留。"}
    }
  }
  func acceptOutcome(_ outcome:TurnOutcome)async {
    outcomes.removeAll{$0.id==outcome.id};outcomes.append(outcome)
    do {try await outcomeReady?(outcome)} catch {roundErrors[outcome.id]="撤销记录暂不可用，原文稿已保留"}
    await refreshActionFacts()
  }
  func recordConclusion(_ outcome:TurnOutcome)async throws {
    let conclusion=NativeTurnConclusion(kind:"native_turn_conclusion",codecVersion:1,documentID:await agent.presentationDocumentID(),taskID:outcome.id,mode:"execute",reason:outcome.endReason,durationMs:outcome.durationMs,saved:outcome.saved)
    _ = try await storage.appendSessionEvent(.init(session:session,eventID:"turn-conclusion-"+outcome.id,sourceInstance:"turn-conclusion-"+outcome.id,sourceSequence:1,parentEvent:nil,turn:nil,task:outcome.id,kind:.recovery,payload:try RecoveryValidation.encode(conclusion),operation:nil,operationStore:nil))
  }
  private func restoreConclusions(_ id:String)async {
    do {
      var after:UInt64=0,conclusions:[NativeTurnConclusion]=[]
      var usageReference:(String,String,Int)?
      while true {
        let page=try await storage.sessionEvents(id,after:after,limit:32)
        for record in page where record.event.event_kind == .recovery {
          guard let json=record.event.payload_inline_json.value else {continue}
          if let body=try JSONSerialization.jsonObject(with:Data(json.utf8)) as? [String:Any],body["kind"] as? String=="provider_model_reply",
            let hash=body["blobHash"] as? String,let length=body["byteLength"] as? Int,length>0,length<=2*1024*1024 {
            usageReference=(record.event.event_id,hash,length)
          }
          guard
            let conclusion=try? JSONDecoder().decode(NativeTurnConclusion.self,from:Data(json.utf8)),
            conclusion.kind=="native_turn_conclusion",conclusion.codecVersion==1,
            conclusion.taskID==record.event.task_id.value,conclusion.mode=="execute",conclusion.durationMs>=0 else {continue}
          conclusions.append(conclusion)
        }
        guard let last=page.last else {break};after=last.event.event_sequence
      }
      if let (owner,hash,length)=usageReference {
        let lease=try await storage.openReferencedBlob(owner:.init(kind:.context,id:owner),hash:hash)
        let bytes:Data
        do {bytes=try await storage.readBlob(lease,offset:0,count:length);try await storage.closeBlob(lease)}
        catch {try? await storage.closeBlob(lease);throw error}
        guard KernelHashes.data(bytes)==hash else {throw StorageError.corruptIdentity}
        if let payload=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],let usage=payload["usageJSON"] as? String {lastUsage=AssistantUsage(Data(usage.utf8))}
      }
      for conclusion in conclusions {
        var outcome:TurnOutcome
        do {
          let facts=try await agent.outcomeFacts(session:id,task:conclusion.taskID)
          outcome=TurnOutcome.assemble(task:conclusion.taskID,reason:conclusion.reason,facts:facts,saved:conclusion.saved,durationMs:conclusion.durationMs)
        } catch {
          outcome=TurnOutcome.assemble(task:conclusion.taskID,reason:conclusion.reason,facts:[],saved:conclusion.saved,durationMs:conclusion.durationMs)
          outcome.factsUnavailable=true
        }
        guard !closed,session==id else {return}
        // Historical display never registers old transactions into the current native undo stack.
        outcomes.removeAll{$0.id==outcome.id};outcomes.append(outcome)
      }
      await refreshActionFacts()
    } catch {if !closed,session==id {transcriptError="部分回合结论暂无法读取，原操作记录仍保留"}}
  }
  func locate(_ cell:String,reducedMotion:Bool) {guard actionCellLabel?(cell) != nil else {return};actionLocate?(cell,reducedMotion)}
  func actionDiff(_ action:AgentAction)->SourceTransactionDiff? {
    guard let diff=actionDiffs[action.id],diff.transaction==(action.transactionRef ?? action.id) else {return nil};return diff
  }
  func loadActionDiff(_ action:AgentAction)async {
    guard !closed,!loadingActions.contains(action.id),let read=actionTransaction else {return}
    if action.transactionRef==nil,let preview=action.preview {
      let diff=await Task.detached(priority:.utility) {SourceTransactionDiff.make(preview:preview,identity:action.id)}.value
      if !closed {actionDiffs[action.id]=diff};return
    }
    loadingActions.insert(action.id);actionErrors[action.id]=nil;defer {loadingActions.remove(action.id)}
    do {
      let record=try await read(action)
      let difference=await Task.detached(priority:.utility) {SourceTransactionDiff.make(record)}.value
      guard !closed,!Task.isCancelled else {return};actionDiffs[action.id]=difference
    } catch {if !closed {actionErrors[action.id]="原始源码差异暂不可用；可能已超出历史保留期。"}}
  }
  func refreshActionFacts()async {
    guard !closed,let read=actionUndoFacts else {return}
    actionFactsSerial+=1;let serial=actionFactsSerial
    let refs=Set(messages.flatMap(\.actions).compactMap(\.transactionRef)+outcomes.flatMap(\.transactions))
    guard !refs.isEmpty else {knownActionTransactions=[];undoneActionTransactions=[];undoneTurns=[];return}
    do {
      var known=Set<String>(),undone=Set<String>()
      let list=refs.sorted()
      for offset in stride(from:0,to:list.count,by:512) {
        let facts=try await read(Set(list[offset..<min(offset+512,list.count)]));known.formUnion(facts.known);undone.formUnion(facts.undone)
      }
      guard !closed,serial==actionFactsSerial else {return}
      knownActionTransactions=known;undoneActionTransactions=undone
      undoneTurns=Set(outcomes.filter{!$0.transactions.isEmpty && Set($0.transactions).isSubset(of:known) && Set($0.transactions).isSubset(of:undone)}.map(\.id))
    } catch {if !closed,serial==actionFactsSerial {knownActionTransactions=[]}}
  }
  func undoAction(_ action:AgentAction)async {
    guard !closed,!busy,!undoing,!historyLoading,unknownRoundChange==nil,let transaction=action.transactionRef,
      action.tool=="apply_notebook_patch",action.status == .ok,(!undoneActionTransactions.contains(transaction) || unknownActionUndo==action.id),
      (knownActionTransactions.contains(transaction) || unknownActionUndo==action.id),actionCanUndo?(action)==true,
      unknownActionUndo==nil || unknownActionUndo==action.id,let perform=undoActionTransaction else {return}
    undoing=true;actionUndoInProgress=action.id;actionErrors[action.id]=nil;defer {undoing=false;actionUndoInProgress=nil}
    let operation=actionOperations[transaction] ?? "action-undo-"+UUID().uuidString.lowercased();actionOperations[transaction]=operation
    do {try await perform(action,operation);unknownActionUndo=nil;actionOperations[transaction]=nil;status="已撤销这笔修改";await refreshActionFacts()}
    catch CommitPortError.unknownOutcome {unknownActionUndo=action.id;status="撤销结果待核对";actionErrors[action.id]="撤销结果待核对；再次点按只核对原操作。"}
    catch {
      if unknownActionUndo==action.id {actionErrors[action.id]="原撤销暂无法核对，操作身份已保留；可再次核对。"}
      else {actionOperations[transaction]=nil;actionErrors[action.id]="撤销未提交。请核对后续编辑或当前计算；原内容保留。"}
    }
  }
  func newConversation() {
    guard !closed,!busy,!undoing,!historyLoading,unknownActionUndo==nil,unknownRoundChange==nil else {return}
    resetConversation(id:"conversation-"+UUID().uuidString.lowercased())
    allowsChanges=UserDefaults.standard.string(forKey:"assistant.default-mode")=="execute"
    applyRegistry(registry,useDefault:true)
  }
  private func resetConversation(id:String) {
    session=id
    messages=[];durableMessages=[];streamedText=[:];streamedOrder=[];pendingText=[:];pendingTurns=[];pendingInput=nil
    streamedContexts=[:];streamedTasks=[:];outcomes=[];undoneTurns=[];undoError=nil;transcriptError=nil;status=configured ? "新对话":"请先配置模型"
    roundErrors=[:];roundOperations=[:];actionDiffs=[:];actionErrors=[:];knownActionTransactions=[];undoneActionTransactions=[]
    lastUsage=nil;lastRequestInput=nil;inputPreview=nil;scheduleContextPreview()
    modelChangeNote=nil
  }
  func send() {
    guard !closed,canSend else {return}
    let text=draft
    guard text.utf8.count<=16384 else {status="请输入问题（最多16KiB）。";return}
    guard let configuration else {showSettings=true;return}
    let candidates=Dictionary(uniqueKeysWithValues:(referenceCandidates?() ?? []).map{($0.id,$0)})
    guard references.allSatisfy({candidates[$0.id] != nil}) else {status="引用的单元格已删除，请移除引用后再发送";return}
    busy=true;stopping=false;status="正在准备";transcriptError=nil;modelChangeNote=nil
    let edits=allowsChanges,refs=references.compactMap{candidates[$0.id]},sequence=draftSequence
    let id="user-"+UUID().uuidString.lowercased(),presentation=AssistantMessageContext(model:configuration.model,mode:edits ? "execute":"discuss",references:refs,attachments:attachments.filter(\.sendable))
    activePresentation=presentation
    pendingInput = .init(id:id,taskID:"",role:"user",context:presentation,parts:[.init(id:"question",value:.text(text))])
    publishMessages();pendingDraftSequence=sequence;acceptedInputSequence=nil;inputWasAccepted=false
    let tools=nextTools
    work=Task {await execute(text,id:id,configuration:configuration,edits:edits,tools:tools,presentation:presentation)}
  }
  private func execute(_ text:String,id:String,configuration:AssistantConfiguration,edits:Bool,tools:[AgentToolName],presentation:AssistantMessageContext)async {
    let started=ContinuousClock.now
    var taskID:String?,reason:TurnEndReason = .failed("请求尚未开始")
    do {
      try await prepare?()
      guard !stopping,!closed else {throw CancellationError()}
      let process=try await AgentProcess.bundled()
      let hello=try await agent.activate(session:session,grants:tools,readonly:!edits,coreVersion:"1.0.4",adapterVersion:"openmath-adapter-1",conversationTitle:text)
      guard !stopping,!closed else {throw CancellationError()}
      let scope=hello.scope
      taskID=scope.task_id;activeTask=scope.task_id
      if let pendingInput {
        self.pendingInput = .init(id:pendingInput.id,taskID:scope.task_id,role:pendingInput.role,context:pendingInput.context,parts:pendingInput.parts)
        publishMessages()
      }
      await agent.observeTranscript { [weak self] in await self?.refreshMessages() }
      let context=AssistantContext(host:host,storage:storage,session:session,task:scope.task_id,configuration:configuration,mode:edits ? .execute:.discuss,tools:tools,schema:process.spec.root.appendingPathComponent("Agent/docs/design/agent-tools.schema.json"),observed:{[weak self] input in await self?.observeInput(input)},completed:{[weak self] usage in await self?.observeUsage(usage)})
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
      let input=try await frozenInput(text:text,id:id,task:scope.task_id,references:presentation.references ?? [],attachments:presentation.attachments ?? [])
      let sequence=pendingDraftSequence,referenceIDs=Set((presentation.references ?? []).map(\.id))
      let attachmentIDs=Set((presentation.attachments ?? []).map(\.id))
      let report=try await agent.run(pipe:actual,hello:hello,modelPort:visible,input:input,inputPresentation:presentation,inputAccepted:{[weak self] in await self?.acceptedDraft(sequence:sequence,text:text,referenceIDs:referenceIDs,attachmentIDs:attachmentIDs)})
      reason=await agent.endReason(report.loopReason,stopped:stopping || closed)
    } catch {
      if taskID==nil,!stopping,!closed {reason = .failed("无法准备当前笔记本或助手运行环境")}
      else {reason=await agent.endReason(nil,stopped:stopping || closed)}
    }
    flushText()
    await adapter?.close();adapter=nil
    if let pipe {await pipe.close();self.pipe=nil}
    await agent.resetTask();await settled?()
    if !inputWasAccepted {pendingInput=nil}
    await refreshMessages()
    if edits,let taskID {
      let elapsed=started.duration(to:.now).components
      let duration=Int(elapsed.seconds*1000+elapsed.attoseconds/1_000_000_000_000_000)
      do {
        let facts=try await agent.outcomeFacts(session:session,task:taskID)
        let saved=try await savedState?() ?? false
        let outcome=TurnOutcome.assemble(task:taskID,reason:reason,facts:facts,saved:saved,durationMs:duration)
        await acceptOutcome(outcome)
        do {
          try await recordConclusion(outcome)
        } catch {transcriptError="本轮结论暂未保存到历史；当前事实和文稿修改仍保留"}
      } catch {
        // Missing evidence is explicit. Do not replace unreadable facts with a claim of no edits.
        var outcome=TurnOutcome.assemble(task:taskID,reason:reason,facts:[],saved:false,durationMs:duration)
        outcome.factsUnavailable=true;await acceptOutcome(outcome)
        try? await recordConclusion(outcome)
      }
    }
    status=UIStrings.end(reason)
    if case .failed=reason,draft.isEmpty,let acceptedInputSequence,draftSequence==acceptedInputSequence {
      draft=text
      for reference in presentation.references ?? [] where !references.contains(where:{$0.id==reference.id}) {references.append(reference)}
      scheduleContextPreview()
    }
    if case .failed=reason {
      for item in presentation.attachments ?? [] where !removedAttachmentIDs.contains(item.id) && !attachments.contains(where:{$0.id==item.id}) {attachments.append(item)}
      scheduleContextPreview()
    }
    if outcomes.last?.id==taskID,outcomes.last?.factsUnavailable==true {status+="；本轮操作事实暂时无法读取，请核对笔记本"}
    busy=false;stopping=false;work=nil
    await refreshHistory()
    await refreshActionFacts()
  }
  private func observeInput(_ input:AssistantInputSnapshot) {lastRequestInput=input;contextError=nil}
  private func observeUsage(_ data:Data) {lastUsage=AssistantUsage(data)}
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
    scheduleContextPreview()
  }
  func stop()async {
    guard busy,!stopping else {return};stopping=true;status="正在停止"
    _ = try? await agent.cancel()
    await adapter?.close()
    if let pipe {await pipe.close()}
  }
  func close()async {closed=true;previewTask?.cancel();await stop();await work?.value}
}
