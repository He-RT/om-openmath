import SwiftUI

struct PlotView: View {
  var request: JSONValue
  var initial: JSONValue = .null
  var controller: NotebookController
  @Environment(\.isEnabled) private var isEnabled
  @State private var data: JSONValue = .null
  @State private var working: JSONValue = .null
  @State private var sampling = false
  @State private var failure: String?
  @State private var revision = 0
  @State private var pending: Task<Void, Never>?
  @State private var position: CGPoint?
  @State private var dragOrigin: JSONValue?
  @State private var zoomOrigin: JSONValue?
  @State private var showSamples = false
  private var colors: [Color] { [.openMathAccent, .blue, .orange, .purple, .red, .cyan] }
  private var scaleMode:String{data["scale"].string.isEmpty ? request["options"]["scale"].string : data["scale"].string}
  private var logX:Bool{["log_x","log_log"].contains(scaleMode)}
  private var logY:Bool{["log_y","log_log"].contains(scaleMode)}
  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      GeometryReader { geometry in
        Canvas { context, size in
          guard !data.isNull else { return }
          let x = data["x_range"].array.map(\.double)
          let y = data["y_range"].array.map(\.double)
          guard x.count == 2, y.count == 2, x[1] > x[0], y[1] > y[0] else { return }
          let rect = CGRect(x: 42, y: 10, width: max(1, size.width - 52), height: size.height - 38)
          func point(_ px: Double, _ py: Double) -> CGPoint {
            CGPoint(
              x: rect.minX + fraction(px,x[0],x[1],logX) * rect.width,
              y: rect.maxY - fraction(py,y[0],y[1],logY) * rect.height)
          }
          for value in axisTicks(x[0],x[1],logX) {
            let p = point(value, 0)
            var line = Path()
            line.move(to: CGPoint(x: p.x, y: rect.minY))
            line.addLine(to: CGPoint(x: p.x, y: rect.maxY))
            context.stroke(line, with: .color(.secondary.opacity(0.15)))
            context.draw(
              Text(format(value)).font(.caption2).foregroundStyle(.secondary),
              at: CGPoint(x: p.x, y: rect.maxY + 13))
          }
          for value in axisTicks(y[0],y[1],logY) {
            let p = point(0, value)
            var line = Path()
            line.move(to: CGPoint(x: rect.minX, y: p.y))
            line.addLine(to: CGPoint(x: rect.maxX, y: p.y))
            context.stroke(line, with: .color(.secondary.opacity(0.15)))
            context.draw(
              Text(format(value)).font(.caption2).foregroundStyle(.secondary),
              at: CGPoint(x: rect.minX - 6, y: p.y), anchor: .trailing)
          }
          let highlights = data["highlights"].isNull ? working : data["highlights"]
          for shade in highlights["shade"].array {
            let left = max(x[0], shade[0].double)
            let right = min(x[1], shade[1].double)
            if right > left {
              context.fill(
                Path(
                  CGRect(
                    x: point(left, 0).x, y: rect.minY, width: point(right, 0).x - point(left, 0).x,
                    height: rect.height)), with: .color(Color.openMathAccent.opacity(0.12)))
            }
          }
          context.clip(to: Path(rect))
          for tile in data["geometry"]["tiles"].array {
            let a=point(tile["bounds"][0][0].double,tile["bounds"][0][1].double)
            let b=point(tile["bounds"][1][0].double,tile["bounds"][1][1].double)
            let rectangle=CGRect(x:min(a.x,b.x),y:min(a.y,b.y),width:abs(a.x-b.x),height:abs(a.y-b.y))
            context.fill(Path(rectangle),with:.color(plotColor(tile["color"].string).opacity(request["kind"].string == "Region" ? 0.28 : 0.85)))
          }
          for arrow in data["geometry"]["arrows"].array {
            let a=point(arrow["start"][0].double,arrow["start"][1].double),b=point(arrow["end"][0].double,arrow["end"][1].double)
            let angle=atan2(b.y-a.y,b.x-a.x);var path=Path();path.move(to:a);path.addLine(to:b)
            path.move(to:b);path.addLine(to:CGPoint(x:b.x-5*cos(angle-0.45),y:b.y-5*sin(angle-0.45)))
            path.move(to:b);path.addLine(to:CGPoint(x:b.x-5*cos(angle+0.45),y:b.y-5*sin(angle+0.45)))
            context.stroke(path,with:.color(plotColor(working["options"]["color"].string)),lineWidth:1)
          }
          for p in data["geometry"]["points"].array {let p=point(p[0].double,p[1].double);context.fill(Path(ellipseIn:CGRect(x:p.x-3,y:p.y-3,width:6,height:6)),with:.color(plotColor(working["options"]["color"].string)))}
          for (index, curve) in data["curves"].array.enumerated() {
            for segment in curve["segments"].array {
              var path = Path()
              for (i, pair) in segment.array.enumerated() {
                let p = point(pair[0].double, pair[1].double)
                if i == 0 { path.move(to: p) } else { path.addLine(to: p) }
              }
              context.stroke(path, with: .color(working["options"]["color"].string.isEmpty ? colors[index % colors.count] : plotColor(working["options"]["color"].string)), lineWidth: 2)
            }
          }
          for value in highlights["points"].array {
            let p = point(value[0].double, value[1].double)
            context.fill(
              Path(ellipseIn: CGRect(x: p.x - 4, y: p.y - 4, width: 8, height: 8)),
              with: .color(Color.openMathAccent))
          }
          if let position {
            var path = Path()
            path.move(to: CGPoint(x: position.x, y: rect.minY))
            path.addLine(to: CGPoint(x: position.x, y: rect.maxY))
            path.move(to: CGPoint(x: rect.minX, y: position.y))
            path.addLine(to: CGPoint(x: rect.maxX, y: position.y))
            context.stroke(
              path, with: .color(.secondary.opacity(0.6)),
              style: StrokeStyle(lineWidth: 1, dash: [4, 4]))
          }
        }
        .opacity(sampling ? 0.5 : 1)
        .contentShape(Rectangle())
        .gesture(
          DragGesture().onChanged { value in
            position = value.location
            if dragOrigin == nil { dragOrigin = working }
            if let origin = dragOrigin {
              var changed = origin
              let x = origin["x_range"].array.map(\.double)
              let y = origin["y_range"].array.map(\.double)
              guard x.count == 2, y.count == 2 else { return }
              let dx =
                Double(value.translation.width / max(1, geometry.size.width - 52)) * (x[1] - x[0])
              let dy =
                Double(value.translation.height / max(1, geometry.size.height - 38)) * (y[1] - y[0])
              if logX{let shift = -Double(value.translation.width / max(1,geometry.size.width-52))*(log(x[1])-log(x[0]));changed["x_range"] = .array([.number(exp(log(x[0])+shift)),.number(exp(log(x[1])+shift))])}
              else{changed["x_range"] = .array([.number(x[0] - dx), .number(x[1] - dx)])}
              if logY{let shift=Double(value.translation.height / max(1,geometry.size.height-38))*(log(y[1])-log(y[0]));changed["y_range"] = .array([.number(exp(log(y[0])+shift)),.number(exp(log(y[1])+shift))])}
              else{changed["y_range"] = .array([.number(y[0] + dy), .number(y[1] + dy)])}
              schedule(changed, delay: 150)
            }
          }.onEnded { _ in dragOrigin = nil }
        )
        .simultaneousGesture(
          MagnifyGesture().onChanged { value in
            if zoomOrigin == nil { zoomOrigin = working }
            if let origin = zoomOrigin { zoom(1 / Double(value.magnification), origin: origin) }
          }.onEnded { _ in zoomOrigin = nil }
        )
        .onTapGesture(count: 2) { reset() }
        .accessibilityElement(children: .ignore)
        .accessibilityIdentifier("plot.canvas")
        .accessibilityLabel(controller.text("函数图像", "Function plot"))
        .accessibilityValue(accessibleSummary)
        .accessibilityAction(named: controller.text("放大", "Zoom in")) { zoom(0.8, origin: working) }
        .accessibilityAction(named: controller.text("缩小", "Zoom out")) {
          zoom(1.25, origin: working)
        }
        .accessibilityAction(named: controller.text("复位", "Reset")) { reset() }
        .accessibilityAction(named: controller.text("左移", "Pan left")) { pan(-0.2, 0) }
        .accessibilityAction(named: controller.text("右移", "Pan right")) { pan(0.2, 0) }
        .accessibilityAction(named: controller.text("上移", "Pan up")) { pan(0, 0.2) }
        .accessibilityAction(named: controller.text("下移", "Pan down")) { pan(0, -0.2) }
      }.frame(height: 320)
      HStack {
        Button {
          zoom(0.8, origin: working)
        } label: {
          Image(systemName: "plus.magnifyingglass")
        }.accessibilityLabel(controller.text("放大", "Zoom in"))
        Button {
          zoom(1.25, origin: working)
        } label: {
          Image(systemName: "minus.magnifyingglass")
        }.accessibilityLabel(controller.text("缩小", "Zoom out"))
        Button(controller.text("复位", "Reset")) { reset() }
        Spacer()
        if sampling { ProgressView() }
      }
      .buttonStyle(.borderless)
      ForEach(working["params"].object.keys.sorted(), id: \.self) { name in
        let ranges = working["param_ranges"][name].array.map(\.double)
        let lower = ranges.first ?? -5
        let upper = ranges.last ?? 5
        HStack {
          Text(name).font(.system(.caption, design: .monospaced))
          Slider(
            value: Binding(
              get: { working["params"][name].double },
              set: { value in
                var changed = working
                changed["params"][name] = .number(value)
                schedule(changed, delay: 33)
              }), in: lower...max(lower + 0.001, upper), step: max(0.001, (upper - lower) / 200)
          )
          .accessibilityLabel(controller.text("参数 ", "Parameter ") + name)
          Text(format(working["params"][name].double)).font(.caption).monospacedDigit()
        }
      }
      if let failure {
        HStack {
          Text(failure).font(.caption).foregroundStyle(.red)
          Button(controller.text("重试", "Retry")) { schedule(working, delay: 0) }
        }
      }
      if !data["geometry"].isNull && !["Data", "Histogram"].contains(request["kind"].string) { Text(controller.text("采样示意，边界未经认证", "Sampled visualization; no certified boundary")).font(.caption).foregroundStyle(.secondary) }
      if data["geometry"]["skipped"].double > 0 {Text(controller.text("跳过非有限/不在定义域样本：", "Skipped nonfinite/out-of-domain samples: ")+"\(Int(data["geometry"]["skipped"].double))").font(.caption).foregroundStyle(.secondary)}
      DisclosureGroup(
        controller.text("采样数据", "Sampled data"), isExpanded: $showSamples
      ) {
        if showSamples {
          ScrollView(.horizontal) {
            Text(sampledCoordinates).accessibilityIdentifier("plot.samples").font(.system(.caption, design: .monospaced))
              .textSelection(.enabled)
          }
        }
      }
    }
    .task {
      working = request
      if !initial.isNull {
        data = initial
        working["x_range"] = initial["x_range"]
        working["y_range"] = initial["y_range"]
      } else {
        schedule(request, delay: 0)
      }
    }
    .onChange(of: request) { _, value in
      revision += 1
      pending?.cancel()
      position = nil
      failure = nil
      working = value
      if !initial.isNull {
        data = initial
        working["x_range"] = initial["x_range"]
        working["y_range"] = initial["y_range"]
      } else {
        data = .null
        schedule(value, delay: 0)
      }
    }
    .onChange(of: initial) { _, value in
      if !value.isNull {
        revision += 1
        pending?.cancel()
        sampling = false
        data = value
        working["x_range"] = value["x_range"]
        working["y_range"] = value["y_range"]
      }
    }
    .onDisappear {
      revision += 1
      pending?.cancel()
    }
  }
  private var accessibleSummary: String {
    var parts: [String] = []
    for axis in ["x", "y"] {
      let range = data[axis + "_range"].array
      if range.count == 2 {
        parts.append("\(axis): \(format(range[0].double)) … \(format(range[1].double))")
      }
    }
    for point in data["highlights"]["points"].array {
      parts.append("(\(format(point[0].double)), \(format(point[1].double)))")
    }
    parts.append(controller.text("曲线/网格/向量/点：", "Curves/tiles/vectors/points: ") + "\(data["curves"].array.count)/\(data["geometry"]["tiles"].array.count)/\(data["geometry"]["arrows"].array.count)/\(data["geometry"]["points"].array.count)")
    if data["geometry"]["skipped"].double > 0 { parts.append(controller.text("跳过样本：", "Skipped samples: ") + "\(Int(data["geometry"]["skipped"].double))") }
    return parts.joined(separator: "; ")
  }
  private var sampledCoordinates: String {
    let curves = data["curves"].array.enumerated().map { index, curve in
      let name =
        request["exprs"].array.indices.contains(index)
        ? request["exprs"][index].string : String(index + 1)
      let segments = curve["segments"].array.map { segment in
        segment.array.map { point in
          "\(point[0].double)\t\(point[1].double)"
        }.joined(separator: "\n")
      }.joined(separator: "\n\n")
      return "\(name)\nx\ty\n\(segments)"
    }.joined(separator: "\n\n")
    return curves + (data["geometry"].isNull ? "" : "\n\n" + data["geometry"].pretty)
  }
  private func pan(_ x: Double, _ y: Double) {
    var value = working
    for (key, fraction) in [("x_range", x), ("y_range", y)] {
      let range = working[key].array.map(\.double)
      if range.count == 2 {
        if (key=="x_range" ? logX : logY){let shift=fraction*(log(range[1])-log(range[0]));value[key] = .array([.number(exp(log(range[0])+shift)),.number(exp(log(range[1])+shift))]);continue}
        let offset = fraction * (range[1] - range[0])
        value[key] = .array([.number(range[0] + offset), .number(range[1] + offset)])
      }
    }
    schedule(value, delay: 0)
  }
  private func reset() {
    var value = request
    value["params"] = working["params"]
    schedule(value, delay: 0)
  }
  private func zoom(_ factor: Double, origin: JSONValue) {
    var value = origin
    for key in ["x_range", "y_range"] {
      let r = origin[key].array.map(\.double)
      if r.count == 2 {
        if (key=="x_range" ? logX : logY){let mid=(log(r[0])+log(r[1]))/2,half=(log(r[1])-log(r[0]))/2*factor;let a=exp(mid-half),b=exp(mid+half);if a>0&&a.isFinite&&b.isFinite{value[key] = .array([.number(a),.number(b)])};continue}
        let mid = (r[0] + r[1]) / 2
        let half = max(1e-9, min(1e9, (r[1] - r[0]) / 2 * factor))
        value[key] = .array([.number(mid - half), .number(mid + half)])
      }
    }
    schedule(value, delay: 150)
  }
  private func schedule(_ value: JSONValue, delay: Int) {
    guard isEnabled else { return }
    working = value
    revision += 1
    let token = revision
    pending?.cancel()
    pending = Task {
      try? await Task.sleep(for: .milliseconds(delay))
      guard !Task.isCancelled, token == revision else { return }
      sampling = true
      defer { if token == revision { sampling = false } }
      do {
        let packet = try await controller.call(
          .object(["type": .string("sample_plot"), "request": value]))
        guard !Task.isCancelled, token == revision else { return }
        data = packet.response.body["data"]
        working["x_range"] = data["x_range"]
        working["y_range"] = data["y_range"]
        failure = nil
      } catch { if token == revision { failure = error.localizedDescription } }
    }
  }
  private func format(_ value: Double) -> String {
    value.formatted(.number.precision(.significantDigits(1...5)))
  }
  private func fraction(_ value:Double,_ low:Double,_ high:Double,_ logarithmic:Bool)->Double{
    logarithmic ? (log(value > 0 ? value : low)-log(low))/(log(high)-log(low)) : (value-low)/(high-low)
  }
  private func axisTicks(_ low:Double,_ high:Double,_ logarithmic:Bool)->[Double]{logarithmic ? ticks(log10(low),log10(high)).map{pow(10,$0)} : ticks(low,high)}
  private func plotColor(_ name:String)->Color{
    if name.hasPrefix("#"),let value=UInt32(name.dropFirst(),radix:16){return Color(red:Double((value>>16)&255)/255,green:Double((value>>8)&255)/255,blue:Double(value&255)/255)}
    switch name{case "blue":return .blue;case "red":return .red;case "orange":return .orange;case "purple":return .purple;case "cyan":return .cyan;default:return .openMathAccent}
  }
  private func ticks(_ low: Double, _ high: Double) -> [Double] {
    guard low.isFinite, high.isFinite, high > low else { return [] }
    let raw = (high - low) / 6
    let power = pow(10, floor(log10(raw)))
    let fraction = raw / power
    let step = (fraction <= 1 ? 1 : fraction <= 2 ? 2 : fraction <= 5 ? 5 : 10) * power
    var result: [Double] = []
    var x = ceil(low / step) * step
    while x <= high && result.count < 20 {
      result.append(x)
      x += step
    }
    return result
  }
}
