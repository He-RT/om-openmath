import AppKit
import SwiftUI
import Foundation
import CryptoKit
import Darwin
import QuartzCore
import MetalKit
// @production-module-import

private struct CorpusRow:Decodable {let id:String,input:String}
private enum MeasurementError:Error {case failed(String)}
private final class BusyCommitOnce:@unchecked Sendable {
  private let lock=NSLock()
  private var count=0
  var attempts:Int {lock.withLock {count}}
  func reach(_ point:String)throws {
    guard point=="before_kernel_commit" else {return}
    let first=lock.withLock {count+=1;return count==1}
    if first {throw NativeHostClientError.rejected(.init(code:.editing_busy,message:"synthetic pre-barrier contention",field_path:.init(nil),operation_ref:.init(nil),outcome_known:true,retryable:true))}
  }
}
@MainActor private final class NativeFrameClock:NSObject {
  private var link:CADisplayLink?
  private var remaining=0
  private var started:UInt64=0
  private(set) var measured:Double?
  init(view:NSView) {
    super.init();let link=view.displayLink(target:self,selector:#selector(frame(_:)))
    link.preferredFrameRateRange=CAFrameRateRange(minimum:60,maximum:60,preferred:60)
    link.add(to:.main,forMode:.common);self.link=link
  }
  func arm() {measured=nil;remaining=2;started=DispatchTime.now().uptimeNanoseconds}
  @objc private func frame(_ link:CADisplayLink) {
    guard remaining>0 else {return};remaining-=1
    if remaining==0 {measured=Double(DispatchTime.now().uptimeNanoseconds-started)/1_000_000}
  }
  func stop() {link?.invalidate();link=nil}
}
@MainActor private final class SceneMotionProbe:NSObject {
  private var link:CADisplayLink?
  private var started:CFTimeInterval?
  private let update:()->Void
  private(set) var done=false,elapsed:Double=0
  private(set) var timestamps:[Double]=[]
  init(view:NSView,update:@escaping ()->Void) {
    self.update=update;super.init()
    let link=view.displayLink(target:self,selector:#selector(frame(_:)))
    link.preferredFrameRateRange=CAFrameRateRange(minimum:60,maximum:60,preferred:60)
    link.add(to:.main,forMode:.common);self.link=link
  }
  @objc private func frame(_ link:CADisplayLink) {
    let now=CACurrentMediaTime();if started==nil {started=now};elapsed=now-started!
    if elapsed>=3 {self.link?.invalidate();self.link=nil;done=true;return}
    timestamps.append(elapsed);update()
  }
}
@main struct PerformanceFixtures {
  @MainActor static func main() async throws {
    let launched=DispatchTime.now().uptimeNanoseconds
    let app=NSApplication.shared;app.setActivationPolicy(.accessory)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
    let corpusBytes=try Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[2]))
    let corpus=try JSONDecoder().decode([CorpusRow].self,from:corpusBytes)
    precondition(corpus.count==53)
    func milliseconds(_ start:UInt64)->Double {Double(DispatchTime.now().uptimeNanoseconds-start)/1_000_000}
    func write(_ value:Any,_ name:String)throws {try JSONSerialization.data(withJSONObject:value,options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent(name),options:.atomic)}
    func opened(_ name:String,source:String,dialect:Dialect = .wolfram)async throws->(NativeAppStorage,NativeFileWorkspace,NativeDocumentSession,NSWindow) {
      let folder=root.appendingPathComponent(name,isDirectory:true);try FileManager.default.createDirectory(at:folder,withIntermediateDirectories:true)
      let file=folder.appendingPathComponent("measurement.omnb")
      let content=NativeSourceFile(version:1,title:"性能测量 · "+name,cells:[.init(id:"measure",kind:.math,source:source,dialect:dialect)])
      try JSONEncoder().encode(content).write(to:file)
      let services=NativeAppStorage(paths:.init(root:folder.appendingPathComponent("storage"),channel:.preview))
      let workspace=NativeFileWorkspace(storage:services)
      guard await workspace.open(file),let session=workspace.session else {throw MeasurementError.failed(workspace.errorText ?? "open failed")}
      let window=NSWindow(contentRect:.init(x:100,y:100,width:1100,height:740),styleMask:[.titled,.resizable],backing:.buffered,defer:false)
      window.title="OpenMath · 性能测量"
      window.contentView=NSHostingView(rootView:WorkspaceView(workspace:workspace,session:session));workspace.attachWindow(window)
      let adapter=try session.editor("measure")
      window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
      let deadline=ContinuousClock.now.advanced(by:.seconds(10))
      while adapter.textView.window==nil || session.runtime.projection==nil {
        guard ContinuousClock.now<deadline else {throw MeasurementError.failed("workspace did not mount")}
        try await Task.sleep(for:.milliseconds(1))
      }
      let clock=NativeFrameClock(view:adapter.textView);clock.arm()
      while clock.measured==nil {guard ContinuousClock.now<deadline else {clock.stop();throw MeasurementError.failed("visible display link did not fire")};try await Task.sleep(for:.milliseconds(1))}
      clock.stop()
      return (services,workspace,session,window)
    }
    if CommandLine.arguments.contains("--busy-commit") {
      let (services,workspace,session,window)=try await opened("busy-commit",source:"2+2")
      let fault=BusyCommitOnce(),operation="same-original-operation"
      let receipt=try await session.runtime.kernel.runCell("measure",operation:operation,faults:.init {try fault.reach($0)})
      let client=ResultClient(host:session.host),manifest=try await client.manifest(receipt.result_id.value!)
      let presenter=ResultPresenter(host:session.host,binding:manifest.bindings.last!)
      await presenter.readOriginalValue()
      let state=try await session.runtime.kernel.state()
      precondition(fault.attempts==3 && receipt.operation_id==operation && receipt.kernel_state_revision.value==1 && state.kernel_state_revision.value==1 && presenter.originalValue?.input_form=="4","pre-barrier contention changed the operation or evaluated twice")
      try write(["injection":"one synthetic EDITING_BUSY before the actual owner barrier; real SQL rollback and original candidate/blob commit","physical_attempts":fault.attempts,"operation":receipt.operation_id,"kernel_revision":receipt.kernel_state_revision.value,"input_form":presenter.originalValue!.input_form],"busy-commit.json")
      await presenter.close();await client.close();window.orderOut(nil);window.contentView=nil
      guard await workspace.shutdown() else {throw MeasurementError.failed(workspace.errorText ?? "close failed")};try await services.close()
      print("One pre-barrier contention: same original operation, exactly one accepted4, kernel revision1.");return
    }
    if CommandLine.arguments.contains("--scene") {
      let source="scene([sphere([0,0,0],1),style(sphere([1.5,0,0],0.7),color:\"orange\",opacity:0.5)],dimensions:3,mesh_points:32)"
      let (services,workspace,session,window)=try await opened("camera",source:source,dialect:.modern)
      session.runtime.run("measure")
      let deadline=ContinuousClock.now.advanced(by:.seconds(30))
      while session.runtime.operation != nil {guard ContinuousClock.now<deadline else {throw MeasurementError.failed("scene run timed out")};try await Task.sleep(for:.milliseconds(1))}
      guard let binding=session.runtime.results["measure"]?.bindings.last else {throw MeasurementError.failed(session.runtime.message)}
      let model=session.runtime.presenter(for:binding).geometryModel()
      func findCanvas(_ view:NSView)->MetalSceneCanvas? {
        if let canvas=view as? MetalSceneCanvas {return canvas}
        for child in view.subviews {if let canvas=findCanvas(child) {return canvas}}
        return nil
      }
      var mounted:MetalSceneCanvas?
      while (mounted?.presentedFrames ?? 0) == 0 {
        guard ContinuousClock.now<deadline,model.renderError==nil else {throw MeasurementError.failed(model.renderError ?? "scene canvas did not present")}
        mounted=window.contentView.flatMap(findCanvas);try await Task.sleep(for:.milliseconds(1))
      }
      let canvas=mounted!,before=canvas.presentedFrames
      let probe=SceneMotionProbe(view:canvas) {model.camera3D.rotate(0.012,0.001)}
      while !probe.done {guard ContinuousClock.now<deadline else {throw MeasurementError.failed("scene display link stopped")};try await Task.sleep(for:.milliseconds(1))}
      try await Task.sleep(for:.milliseconds(120))
      let frames=canvas.presentedFrames-before
      let idle=canvas.submittedFrames;try await Task.sleep(for:.milliseconds(250))
      let idleFrames=canvas.submittedFrames-idle
      let intervals=zip(probe.timestamps,probe.timestamps.dropFirst()).map{($0.1-$0.0)*1000}.sorted()
      let fps=Double(frames)/probe.elapsed
      try write(["source":source,"original_triangles":model.scene!.meshes.reduce(0){$0+$1.triangles.count},"device":canvas.device?.name ?? "unavailable","drawable_width":canvas.drawableSize.width,"drawable_height":canvas.drawableSize.height,"camera_updates":probe.timestamps.count,"presented_frames":frames,"motion_seconds":probe.elapsed,"presented_fps":fps,"update_interval_p95_ms":intervals[Int(ceil(Double(intervals.count)*0.95))-1],"idle_additional_submissions":idleFrames,"definition":"60Hz display-link camera updates through the production SwiftUI model; actual CAMetalDrawable presented callbacks, with a final120ms drain"],"mac-scene.json")
      print("Native camera \(fps) presented fps, \(frames) frames, \(probe.timestamps.count) updates; idle additional submissions \(idleFrames)")
      guard frames>0,idleFrames==0,model.renderError==nil else {throw MeasurementError.failed("scene rendering failed or continued while idle")}
      window.orderOut(nil);window.contentView=nil
      guard await workspace.shutdown() else {throw MeasurementError.failed(workspace.errorText ?? "close failed")};try await services.close();return
    }
    var rows:[[String:Any]]=[],failures:[String]=[]
    var firstFrame:Double?
    let diagnostic=CommandLine.arguments.count>4 && CommandLine.arguments[3]=="--case" ? CommandLine.arguments[4]:nil
    for row in corpus where diagnostic==nil || row.id==diagnostic {
      let (services,workspace,session,window)=try await opened("row-"+row.id,source:row.input)
      if firstFrame==nil {firstFrame=milliseconds(launched)}
      let start=DispatchTime.now().uptimeNanoseconds
      var stages:[String:Double]=[:]
      session.runtime.runObserved={stage in stages[stage]=milliseconds(start)}
      session.runtime.run("measure")
      let deadline=ContinuousClock.now.advanced(by:.seconds(20))
      while session.runtime.operation != nil {
        guard ContinuousClock.now<deadline else {throw MeasurementError.failed("row "+row.id+" timed out")}
        try await Task.sleep(for:.milliseconds(1))
      }
      let duration=milliseconds(start)
      guard let actual=session.runtime.results["measure"],actual.receipt.producer.terminal_status == .done,let value=actual.values.last else {throw MeasurementError.failed("row "+row.id+": "+session.runtime.message)}
      rows.append(["id":row.id,"input":row.input,"input_form":value.source,"timing_ms":duration,"stages_ms":stages,"operation":actual.receipt.operation_id,"kernel_revision":actual.receipt.producer.kernel_state_revision.value])
      try write(rows,"mac-corpus.json")
      print("Mac #\(row.id): \(String(format:"%.3f",duration)) ms")
      if duration>=1000 {failures.append("Mac #\(row.id): \(duration)ms")}
      window.orderOut(nil);window.contentView=nil
      guard await workspace.shutdown() else {throw MeasurementError.failed(workspace.errorText ?? "close failed")};try await services.close()
    }
    if diagnostic != nil {return}
    let input="#"+String(repeating:"x",count:10239)
    let (services,workspace,session,window)=try await opened("input-10KiB",source:input)
    let adapter=try session.editor("measure"),editor=adapter.textView
    window.makeFirstResponder(editor);editor.setSelectedRange(.init(location:editor.string.utf16.count,length:0));editor.scrollRangeToVisible(editor.selectedRange())
    try await Task.sleep(for:.milliseconds(100))
    let clock=NativeFrameClock(view:editor)
    var samples:[Double]=[]
    for index in 0..<100 {
      clock.arm()
      editor.insertText(index%4==0 ? "中":index%4==1 ? "🙂":"x",replacementRange:.init(location:NSNotFound,length:0))
      let deadline=ContinuousClock.now.advanced(by:.seconds(2))
      while clock.measured==nil {
        guard ContinuousClock.now<deadline else {clock.stop();try write(samples,"mac-input-samples.json");throw MeasurementError.failed("native display link did not fire after input \(index)")}
        try await Task.sleep(for:.milliseconds(1))
      }
      samples.append(clock.measured!)
    }
    clock.stop()
    let ordered=samples.sorted(),p95=ordered[Int(ceil(Double(ordered.count)*0.95))-1]
    var usage=rusage();getrusage(RUSAGE_SELF,&usage)
    try write(["source_bytes":10240,"samples_ms":samples,"p95_ms":p95,"max_ms":ordered.last!,"definition":"native insertText to second display-synchronized frame of the visible TextKit 2 view, bounding the first frame from above"],"mac-input.json")
    if p95>50 {failures.append("input p95 \(p95)ms")}
    try write(["corpus_sha256":SHA256.hash(data:corpusBytes).map{String(format:"%02x",$0)}.joined(),"first_workspace_draw_ms":firstFrame!,"peak_resident_bytes":usage.ru_maxrss,"failures":failures],"mac-summary.json")
    print("Native 10KiB input p95 \(p95) ms, max \(ordered.last!) ms; first workspace \(firstFrame!) ms; peak RSS \(usage.ru_maxrss) bytes")
    _ = try await session.document.saveNative()
    window.orderOut(nil);window.contentView=nil
    guard await workspace.shutdown() else {throw MeasurementError.failed(workspace.errorText ?? "close failed")};try await services.close()
    guard failures.isEmpty else {throw MeasurementError.failed(failures.joined(separator:"; "))}
  }
}
