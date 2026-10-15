import AppKit
import Foundation
import MetalKit
import SwiftUI

@main struct GeometryFixtures {
  @MainActor static func main() async throws {
    let app=NSApplication.shared;app.setActivationPolicy(.accessory)
    precondition(MemoryLayout<SceneGPUVertex>.stride==48 && MemoryLayout<SceneUniforms>.stride==144)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    let document="00000000-0000-0000-0000-000000000412"
    var snapshot=NativeSourceSnapshot(codec_version:1,document_id:document,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:.init(version:1,title:"图形验证",cells:[
      .init(id:"curve",kind:.math,source:"plot(sin(x),x:-pi..pi)",dialect:.modern),
      .init(id:"broken",kind:.math,source:"plot(1/x,x:-2..2)",dialect:.modern),
      .init(id:"sphere",kind:.math,source:"scene([sphere([0,0,0],1),style(sphere([1.5,0,0],0.7),color:\"orange\",opacity:0.5)],dimensions:3,mesh_points:8)",dialect:.modern)
    ]),cell_revisions:[],snapshot_hash:"")
    snapshot.cell_revisions=try snapshot.file.cells.map{.init(cell_id:$0.id,revision:try HostSerial(0))};snapshot.snapshot_hash=SourceHashes.snapshot(snapshot)
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));let info=try await storage.openDocument(document)
    _ = try await storage.initializeSource(snapshot)
    let host=try await NativeHostClient.open(eventCapacity:128)
    let open=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(open.owner_time_ms.value)
    let drafts=DraftStore(source:snapshot,runtime:host.runtimeInstanceID,generation:open.document_generation.value,clock:{clock.now()})
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock),kernel=KernelCommitPort(client:host,storage:storage,source:source)
    let client=ResultClient(host:host)
    func load(_ cell:String)async throws->NativeGeometryModel {
      let receipt=try await kernel.runCell(cell,operation:"geometry-"+cell)
      let manifest=try await client.manifest(receipt.result_id.value!)
      let binding=manifest.bindings.last!;try client.select(binding)
      let model=NativeGeometryModel(host:host,binding:binding);await model.load()
      precondition(model.error==nil,"real geometry decoding failed: \(model.error ?? "")")
      return model
    }
    let curve=try await load("curve")
    let raw=try await client.inspect(.inspect_result_geometry_page(.init(kind:.geometry_page,channel:.curve_points,object_index:0,segment_index:0,offset:0,limit:2)))!
    let body=try JSONSerialization.jsonObject(with:JSONEncoder().encode(raw.payload)) as! [String:Any]
    let actual=body["values"] as! [[Double]]
    precondition(curve.plot!.paths[0].points.prefix(2).enumerated().allSatisfy {i,p in p.x==actual[i][0] && p.y==actual[i][1]},"plot replaced original coordinates")
    let broken=try await load("broken")
    precondition(broken.plot!.paths.count>1,"discontinuous curve was joined")
    let scene=try await load("sphere"),data=scene.scene!
    let vertices=try await client.inspect(.inspect_result_geometry_page(.init(kind:.geometry_page,channel:.mesh_positions,object_index:0,segment_index:0,offset:0,limit:2)))!
    let rawVertices=try JSONSerialization.jsonObject(with:JSONEncoder().encode(vertices.payload)) as! [String:Any]
    let positions=rawVertices["values"] as! [[Double]]
    precondition(data.meshes[0].positions.prefix(2).enumerated().allSatisfy{i,p in p.array==positions[i]},"scene replaced original coordinates")
    let batches=try MetalSceneCanvas.prepare(data)
    precondition(batches.contains{$0.transparent} && batches.contains{!$0.transparent},"opaque/transparent paths were not both exercised")
    precondition(batches[0].indices==data.meshes[0].triangles.flatMap{$0},"renderer changed original triangle indices")
    let before=try await kernel.state()
    let window=NSWindow(contentRect:.init(x:100,y:100,width:850,height:740),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
    window.title="OpenMath · 实际图形验证"
    let canvas=MetalSceneCanvas(frame:.init(x:0,y:0,width:850,height:380),device:MTLCreateSystemDefaultDevice())
    window.contentView=canvas;canvas.configure(scene,data:data);window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    for _ in 0..<600 {if canvas.presentedFrames>0 || scene.renderError != nil {break};try await Task.sleep(for:.milliseconds(20))}
    precondition(scene.renderError==nil && canvas.presentedFrames>0,"actual Metal drawable was not presented: \(scene.renderError ?? "timeout")")
    let initial=scene.camera3D;scene.camera3D.rotate(0.3,0.1);canvas.configure(scene,data:data)
    let moved=canvas.presentedFrames
    for _ in 0..<100 {if canvas.presentedFrames>moved {break};try await Task.sleep(for:.milliseconds(20))}
    precondition(canvas.presentedFrames>moved && scene.camera3D != initial,"camera did not present a changed frame")
    scene.reset();canvas.configure(scene,data:data)
    try await Task.sleep(for:.milliseconds(200))
    let idle=canvas.submittedFrames;try await Task.sleep(for:.milliseconds(500))
    precondition(canvas.submittedFrames==idle,"stationary scene continuously submits frames")
    canvas.isHidden=true;scene.camera3D.rotate(0.1,0);canvas.configure(scene,data:data)
    try await Task.sleep(for:.milliseconds(100));precondition(canvas.submittedFrames==idle,"hidden scene submitted a frame")
    let after=try await kernel.state();precondition(before.kernel_state_revision.value==after.kernel_state_revision.value,"camera submitted computation")
    print("Actual curves \(curve.plot!.paths.reduce(0){$0+$1.points.count}) points; discontinuity \(broken.plot!.paths.count) segments; sphere \(data.meshes.reduce(0){$0+$1.triangles.count}) original triangles; \(canvas.presentedFrames) Metal presentations, idle/hidden no frames; kernel revision unchanged.")
    canvas.stopRendering();window.orderOut(nil)
    await curve.close();await broken.close();await scene.close();await client.close();await kernel.close();await source.close();try await host.close();try await storage.close()
  }
}
