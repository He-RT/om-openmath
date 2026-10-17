import AppKit
import SwiftUI

extension GeometryColor {
  @MainActor var native:NSColor {NSColor(srgbRed:CGFloat(r),green:CGFloat(g),blue:CGFloat(b),alpha:CGFloat(a))}
}
/// Only automatically assigned series use theme colors; explicit producer styles remain intact.
@MainActor func nativePlotSeriesColor(_ index:Int)->NSColor {
  let slot=index%6
  if slot<2,let color=NSColor(named:NSColor.Name(slot==0 ? "Plot1":"Plot2")) {return color}
  let light:[UInt32]=[0x2F6BD8,0xC8641B,0x916321,0xA04D7C,0x267D88,0xB14E45]
  let dark:[UInt32]=[0x7AA7F5,0xF0A060,0xDCB477,0xDF95BF,0x77C4D0,0xE89B91]
  return NSColor(name:nil) {appearance in
    let value=(appearance.bestMatch(from:[.aqua,.darkAqua]) == .darkAqua ? dark:light)[slot]
    return NSColor(srgbRed:CGFloat((value>>16)&255)/255,green:CGFloat((value>>8)&255)/255,blue:CGFloat(value&255)/255,alpha:1)
  }
}
struct NativePlotView:NSViewRepresentable {
  @ObservedObject var model:NativeGeometryModel
  let data:NativePlotData
  func makeNSView(context:Context)->PlotCanvas {let view=PlotCanvas();view.configure(model,data:data);return view}
  func updateNSView(_ view:PlotCanvas,context:Context) {view.configure(model,data:data)}
}
@MainActor final class PlotCanvas:NSView {
  private weak var model:NativeGeometryModel?
  private var data:NativePlotData?
  private var camera:NativeCamera2D?
  private var previous:NSPoint?
  override var isFlipped:Bool {true}
  override var acceptsFirstResponder:Bool {true}
  override func becomeFirstResponder()->Bool {let accepted=super.becomeFirstResponder();if accepted {model?.plotFocused=true};return accepted}
  override func resignFirstResponder()->Bool {let accepted=super.resignFirstResponder();if accepted {model?.plotFocused=false};return accepted}
  private var plotRect:NSRect {bounds.insetBy(dx:44,dy:30)}
  // Tick placement is only a viewport decoration; producer coordinates remain untouched.
  private func ticks(_ lo:Double,_ hi:Double)->[Double] {
    let raw=(hi-lo)/6
    guard lo.isFinite,hi.isFinite,raw.isFinite,raw>0 else {return []}
    let magnitude=pow(10,floor(log10(raw))),fraction=raw/magnitude
    let step=(fraction<=1 ? 1.0:fraction<=2 ? 2.0:fraction<=5 ? 5.0:10.0)*magnitude
    guard step.isFinite,step>0 else {return []}
    let first=ceil(lo/step)*step
    return (0..<12).map{first+Double($0)*step}.filter{$0.isFinite && $0>=lo && $0<=hi}
  }
  func configure(_ model:NativeGeometryModel,data:NativePlotData) {
    let changed=self.model !== model || camera != model.camera2D
    self.model=model;self.data=data;camera=model.camera2D
    if changed {needsDisplay=true}
    setAccessibilityElement(true);setAccessibilityRole(.image)
    setAccessibilityLabel("二维采样图。点击聚焦后方向键平移，正负键缩放，Home复位。")
    setAccessibilityIdentifier("native-plot-"+model.original.result_ref)
  }
  override func draw(_ dirtyRect:NSRect) {
    guard let context=NSGraphicsContext.current?.cgContext,let data,let camera,plotRect.width>0,plotRect.height>0 else {return}
    NSColor.textBackgroundColor.setFill();bounds.fill()
    let area=plotRect,scale=data.metadata.scale ?? "linear",logX=scale=="log_x" || scale=="log_log",logY=scale=="log_y" || scale=="log_log"
    func point(_ value:GeometryPoint2)->NSPoint? {
      if logX && value.x<=0 || logY && value.y<=0 {return nil}
      let x=logX ? log10(value.x):value.x,y=logY ? log10(value.y):value.y
      let px=area.minX+(x-camera.x0)/(camera.x1-camera.x0)*area.width,py=area.maxY-(y-camera.y0)/(camera.y1-camera.y0)*area.height
      guard px.isFinite,py.isFinite else {return nil};return NSPoint(x:px,y:py)
    }
    let xTicks=ticks(camera.x0,camera.x1),yTicks=ticks(camera.y0,camera.y1)
    func xPosition(_ value:Double)->CGFloat {area.minX+(value-camera.x0)/(camera.x1-camera.x0)*area.width}
    func yPosition(_ value:Double)->CGFloat {area.maxY-(value-camera.y0)/(camera.y1-camera.y0)*area.height}
    context.saveGState();context.clip(to:area)
    if let highlights=data.metadata.highlights {
      NSColor.systemBlue.withAlphaComponent(0.10).setFill()
      for interval in highlights.shade where interval.count==2 {
        let left=logX ? (interval[0]>0 ? log10(interval[0]):-Double.infinity):interval[0]
        let right=logX ? (interval[1]>0 ? log10(interval[1]):-Double.infinity):interval[1]
        let lo=max(camera.x0,left),hi=min(camera.x1,right)
        guard hi>lo else {continue}
        let x=area.minX+(lo-camera.x0)/(camera.x1-camera.x0)*area.width
        NSRect(x:x,y:area.minY,width:(hi-lo)/(camera.x1-camera.x0)*area.width,height:area.height).fill()
      }
    }
    for tile in data.tiles {
      guard let a=point(tile.bounds[0]),let b=point(tile.bounds[1]) else {continue}
      tile.color.native.setFill();NSRect(x:min(a.x,b.x),y:min(a.y,b.y),width:abs(b.x-a.x)+0.4,height:abs(b.y-a.y)+0.4).fill()
    }
    for polygon in data.polygons {
      let path=NSBezierPath();var started=false
      for p in polygon.points {guard let p=point(p) else {continue};if started {path.line(to:p)} else {path.move(to:p);started=true}}
      path.close();polygon.color.opacity(polygon.opacity).native.setFill();path.fill()
    }
    NSColor.separatorColor.withAlphaComponent(0.24).setStroke()
    let grid=NSBezierPath();grid.lineWidth=0.5
    for value in xTicks {let x=xPosition(value);grid.move(to:.init(x:x,y:area.minY));grid.line(to:.init(x:x,y:area.maxY))}
    for value in yTicks {let y=yPosition(value);grid.move(to:.init(x:area.minX,y:y));grid.line(to:.init(x:area.maxX,y:y))}
    grid.stroke()
    NSColor.secondaryLabelColor.withAlphaComponent(0.4).setStroke()
    let axes=NSBezierPath();axes.lineWidth=0.8
    if !logX,camera.x0<=0,camera.x1>=0 {let x=xPosition(0);axes.move(to:.init(x:x,y:area.minY));axes.line(to:.init(x:x,y:area.maxY))}
    if !logY,camera.y0<=0,camera.y1>=0 {let y=yPosition(0);axes.move(to:.init(x:area.minX,y:y));axes.line(to:.init(x:area.maxX,y:y))}
    axes.stroke()
    for curve in data.paths {
      let path=NSBezierPath();path.lineWidth=CGFloat(max(0.2,min(32,curve.width)));var started=false
      for p in curve.points {guard let p=point(p) else {started=false;continue};if started {path.line(to:p)} else {path.move(to:p);started=true}}
      (curve.seriesIndex.map{nativePlotSeriesColor($0).withAlphaComponent(curve.opacity)} ?? curve.color.opacity(curve.opacity).native).setStroke();path.stroke()
    }
    NSColor.systemBlue.setStroke()
    for arrow in data.arrows {
      guard let a=point(arrow.start),let b=point(arrow.end) else {continue}
      let path=NSBezierPath();path.lineWidth=1.1;path.move(to:a);path.line(to:b)
      let angle=atan2(b.y-a.y,b.x-a.x),length=min(6,hypot(b.x-a.x,b.y-a.y)*0.3)
      for delta in [-0.45,0.45] {path.move(to:b);path.line(to:.init(x:b.x-length*cos(angle+delta),y:b.y-length*sin(angle+delta)))};path.stroke()
    }
    func marker(_ p:GeometryPoint2,radius:Double,color:GeometryColor) {
      guard let p=point(p) else {return};color.native.setFill();let r=CGFloat(min(64,max(1,radius)));NSBezierPath(ovalIn:.init(x:p.x-r,y:p.y-r,width:r*2,height:r*2)).fill()
    }
    for p in data.points {marker(p,radius:3,color:.init(0.15,0.53,0.91))}
    for p in data.markers {marker(p.position,radius:p.radius,color:p.color.opacity(p.opacity))}
    for p in data.metadata.highlights?.points ?? [] {marker(p,radius:4,color:.init(0.95,0.4,0.1))}
    for label in data.labels {
      guard let p=point(label.position) else {continue}
      (label.text as NSString).draw(at:.init(x:p.x+label.offset.x,y:p.y+label.offset.y),withAttributes:[.font:NSFont.systemFont(ofSize:12),.foregroundColor:label.color.opacity(label.opacity).native])
    }
    context.restoreGState()
    NSColor.separatorColor.setStroke();let border=NSBezierPath(rect:area);border.lineWidth=0.7;border.stroke()
    let attrs:[NSAttributedString.Key:Any]=[.font:NSFont.monospacedDigitSystemFont(ofSize:10,weight:.regular),.foregroundColor:NSColor.secondaryLabelColor]
    func tick(_ value:Double,log:Bool)->String {log ? String(format:"10^%.3g",value):String(format:"%.4g",value)}
    for value in xTicks {
      let text=tick(value,log:logX) as NSString
      text.draw(at:.init(x:xPosition(value)-text.size(withAttributes:attrs).width/2,y:area.maxY+6),withAttributes:attrs)
    }
    for value in yTicks {
      let text=tick(value,log:logY) as NSString
      text.draw(at:.init(x:max(0,area.minX-text.size(withAttributes:attrs).width-6),y:yPosition(value)-6),withAttributes:attrs)
    }
    if window?.firstResponder === self {NSColor.controlAccentColor.setStroke();let focus=NSBezierPath(roundedRect:bounds.insetBy(dx:1,dy:1),xRadius:5,yRadius:5);focus.lineWidth=1;focus.stroke()}
  }
  override func mouseDown(with event:NSEvent) {window?.makeFirstResponder(self);previous=convert(event.locationInWindow,from:nil);if event.clickCount==2 {model?.reset()};needsDisplay=true}
  override func mouseDragged(with event:NSEvent) {
    let current=convert(event.locationInWindow,from:nil);guard let last=previous,var camera=model?.camera2D else {return};previous=current
    camera.pan(-Double(current.x-last.x)/max(1,plotRect.width),Double(current.y-last.y)/max(1,plotRect.height));model?.camera2D=camera
  }
  override func mouseUp(with event:NSEvent) {previous=nil}
  override func scrollWheel(with event:NSEvent) {
    guard window?.firstResponder === self,var camera=model?.camera2D else {super.scrollWheel(with:event);return}
    let point=convert(event.locationInWindow,from:nil),area=plotRect
    let anchor=GeometryPoint2(Double((point.x-area.minX)/max(1,area.width)),Double((area.maxY-point.y)/max(1,area.height)))
    camera.zoom(exp(max(-0.7,min(0.7,Double(event.scrollingDeltaY)*0.01))),anchor:anchor);model?.camera2D=camera
  }
  override func magnify(with event:NSEvent) {guard var camera=model?.camera2D else {return};camera.zoom(exp(-Double(event.magnification)));model?.camera2D=camera}
  override func keyDown(with event:NSEvent) {
    guard var camera=model?.camera2D else {return}
    switch event.keyCode {
    case 123:camera.pan(-0.1,0)
    case 124:camera.pan(0.1,0)
    case 125:camera.pan(0,-0.1)
    case 126:camera.pan(0,0.1)
    case 115:model?.reset();return
    default:
      if ["+","="].contains(event.charactersIgnoringModifiers ?? "") {camera.zoom(0.8)}
      else if event.charactersIgnoringModifiers=="-" {camera.zoom(1.25)} else {super.keyDown(with:event);return}
    }
    model?.camera2D=camera
  }
  override func viewDidChangeEffectiveAppearance() {super.viewDidChangeEffectiveAppearance();needsDisplay=true}
}
