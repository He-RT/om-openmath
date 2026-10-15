import Foundation

struct GeometryPoint2:Decodable,Sendable,Equatable {
  let x:Double,y:Double
  init(_ x:Double,_ y:Double) {self.x=x;self.y=y}
  init(from decoder:Decoder)throws {var c=try decoder.unkeyedContainer();x=try c.decode(Double.self);y=try c.decode(Double.self);guard c.isAtEnd,x.isFinite,y.isFinite else {throw GeometryError.invalid}}
}
struct GeometryPoint3:Decodable,Sendable {
  let x:Double,y:Double,z:Double
  init(_ x:Double,_ y:Double,_ z:Double) {self.x=x;self.y=y;self.z=z}
  init(from decoder:Decoder)throws {var c=try decoder.unkeyedContainer();x=try c.decode(Double.self);y=try c.decode(Double.self);z=try c.decode(Double.self);guard c.isAtEnd,x.isFinite,y.isFinite,z.isFinite else {throw GeometryError.invalid}}
  var array:[Double] {[x,y,z]}
}
struct GeometryColor:Decodable,Sendable {
  let r:Double,g:Double,b:Double,a:Double
  init(_ r:Double,_ g:Double,_ b:Double,_ a:Double=1) {self.r=r;self.g=g;self.b=b;self.a=a}
  init(from decoder:Decoder)throws {
    let c=try decoder.singleValueContainer()
    if let text=try? c.decode(String.self) {self=try Self.hex(text);return}
    let v=try c.decode([Double].self)
    guard v.count==4,v.allSatisfy({$0.isFinite && (0...1).contains($0)}) else {throw GeometryError.invalid}
    self.init(v[0],v[1],v[2],v[3])
  }
  static func hex(_ source:String)throws->Self {
    let names=["red":"#ff0000","green":"#008000","blue":"#0000ff","black":"#000000","white":"#ffffff","orange":"#ffa500","purple":"#800080","yellow":"#ffff00","cyan":"#00ffff","magenta":"#ff00ff","gray":"#808080","grey":"#808080"]
    var text=names[source.lowercased()] ?? source
    guard text.hasPrefix("#") else {throw GeometryError.invalid}
    text.removeFirst();if text.count==3 {text=text.map{String(repeating:String($0),count:2)}.joined()}
    guard [6,8].contains(text.count),let n=UInt32(text,radix:16) else {throw GeometryError.invalid}
    if text.count==6 {return .init(Double((n>>16)&255)/255,Double((n>>8)&255)/255,Double(n&255)/255)}
    return .init(Double((n>>24)&255)/255,Double((n>>16)&255)/255,Double((n>>8)&255)/255,Double(n&255)/255)
  }
  func opacity(_ value:Double)->Self {.init(r,g,b,a*value)}
}
enum GeometryError:Error {case invalid,budget,unavailable(String),unrepresentable}
struct NativeGeometrySummary:Decodable,Sendable {let kind:String,available:Bool,metadata:HostJSONValue,unavailable:String?}
struct NativePlotMetadata:Decodable,Sendable {
  struct Curve:Decodable,Sendable {let label:String,segments:[Int]}
  struct Path:Decodable,Sendable {let points:Int,color:GeometryColor,opacity:Double,width:Double?}
  struct Highlights:Decodable,Sendable {let points:[GeometryPoint2],shade:[[Double]]}
  let x_range:[Double],y_range:[Double],scale:String?,curves:[Curve]
  let tiles:Int,arrows:Int,points:Int,markers:Int,labels:Int,skipped:Int
  let paths:[Path]?,polygons:[Path]?,color_range:[Double]?,highlights:Highlights?
}
struct NativeSceneMetadata:Decodable,Sendable {
  struct Mesh:Decodable,Sendable {let vertices:Int,normals:Int,colors:Int,triangles:Int,label:String}
  struct Line:Decodable,Sendable {let positions:Int,colors:Int,width:Double}
  let bounds:[GeometryPoint3],sampled:Bool,skipped:Int,meshes:[Mesh],lines:[Line],points:Int,labels:Int
}
struct NativePlotPath:Sendable {let points:[GeometryPoint2],color:GeometryColor,opacity:Double,width:Double,label:String?}
struct NativePlotTile:Decodable,Sendable {let bounds:[GeometryPoint2],value:Double,color:GeometryColor}
struct NativePlotArrow:Decodable,Sendable {let start:GeometryPoint2,end:GeometryPoint2,value:GeometryPoint2}
struct NativePlotMarker:Decodable,Sendable {let position:GeometryPoint2,color:GeometryColor,opacity:Double,radius:Double}
struct NativePlotLabel:Decodable,Sendable {let position:GeometryPoint2,text:String,color:GeometryColor,opacity:Double,offset:GeometryPoint2}
struct NativePlotData:Sendable {
  let metadata:NativePlotMetadata,paths:[NativePlotPath],polygons:[NativePlotPath],tiles:[NativePlotTile],arrows:[NativePlotArrow],points:[GeometryPoint2],markers:[NativePlotMarker],labels:[NativePlotLabel]
}
struct NativeSceneMesh:Sendable {let positions:[GeometryPoint3],normals:[GeometryPoint3],colors:[GeometryColor],triangles:[[UInt32]],label:String}
struct NativeSceneLine:Sendable {let positions:[GeometryPoint3],colors:[GeometryColor],width:Double}
struct NativeScenePoint:Decodable,Sendable {let position:GeometryPoint3,color:GeometryColor,radius:Double}
struct NativeSceneLabel:Decodable,Sendable {let position:GeometryPoint3,text:String,color:GeometryColor,offset:GeometryPoint2}
struct NativeSceneData:Sendable {let metadata:NativeSceneMetadata,meshes:[NativeSceneMesh],lines:[NativeSceneLine],points:[NativeScenePoint],labels:[NativeSceneLabel]}

struct NativeCamera2D:Sendable,Equatable {
  var x0:Double,x1:Double,y0:Double,y1:Double
  static func fitted(_ meta:NativePlotMetadata)throws->Self {
    guard meta.x_range.count==2,meta.y_range.count==2 else {throw GeometryError.invalid}
    let scale=meta.scale ?? "linear"
    func axis(_ values:[Double],log:Bool)throws->(Double,Double) {
      guard values.allSatisfy(\.isFinite),values[1]>values[0],!log || values[0]>0 else {throw GeometryError.invalid}
      let lo=log ? log10(values[0]):values[0],hi=log ? log10(values[1]):values[1]
      guard (hi-lo).isFinite,hi>lo else {throw GeometryError.unrepresentable};return (lo,hi)
    }
    let x=try axis(meta.x_range,log:scale=="log_x" || scale=="log_log"),y=try axis(meta.y_range,log:scale=="log_y" || scale=="log_log")
    return .init(x0:x.0,x1:x.1,y0:y.0,y1:y.1)
  }
  mutating func zoom(_ factor:Double,anchor:GeometryPoint2 = .init(0.5,0.5)) {
    guard factor.isFinite,factor>0 else {return}
    let dx=(x1-x0)*factor,dy=(y1-y0)*factor,ax=x0+(x1-x0)*anchor.x,ay=y0+(y1-y0)*anchor.y
    let next=Self(x0:ax-dx*anchor.x,x1:ax+dx*(1-anchor.x),y0:ay-dy*anchor.y,y1:ay+dy*(1-anchor.y))
    if [next.x0,next.x1,next.y0,next.y1].allSatisfy(\.isFinite),next.x1>next.x0,next.y1>next.y0 {self=next}
  }
  mutating func pan(_ dx:Double,_ dy:Double) {
    let x=(x1-x0)*dx,y=(y1-y0)*dy
    let next=Self(x0:x0+x,x1:x1+x,y0:y0+y,y1:y1+y)
    if [next.x0,next.x1,next.y0,next.y1].allSatisfy(\.isFinite) {self=next}
  }
}
struct NativeCamera3D:Sendable,Equatable {
  var yaw = -0.65,pitch = -0.45,distance=5.3,panX=0.0,panY=0.0
  mutating func zoom(_ factor:Double) {if factor.isFinite,factor>0 {distance=min(30,max(1.5,distance*factor))}}
  mutating func rotate(_ dx:Double,_ dy:Double) {yaw+=dx;pitch=max(-1.5,min(1.5,pitch+dy))}
}
