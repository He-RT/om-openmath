import AppKit
import MetalKit
import SwiftUI
import simd

struct SceneGPUVertex:Sendable {let position:SIMD4<Float>,normal:SIMD4<Float>,color:SIMD4<Float>}
struct SceneCPUBatch:Sendable {let vertices:[SceneGPUVertex],indices:[UInt32],mode:Int,transparent:Bool}
struct SceneDisplayTransform:Sendable {
  let center:GeometryPoint3,scale:Double
  init(_ bounds:[GeometryPoint3])throws {
    guard bounds.count==2 else {throw GeometryError.invalid}
    let lo=bounds[0],hi=bounds[1],span=zip(lo.array,hi.array).map{$0.1-$0.0}
    guard span.allSatisfy({$0.isFinite && $0>=0}),let width=span.max() else {throw GeometryError.unrepresentable}
    center = .init(lo.x+span[0]/2,lo.y+span[1]/2,lo.z+span[2]/2);scale=width>0 ? width/2:1
    guard scale.isFinite,scale>0 else {throw GeometryError.unrepresentable}
  }
  func normalize(_ p:GeometryPoint3)throws->SIMD3<Float> {
    let result=SIMD3<Float>(Float((p.x-center.x)/scale),Float((p.y-center.y)/scale),Float((p.z-center.z)/scale))
    guard result.x.isFinite,result.y.isFinite,result.z.isFinite else {throw GeometryError.unrepresentable};return result
  }
}
struct SceneUniforms {let mvp:simd_float4x4,normal:simd_float4x4,pixelScale:Float,lit:Float,padding:SIMD2<Float>}
func nativeSceneMatrices(_ camera:NativeCamera3D,aspect:Float)->(simd_float4x4,simd_float4x4) {
  let y=Float(cos(camera.yaw)),s=Float(sin(camera.yaw)),p=Float(cos(camera.pitch)),t=Float(sin(camera.pitch))
  let view=simd_float4x4(columns:(.init(y,t*s,-p*s,0),.init(0,p,t,0),.init(s,-t*y,p*y,0),.init(Float(camera.panX),Float(camera.panY),-Float(camera.distance)*max(1,1/aspect),1)))
  let f=Float(1/tan(Double.pi/8)),near:Float=0.01,far:Float=100
  let projection=simd_float4x4(columns:(.init(f/aspect,0,0,0),.init(0,f,0,0),.init(0,0,far/(near-far),-1),.init(0,0,near*far/(near-far),0)))
  return (projection*view,view)
}

@MainActor private final class SceneMetalPipelines {
  let device:MTLDevice,queue:MTLCommandQueue,opaque:MTLRenderPipelineState,transparent:MTLRenderPipelineState,points:MTLRenderPipelineState,depth:MTLDepthStencilState,readDepth:MTLDepthStencilState
  private static var cached:Task<SceneMetalPipelines,Error>?
  static func prepared()async throws->SceneMetalPipelines {
    if let cached {return try await cached.value}
    let task=Task {try await build()};cached=task
    do {return try await task.value} catch {cached=nil;throw error}
  }
  private init(device:MTLDevice,queue:MTLCommandQueue,opaque:MTLRenderPipelineState,transparent:MTLRenderPipelineState,points:MTLRenderPipelineState,depth:MTLDepthStencilState,readDepth:MTLDepthStencilState) {self.device=device;self.queue=queue;self.opaque=opaque;self.transparent=transparent;self.points=points;self.depth=depth;self.readDepth=readDepth}
  private static func build()async throws->SceneMetalPipelines {
    guard let device=MTLCreateSystemDefaultDevice(),let queue=device.makeCommandQueue() else {throw GeometryError.unavailable("Metal设备不可用")}
    let library:MTLLibrary=try await withCheckedThrowingContinuation {continuation in device.makeLibrary(source:shader,options:nil) {library,error in if let library {continuation.resume(returning:library)} else {continuation.resume(throwing:error ?? GeometryError.invalid)}}}
    func pipeline(blend:Bool,point:Bool=false)async throws->MTLRenderPipelineState {
      let desc=MTLRenderPipelineDescriptor();desc.vertexFunction=library.makeFunction(name:"geometryVertex");desc.fragmentFunction=library.makeFunction(name:point ? "pointFragment":"geometryFragment")
      desc.colorAttachments[0].pixelFormat = .bgra8Unorm;desc.depthAttachmentPixelFormat = .depth32Float
      if blend {let color=desc.colorAttachments[0]!;color.isBlendingEnabled=true;color.sourceRGBBlendFactor = .sourceAlpha;color.destinationRGBBlendFactor = .oneMinusSourceAlpha;color.sourceAlphaBlendFactor = .one;color.destinationAlphaBlendFactor = .oneMinusSourceAlpha}
      return try await withCheckedThrowingContinuation {continuation in device.makeRenderPipelineState(descriptor:desc) {state,error in if let state {continuation.resume(returning:state)} else {continuation.resume(throwing:error ?? GeometryError.invalid)}}}
    }
    let opaque=try await pipeline(blend:false),transparent=try await pipeline(blend:true),points=try await pipeline(blend:true,point:true)
    let desc=MTLDepthStencilDescriptor();desc.depthCompareFunction = .lessEqual;desc.isDepthWriteEnabled=true
    guard let depth=device.makeDepthStencilState(descriptor:desc) else {throw GeometryError.invalid};desc.isDepthWriteEnabled=false
    guard let read=device.makeDepthStencilState(descriptor:desc) else {throw GeometryError.invalid}
    return .init(device:device,queue:queue,opaque:opaque,transparent:transparent,points:points,depth:depth,readDepth:read)
  }
  private static let shader="""
  #include <metal_stdlib>
  using namespace metal;
  struct Vertex { float4 position; float4 normal; float4 color; };
  struct Uniforms { float4x4 mvp; float4x4 normal; float pixelScale; float lit; float2 padding; };
  struct Varying { float4 position [[position]]; float3 normal; float4 color; float size [[point_size]]; };
  vertex Varying geometryVertex(uint id [[vertex_id]], const device Vertex* vertices [[buffer(0)]], constant Uniforms& u [[buffer(1)]]) {
    Varying out; Vertex v=vertices[id]; out.position=u.mvp*float4(v.position.xyz,1); out.normal=(u.normal*float4(v.normal.xyz,0)).xyz; out.color=v.color; out.size=clamp(v.position.w*u.pixelScale,1.0f,128.0f); return out;
  }
  fragment float4 geometryFragment(Varying in [[stage_in]], constant Uniforms& u [[buffer(1)]]) {
    float light=1; float length=metal::length(in.normal); if(u.lit>0.5) light=length>0.000001 ? 0.35+0.65*abs(dot(in.normal/length,normalize(float3(.4,.7,1)))) : 0.35;
    return float4(in.color.rgb*light,in.color.a);
  }
  fragment float4 pointFragment(Varying in [[stage_in]], float2 point [[point_coord]]) {
    if(distance(point,float2(.5))>.5) discard_fragment(); return in.color;
  }
  """
}
private struct SceneMetalBatch {
  let vertex:MTLBuffer,index:MTLBuffer?,cpu:SceneCPUBatch
}
struct NativeMetalSceneView:NSViewRepresentable {
  @ObservedObject var model:NativeGeometryModel
  let data:NativeSceneData
  func makeNSView(context:Context)->MetalSceneCanvas {let view=MetalSceneCanvas(frame:.zero,device:MTLCreateSystemDefaultDevice());view.configure(model,data:data);return view}
  func updateNSView(_ view:MetalSceneCanvas,context:Context) {view.configure(model,data:data)}
  static func dismantleNSView(_ view:MetalSceneCanvas,coordinator:()) {view.stopRendering()}
}
@MainActor final class MetalSceneCanvas:MTKView,MTKViewDelegate {
  private weak var model:NativeGeometryModel?
  private var scene:NativeSceneData?,pipelines:SceneMetalPipelines?,batches:[SceneMetalBatch]=[]
  private var preparing:Task<Void,Never>?
  private var alive=true,generation:UInt64=0,inFlight=0,pendingFrame=false
  private var camera=NativeCamera3D(),previous:NSPoint?
  private let labelOverlay=SceneLabelOverlay()
  private(set) var submittedFrames:UInt64=0,presentedFrames:UInt64=0
  override var acceptsFirstResponder:Bool {true}
  override init(frame:NSRect,device:MTLDevice?) {
    super.init(frame:frame,device:device);isPaused=true;enableSetNeedsDisplay=true;autoResizeDrawable=true
    colorPixelFormat = .bgra8Unorm;depthStencilPixelFormat = .depth32Float;framebufferOnly=true;delegate=self
    addSubview(labelOverlay)
    setAccessibilityElement(true);setAccessibilityRole(.image);setAccessibilityLabel("三维网格。拖动旋转，Shift拖动平移，聚焦后滚轮缩放，Home复位。")
  }
  required init(coder:NSCoder) {fatalError("Metal scene data is required")}
  func configure(_ model:NativeGeometryModel,data:NativeSceneData) {
    let changed=self.model !== model,cameraChanged=camera != model.camera3D
    self.model=model;camera=model.camera3D
    if changed {
      scene=data;generation+=1;let current=generation
      setAccessibilityIdentifier("native-metal-"+model.original.result_ref)
      preparing=Task { [weak self] in
        do {
          self?.model?.rendererReady=false
          let pipelines=try await SceneMetalPipelines.prepared()
          let cpu=try await Task.detached(priority:.userInitiated) {try Self.prepare(data)}.value
          guard let self,alive,generation==current,!Task.isCancelled else {return}
          self.device=pipelines.device;self.pipelines=pipelines
          var uploaded:[SceneMetalBatch]=[],bytes=0
          for batch in cpu where !batch.vertices.isEmpty {
            bytes+=batch.vertices.count*MemoryLayout<SceneGPUVertex>.stride+batch.indices.count*4
            guard bytes<=64*1024*1024 else {throw GeometryError.budget}
            let vertex=batch.vertices.withUnsafeBufferPointer {pipelines.device.makeBuffer(bytes:$0.baseAddress!,length:$0.count*MemoryLayout<SceneGPUVertex>.stride,options:.storageModeShared)}
            let index=batch.indices.isEmpty ? nil:batch.indices.withUnsafeBufferPointer {pipelines.device.makeBuffer(bytes:$0.baseAddress!,length:$0.count*4,options:.storageModeShared)}
            guard let vertex,batch.indices.isEmpty || index != nil else {throw GeometryError.budget}
            uploaded.append(.init(vertex:vertex,index:index,cpu:batch))
          }
          batches=uploaded;needsDisplay=true;updateLabels()
        } catch {guard let self,alive,generation==current else {return};self.model?.renderError="三维图形暂无法显示，可重试；原式和图形数据已保留。"}
      }
    }
    if cameraChanged {needsDisplay=true;updateLabels()}
  }
  nonisolated static func prepare(_ data:NativeSceneData)throws->[SceneCPUBatch] {
    let transform=try SceneDisplayTransform(data.metadata.bounds)
    func vertex(_ p:GeometryPoint3,_ normal:GeometryPoint3,_ color:GeometryColor,size:Float=1)throws->SceneGPUVertex {
      let p=try transform.normalize(p),n=SIMD4<Float>(Float(normal.x),Float(normal.y),Float(normal.z),0),c=SIMD4<Float>(Float(color.r),Float(color.g),Float(color.b),Float(color.a))
      guard n.x.isFinite,n.y.isFinite,n.z.isFinite else {throw GeometryError.invalid}
      return .init(position:.init(p.x,p.y,p.z,size),normal:n,color:c)
    }
    var result:[SceneCPUBatch]=[]
    for mesh in data.meshes {
      guard mesh.positions.count==mesh.normals.count,mesh.positions.count==mesh.colors.count,mesh.triangles.allSatisfy({$0.count==3 && $0.allSatisfy{Int($0)<mesh.positions.count}}) else {throw GeometryError.invalid}
      var values:[SceneGPUVertex]=[];values.reserveCapacity(mesh.positions.count)
      for i in mesh.positions.indices {values.append(try vertex(mesh.positions[i],mesh.normals[i],mesh.colors[i]))}
      result.append(.init(vertices:values,indices:mesh.triangles.flatMap{$0},mode:0,transparent:mesh.colors.contains{$0.a<1}))
    }
    for line in data.lines {
      guard line.positions.count==line.colors.count else {throw GeometryError.invalid}
      var values:[SceneGPUVertex]=[];values.reserveCapacity(line.positions.count)
      for i in line.positions.indices {values.append(try vertex(line.positions[i],.init(0,0,0),line.colors[i]))}
      result.append(.init(vertices:values,indices:[],mode:1,transparent:line.colors.contains{$0.a<1}))
    }
    var points:[SceneGPUVertex]=[]
    for p in data.points {points.append(try vertex(p.position,.init(0,0,0),p.color,size:Float(min(64,max(1,p.radius*2)))))}
    if !points.isEmpty {result.append(.init(vertices:points,indices:[],mode:2,transparent:true))}
    let lo=data.metadata.bounds[0],hi=data.metadata.bounds[1]
    let axes:[GeometryPoint3]=[lo,.init(hi.x,lo.y,lo.z),lo,.init(lo.x,hi.y,lo.z),lo,.init(lo.x,lo.y,hi.z)]
    let colors=[GeometryColor(0.7,0.3,0.3),.init(0.3,0.6,0.4),.init(0.3,0.45,0.8)]
    let vertices=try axes.enumerated().map{try vertex($0.element,.init(0,0,0),colors[$0.offset/2])}
    result.append(.init(vertices:vertices,indices:[],mode:3,transparent:false));return result
  }
  func mtkView(_ view:MTKView,drawableSizeWillChange size:CGSize) {needsDisplay=true;updateLabels()}
  func draw(in view:MTKView) {
    guard alive,window?.isVisible==true,!isHiddenOrHasHiddenAncestor,let pipelines,!batches.isEmpty else {return}
    guard inFlight<3 else {pendingFrame=true;return}
    guard let pass=currentRenderPassDescriptor,let drawable=currentDrawable,let buffer=pipelines.queue.makeCommandBuffer() else {return}
    let background=NSColor.textBackgroundColor.usingColorSpace(.sRGB) ?? .black
    pass.colorAttachments[0].clearColor = .init(red:Double(background.redComponent),green:Double(background.greenComponent),blue:Double(background.blueComponent),alpha:1)
    guard let encoder=buffer.makeRenderCommandEncoder(descriptor:pass) else {return}
    let aspect=Float(max(1,drawableSize.width)/max(1,drawableSize.height)),(mvp,normal)=nativeSceneMatrices(camera,aspect:aspect)
    let ratio=Float(max(1,drawableSize.width)/max(1,bounds.width)),current=generation
    func depth(_ batch:SceneCPUBatch)->Float {guard !batch.vertices.isEmpty else {return 0};return batch.vertices.reduce(0){$0+(normal*SIMD4<Float>($1.position.x,$1.position.y,$1.position.z,1)).z}/Float(batch.vertices.count)}
    let ordered=batches.filter{!$0.cpu.transparent}+batches.filter{$0.cpu.transparent}.sorted{depth($0.cpu)<depth($1.cpu)}
    encoder.setCullMode(.none)
    for batch in ordered {
      encoder.setRenderPipelineState(batch.cpu.mode==2 ? pipelines.points:batch.cpu.transparent ? pipelines.transparent:pipelines.opaque)
      encoder.setDepthStencilState(batch.cpu.transparent ? pipelines.readDepth:pipelines.depth)
      encoder.setVertexBuffer(batch.vertex,offset:0,index:0)
      var uniforms=SceneUniforms(mvp:mvp,normal:normal,pixelScale:ratio,lit:batch.cpu.mode==0 ? 1:0,padding:.zero)
      encoder.setVertexBytes(&uniforms,length:MemoryLayout<SceneUniforms>.stride,index:1);encoder.setFragmentBytes(&uniforms,length:MemoryLayout<SceneUniforms>.stride,index:1)
      if let original=batch.index {
        var indices=original
        if batch.cpu.transparent {
          let raw=batch.cpu.indices
          var order:[(offset:Int,depth:Float)]=[];order.reserveCapacity(raw.count/3)
          for i in stride(from:0,to:raw.count,by:3) {
            var depth:Float=0
            for j in 0..<3 {let p=batch.cpu.vertices[Int(raw[i+j])].position;depth+=(normal*SIMD4<Float>(p.x,p.y,p.z,1)).z}
            order.append((i,depth))
          }
          order.sort{$0.depth<$1.depth}
          var sorted:[UInt32]=[];sorted.reserveCapacity(raw.count)
          for triangle in order {sorted.append(contentsOf:raw[triangle.offset..<triangle.offset+3])}
          guard let ordered=sorted.withUnsafeBufferPointer({pipelines.device.makeBuffer(bytes:$0.baseAddress!,length:$0.count*4,options:.storageModeShared)}) else {continue};indices=ordered
        }
        encoder.drawIndexedPrimitives(type:.triangle,indexCount:batch.cpu.indices.count,indexType:.uint32,indexBuffer:indices,indexBufferOffset:0)
      } else {encoder.drawPrimitives(type:batch.cpu.mode==2 ? .point:batch.cpu.mode==3 ? .line:.lineStrip,vertexStart:0,vertexCount:batch.cpu.vertices.count)}
    }
    encoder.endEncoding();inFlight+=1;submittedFrames+=1
    drawable.addPresentedHandler { [weak self] _ in Task { @MainActor in
      guard let self,self.alive,self.generation==current else {return}
      self.presentedFrames+=1;if self.model?.rendererReady != true {self.model?.rendererReady=true}
    }}
    buffer.addCompletedHandler { [weak self] command in
      let succeeded=command.status == .completed
      Task { @MainActor in
        guard let self else {return};self.inFlight=max(0,self.inFlight-1)
        guard self.alive,self.generation==current else {return}
        if !succeeded {self.model?.renderError="三维图形绘制未完成，可重试；原几何数据仍保留。"}
        if self.pendingFrame {self.pendingFrame=false;self.needsDisplay=true}
      }
    }
    buffer.present(drawable);buffer.commit()
  }
  override func layout() {super.layout();labelOverlay.frame=bounds;updateLabels()}
  private func updateLabels() {labelOverlay.data=scene;labelOverlay.camera=camera;labelOverlay.needsDisplay=true}
  override func mouseDown(with event:NSEvent) {window?.makeFirstResponder(self);previous=convert(event.locationInWindow,from:nil);if event.clickCount==2 {model?.reset()}}
  override func mouseDragged(with event:NSEvent) {
    let p=convert(event.locationInWindow,from:nil);guard let old=previous,var camera=model?.camera3D else {return};previous=p
    let dx=Double(p.x-old.x),dy=Double(p.y-old.y)
    if event.modifierFlags.contains(.shift) {camera.panX+=dx/120;camera.panY+=dy/120}
    else {camera.rotate(dx*0.006,-dy*0.006)};model?.camera3D=camera
  }
  override func mouseUp(with event:NSEvent) {previous=nil}
  override func scrollWheel(with event:NSEvent) {guard window?.firstResponder === self,var camera=model?.camera3D else {super.scrollWheel(with:event);return};camera.zoom(exp(max(-0.7,min(0.7,Double(event.scrollingDeltaY)*0.01))));model?.camera3D=camera}
  override func magnify(with event:NSEvent) {guard var camera=model?.camera3D else {return};camera.zoom(exp(-Double(event.magnification)));model?.camera3D=camera}
  override func keyDown(with event:NSEvent) {
    guard var camera=model?.camera3D else {return}
    if event.keyCode==115 {model?.reset();return}
    if event.modifierFlags.contains(.shift),[123,124,125,126].contains(event.keyCode) {
      camera.panX+=event.keyCode==123 ? -0.1:event.keyCode==124 ? 0.1:0;camera.panY+=event.keyCode==126 ? 0.1:event.keyCode==125 ? -0.1:0
    } else {switch event.keyCode {
      case 123:camera.rotate(-0.1,0)
      case 124:camera.rotate(0.1,0)
      case 125:camera.rotate(0,-0.1)
      case 126:camera.rotate(0,0.1)
      default:if ["+","="].contains(event.charactersIgnoringModifiers ?? "") {camera.zoom(0.8)} else if event.charactersIgnoringModifiers=="-" {camera.zoom(1.25)} else {super.keyDown(with:event);return}
    }};model?.camera3D=camera
  }
  override func viewDidChangeEffectiveAppearance() {super.viewDidChangeEffectiveAppearance();needsDisplay=true;updateLabels()}
  func stopRendering() {alive=false;generation+=1;preparing?.cancel();preparing=nil;delegate=nil;batches=[];scene=nil}
}
@MainActor private final class SceneLabelOverlay:NSView {
  var data:NativeSceneData?,camera=NativeCamera3D()
  override var isFlipped:Bool {true}
  override func hitTest(_ point:NSPoint)->NSView? {nil}
  override func draw(_ dirtyRect:NSRect) {
    guard let data,let transform=try? SceneDisplayTransform(data.metadata.bounds),bounds.width>0,bounds.height>0 else {return}
    let matrix=nativeSceneMatrices(camera,aspect:Float(bounds.width/bounds.height)).0
    func projected(_ world:GeometryPoint3)->NSPoint? {
      guard let n=try? transform.normalize(world) else {return nil};let p=matrix*SIMD4<Float>(n.x,n.y,n.z,1)
      guard p.w>0,p.z>=0,p.z<=p.w else {return nil};return .init(x:CGFloat((p.x/p.w+1)/2)*bounds.width,y:CGFloat((1-p.y/p.w)/2)*bounds.height)
    }
    let lo=data.metadata.bounds[0],hi=data.metadata.bounds[1]
    let axes:[(String,GeometryPoint3)]=[("x",.init(hi.x,lo.y,lo.z)),("y",.init(lo.x,hi.y,lo.z)),("z",.init(lo.x,lo.y,hi.z))]
    for (text,world) in axes {if let p=projected(world) {(text as NSString).draw(at:p,withAttributes:[.font:NSFont.systemFont(ofSize:12),.foregroundColor:NSColor.secondaryLabelColor])}}
    for label in data.labels {if let p=projected(label.position) {(label.text as NSString).draw(at:.init(x:p.x+label.offset.x,y:p.y+label.offset.y),withAttributes:[.font:NSFont.systemFont(ofSize:12),.foregroundColor:label.color.native])}}
  }
}
