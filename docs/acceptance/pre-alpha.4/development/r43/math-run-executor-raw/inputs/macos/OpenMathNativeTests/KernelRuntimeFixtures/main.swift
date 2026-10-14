import Foundation
import SQLite3
import AppKit

private final class Pause:@unchecked Sendable {
  private let lock=NSCondition()
  private var arrived=false,released=false
  func reach() {lock.lock();arrived=true;lock.broadcast();while !released {lock.wait()};lock.unlock()}
  func release() {lock.lock();released=true;lock.broadcast();lock.unlock()}
  private func hasArrived()->Bool {lock.lock();defer {lock.unlock()};return arrived}
  func wait() async throws {
    for _ in 0..<2000 {
      let done=hasArrived()
      if done {return};try await Task.sleep(for:.milliseconds(5))
    }
    fatalError("actual IO pause not reached")
  }
}
private func originalOutput(_ storage:StorageService,_ receipt:NativeKernelReceipt,cell:String) async throws->String? {
  let reader=try await storage.openReferencedBlob(owner:.init(kind:.checkpoint,id:receipt.checkpoint_id),hash:receipt.checkpoint_blob_hash,document:receipt.document_id)
  var bytes=Data(),offset:UInt64=0
  while offset<receipt.checkpoint_byte_length.value {
    let chunk=try await storage.readBlob(reader,offset:offset,count:min(65536,Int(receipt.checkpoint_byte_length.value-offset)));bytes.append(chunk);offset+=UInt64(chunk.count)
  }
  try await storage.closeBlob(reader)
  precondition(bytes.prefix(5)==Data([79,77,75,83,1]))
  let size=bytes[5..<9].enumerated().reduce(UInt32(0)) {$0 | (UInt32($1.element) << UInt32($1.offset*8))}
  let meta=try JSONSerialization.jsonObject(with:bytes.subdata(in:21..<21+Int(size))) as! [String:Any]
  let cells=meta["cells"] as! [[String:Any]]
  let actual=cells.first{($0["input"] as? [String:Any])?["id"] as? String==cell}!
  let output=actual["output"] as? [String:Any],items=output?["items"] as? [[String:Any]]
  return items?.last?["input_form"] as? String
}
@main struct KernelRuntimeFixtures {
  private static func mark(_ stage:String) {FileHandle.standardOutput.write(Data((stage+"\n").utf8))}
  static func main() async throws {
    precondition(om_fixture_install_sync_probe()==SQLITE_OK)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    var initial=try JSONDecoder().decode(NativeSourceSnapshot.self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[2])))
    initial.file.cells += [.init(id:"long",kind:.math,source:"let ephemeral=8; map(fn(k)=>sin(k),range(1,1000000))",dialect:.modern),.init(id:"probe",kind:.math,source:"ephemeral",dialect:.modern)]
    initial.cell_revisions += [.init(cell_id:"long",revision:try HostSerial(0)),.init(cell_id:"probe",revision:try HostSerial(0))]
    initial.snapshot_hash=SourceHashes.snapshot(initial)
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));let info=try await storage.openDocument(initial.document_id)
    mark("storage-open")
    _ = try await storage.initializeSource(initial)
    let host=try await NativeHostClient.open()
    mark("host-open")
    let open=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:initial,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    mark("source-open")
    let clock=SourceOwnerClock();clock.calibrate(open.owner_time_ms.value)
    let drafts=try await MainActor.run {
      let drafts=DraftStore(source:initial,runtime:host.runtimeInstanceID,generation:open.document_generation.value,clock:{clock.now()})
      try drafts.attachEditor(cell:"a",generation:1);return drafts
    }
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock)
    let kernel=KernelCommitPort(client:host,storage:storage,source:source)
    let bootState=try await kernel.state();mark("kernel-state:"+bootState.phase.rawValue)
    let a=try await kernel.runCell("a",operation:"run-a2")
    mark("a-accepted")
    let aValue=try await originalOutput(storage,a,cell:"a");precondition(aValue=="2" && a.kernel_state_revision.value==1)
    let b=try await kernel.runCell("b",operation:"run-b3")
    let bValue=try await originalOutput(storage,b,cell:"b");precondition(bValue=="3" && b.kernel_state_revision.value==2)
    let duplicate=try await host.kernelCommand(.kernel_completed(.init(type:.kernel_completed,operation_id:"run-a2",receipt:a)))
    precondition(duplicate.kernel_state_revision.value==2 && duplicate.receipt.value?.kernel_state_revision.value==1)
    let events=try await host.nextEvents();precondition(events.events.filter{$0.event_kind == .result_accepted}.count==2)
    try await MainActor.run {try drafts.edited(cell:"a",generation:1,source:"let a=5",hasMarkedText:false)}
    do {_ = try await kernel.runCell("b",operation:"missing-prerequisite");fatalError("old a2 treated as current a5")}
    catch KernelPortError.failed(let code) {precondition(code=="DEPENDENCY_NOT_READY")}
    let stale=try await kernel.state();precondition(!stale.definitions_current && stale.kernel_state_revision.value==2)
    let five=try await kernel.runCell("a",operation:"run-a5"),six=try await kernel.runCell("b",operation:"run-b6")
    let fiveValue=try await originalOutput(storage,five,cell:"a"),sixValue=try await originalOutput(storage,six,cell:"b")
    precondition(fiveValue=="5" && sixValue=="6")
    let pause=Pause(),operation="stop-before-actual-commit"
    let work=Task {try await kernel.runCell("b",operation:operation,faults:.init {point in if point=="before_kernel_commit" {pause.reach()} })}
    try await pause.wait()
    let during=try await kernel.state();precondition(during.pending_document_write && during.kernel_state_revision.value==4)
    try await MainActor.run {try drafts.edited(cell:"a",generation:1,source:"let a=9",hasMarkedText:false)}
    do {_ = try await source.synchronizeDrafts();fatalError("source overwrote pending kernel acceptance")}
    catch NativeHostClientError.rejected { }
    let overlay=drafts.overlay("a")?.source;precondition(overlay=="let a=9")
    await kernel.cancel(operation);pause.release()
    do {_ = try await work.value;fatalError("early stop accepted main state")} catch { }
    let absent=try await storage.kernelReceipt(document:initial.document_id,operation:operation)
    let after=try await kernel.state();precondition(absent==nil && after.kernel_state_revision.value==4 && !after.pending_document_write)
    let nine=try await kernel.runCell("a",operation:"run-a9")
    let nineValue=try await originalOutput(storage,nine,cell:"a");precondition(nineValue=="9")
    let beforeTitle=try await host.sourceCommand(.source_read(.init(type:.source_read)))
    _ = try await host.kernelCommand(.kernel_run_cell(.init(type:.kernel_run_cell,operation_id:"title-after-production",cell_id:"b",expected_source_hash:beforeTitle.snapshot.value!.snapshot_hash)))
    let preview=try await source.preview(.preview_patch_input(.init(kind:.patch,operations:[.preview_rename_notebook(.init(type:.rename_notebook,title:"只改标题，保留计算来源"))])))
    _ = try await source.apply(preview.preview_ref.value!)
    let titled=try await kernel.finishExecution("title-after-production")
    precondition(titled.producer.source_revision.value < titled.accepted_source_revision.value && titled.producer.execution_epoch.value==nine.producer.execution_epoch.value)
    let late=Pause(),lateID="stop-after-actual-commit"
    let lateWork=Task {try await kernel.runCell("b",operation:lateID,faults:.init {point in if point=="kernel_committed" {late.reach()} })}
    try await late.wait();await kernel.cancel(lateID);late.release()
    let lateResult=try await lateWork.value
    let tenValue=try await originalOutput(storage,lateResult,cell:"b");precondition(tenValue=="10" && lateResult.kernel_state_revision.value==7)
    let lost=StorageFaults {point in if point=="kernel_committed" {throw StorageError.system(EIO,"kernel-runtime-lost-ack")} }
    let recovered=try await kernel.runCell("b",operation:"lost-main-ack",faults:lost)
    precondition(recovered.kernel_state_revision.value==8)
    let original=try await kernel.reconcile("lost-main-ack");precondition(original?.checkpoint_id==recovered.checkpoint_id)
    let prefix=await kernel.runCells(["a","error","b"],operation:"actual-prefix")
    precondition(prefix.receipts.count==2 && prefix.receipts.last?.producer.terminal_status == .error && prefix.stopped=="FAILED")
    let prefixState=try await kernel.state();precondition(prefixState.kernel_state_revision.value==10)
    let physicalHead=try await storage.acceptedKernelHead(document:initial.document_id)!;precondition(physicalHead.kernel_state_revision.value==10)
    let syncID="unstable-original-kernel"
    let syncFailure=StorageFaults {point in if point=="before_kernel_commit" {om_fixture_fail_fullsync(1)} }
    do {_ = try await kernel.runCell("b",operation:syncID,faults:syncFailure);fatalError("unstable COMMIT advanced main state")}
    catch KernelPortError.unknownOutcome { }
    let unknown=try await kernel.state();precondition(unknown.kernel_state_revision.value==10 && unknown.pending_document_write)
    do {_ = try await kernel.runCell("a",operation:"must-wait-for-original");fatalError("new work bypassed unknown original ID")}
    catch KernelPortError.unknownOutcome { }
    om_fixture_fail_fullsync(0)
    let confirmed=try await kernel.reconcile(syncID);precondition(confirmed?.kernel_state_revision.value==11)
    let committed=try await host.sourceCommand(.source_read(.init(type:.source_read)))
    _ = try await host.kernelCommand(.kernel_run_cell(.init(type:.kernel_run_cell,operation_id:"stale-long",cell_id:"long",expected_source_hash:committed.snapshot.value!.snapshot_hash)))
    var running=false
    for _ in 0..<1000 {
      let state=try await kernel.status("stale-long")
      if state.phase == .running {running=true;break}
      if state.phase == .failed || state.phase == .candidate {break}
      try await Task.sleep(for:.milliseconds(1))
    }
    precondition(running,"actual main CAS did not reach running")
    let readStart=ContinuousClock.now
    _ = try await host.sourceCommand(.source_read(.init(type:.source_read)))
    try await MainActor.run {
      try drafts.attachEditor(cell:"long",generation:1)
      try drafts.edited(cell:"long",generation:1,source:"2+2",hasMarkedText:false)
    }
    _ = try await source.synchronizeDrafts()
    let delta=readStart.duration(to:.now)
    precondition(delta < .seconds(1),"main CAS blocked source read/edit")
    _ = try await host.cancel(operation:"stale-long")
    for _ in 0..<2000 {
      let state=try await kernel.status("stale-long")
      if state.phase == .failed || state.phase == .discarded {break}
      if state.phase == .candidate {_ = try await host.kernelCommand(.kernel_discard(.init(type:.kernel_discard,operation_id:"stale-long")));break}
      try await Task.sleep(for:.milliseconds(5))
    }
    let untouched=try await kernel.state();precondition(untouched.kernel_state_revision.value==11 && !untouched.definitions_current)
    let four=try await kernel.runCell("long",operation:"fresh-long"),probe=try await kernel.runCell("probe",operation:"actual-no-leak")
    let fourValue=try await originalOutput(storage,four,cell:"long"),probeValue=try await originalOutput(storage,probe,cell:"probe")
    precondition(fourValue=="4" && probeValue=="ephemeral")
    try await MainActor.run {try drafts.edited(cell:"long",generation:1,source:"map(fn(k)=>sin(k),range(1,1000000))",hasMarkedText:false)}
    let closingWork=Task {try await kernel.runCell("long",operation:"close-running-main")}
    var closingRun=false
    for _ in 0..<1000 {
      if let status=try? await kernel.status("close-running-main"),status.phase == .running {closingRun=true;break}
      try await Task.sleep(for:.milliseconds(1))
    }
    precondition(closingRun,"actual main close did not reach running")
    await kernel.close()
    do {_ = try await closingWork.value;fatalError("closing work accepted abandoned main state")} catch { }
    await kernel.close();await source.close();try await host.close();try await storage.close()
    print("Actual C ABI main kernel + shared source gate + Swift/SQLite: a2->3/a5->6/a9->10; actual title change keeps original producer; stale dependency refusal; original-ID event once; typing overlay while IO; early stop rollback, late stop durable prefix, lost ACK/error prefix, actual fullsync unknown blocks new work until original readback, long CAS source edit/read and abandoned definition no-leak passed")
  }
}
