import AppKit
import Foundation
import CryptoKit

private actor DeliveryPause {
  private var arrived=false,released=false
  private var waiter:CheckedContinuation<Void,Never>?
  func pause() async {
    arrived=true
    if !released {await withCheckedContinuation {waiter=$0}}
  }
  func wait() async throws {
    for _ in 0..<2000 {if arrived {return};try await Task.sleep(for:.milliseconds(5))}
    fatalError("real decoded reply never reached delivery")
  }
  func release() {released=true;waiter?.resume();waiter=nil}
}
private func object(_ value:HostJSONValue)throws->[String:Any] {
  try JSONSerialization.jsonObject(with:JSONEncoder().encode(value)) as! [String:Any]
}
private func binding(_ manifest:NativeResultManifest)->ResultBinding {manifest.bindings.last!}
@main struct ResultFixtures {
  static func main() async throws {
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true),document="00000000-0000-0000-0000-000000000410"
    let text=String(repeating:"中文🙂 e\u{301} / ",count:4000)
    let textSource=String(data:try JSONEncoder().encode(text),encoding:.utf8)!
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"实际结果读取",cells:[
      .init(id:"a",kind:.math,source:"let a=2; seed_random(42); a",dialect:.modern),
      .init(id:"list",kind:.math,source:"[1/3,decimal(\"0.1\",precision:80),"+textSource+"]",dialect:.modern),
      .init(id:"solve",kind:.math,source:"solve(x^2-5*x+6==0,x)",dialect:.modern),
      .init(id:"plot",kind:.math,source:"plot(sin(x),x:-pi..pi)",dialect:.modern),
      .init(id:"sphere",kind:.math,source:"scene([sphere([0,0,0],1)],dimensions:3,mesh_points:8)",dialect:.modern),
      .init(id:"error",kind:.math,source:"let broken=",dialect:.modern)
    ]),cell_revisions:[],snapshot_hash:"")
    snapshot.cell_revisions=try snapshot.file.cells.map{.init(cell_id:$0.id,revision:try HostSerial(0))};snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let initial=snapshot
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));let info=try await storage.openDocument(document)
    _ = try await storage.initializeSource(initial)
    let host=try await NativeHostClient.open(eventCapacity:128)
    let open=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:initial,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(open.owner_time_ms.value)
    let drafts=try await MainActor.run {let drafts=DraftStore(source:initial,runtime:host.runtimeInstanceID,generation:open.document_generation.value,clock:{clock.now()});try drafts.attachEditor(cell:"a",generation:1);return drafts}
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
    let client=await MainActor.run {ResultClient(host:host)}
    let a=try await kernel.runCell("a",operation:"a-result"),list=try await kernel.runCell("list",operation:"list-result")
    let manifest=try await client.manifest(list.result_id.value!)
    do {
      _ = try await host.resultCommand(.result_manifest(.init(type:.result_manifest,request_id:"list-result",root_result_id:list.result_id.value!,offset:0,limit:1)))
      fatalError("result request shadowed main cancellation identity")
    } catch NativeHostClientError.rejected { }
    let originalBinding=binding(manifest)
    try client.select(originalBinding)
    let page=try await client.inspect(.inspect_result_page(.init(kind:.page,path:[],offset:0,limit:1,column_offset:0,column_limit:1)))!
    let pageBody=try object(page.payload),rows=pageBody["rows"] as! [[String:Any]],cells=rows[0]["cells"] as! [[String:Any]]
    precondition(cells[0]["nature"] as? String=="exact" && (cells[0]["source"] as! [String:Any])["input_form"] as? String=="1/3")
    let precision=try await client.inspect(.inspect_result_page(.init(kind:.page,path:[],offset:1,limit:1,column_offset:0,column_limit:1)))!
    let precisionBody=try object(precision.payload);let precise=((precisionBody["rows"] as! [[String:Any]])[0]["cells"] as! [[String:Any]])[0]
    precondition(precise["nature"] as? String=="high_precision")
    let complete=try await client.completeSource(format:.input_form,path:[2])!
    let decoded=try JSONDecoder().decode(String.self,from:Data(complete.utf8));precondition(decoded.utf8.elementsEqual(text.utf8))
    let mathSource=try await client.completeSource(format:.cell_source)!
    precondition(mathSource.utf8.elementsEqual(initial.file.cells[1].source.utf8))
    let numeric=try await client.inspect(.inspect_result_numeric(.init(kind:.numeric,path:[0],digits:50)))!
    let numericBody=try object(numeric.payload);precondition(numericBody["input_form"] as! String != "1/3")
    do {_ = try await client.inspect(.inspect_result_expression(.init(kind:.readonly_expression,source:"Set[a,99]",numeric:false)));fatalError("readonly Set accepted")}
    catch ResultClientError.failed { }
    let scratch=try await client.inspect(.inspect_result_scratch(.init(kind:.scratch,source:"let a=99; random_uniform(); a",dialect:.modern)))!
    let scratchBody=try object(scratch.payload);precondition(scratchBody["effect_committed"] as? Bool==false)
    let original=try await client.inspect(.inspect_result_expression(.init(kind:.readonly_expression,source:"a",numeric:false)))!
    let originalBody=try object(original.payload);precondition(originalBody["input_form"] as? String=="2")
    let out=try await client.inspect(.inspect_result_expression(.init(kind:.readonly_expression,source:"Out[1]",numeric:false)))!
    let outBody=try object(out.payload);precondition(outBody["input_form"] as? String=="2")
    let random1=try await client.inspect(.inspect_result_expression(.init(kind:.readonly_expression,source:"RandomUniform[]",numeric:false)))!
    let random2=try await client.inspect(.inspect_result_expression(.init(kind:.readonly_expression,source:"RandomUniform[]",numeric:false)))!
    let firstRandom=try object(random1.payload),secondRandom=try object(random2.payload);precondition(firstRandom["input_form"] as? String==secondRandom["input_form"] as? String)
    let before=try await kernel.state();precondition(before.kernel_state_revision.value==2)
    let solve=try await kernel.runCell("solve",operation:"solve-result");let solved=try await client.manifest(solve.result_id.value!)
    try client.select(binding(solved))
    let presentation=try await client.inspect(.inspect_result_presentation(.init(kind:.presentation,offset:0,limit:1)))!
    let presented=try object(presentation.payload)["presentation"] as! [String:Any]
    let solutions=presented["solutions"] as! [[String:Any]]
    precondition(presented["kind"] as? String=="solutions" && presented["total_solutions"] as? Int==2 && solutions.count==1 && solutions[0]["verified"] as? String=="Exact")
    let steps=try await client.inspect(.inspect_result_steps(.init(kind:.steps,parent_path:[],offset:0,limit:1)))!
    let stepBody=try object(steps.payload),entries=stepBody["entries"] as! [[String:Any]]
    precondition(stepBody["recorded"] as? Bool==true && entries.count==1 && (entries[0]["step"] as! [String:Any])["id"] as? String=="S1")
    let plot=try await kernel.runCell("plot",operation:"plot-result");try client.select(binding(try await client.manifest(plot.result_id.value!)))
    let geometry=try await client.inspect(.inspect_result_geometry(.init(kind:.geometry)))!;let geometryBody=try object(geometry.payload);precondition(geometryBody["available"] as? Bool==true)
    let samples=try await client.inspect(.inspect_result_geometry_page(.init(kind:.geometry_page,channel:.curve_points,object_index:0,segment_index:0,offset:0,limit:2)))!
    let sampleBody=try object(samples.payload);precondition(sampleBody["values"] as! [[Double]] != [])
    let sphere=try await kernel.runCell("sphere",operation:"sphere-result");try client.select(binding(try await client.manifest(sphere.result_id.value!)))
    let vertices=try await client.inspect(.inspect_result_geometry_page(.init(kind:.geometry_page,channel:.mesh_positions,object_index:0,segment_index:0,offset:0,limit:2)))!
    let vertexBody=try object(vertices.payload);precondition(vertexBody["values"] as! [[Double]] != [])
    try await MainActor.run {try drafts.edited(cell:"a",generation:1,source:"let a=5; a",hasMarkedText:false)}
    _ = try await kernel.runCell("a",operation:"changed-a-result")
    try client.select(originalBinding)
    let old=try await client.inspect(.inspect_result_expression(.init(kind:.readonly_expression,source:"a",numeric:false)))!
    let oldBody=try object(old.payload);precondition(old.binding.value?.acceptance == .history_only && old.binding.value?.freshness == .stale && oldBody["input_form"] as? String=="2")
    let pause=DeliveryPause()
    let late=await MainActor.run {ResultClient(host:host,probes:.init(beforeDelivery:{reply in if reply.binding.value != nil {await pause.pause()}}))}
    let lateManifest=try await late.manifest(list.result_id.value!);try late.select(binding(lateManifest))
    let delayed=Task {try await late.inspect(.inspect_result_summary(.init(kind:.summary)))}
    try await pause.wait();try late.select(binding(solved));await pause.release()
    let lateReply=try await delayed.value;precondition(lateReply==nil)
    let pauseForMain=DeliveryPause()
    let outdated=await MainActor.run {ResultClient(host:host,probes:.init(beforeDelivery:{reply in if reply.binding.value != nil {await pauseForMain.pause()}}))}
    let outdatedManifest=try await outdated.manifest(list.result_id.value!);try outdated.select(binding(outdatedManifest))
    let oldScopeReply=Task {try await outdated.inspect(.inspect_result_summary(.init(kind:.summary)))}
    try await pauseForMain.wait()
    _ = try await kernel.runCell("a",operation:"advanced-main-during-inspection")
    await pauseForMain.release();let outdatedReply=try await oldScopeReply.value;precondition(outdatedReply==nil)
    let final=try await kernel.state();precondition(final.kernel_state_revision.value==7 && a.kernel_state_revision.value==1)
    let error=try await kernel.runCell("error",operation:"actual-parse-error-result")
    let errorManifest=try await client.manifest(error.result_id.value!);try client.select(binding(errorManifest))
    precondition(binding(errorManifest).out_index.value==0 && binding(errorManifest).view_id.value==nil && binding(errorManifest).freshness == .partial)
    let failure=try await client.inspect(.inspect_result_presentation(.init(kind:.presentation,offset:0,limit:1)))!
    let failureBody=try object(failure.payload);precondition(failureBody["kind"] as? String=="error" && !(failureBody["items"] as! [[String:Any]]).isEmpty)
    let invalidSource=try await client.completeSource(format:.cell_source)!;precondition(invalidSource=="let broken=")
    let batch=try await host.nextEvents();let encodedEvents=try JSONEncoder().encode(batch)
    precondition(encodedEvents.count<32*1024 && !String(decoding:encodedEvents,as:UTF8.self).contains(text.prefix(200)))
    try await client.revoke(originalBinding.result_ref)
    try client.select(originalBinding)
    do {_ = try await client.inspect(.inspect_result_summary(.init(kind:.summary)));fatalError("revoked result ref silently revived")}
    catch ResultClientError.failed { }
    await outdated.close();await late.close();await client.close();await kernel.close();await source.close();try await host.close();try await storage.close()
    print("Actual C ABI immutable result worker + Swift ResultClient: exact/80-digit pages, Unicode full source/hash fragments, original steps/2D/3D samples, readonly Set/random/Out and scratch isolation, stale history, real late reply discarded, summary-only events and revocation passed")
  }
}
