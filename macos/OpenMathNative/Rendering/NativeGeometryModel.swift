import Foundation
import SwiftUI

/// Immutable producer geometry, paged from the same ResultBinding. Loading and camera movement
/// never submit a mathematical operation or manufacture sampling data.
@MainActor final class NativeGeometryModel:ObservableObject {
  let original:ResultBinding
  @Published private(set) var plot:NativePlotData?
  @Published private(set) var scene:NativeSceneData?
  @Published var renderError:String?
  @Published var rendererReady=false
  @Published var rendererAttempt=UUID()
  @Published private(set) var loading=false
  @Published private(set) var error:String?
  @Published var camera2D:NativeCamera2D?
  @Published var camera3D=NativeCamera3D()
  @Published var plotFocused=false
  @Published var sceneFocused=false
  @Published private(set) var sceneInteracted=false
  func noteSceneInteraction() {if !sceneInteracted {sceneInteracted=true}}
  private let client:ResultClient
  private var charged=0,closed=false
  init(host:NativeHostClient,binding:ResultBinding) {original=binding;client=ResultClient(host:host)}
  private func decode<T:Decodable & Sendable>(_ value:HostJSONValue,_ type:T.Type)async throws->T {try await Task.detached {try JSONDecoder().decode(type,from:JSONEncoder().encode(value))}.value}
  private func charge(_ count:Int,_ stride:Int)throws {
    guard count>=0,count<=1_200_000,stride>0,count<=(64*1024*1024-charged)/stride else {throw GeometryError.budget};charged+=count*stride
  }
  private func read<T:Decodable & Sendable>(_ channel:InspectResultGeometryPageChannel,count:Int,object:Int=0,segment:Int=0,as type:T.Type,stride:Int)async throws->[T] {
    try charge(count,stride);var result:[T]=[],offset=0;result.reserveCapacity(count)
    while offset<count {
      try Task.checkCancellation();guard !closed else {throw ResultClientError.closing}
      let query=InspectResultGeometryPage(kind:.geometry_page,channel:channel,object_index:UInt32(object),segment_index:UInt32(segment),offset:UInt32(offset),limit:UInt32(min(1024,count-offset)))
      guard let reply=try await client.inspect(.inspect_result_geometry_page(query)) else {throw ResultClientError.staleSelection}
      let page:GeometryPage<T>=try await decode(reply.payload,GeometryPage<T>.self)
      guard page.view_id==original.view_id.value,page.channel==channel.rawValue,page.object_index==object,page.segment_index==segment,page.offset==offset,page.total==count,page.values.count==min(1024,count-offset) else {throw GeometryError.invalid}
      result+=page.values;offset=result.count
    }
    return result
  }
  func load()async {
    guard !closed,!loading,plot==nil,scene==nil else {return};loading=true;error=nil;charged=0;defer {loading=false}
    do {
      try client.select(original)
      guard let reply=try await client.inspect(.inspect_result_geometry(.init(kind:.geometry))) else {throw ResultClientError.staleSelection}
      let summary:NativeGeometrySummary=try await decode(reply.payload,NativeGeometrySummary.self)
      guard summary.available else {throw GeometryError.unavailable(summary.unavailable ?? "此结果尚无采样几何。")}
      if summary.kind=="plot2d" {try await loadPlot(summary.metadata)}
      else if summary.kind=="scene3d" {try await loadScene(summary.metadata)}
      else {throw GeometryError.unavailable("此结果没有图形数据。")}
    } catch {if !closed,!Task.isCancelled {self.error="图形暂无法显示："+String(describing:error)}}
  }
  private func loadPlot(_ value:HostJSONValue)async throws {
    let meta:NativePlotMetadata=try await decode(value,NativePlotMetadata.self),camera=try NativeCamera2D.fitted(meta)
    if let highlights=meta.highlights {
      try charge(highlights.points.count,48);try charge(highlights.shade.count,16)
      guard highlights.shade.allSatisfy({$0.count==2 && $0.allSatisfy(\.isFinite)}) else {throw GeometryError.invalid}
    }
    var paths:[NativePlotPath]=[],polygons:[NativePlotPath]=[]
    let colors=[GeometryColor(0.15,0.53,0.91),.init(0.9,0.35,0.2),.init(0.35,0.65,0.35),.init(0.65,0.35,0.8)]
    for (i,curve) in meta.curves.enumerated() {for (segment,count) in curve.segments.enumerated() {
      let points:[GeometryPoint2]=try await read(.curve_points,count:count,object:i,segment:segment,as:GeometryPoint2.self,stride:48)
      paths.append(.init(points:points,color:colors[i%colors.count],opacity:1,width:1.7,label:segment==0 ? curve.label:nil,seriesIndex:i))
    }}
    for (i,path) in (meta.paths ?? []).enumerated() {
      let points:[GeometryPoint2]=try await read(.path_points,count:path.points,object:i,as:GeometryPoint2.self,stride:48)
      guard path.opacity.isFinite,(0...1).contains(path.opacity),(path.width ?? 1).isFinite else {throw GeometryError.invalid}
      paths.append(.init(points:points,color:path.color,opacity:path.opacity,width:path.width ?? 1,label:nil))
    }
    for (i,path) in (meta.polygons ?? []).enumerated() {
      let points:[GeometryPoint2]=try await read(.polygon_points,count:path.points,object:i,as:GeometryPoint2.self,stride:48)
      guard path.opacity.isFinite,(0...1).contains(path.opacity) else {throw GeometryError.invalid}
      polygons.append(.init(points:points,color:path.color,opacity:path.opacity,width:0,label:nil))
    }
    let tiles:[NativePlotTile]=try await read(.plot_tiles,count:meta.tiles,as:NativePlotTile.self,stride:96)
    let arrows:[NativePlotArrow]=try await read(.plot_arrows,count:meta.arrows,as:NativePlotArrow.self,stride:64)
    let points:[GeometryPoint2]=try await read(.plot_points,count:meta.points,as:GeometryPoint2.self,stride:48)
    let markers:[NativePlotMarker]=try await read(.plot_markers,count:meta.markers,as:NativePlotMarker.self,stride:96)
    let labels:[NativePlotLabel]=try await read(.plot_labels,count:meta.labels,as:NativePlotLabel.self,stride:128)
    guard tiles.allSatisfy({$0.bounds.count==2 && $0.value.isFinite}),markers.allSatisfy({$0.radius.isFinite && $0.radius>=0 && $0.opacity.isFinite && (0...1).contains($0.opacity)}),labels.allSatisfy({$0.text.utf8.count<=65536 && $0.opacity.isFinite && (0...1).contains($0.opacity)}) else {throw GeometryError.invalid}
    guard !closed,!Task.isCancelled else {return}
    plot = .init(metadata:meta,paths:paths,polygons:polygons,tiles:tiles,arrows:arrows,points:points,markers:markers,labels:labels);if camera2D==nil {camera2D=camera}
  }
  private func loadScene(_ value:HostJSONValue)async throws {
    let meta:NativeSceneMetadata=try await decode(value,NativeSceneMetadata.self)
    guard meta.bounds.count==2 else {throw GeometryError.invalid}
    let spans=zip(meta.bounds[0].array,meta.bounds[1].array).map{$0.1-$0.0}
    guard spans.allSatisfy({$0.isFinite && $0>=0}),let maximum=spans.max(),maximum.isFinite else {throw GeometryError.unrepresentable}
    var meshes:[NativeSceneMesh]=[],lines:[NativeSceneLine]=[],vertices=0
    for (i,mesh) in meta.meshes.enumerated() {
      guard mesh.vertices>=0,mesh.vertices<=200000-vertices,mesh.vertices==mesh.normals,mesh.vertices==mesh.colors else {throw GeometryError.budget}
      vertices+=mesh.vertices
      let positions:[GeometryPoint3]=try await read(.mesh_positions,count:mesh.vertices,object:i,as:GeometryPoint3.self,stride:48)
      let normals:[GeometryPoint3]=try await read(.mesh_normals,count:mesh.normals,object:i,as:GeometryPoint3.self,stride:32)
      let colors:[GeometryColor]=try await read(.mesh_colors,count:mesh.colors,object:i,as:GeometryColor.self,stride:32)
      let triangles:[[UInt32]]=try await read(.mesh_triangles,count:mesh.triangles,object:i,as:[UInt32].self,stride:24)
      guard triangles.allSatisfy({$0.count==3 && $0.allSatisfy{Int($0)<positions.count}}) else {throw GeometryError.invalid}
      meshes.append(.init(positions:positions,normals:normals,colors:colors,triangles:triangles,label:mesh.label))
    }
    for (i,line) in meta.lines.enumerated() {
      guard line.positions>=0,line.positions<=200000-vertices,line.positions==line.colors,line.width.isFinite,line.width>=0 else {throw GeometryError.budget}
      vertices+=line.positions
      let positions:[GeometryPoint3]=try await read(.line_positions,count:line.positions,object:i,as:GeometryPoint3.self,stride:48)
      let colors:[GeometryColor]=try await read(.line_colors,count:line.colors,object:i,as:GeometryColor.self,stride:32)
      lines.append(.init(positions:positions,colors:colors,width:line.width))
    }
    let points:[NativeScenePoint]=try await read(.scene_points,count:meta.points,as:NativeScenePoint.self,stride:96)
    let labels:[NativeSceneLabel]=try await read(.scene_labels,count:meta.labels,as:NativeSceneLabel.self,stride:128)
    guard vertices+points.count<=200000,points.allSatisfy({$0.radius.isFinite && $0.radius>=0}),labels.allSatisfy({$0.text.utf8.count<=65536}) else {throw GeometryError.budget}
    guard !closed,!Task.isCancelled else {return};scene = .init(metadata:meta,meshes:meshes,lines:lines,points:points,labels:labels)
  }
  func reset() {
    if let plot {camera2D=try? .fitted(plot.metadata)};camera3D = .init()
  }
  func close()async {closed=true;await client.close()}
}
private struct GeometryPage<T:Decodable & Sendable>:Decodable,Sendable {let view_id:String,channel:String,object_index:Int,segment_index:Int,total:Int,offset:Int,values:[T]}
