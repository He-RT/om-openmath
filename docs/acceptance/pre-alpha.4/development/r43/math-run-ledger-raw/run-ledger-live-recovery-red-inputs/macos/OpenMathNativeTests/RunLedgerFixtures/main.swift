import Foundation
import SQLite3

struct RunContext:Decodable {let kernel_build:String,general_hash:String}
private actor Counter {
  let url:URL
  init(_ url:URL) {self.url=url}
  func hit(_ stage:String)throws {
    var list=(try? JSONDecoder().decode([String].self,from:Data(contentsOf:url))) ?? []
    list.append(stage);try RecoveryValidation.encode(list).write(to:url,options:.atomic)
  }
}
private final class FaultCounter:@unchecked Sendable {
  private let lock=NSLock();private var count=0
  func hit()->Int {lock.withLock {count+=1;return count}}
}
@main struct RunLedgerFixtures {
  static let session="run-session",turn="run-turn",task="run-task",admissionID="run-admission",group="run-group-original"
  static func rust(_ executable:URL,_ args:[String])async throws {
    try await Task.detached {
      let process=Process();process.executableURL=executable;process.arguments=args
      try process.run();process.waitUntilExit();precondition(process.terminationStatus==0,"original Rust math/receipt verification failed")
    }.value
  }
  static func faults(_ stage:String,_ selected:String?,_ point:String?)->StorageFaults {
    .init {current in
      if let point,let selected,stage==selected,current==point {
        FileHandle.standardOutput.write(Data(("STOP "+point+" "+stage+"\n").utf8));raise(SIGSTOP)
      }
    }
  }
  static func admission(initial:NativeSourceSnapshot,context:RunContext,store:String,order:[String],continueOnError:Bool=true)throws->(SessionToolAdmission,Data) {
    let args=try JSONSerialization.data(withJSONObject:["snapshot_ref":"snapshot-native-ledger-fixture","cell_ids":order,"scope":"selected","continue_on_error":continueOnError],options:[.sortedKeys,.withoutEscapingSlashes])
    let target=SessionOperationTarget(documentID:initial.document_id,storeID:store,operationID:group,kind:.kernel,expectedHash:initial.snapshot_hash)
    let base=SessionToolAdmission(codecVersion:1,callID:"run-call",proposalEvent:"run-proposal",toolName:"run_cells",argumentsHash:KernelHashes.data(args),target:target)
    let children=try ([String?](arrayLiteral:nil)+order.map {Optional($0)}).enumerated().map {index,id in
      SessionRunChild(operationID:try SessionRunSemantics.childOperation(base,ordinal:index),cellID:id,cellSourceHash:id.map {id in KernelHashes.data(Data(initial.file.cells.first{$0.id==id}!.source.utf8))})
    }
    let plan=SessionRunPlan(codecVersion:1,scope:"selected",requestedCellIDs:order,children:children,blockedCells:[],continueOnError:continueOnError,
      documentGeneration:1,sourceRevision:initial.revision.value,executionEpoch:initial.execution_epoch.value,configRevision:0,
      kernelBuild:context.kernel_build,generalHash:context.general_hash,initialCheckpointID:nil,initialCheckpointHash:nil,initialKernelRevision:nil)
    return (.init(codecVersion:1,callID:base.callID,proposalEvent:base.proposalEvent,toolName:base.toolName,argumentsHash:base.argumentsHash,target:target,run:plan),args)
  }
  static func fakeReceipt(_ p:NativeKernelCommit)->NativeKernelReceipt {
    .init(protocol_version:1,store_id:p.store_id,document_id:p.producer.document_id,operation_id:p.operation_id,request_hash:p.request_hash,
      checkpoint_id:p.checkpoint_id,checkpoint_blob_hash:p.checkpoint_blob_hash,checkpoint_byte_length:p.checkpoint_byte_length,codec_version:1,
      producer:p.producer,accepted_source_revision:p.acceptance_source.revision,accepted_snapshot_hash:p.acceptance_source.snapshot_hash,
      kernel_state_revision:p.producer.kernel_state_revision,result_id:p.result_id,outbox_event_id:p.outbox_event_id,committed_at:"2026-10-15T00:00:00Z")
  }
  static func rejectUnpublishedResult(storage:StorageService,ledger:SessionLedgerPort,plan:NativeKernelCommit)async throws {
    let start=try await storage.sessionRunSnapshot(session,admission:admissionID),i=start.activeChild!
    let target=try SessionRunSemantics.target(start.admission,index:i)
    let original=SessionRunOriginalReceipt(sourceEventID:plan.outbox_event_id,sourceEffectHash:String(repeating:"a",count:64),rawReceiptHash:KernelHashes.data(try RecoveryValidation.encode(fakeReceipt(plan))),receipt:fakeReceipt(plan))
    let progress=try SessionRunSemantics.applying(original,to:start.progress,child:start.admission.run!.children[i],plan:start.admission.run!,target:target)
    let forged=SessionRunTransition(kind:.run_child_result,codecVersion:1,groupAdmission:admissionID,childIndex:i,childAdmission:start.transition!.childAdmission,original:original,code:nil,phase:nil,recovered:false,progress:progress)
    do {
      _ = try await ledger.record(forged,id:"forged-unpublished-result",kind:.task_transition,parent:start.latest.event_id,turn:turn,task:task,target:target)
      fatalError("fabricated same-shape receipt passed original Library effect check")
    } catch SessionLedgerError.wrongSourceEffect { }
    let missing=try await storage.sessionEvent(session,event:"forged-unpublished-result");precondition(missing==nil)
    let branch=SessionRunTransition(kind:.run_child_started,codecVersion:1,groupAdmission:admissionID,childIndex:i,childAdmission:start.transition!.childAdmission,original:nil,code:nil,phase:nil,recovered:false,progress:start.progress)
    do {_ = try await ledger.record(branch,id:"forked-child-start",kind:.task_transition,parent:branch.childAdmission,turn:turn,task:task,target:target);fatalError("child claim forked")}
    catch SessionLedgerError.invalidTransition { }
    let completed=SessionRunTransition(kind:.run_finished,codecVersion:1,groupAdmission:admissionID,childIndex:nil,childAdmission:nil,original:nil,code:nil,phase:.completed,recovered:false,progress:start.progress)
    do {_ = try await ledger.record(completed,id:"premature-group-success",kind:.task_transition,parent:start.latest.event_id,turn:turn,task:task,target:start.admission.target);fatalError("active child became whole group success")}
    catch SessionLedgerError.invalidTransition { }
    var stoppedProgress=start.progress;stoppedProgress.nextChild+=1;stoppedProgress.stoppedChildren+=1
    let unknownAsFailure=SessionRunTransition(kind:.run_child_stopped,codecVersion:1,groupAdmission:admissionID,childIndex:i,childAdmission:start.transition!.childAdmission,original:nil,code:"UNKNOWN_OUTCOME",phase:nil,recovered:false,progress:stoppedProgress)
    do {_ = try await ledger.record(unknownAsFailure,id:"unknown-as-definite-stop",kind:.task_transition,parent:start.latest.event_id,turn:turn,task:task,target:target);fatalError("unknown was downgraded to definite failure")}
    catch SessionLedgerError.invalidTransition { }
    let bootstrap=start.admission.run!.children[0],bootstrapTarget=try SessionRunSemantics.target(start.admission,index:0)
    let effect=try await storage.originalOperationEffect(bootstrapTarget)!
    let singleResult=SessionToolResult(codecVersion:1,admissionEvent:admissionID,sourceEventID:effect.eventID,sourceEffectHash:try effect.stableHash,rawReceiptHash:effect.rawReceiptHash,recovered:true,outcome:.completed)
    do {_ = try await ledger.record(singleResult,id:"single-receipt-for-group",kind:.tool_result,parent:admissionID,turn:turn,task:task,target:start.admission.target);fatalError("one original receipt became entire group success")}
    catch SessionLedgerError.invalidTransition { }
    _ = try await storage.createAgentSession("single-reclaim-test")
    let other=SessionLedgerPort(storage:storage,session:"single-reclaim-test",sourceInstance:"other-single-producer"),args=Data("{}".utf8)
    _ = try await other.record(SessionAssistantEnd(codecVersion:1,messageID:"other-proposal",textUTF8:Data(),calls:[.init(callID:"other-call",toolName:"run_cells",arguments:args)]),id:"other-proposal",kind:.assistant_end,turn:turn,task:task)
    let reuse=SessionToolAdmission(codecVersion:1,callID:"other-call",proposalEvent:"other-proposal",toolName:"run_cells",argumentsHash:KernelHashes.data(args),target:bootstrapTarget)
    do {_ = try await SessionToolPort(storage:storage,ledger:other).perform(admission:reuse,admissionID:"other-claim",startedID:"other-start",resultID:"other-result",turn:turn,task:task) {fatalError("single callback reclaimed a run child")};fatalError("run child claim reinterpreted as ordinary single operation")}
    catch SessionLedgerError.executionAlreadyClaimed { }
    precondition(bootstrap.operationID==bootstrapTarget.operationID)
    print("Rejected unpublished typed receipt, forked child start, premature group success, unknown-as-failure, single-receipt group success and cross-session child reclaim")
  }
  static func recover(storage:StorageService,ledger:SessionLedgerPort,root:URL,contract:URL,rustCLI:URL,initial:NativeSourceSnapshot)async throws {
    let counter=root.appendingPathComponent("executed.json"),before=(try? Data(contentsOf:counter)) ?? Data()
    let sourceBefore=try await storage.committedSource(initial.document_id)!,headBefore=try await storage.acceptedKernelHead(document:initial.document_id)
    let pageBefore=try await storage.sessionRecovery(session)
    let recovered=try await RecoveryService(storage:storage).recoverSession(session,ledger:ledger)
    let again=try await RecoveryService(storage:storage).recoverSession(session,ledger:ledger)
    let snapshot=try await storage.sessionRunSnapshot(session,admission:admissionID)
    precondition(recovered.tools.count==1 && again.tools.count==1 && recovered.requests.isEmpty)
    precondition(recovered.tools[0].phase==again.tools[0].phase)
    precondition(snapshot.summary.acceptedCells==recovered.tools[0].run!.acceptedCells)
    let headAfter=try await storage.acceptedKernelHead(document:initial.document_id),sourceAfter=try await storage.committedSource(initial.document_id)!
    precondition(headAfter?.checkpoint_id==headBefore?.checkpoint_id && sourceAfter.snapshot_hash==sourceBefore.snapshot_hash && sourceAfter.snapshot_hash==initial.snapshot_hash)
    precondition(((try? Data(contentsOf:counter)) ?? Data())==before)
    let replay=SessionRunPort(storage:storage,ledger:ledger)
    do {_ = try await replay.begin(snapshot.admission,id:admissionID,turn:turn,task:task);fatalError("recovered group restored execution authority")}
    catch SessionLedgerError.reconciliationRequired { }
    do {_ = try await replay.performChild(index:snapshot.progress.nextChild) {_ in fatalError("recovery executed a later cell")};fatalError("recovery callback was admitted")}
    catch SessionLedgerError.reconciliationRequired { }
    for child in snapshot.admission.run!.children {
      let stage=child.cellID ?? "bootstrap",folder=contract.appendingPathComponent(stage)
      if let receipt=try await storage.kernelReceipt(document:initial.document_id,operation:child.operationID) {
        try RecoveryValidation.encode(receipt).write(to:folder.appendingPathComponent("receipt.json"))
        try await rust(rustCLI,["verify",contract.path,stage,stage])
      }
    }
    let output:[String:Any]=["before":pageBefore.tools[0].phase.rawValue,"after":recovered.tools[0].phase.rawValue,
      "accepted_cells":snapshot.progress.acceptedCells,"error_cells":snapshot.progress.errorCells,"stopped_children":snapshot.progress.stoppedChildren,
      "next_child":snapshot.progress.nextChild,"active_original":snapshot.summary.originalActiveOperation ?? NSNull(),
      "receipt_chain":snapshot.progress.receiptChainHash,"unknown_observed":snapshot.progress.outcomeWasUnknown,
      "execution_callbacks":(try? JSONDecoder().decode([String].self,from:before)) ?? [],"source_unchanged":true,"kernel_head_unchanged":true]
    try JSONSerialization.data(withJSONObject:output,options:[.sortedKeys,.prettyPrinted]).write(to:root.appendingPathComponent("recovered.json"))
    print("Recovered original group only: "+String(decoding:try JSONSerialization.data(withJSONObject:output,options:[.sortedKeys]),as:UTF8.self))
  }
  static func main()async throws {
    precondition(om_fixture_install_sync_probe()==SQLITE_OK)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true),mode=CommandLine.arguments[2],rustCLI=URL(fileURLWithPath:CommandLine.arguments[3])
    let contract=root.appendingPathComponent("contract"),physical=root.appendingPathComponent("physical")
    let initial=try JSONDecoder().decode(NativeSourceSnapshot.self,from:Data(contentsOf:contract.appendingPathComponent("source.json")))
    let context=try JSONDecoder().decode(RunContext.self,from:Data(contentsOf:contract.appendingPathComponent("run-context.json")))
    let (storage,_)=try await StorageService.open(paths:.init(root:physical,channel:.preview)),info=try await storage.openDocument(initial.document_id)
    let ledger=SessionLedgerPort(storage:storage,session:session,sourceInstance:mode=="recover" ? "recovery-producer":"original-run-producer")
    if mode=="recover" {
      try await recover(storage:storage,ledger:ledger,root:root,contract:contract,rustCLI:rustCLI,initial:initial)
      try await storage.close();return
    }
    _ = try await storage.initializeSource(initial)
    try RecoveryValidation.encode(info).write(to:contract.appendingPathComponent("store-info.json"))
    _ = try await storage.createAgentSession(session)
    let order=mode=="stop-error" ? ["a","error","b"]:["a","b","error"]
    let (admit,args)=try admission(initial:initial,context:context,store:info.identity.storeID,order:order,continueOnError:mode != "stop-error")
    let ids=Dictionary(uniqueKeysWithValues:admit.run!.children.map {($0.cellID ?? "bootstrap",$0.operationID)})
    try RecoveryValidation.encode(ids).write(to:contract.appendingPathComponent("run-operation-ids.json"))
    let proposal=SessionAssistantEnd(codecVersion:1,messageID:"run-assistant",textUTF8:Data("Execute original selected cells".utf8),calls:[.init(callID:admit.callID,toolName:admit.toolName,arguments:args)])
    _ = try await ledger.record(proposal,id:admit.proposalEvent,kind:.assistant_end,turn:turn,task:task)
    let selected=CommandLine.arguments.count>4 ? CommandLine.arguments[4]:nil,point=CommandLine.arguments.count>5 ? CommandLine.arguments[5]:nil
    let port=SessionRunPort(storage:storage,ledger:ledger),counter=Counter(root.appendingPathComponent("executed.json"))
    _ = try await port.begin(admit,id:admissionID,turn:turn,task:task,faults:faults("group",selected,point))
    do {
      _ = try await SessionToolPort(storage:storage,ledger:ledger).perform(admission:admit,admissionID:"not-a-single-target",startedID:"not-single-start",resultID:"not-single-result",turn:turn,task:task) {fatalError("group used a single effect callback")}
      fatalError("group masqueraded as one kernel effect")
    } catch SessionLedgerError.invalidTransition { }
    var previous="bootstrap"
    let ackCounter=FaultCounter()
    for (index,child) in admit.run!.children.enumerated() {
      let stage=child.cellID ?? "bootstrap",previousStage=previous
      do {
        let childFaults=StorageFaults {current in
          try faults(stage,selected,point).reach(current)
          if stage=="a",["admission-ack-loss","started-ack-loss"].contains(mode),current=="session_event_committed.task_transition" {
            let lostAt=mode=="admission-ack-loss" ? 1:2
            if ackCounter.hit()==lostAt {throw StorageError.system(EIO,"lost original child ledger ACK")}
          }
        }
        _ = try await port.performChild(index:index,faults:childFaults) {actual in
          precondition(actual==child)
          let snapshot=try await storage.sessionRunSnapshot(session,admission:admissionID)
          precondition(snapshot.transition?.kind == .run_child_started && snapshot.activeChild==index)
          let oldReceipt=try await storage.kernelReceipt(document:initial.document_id,operation:child.operationID)
          precondition(oldReceipt==nil)
          try await counter.hit(stage)
          if mode=="cancel" && stage=="a" {return .stopped("CANCELLED")}
          if mode=="native-failure" && stage=="a" {return .stopped("DEPENDENCY_NOT_READY")}
          try await rust(rustCLI,["prepare",contract.path,stage,previousStage])
          let folder=contract.appendingPathComponent(stage),plan=try JSONDecoder().decode(NativeKernelCommit.self,from:Data(contentsOf:folder.appendingPathComponent("plan.json")))
          precondition(plan.operation_id==child.operationID)
          if mode=="normal" && stage=="a" {try await rejectUnpublishedResult(storage:storage,ledger:ledger,plan:plan)}
          let bytes=try Data(contentsOf:folder.appendingPathComponent("checkpoint.bin"))
          let publication=try await storage.publishBlob(data:bytes,expectedHash:plan.checkpoint_blob_hash)
          defer {Task {try? await storage.releaseBlob(publication)}}
          let physicalFaults=StorageFaults {current in
            try faults(stage,selected,point).reach(current)
            if mode=="lost-ack" && stage=="a" && current=="kernel_committed" {throw StorageError.system(EIO,"lost original child receipt ACK")}
          }
          // Physical-only fixture. Rust RunExecutor's actual live acceptance barriers have
          // their own tests; this script does not claim Agent/C ABI/Pi run dispatch integration.
          let receipt=try await storage.commitKernel(plan,publication:publication,controls:.init(ownerBarrier:{}),faults:physicalFaults)
          try RecoveryValidation.encode(receipt).write(to:folder.appendingPathComponent("receipt.json"))
          try await rust(rustCLI,["verify",contract.path,stage,stage])
          return .accepted
        }
      } catch {
        if ["admission-ack-loss","started-ack-loss"].contains(mode),stage=="a" {
          guard case StorageError.unknownCommit=error else {throw error}
          break
        }
        if mode=="lost-ack",stage=="a" {
          let unknown=try await port.finish(.unknown,code:"UNKNOWN_OUTCOME")
          precondition(unknown.summary.originalActiveOperation==child.operationID)
          break
        }
        throw error
      }
      previous=stage
      if mode=="normal",stage=="bootstrap" {
        let before=try await storage.sessionRunSnapshot(session,admission:admissionID)
        let history=SessionLedgerPort(storage:storage,session:session,sourceInstance:"active-history-reader")
        _ = try await RecoveryService(storage:storage).recoverSession(session,ledger:history)
        let after=try await storage.sessionRunSnapshot(session,admission:admissionID)
        precondition(after.latest.event_id==before.latest.event_id,"reading history terminated a still-live run between accepted children")
        print("Concurrent storage recovery left the actual live group unchanged between child boundaries")
      }
      if mode=="cancel" && stage=="a" {_ = try await port.finish(.cancelled,code:"CANCELLED");break}
      if mode=="native-failure" && stage=="a" {_ = try await port.finish(.failed,code:"DEPENDENCY_NOT_READY");break}
      if mode=="stop-error" && stage=="error" {
        do {_ = try await port.performChild(index:index+1) {_ in fatalError("error policy ignored")};fatalError("admitted after error despite original continue_on_error=false")}
        catch SessionLedgerError.invalidTransition { }
        _ = try await port.finish(.partial,code:"CELL_ERROR");break
      }
    }
    if mode=="normal" {_ = try await port.finish(.partial,code:"CELL_ERROR",faults:faults("group",selected,point))}
    try await recover(storage:storage,ledger:ledger,root:root,contract:contract,rustCLI:rustCLI,initial:initial)
    try await storage.close()
    print("Actual durable run group/child claims, original physical kernel receipts and storage-only recovery passed: "+mode)
  }
}
