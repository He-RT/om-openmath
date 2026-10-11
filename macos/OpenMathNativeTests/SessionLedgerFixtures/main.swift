import Foundation
import SQLite3

private struct Plans:Decodable {let initial:NativeSourceSnapshot;let first:NativeSourceCommit}
private final class Count:@unchecked Sendable {
  private let lock=NSLock();private var count=0
  func hit() {lock.withLock {count+=1}}
  var value:Int {lock.withLock {count}}
}
private final class Once:@unchecked Sendable {
  private let lock=NSLock();private var used=false
  func claim()->Bool {lock.withLock {guard !used else {return false};used=true;return true}}
}
@main struct SessionLedgerFixtures {
  static let session="session-fixture",turn="turn-fixture",task="task-fixture"
  private static func fixtureHTTP(_ wire:Data) async throws->SessionWireResponse {
    guard let value=ProcessInfo.processInfo.environment["OPENMATH_SESSION_FIXTURE_URL"],let url=URL(string:value),url.scheme=="http",url.host=="127.0.0.1" else {throw StorageError.corruptIdentity}
    let configuration=URLSessionConfiguration.ephemeral;configuration.timeoutIntervalForRequest=5
    let session=URLSession(configuration:configuration);defer {session.invalidateAndCancel()}
    var request=URLRequest(url:url);request.httpMethod="POST";request.httpBody=wire;request.setValue("application/json",forHTTPHeaderField:"Content-Type")
    let (bytes,response)=try await session.data(for:request)
    guard let http=response as? HTTPURLResponse else {throw StorageError.corruptIdentity}
    return .init(bytes:bytes,successful:http.statusCode==200)
  }
  private static func prepare(root:URL,plans:Plans) async throws->(StorageService,SessionLedgerPort,SessionToolAdmission) {
    let (storage,library)=try await StorageService.open(paths:.init(root:root,channel:.preview));precondition(library.identity.storeVersion==4)
    let info=try await storage.openDocument(plans.initial.document_id);_ = try await storage.initializeSource(plans.initial)
    _ = try await storage.createAgentSession(session)
    let ledger=SessionLedgerPort(storage:storage,session:session,sourceInstance:"host-original")
    let arguments=Data("{\"instruction\":\"本轮合成修改🙂\"}".utf8)
    let call=SessionToolCall(callID:"call-edit",toolName:"notebook.apply_patch",arguments:arguments)
    let proposal=SessionAssistantEnd(codecVersion:1,messageID:"assistant-message",textUTF8:Data("编辑一个单元格".utf8),calls:[call])
    _ = try await ledger.record(proposal,id:"proposal",kind:.assistant_end,turn:turn,task:task)
    let target=SessionOperationTarget(documentID:plans.initial.document_id,storeID:info.identity.storeID,operationID:plans.first.commit.operation_id,kind:.source,expectedHash:plans.first.commit.request_hash)
    return (storage,ledger,.init(codecVersion:1,callID:call.callID,proposalEvent:"proposal",toolName:call.toolName,argumentsHash:KernelHashes.data(arguments),target:target))
  }
  private static func checkRecovery(_ root:URL,plans:Plans) async throws {
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));_ = try await storage.openDocument(plans.initial.document_id)
    let before=try await storage.sessionRecovery(session),sourceBefore=try await storage.committedSource(plans.initial.document_id)!
    let ledger=SessionLedgerPort(storage:storage,session:session,sourceInstance:"host-recovery")
    let recovered=try await RecoveryService(storage:storage).recoverSession(session,ledger:ledger)
    let repeated=try await RecoveryService(storage:storage).recoverSession(session,ledger:ledger)
    let sourceAfter=try await storage.committedSource(plans.initial.document_id)!
    precondition(sourceAfter.snapshot_hash==sourceBefore.snapshot_hash && recovered.tools[0].phase==repeated.tools[0].phase)
    let events=try await storage.sessionEvents(session,limit:32)
    let json:[String:Any]=["before":before.tools[0].phase.rawValue,"after":recovered.tools[0].phase.rawValue,"source_revision":sourceAfter.revision.value,"events":events.count,"requests":recovered.requests.map{["context":$0.contextID,"phase":$0.phase.rawValue]}]
    print(String(decoding:try JSONSerialization.data(withJSONObject:json,options:[.sortedKeys]),as:UTF8.self))
    try await storage.close()
  }
  static func main() async throws {
    precondition(om_fixture_install_sync_probe()==SQLITE_OK)
    let plans=try JSONDecoder().decode(Plans.self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[1]))),root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    if CommandLine.arguments.count>3,CommandLine.arguments[3]=="damage-check" {
      let database=root.appendingPathComponent("Library/Generations/000001/library.sqlite"),before=try Data(contentsOf:database)
      do {_ = try await StorageService.open(paths:.init(root:root,channel:.preview));fatalError("damaged full event swallowed")} catch StorageError.corruptIdentity { }
      let workspace=try await StorageService.openWorkspace(paths:.init(root:root,channel:.preview))
      guard case .unavailable(.corruptIdentity)=await workspace.libraryAvailability() else {fatalError("corrupt Library became ready")}
      _ = try await workspace.openDocument(plans.initial.document_id)
      let retained=try await workspace.committedSource(plans.initial.document_id)!;precondition(retained.revision.value==1)
      try await workspace.close();let after=try Data(contentsOf:database);precondition(before==after)
      print("Complete canonical event corruption rejected; original Library bytes and healthy source retained")
      return
    }
    if CommandLine.arguments.count>3,CommandLine.arguments[3]=="recover" {try await checkRecovery(root,plans:plans);return}
    let (storage,ledger,admission)=try await prepare(root:root,plans:plans),executions=Count()
    let tool=SessionToolPort(storage:storage,ledger:ledger)
    if CommandLine.arguments.count>4,CommandLine.arguments[3]=="crash" {
      let stage=CommandLine.arguments[4]
      let faults=StorageFaults {point in if point==stage {FileHandle.standardOutput.write(Data((point+"\n").utf8));raise(SIGSTOP)}}
      if stage.hasPrefix("request_") {
        let request=SessionRequestPort(storage:storage,ledger:ledger)
        _ = try await request.send(context:"context-crash",wire:Data("{\"messages\":[\"数学问题🙂\"]}".utf8),adapter:"fixture-v1",turn:turn,task:task,faults:faults) {wire in
          let response=try await fixtureHTTP(wire);try Data("dispatch-once".utf8).write(to:root.appendingPathComponent("dispatch-count"));return response
        }
      } else {
        _ = try await tool.perform(admission:admission,admissionID:"admit",startedID:"start",resultID:"result",turn:turn,task:task,faults:faults) {
          _ = try await storage.commitSource(plans.first,faults:faults)
        }
      }
      fatalError("actual requested crash boundary not reached")
    }
    let initial=try await storage.sessionRecovery(session);precondition(initial.tools.count==1 && initial.tools[0].phase == .tool_not_started)
    let admitted=try await ledger.record(admission,id:"admit",kind:.tool_admitted,parent:"proposal",turn:turn,task:task,target:admission.target)
    precondition(admitted.inserted)
    let fake=SessionToolResult(codecVersion:1,admissionEvent:"admit",sourceEventID:"unpublished",sourceEffectHash:String(repeating:"a",count:64),rawReceiptHash:String(repeating:"b",count:64),recovered:true,outcome:.completed)
    do {_ = try await ledger.record(fake,id:"fake",kind:.tool_result,parent:"admit",turn:turn,task:task,target:admission.target);fatalError("fake result stored")} catch SessionLedgerError.wrongSourceEffect { }
    do {_ = try await tool.perform(admission:admission,admissionID:"admit",startedID:"start",resultID:"result",turn:turn,task:task) {executions.hit();_ = try await storage.commitSource(plans.first)};fatalError("old admission reexecuted")} catch SessionLedgerError.reconciliationRequired { }
    precondition(executions.value==0)
    _ = try await storage.createAgentSession("unresolved-other-session")
    let blockedLedger=SessionLedgerPort(storage:storage,session:"unresolved-other-session",sourceInstance:"blocked-host")
    let blockedArgs=Data("{\"instruction\":\"本轮合成修改🙂\"}".utf8)
    _ = try await blockedLedger.record(SessionAssistantEnd(codecVersion:1,messageID:"blocked-proposal",textUTF8:Data(),calls:[.init(callID:"blocked-call",toolName:admission.toolName,arguments:blockedArgs)]),id:"blocked-proposal",kind:.assistant_end,turn:turn,task:task)
    let blocked=SessionToolAdmission(codecVersion:1,callID:"blocked-call",proposalEvent:"blocked-proposal",toolName:admission.toolName,argumentsHash:KernelHashes.data(blockedArgs),target:admission.target)
    do {_ = try await SessionToolPort(storage:storage,ledger:blockedLedger).perform(admission:blocked,admissionID:"blocked-admit",startedID:"blocked-start",resultID:"blocked-result",turn:turn,task:task) {executions.hit();fatalError("unresolved cross-session replay")};fatalError("unresolved operation claimed by another session")} catch SessionLedgerError.reconciliationRequired { }
    precondition(executions.value==0)
    _ = try await storage.commitSource(plans.first)
    let outcome=try await RecoveryService(storage:storage).recoverSession(session,ledger:ledger)
    precondition(outcome.tools[0].phase == .completed)
    let again=try await RecoveryService(storage:storage).recoverSession(session,ledger:ledger);precondition(again.tools[0].resultEvent==outcome.tools[0].resultEvent)
    let wrongTarget=SessionOperationTarget(documentID:admission.target.documentID,storeID:admission.target.storeID,operationID:admission.target.operationID,kind:.source,expectedHash:String(repeating:"c",count:64))
    do {_ = try await storage.originalOperationEffect(wrongTarget);fatalError("mismatched original effect accepted")} catch SessionLedgerError.wrongSourceEffect { }
    let lostOnce=Once(),lost=StorageFaults {point in if point=="session_event_committed" && lostOnce.claim() {throw StorageError.system(EIO,"lost-event-ACK")}}
    let user=SessionAssistantEnd(codecVersion:1,messageID:"user-data",textUTF8:Data("中文🙂".utf8),calls:[])
    do {_ = try await ledger.record(user,id:"lost-event",kind:.user_message,turn:turn,task:task,faults:lost);fatalError("unknown ACK confirmed")} catch StorageError.unknownCommit { }
    do {_ = try await ledger.record(user,id:"new-event-before-readback",kind:.user_message,turn:turn,task:task);fatalError("unknown sequence reused")} catch SessionLedgerError.reconciliationRequired { }
    let original=try await ledger.record(user,id:"lost-event",kind:.user_message,turn:turn,task:task);precondition(!original.inserted)
    _ = try await ledger.record(user,id:"new-event-after-readback",kind:.user_message,turn:turn,task:task)
    let syncOnce=Once(),syncFailure=StorageFaults {point in if point=="session_event_before_commit" && syncOnce.claim() {om_fixture_fail_fullsync(1)}}
    do {_ = try await ledger.record(user,id:"real-fullsync-unknown",kind:.user_message,turn:turn,task:task,faults:syncFailure);fatalError("real failed durability acknowledged")} catch StorageError.unknownCommit { }
    do {_ = try await storage.sessionEvent(session,event:"real-fullsync-unknown");fatalError("unsynced real event confirmed")} catch StorageError.unknownCommit { }
    om_fixture_fail_fullsync(0)
    let actualSynced=try await ledger.record(user,id:"real-fullsync-unknown",kind:.user_message,turn:turn,task:task);precondition(!actualSynced.inserted)
    let gap=SessionAppendInput(session:session,eventID:"gap",sourceInstance:"gap-producer",sourceSequence:2,parentEvent:nil,turn:turn,task:task,kind:.user_message,payload:Data("{}".utf8),operation:nil,operationStore:nil)
    do {_ = try await storage.appendSessionEvent(gap);fatalError("producer gap silently repaired")} catch SessionLedgerError.sequenceGap { }
    let stream=SessionAssistantEnd(codecVersion:1,messageID:"stream",textUTF8:Data("partial".utf8),calls:[.init(callID:"forged-partial-call",toolName:"notebook.apply_patch",arguments:Data("{}".utf8))])
    do {_ = try await ledger.record(stream,id:"partial-call",kind:.assistant_batch,turn:turn,task:task);fatalError("partial stream admitted tool parameters")} catch SessionLedgerError.invalidTransition { }
    let request=SessionRequestPort(storage:storage,ledger:ledger),requests=Count()
    let reply=try await request.send(context:"context-completed",wire:Data("{\"messages\":[\"问题🙂\"]}".utf8),adapter:"fixture-v1",turn:turn,task:task) {wire in
      requests.hit();precondition(String(data:wire,encoding:.utf8)!.contains("问题🙂"));return try await fixtureHTTP(wire)
    };precondition(String(data:reply.bytes,encoding:.utf8)=="fixture答复🙂" && requests.value==1)
    do {_ = try await request.send(context:"context-completed",wire:Data("{\"messages\":[\"问题🙂\"]}".utf8),adapter:"fixture-v1",turn:turn,task:task) {_ in requests.hit();return .init(bytes:Data("duplicate".utf8),successful:true)};fatalError("same context dispatched twice")} catch SessionLedgerError.reconciliationRequired { }
    precondition(requests.value==1)
    let failed=try await request.send(context:"context-http-error",wire:Data("{\"fixture_failure\":true}".utf8),adapter:"fixture-v1",turn:turn,task:task) {wire in try await fixtureHTTP(wire)}
    precondition(!failed.successful && String(data:failed.bytes,encoding:.utf8)=="provider unavailable")
    let final=try await storage.sessionRecovery(session);precondition(final.requests.first{$0.contextID=="context-completed"}?.phase == .finished)
    precondition(final.requests.first{$0.contextID=="context-http-error"}?.phase == .failed)
    _ = try await storage.createAgentSession("second-session")
    let second=SessionLedgerPort(storage:storage,session:"second-session",sourceInstance:"host-second")
    _ = try await second.record(user,id:"lost-event",kind:.user_message)
    let oldCall=SessionToolCall(callID:"same-original-call",toolName:admission.toolName,arguments:Data("{\"instruction\":\"本轮合成修改🙂\"}".utf8))
    _ = try await second.record(SessionAssistantEnd(codecVersion:1,messageID:"original-fact-proposal",textUTF8:Data(),calls:[oldCall]),id:"original-fact-proposal",kind:.assistant_end,turn:turn,task:task)
    let originalAdmission=SessionToolAdmission(codecVersion:1,callID:oldCall.callID,proposalEvent:"original-fact-proposal",toolName:oldCall.toolName,argumentsHash:KernelHashes.data(oldCall.arguments),target:admission.target)
    let secondTool=SessionToolPort(storage:storage,ledger:second)
    let existing=try await secondTool.perform(admission:originalAdmission,admissionID:"original-fact-admit",startedID:"original-fact-start",resultID:"original-fact-result",turn:turn,task:task) {executions.hit();fatalError("existing original operation executed again")}
    precondition(existing.stored.event.event_kind == .tool_result && executions.value==0)
    let firstPage=try await storage.sessionEvents(session,limit:1),nextPage=try await storage.sessionEvents(session,after:firstPage[0].event.event_sequence,limit:1)
    precondition(nextPage.count==1 && nextPage[0].event.event_sequence>firstPage[0].event.event_sequence)
    try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));_ = try await reopened.openDocument(plans.initial.document_id)
    let recovered=try await reopened.sessionRecovery(session),source=try await reopened.committedSource(plans.initial.document_id)!
    precondition(recovered.tools[0].phase == .completed && recovered.requests[0].phase == .finished && source.revision.value==1)
    try await reopened.close()
    // Real long history crosses both former limits. Small indexed reads must still admit
    // new calls and recover older/later calls without dropping unrelated raw events.
    let longRoot=root.appendingPathComponent("long-history",isDirectory:true)
    let (longStore,_)=try await StorageService.open(paths:.init(root:longRoot,channel:.preview))
    _ = try await longStore.createAgentSession("long-session")
    let longLedger=SessionLedgerPort(storage:longStore,session:"long-session",sourceInstance:"long-host")
    let text=Data(String(repeating:"中文🙂 ",count:512).utf8)
    let longMessage=SessionAssistantEnd(codecVersion:1,messageID:"long-stream",textUTF8:text,calls:[])
    for index in 0..<4300 {_ = try await longLedger.record(longMessage,id:"history-\(index)",kind:.assistant_batch,turn:turn,task:task)}
    var proposalIDs:[String]=[]
    for index in 0..<37 {
      let eventID="after-limit-proposal-\(index)";proposalIDs.append(eventID)
      _ = try await longLedger.record(SessionAssistantEnd(codecVersion:1,messageID:eventID,textUTF8:Data(),calls:[.init(callID:"long-call-\(index)",toolName:"read_notebook",arguments:Data("{}".utf8))]),id:eventID,kind:.assistant_end,turn:turn,task:task)
    }
    var cursor:SessionRecoveryCursor?,allCalls:[String]=[],finished=false
    for _ in 0..<10 {
      let page=try await longStore.sessionRecovery("long-session",cursor:cursor,limit:5)
      precondition(page.tools.count<=5 && page.tools.allSatisfy{$0.phase == .tool_not_started})
      allCalls+=page.tools.map(\.callID);cursor=page.nextCursor;finished=page.complete
      if finished {break}
    }
    precondition(finished && allCalls.count==37 && Set(allCalls).count==37)
    let wire=try await longStore.publishBlob(data:Data("{\"messages\":[]}".utf8))
    for index in 0..<37 {
      let context="long-context-\(index)"
      let plan=SessionRequestPlan(codecVersion:1,contextID:context,wireBlobHash:wire.descriptor.hash,wireByteLength:wire.descriptor.byteLength,adapterVersion:"fixture-v1")
      _ = try await longLedger.record(SessionRequestLifecycle(kind:.request_planned,plan:plan,transition:nil),id:context+".planned",kind:.task_transition,turn:turn,task:task,resource:wire,resourceOwner:.init(kind:.context,id:"request-long-session-"+context))
    }
    try await longStore.releaseBlob(wire)
    cursor=nil;allCalls=[];var requestIDs:[String]=[];finished=false
    for _ in 0..<10 {
      let page=try await longStore.sessionRecovery("long-session",cursor:cursor,limit:5)
      precondition(page.tools.count<=5 && page.requests.count<=5 && page.requests.allSatisfy{$0.phase == .not_dispatched})
      allCalls+=page.tools.map(\.callID);requestIDs+=page.requests.map(\.contextID);cursor=page.nextCursor;finished=page.complete
      if finished {break}
    }
    precondition(finished && Set(allCalls).count==37 && Set(requestIDs).count==37)
    let firstLongPage=try await longStore.sessionEvents("long-session",limit:32)
    precondition(firstLongPage.count==32 && firstLongPage[0].event.event_sequence==1)
    let identities=try await longStore.agentSessions();precondition(identities.count==1 && identities[0].eventCount==4374)
    try await longStore.close()
    let (longReopen,_)=try await StorageService.open(paths:.init(root:longRoot,channel:.preview))
    let firstRecovery=try await longReopen.sessionRecovery("long-session",limit:5);precondition(!firstRecovery.complete && firstRecovery.tools.count==5)
    try await longReopen.close()
    let longDB=longRoot.appendingPathComponent("Library/Generations/000001/library.sqlite")
    let actualSizes=try await Task.detached {let db=try SQLiteDatabase(url:longDB,readonly:true);defer {try? db.close()};return (try db.integer("SELECT count(*) FROM session_events"),try db.integer("SELECT sum(length(record)) FROM session_events"))}.value
    precondition(actualSizes.0==4374 && actualSizes.1>16*1024*1024)
    print("Actual long session4374 events/>16MiB: new calls admitted beyond4096, all37 calls and37 request plans recovered in bounded5-row pages, original raw prefix retained and restart passed")
    print("Canonical actual SQLite session: proposal/admission/unknown/original document recovered result, no callback replay; sequence/idempotency/gap/stream refusal, original ACK and request planned/dispatch/response/finish byte facts passed")
  }
}
