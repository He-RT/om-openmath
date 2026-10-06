import SwiftUI

struct CellOutputView: View {
  var cell: NotebookCell
  var controller: NotebookController
  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      if cell.status == .Stale && !cell.output.isNull {
        Label(controller.text("结果已过期", "Out of date"), systemImage: "clock").font(.caption)
          .foregroundStyle(.orange)
      }
      ForEach(Array(cell.output["items"].array.enumerated()), id: \.offset) { _, item in
        if item["type"].string == "scene3_d" {
          SceneUnavailableView(item: item, controller: controller)
        } else if item["type"].string == "explore" {
          ExploreOutputView(item: item, cell: cell, controller: controller)
            .id("\(cell.id):\(item["out_index"].double):\(item["view_id"].string)")
        } else if item["type"].string == "solutions" {
          SolutionCards(item: item, cell: cell, controller: controller)
        } else if item["type"].string == "plot" {
          PlotView(request: item["request"], initial: item["data"], controller: controller)
            .disabled(cell.status != .Done)
        } else if item["type"].string == "expr" {
          if !item["presentation"].isNull {
            ValueOutputView(item: item, cell: cell, controller: controller)
              .id("\(cell.id):\(item["out_index"].double):\(item["presentation"]["view_id"].string)")
          } else {
            ExpressionOutput(item: item, controller: controller, fresh: cell.status == .Done)
          }
        } else {
          Text(item["message"].string).foregroundStyle(.red).textSelection(.enabled)
        }
      }
      if !cell.output["messages"].array.isEmpty {
        DisclosureGroup(controller.text("消息", "Messages")) {
          ForEach(Array(cell.output["messages"].array.enumerated()), id: \.offset) { _, message in
            Text(message["text"].string.isEmpty ? message.pretty : message["text"].string).font(
              .caption
            ).foregroundStyle(message["level"].string == "Error" ? .red : .secondary).textSelection(
              .enabled)
          }
        }
      }
      if !cell.output.isNull {
        Text(
          "\(cell.output["timing_ms"].double.formatted(.number.precision(.fractionLength(0...2)))) ms"
        ).font(.caption2).foregroundStyle(.secondary).frame(
          maxWidth: .infinity, alignment: .trailing)
      }
    }
  }
}
struct ExpressionOutput: View {
  var item: JSONValue
  var controller: NotebookController
  var fresh = true
  var inspect: ((JSONValue) async throws -> KernelPacket)? = nil
  @State private var numeric: JSONValue = .null
  @State private var numericSource: String?
  private var showsNumeric: Bool { !numeric.isNull && numericSource == item["input_form"].string }
  private var displayed: JSONValue { showsNumeric ? numeric : item }
  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      MathView(
        latex: displayed["latex"].string,
        source: displayed["input_form"].string
      ).accessibilityIdentifier("expression.\(item["input_form"].string)")
      HStack {
        Menu(controller.text("复制", "Copy")) {
          Button("LaTeX") { UIPasteboard.general.string = displayed["latex"].string }
          Button("Wolfram") { UIPasteboard.general.string = displayed["input_form"].string }
          Button(controller.text("现代语法", "Modern")) {
            UIPasteboard.general.string = displayed["modern_form"].string
          }
        }
        Button(controller.text(showsNumeric ? "精确值" : "数值 ≈", showsNumeric ? "Exact" : "Numeric ≈"))
        {
          if showsNumeric {
            numeric = .null
            numericSource = nil
            return
          }
          let source = item["input_form"].string
          Task {
            let operation = inspect ?? controller.call
            if let result = try? await operation(
              .object([
                "type": .string("inspect_expression"), "source": .string(source),
                "numeric": .bool(true),
              ]))
            {
              numericSource = source
              numeric = result.response.body["value"]
            }
          }
        }.disabled(!fresh)
        Button(controller.text("插入", "Insert")) {
          controller.add(source: displayed["modern_form"].string)
        }
      }.font(.caption).buttonStyle(.borderless)
    }
  }
}
struct SolutionCards: View {
  var item: JSONValue
  var cell: NotebookCell
  var controller: NotebookController
  @State private var showPlot = false
  private var view: JSONValue { item["view"] }
  private var grouped: [(solution: JSONValue, count: Int)] {
    var results: [(JSONValue, Int)] = []
    for solution in view["solutions"].array {
      if let i = results.firstIndex(where: { $0.0 == solution }) {
        results[i].1 += 1
      } else {
        results.append((solution, 1))
      }
    }
    return results
  }
  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      if view["kind"].string == "none" {
        Text(controller.text("无解", "No solutions")).font(.headline)
      } else if view["kind"].string == "all" {
        Text(controller.text("对所有值成立", "All values")).font(.headline)
      } else if view["kind"].string == "region" {
        MathView(latex: view["region_latex"].string)
        NumberLineView(intervals: view["intervals"], controller: controller)
      } else {
        Text("\(view["solutions"].array.count) " + controller.text("个解", "solutions")).font(
          .headline)
        FlowLayout(spacing: 10) {
          ForEach(Array(grouped.enumerated()), id: \.offset) { _, group in
            VStack(alignment: .leading, spacing: 6) {
              ForEach(Array(group.solution["bindings"].array.enumerated()), id: \.offset) {
                _, binding in BindingView(binding: binding, controller: controller)
              }
              if group.count > 1 {
                Text(controller.text("重数 \(group.count)", "Multiplicity \(group.count)")).font(
                  .caption
                ).foregroundStyle(.secondary)
              }
              let condition =
                group.solution["condition_display_latex"].string.isEmpty
                ? group.solution["condition_latex"].string
                : group.solution["condition_display_latex"].string
              if !condition.isEmpty { MathView(latex: condition, fontSize: 16) }
              Text(verification(group.solution["verified"])).font(.caption).foregroundStyle(
                Color.openMathAccent)
            }.padding(12).background(
              Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 12))
          }
        }
      }
      HStack {
        if !item["steps"].isNull {
          Button(controller.text("步骤", "Steps")) {
            controller.stepCell = cell.id
            controller.stepIndex = Int(item["out_index"].double)
            controller.inspector = "steps"
            controller.showInspector = true
          }.accessibilityIdentifier("inspector.steps")
        }
        if !item["plot"].isNull { Button(controller.text("绘图", "Plot")) { showPlot.toggle() } }
        Menu(controller.text("复制全部", "Copy all")) {
          Button("Wolfram") { UIPasteboard.general.string = item["input_form"].string }
          Button(controller.text("现代语法", "Modern")) {
            UIPasteboard.general.string = item["modern_form"].string
          }
          Button("LaTeX") {
            UIPasteboard.general.string =
              view["region_latex"].string.isEmpty
              ? item["latex"].string : view["region_latex"].string
          }
        }
        Button(controller.text("插入", "Insert")) {
          controller.add(source: item["modern_form"].string, after: cell.id)
        }
      }.font(.caption).buttonStyle(.borderless)
      if showPlot {
        PlotView(request: item["plot"], controller: controller).disabled(cell.status != .Done)
      }
    }
  }
  private func verification(_ value: JSONValue) -> String {
    if value.string == "Exact" { return controller.text("✓ 精确验证", "✓ Exact") }
    if value.string == "ByConstruction" { return controller.text("✓ 由构造保证", "✓ By construction") }
    if !value["Numeric"].isNull {
      return controller.text("≈ 数值验证", "≈ Numerically verified")
        + " \(Int(value["Numeric"]["digits"].double))"
    }
    return controller.text("未验证", "Unverified")
  }
}
struct BindingView: View {
  var binding: JSONValue
  var controller: NotebookController
  @State private var radical = false
  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      let value =
        radical && !binding["radicals"].isNull
        ? binding["radicals"]["latex"].string : binding["latex"].string
      MathView(
        latex:
          "\(binding["var_latex"].string.isEmpty ? binding["var"].string : binding["var_latex"].string) = \(value)",
        source: "\(binding["var"].string) = \(binding["input_form"].string)")
      Text("\(binding["var"].string) = \(binding["input_form"].string)").font(
        .system(.caption, design: .monospaced)
      ).textSelection(.enabled).accessibilityIdentifier("solution.\(binding["input_form"].string)")
      if !binding["numeric"].string.isEmpty {
        Text("≈ " + NumericDisplay.human(binding["numeric"].string)).font(.caption).foregroundStyle(
          .secondary
        ).textSelection(.enabled)
      }
      if !binding["radicals"].isNull {
        Button(
          radical
            ? controller.text("Root 形式", "Root form") : controller.text("根式形式", "Radical form")
        ) { radical.toggle() }.font(.caption)
      }
      Button(controller.text("代入…", "Substitute…")) {
        controller.add(
          source: "expr /. {\(binding["var"].string)->(\(binding["input_form"].string))}")
      }.font(.caption)
    }
  }
}
struct NumberLineView: View {
  var intervals: JSONValue
  var controller: NotebookController
  var body: some View {
    VStack(alignment: .leading) {
      Canvas { context, size in
        let endpoints = intervals.array.flatMap { i in
          [i["lo_value"], i["hi_value"]].filter { !$0.isNull }.map(\.double)
        }
        let low = min(-5, endpoints.min() ?? -5)
        let high = max(5, endpoints.max() ?? 5)
        func x(_ value: Double) -> Double { 12 + (value - low) / (high - low) * (size.width - 24) }
        var line = Path()
        line.move(to: CGPoint(x: 8, y: 24))
        line.addLine(to: CGPoint(x: size.width - 8, y: 24))
        context.stroke(line, with: .color(.secondary), lineWidth: 1)
        for interval in intervals.array {
          let a = interval["lo_value"].isNull ? low : interval["lo_value"].double
          let b = interval["hi_value"].isNull ? high : interval["hi_value"].double
          var range = Path()
          range.move(to: CGPoint(x: x(a), y: 24))
          range.addLine(to: CGPoint(x: x(b), y: 24))
          context.stroke(range, with: .color(Color.openMathAccent), lineWidth: 5)
          for (value, closed) in [(a, interval["lo_closed"].bool), (b, interval["hi_closed"].bool)]
          {
            let circle = Path(ellipseIn: CGRect(x: x(value) - 4, y: 20, width: 8, height: 8))
            context.fill(circle, with: .color(closed ? .green : Color(.systemBackground)))
            context.stroke(circle, with: .color(Color.openMathAccent), lineWidth: 1)
          }
        }
      }.frame(height: 48).accessibilityLabel(controller.text("解集数轴", "Solution number line"))
      ForEach(Array(intervals.array.enumerated()), id: \.offset) { _, interval in
        Text(
          "\(interval["lo_closed"].bool ? "[" : "(")\(interval["lo"].isNull ? "−∞" : interval["lo"].string), \(interval["hi"].isNull ? "∞" : interval["hi"].string)\(interval["hi_closed"].bool ? "]" : ")")"
        ).font(.caption).textSelection(.enabled)
      }
    }
  }
}

enum NumericDisplay {
  static func human(_ source: String) -> String {
    source.replacingOccurrences(of: #"`[0-9]+(?:\.[0-9]*)?"#, with: "", options: .regularExpression)
      .replacingOccurrences(of: #"\.(?=$|[ +),])"#, with: "", options: .regularExpression)
      .replacingOccurrences(of: "*^", with: "×10^")
  }
}

/// Native 3D display is explicitly outside this first mobile renderer; never show an empty image or auto-sample an unseen mesh.
struct SceneUnavailableView: View {
  var item: JSONValue
  var controller: NotebookController
  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      Text(controller.text("三维场景", "3D scene")).font(.headline)
      Text(controller.text("iOS/iPadOS 三维展示尚未适配。可在桌面/Web 查看或导出 OBJ。", "iOS/iPadOS 3D rendering is not yet supported. View or export OBJ on desktop/Web."))
        .fixedSize(horizontal: false, vertical: true)
      Text(item["request"]["expressions"].array.map(\.string).joined(separator: ", "))
        .font(.system(.body, design: .monospaced)).textSelection(.enabled)
    }.accessibilityElement(children: .contain).accessibilityIdentifier("scene.unsupported")
  }
}
