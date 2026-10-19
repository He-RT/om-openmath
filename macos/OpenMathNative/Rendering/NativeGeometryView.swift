import AppKit
import SwiftUI

struct NativeGeometryView:View {
  @ObservedObject var model:NativeGeometryModel
  @State private var showingDetails=false
  @State private var plotHovered=false
  @State private var sceneHovered=false
  @State private var fullscreen=false
  @Environment(\.colorScheme) private var scheme
  var body:some View {
    VStack(alignment:.leading,spacing:10) {
      if model.loading {WorkbenchProgress("正在读取图形数据…").controlSize(.small)}
      if let plot=model.plot {
        NativePlotView(model:model,data:plot).frame(height:250)
          .clipShape(RoundedRectangle(cornerRadius:8))
          .overlay(RoundedRectangle(cornerRadius:8).stroke(Theme.Color.line))
          .overlay(alignment:.topTrailing) {
            controlBar(scene:false).padding(8)
              .opacity(plotHovered || model.plotFocused || NSApp.isFullKeyboardAccessEnabled ? 1:0)
          }.onHover {plotHovered=$0}
        let legends=plot.paths.enumerated().filter{$0.element.label != nil}
        if !legends.isEmpty {ScrollView(.horizontal) {HStack(spacing:16) {ForEach(legends,id:\.offset) {_,path in
          HStack(spacing:5) {
            Circle().fill(Color(nsColor:path.seriesIndex.map{nativePlotSeriesColor($0)} ?? path.color.native)).frame(width:7,height:7)
            Text(path.label ?? "").lineLimit(1)
          }.font(.system(size:12))
        }}}.scrollIndicators(.hidden)}
        DisclosureGroup("图形信息",isExpanded:$showingDetails) {
          VStack(alignment:.leading,spacing:5) {
            Text("\(plot.paths.reduce(0){$0+$1.points.count}) 个曲线采样点")
            Text("有限机器采样；不是认证的曲线边界。")
            Text("x: \(range(plot.metadata.x_range))；y: \(range(plot.metadata.y_range))")
            if plot.metadata.skipped>0 {Text("跳过 \(plot.metadata.skipped) 个未定义样本。")}
            if let values=plot.metadata.color_range,values.count==2 {Text("色值范围："+range(values))}
          }.font(.caption).foregroundStyle(.secondary).textSelection(.enabled).padding(.top,4)
        }.font(.system(size:12))
      }
      if let scene=model.scene {
        sceneCanvas(scene,height:400,expanded:false)
        DisclosureGroup("图形信息",isExpanded:$showingDetails) {
          VStack(alignment:.leading,spacing:5) {
            Text("\(scene.meshes.reduce(0){$0+$1.triangles.count}) 个三角形")
            Text(scene.metadata.sampled ? "曲面由有限样本生成，边界未经认证。":"图元几何。")
            if scene.metadata.skipped>0 {Text("跳过 \(scene.metadata.skipped) 个未定义或不连续样本。")}
            if scene.meshes.contains(where:{$0.colors.contains{$0.a<1}}) {Text("透明重叠使用近似深度排序。")}
            if scene.lines.contains(where:{$0.width != 1}) {Text("三维线条按 1 像素显示。")}
            ForEach(0..<3,id:\.self) {axis in
              Text("\(["x","y","z"][axis]): \(scene.metadata.bounds[0].array[axis].formatted(.number.precision(.significantDigits(6)))) … \(scene.metadata.bounds[1].array[axis].formatted(.number.precision(.significantDigits(6))))")
            }
          }.font(.caption).foregroundStyle(.secondary).textSelection(.enabled).padding(.top,4)
        }.font(.system(size:12))
      }
      if let error=model.error {Text(error).font(.callout).foregroundStyle(.secondary);Button("重新读取图形") {Task {await model.load()}}}
      if let error=model.renderError {Text(error).font(.callout).foregroundStyle(.secondary);Button("重试三维显示") {model.renderError=nil;model.rendererReady=false;model.rendererAttempt=UUID()}}
    }.task {await model.load()}
      .sheet(isPresented:$fullscreen) {
        GeometryReader {geometry in
          if let scene=model.scene {sceneCanvas(scene,height:geometry.size.height,expanded:true)}
        }.frame(minWidth:900,idealWidth:1100,minHeight:650,idealHeight:800)
          .background(Theme.Color.paper).preferredColorScheme(scheme)
      }
  }
  private func range(_ values:[Double])->String {values.map{$0.formatted(.number.precision(.significantDigits(6)))}.joined(separator:" … ")}
  private func sceneCanvas(_ scene:NativeSceneData,height:CGFloat,expanded:Bool)->some View {
    NativeMetalSceneView(model:model,data:scene).id(model.rendererAttempt).frame(height:height)
      .overlay {if !model.rendererReady,model.renderError==nil {WorkbenchProgress("正在准备三维图形…").controlSize(.small)}}
      .overlay(alignment:.topTrailing) {
        HStack(spacing:8) {
          controls(scene:true)
          Button(expanded ? "退出全屏":"全屏",systemImage:expanded ? "arrow.down.right.and.arrow.up.left":"arrow.up.left.and.arrow.down.right") {
            model.noteSceneInteraction();fullscreen.toggle()
          }.labelStyle(.iconOnly).buttonStyle(.borderless).help(expanded ? "退出全屏":"全屏查看图形")
            .keyboardShortcut(expanded ? .escape:KeyEquivalent("f"),modifiers:expanded ? []:[.command,.shift])
        }.padding(6).background(.regularMaterial,in:RoundedRectangle(cornerRadius:8))
          .padding(8).opacity(sceneHovered || model.sceneFocused || expanded || NSApp.isFullKeyboardAccessEnabled ? 1:0)
      }
      .overlay(alignment:.bottomLeading) {
        if !model.sceneInteracted {Text("拖动旋转，滚动缩放").font(.system(size:12)).foregroundStyle(Theme.Color.ink2)
          .padding(.horizontal,10).padding(.vertical,6).background(.regularMaterial,in:Capsule()).padding(10).allowsHitTesting(false)}
      }
      .clipShape(RoundedRectangle(cornerRadius:8))
      .overlay(RoundedRectangle(cornerRadius:8).stroke(Theme.Color.line))
      .onHover {sceneHovered=$0}
  }
  private func controlBar(scene:Bool)->some View {
    controls(scene:scene).padding(6).background(.regularMaterial,in:RoundedRectangle(cornerRadius:8))
  }
  private func controls(scene:Bool)->some View {
    HStack(spacing:12) {
      Button("放大",systemImage:"plus.magnifyingglass") {
        if scene {model.noteSceneInteraction();model.camera3D.zoom(0.8)} else {model.camera2D?.zoom(0.8)}
      }.labelStyle(.iconOnly).help("放大")
      Button("缩小",systemImage:"minus.magnifyingglass") {
        if scene {model.noteSceneInteraction();model.camera3D.zoom(1.25)} else {model.camera2D?.zoom(1.25)}
      }.labelStyle(.iconOnly).help("缩小")
      Button("复位",systemImage:"arrow.counterclockwise") {if scene {model.noteSceneInteraction()};model.reset()}.labelStyle(.iconOnly).help("恢复初始视窗")
      Menu("视图操作",systemImage:"ellipsis") {
        if scene {
          Button("正面") {model.noteSceneInteraction();model.camera3D.yaw=0;model.camera3D.pitch=0}
          Button("顶面") {model.noteSceneInteraction();model.camera3D.yaw=0;model.camera3D.pitch=1.5}
          Button("侧面") {model.noteSceneInteraction();model.camera3D.yaw=Double.pi/2;model.camera3D.pitch=0}
          Divider()
          Button("向左旋转") {model.camera3D.rotate(-0.15,0)}
          Button("向右旋转") {model.camera3D.rotate(0.15,0)}
          Button("向上旋转") {model.camera3D.rotate(0,0.15)}
          Button("向下旋转") {model.camera3D.rotate(0,-0.15)}
        } else {
          Button("向左平移") {model.camera2D?.pan(-0.1,0)}
          Button("向右平移") {model.camera2D?.pan(0.1,0)}
          Button("向上平移") {model.camera2D?.pan(0,0.1)}
          Button("向下平移") {model.camera2D?.pan(0,-0.1)}
        }
      }.labelStyle(.iconOnly).menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
    }.buttonStyle(.borderless)
  }
}
