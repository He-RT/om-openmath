import SwiftUI

/// Each slider owns a separate credential-free kernel. Its cancellation never interrupts the notebook session.
struct ExploreOutputView: View {
  var item: JSONValue
  var cell: NotebookCell
  var controller: NotebookController
  @Environment(\.scenePhase) private var scenePhase
  @State private var context: String?
  @State private var client: KernelClient?
  @State private var values: JSONValue = .null
  @State private var result: JSONValue = .null
  @State private var pending: Task<Void, Never>?
  @State private var generation = 0
  @State private var alive = false
  @State private var busy = false
  @State private var failure: String?
  private var displayed: JSONValue { result.isNull ? item["result"] : result }
  private var selected: JSONValue { values.isNull ? item["result"]["values"] : values }
  private var fresh: Bool { cell.status == .Done }
  private var changed: Bool { selected != displayed["values"] }
  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      ViewThatFits(in: .horizontal) {
        HStack { Text(controller.text("参数探索", "Parameter exploration")).font(.headline); Text(controller.text("独立只读作用域", "Isolated readonly scope")).font(.caption).foregroundStyle(.secondary) }
        VStack(alignment: .leading) { Text(controller.text("参数探索", "Parameter exploration")).font(.headline); Text(controller.text("独立只读作用域", "Isolated readonly scope")).font(.caption).foregroundStyle(.secondary) }
      }
      ForEach(item["controls"].array.map { $0["name"].string }, id: \.self) { name in
        let control = item["controls"].array.first { $0["name"].string == name } ?? .null
        let low = control["range"][0].double, high = control["range"][1].double
        HStack {
          Text(name).font(.system(.caption, design: .monospaced))
          Slider(value: Binding(get: { selected[name].double }, set: { value in
            var next = selected; next[name] = .number(value); run(next)
          }), in: low...high, step: max(Double.leastNonzeroMagnitude, (high-low)/200))
            .disabled(!fresh || context == nil).accessibilityLabel(controller.text("探索参数 ", "Explore parameter ")+name).accessibilityIdentifier("explore.slider.\(name)")
          Text(selected[name].double.formatted(.number.precision(.significantDigits(1...6)))).font(.caption).monospacedDigit()
        }
      }
      ViewThatFits(in: .horizontal) {
        HStack { actions }
        VStack(alignment: .leading) { actions }
      }.buttonStyle(.borderless).font(.caption)
      if busy { ProgressView(controller.text("正在计算参数…", "Computing parameters…")) }
      if changed { Text(controller.text("下方保留上次成功参数的结果", "Below: last successful parameter result")).font(.caption).foregroundStyle(.secondary) }
      if let failure { Text(failure).foregroundStyle(.red).textSelection(.enabled) }
      let output = displayed["item"]
      if output["type"].string == "expr" {
        ExpressionOutput(item: output, controller: controller, fresh: fresh && !busy && !changed, inspect: inspect)
          .id("\(item["view_id"].string):\(displayed["revision"].double)")
      } else if output["type"].string == "plot" {
        PlotView(request: output["request"], initial: output["data"], controller: controller, sample: sample, exportParameters: displayed["values"])
          .id("\(item["view_id"].string):\(displayed["revision"].double)").disabled(!fresh || busy || changed)
      } else if output["type"].string == "scene3_d" {
        SceneUnavailableView(item: output, controller: controller)
      } else if output["type"].string == "solutions" {
        SolutionCards(item: output, cell: cell, controller: controller)
      } else { Text(output["message"].string).foregroundStyle(.red) }
      Text("\(displayed["timing_ms"].double.formatted(.number.precision(.fractionLength(0...2)))) ms").font(.caption2).foregroundStyle(.secondary)
      if !displayed["value_token"].isNull {
        HStack {
          ForEach(["csv", "json"], id: \.self) { format in
            ExportArtifactButton(title: controller.text("导出 ", "Export ") + format.uppercased(), name: "OpenMath-explore-data", version: "\(displayed["revision"].double)", enabled: fresh && !busy && !changed, controller: controller) {
              .object(["type": .string("export_value_token"), "format": .string(format), "token": displayed["value_token"]])
            }
          }
        }.font(.caption).buttonStyle(.borderless)
      }
      ForEach(Array(displayed["messages"].array.enumerated()), id: \.offset) { _, message in
        Text(message["text"].string).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
      }
    }.accessibilityElement(children: .contain).accessibilityIdentifier("explore.output")
      .task {
        alive = true
        do {
          if client == nil { client = try KernelClient() }
          let token = generation
          let packet = try await controller.call(.object(["type": .string("get_explore_context"), "query": .object([
            "cell_id": .string(cell.id), "out_index": item["out_index"], "view_id": item["view_id"],
          ])]))
          guard alive && generation == token && !Task.isCancelled && fresh else { return }
          let body = packet.response.body
          guard body["type"].string == "explore_context" && body["view_id"] == item["view_id"] else { throw KernelError.message("参数探索快照无效") }
          context = body["context"].string
        } catch { if alive { failure = error.localizedDescription } }
      }
      .onChange(of: fresh) { _, value in if !value { cancel(); context = nil } }
      .onChange(of: scenePhase) { _, value in if value != .active { cancel() } }
      .onDisappear { alive = false; cancel(); client?.close(); client = nil; context = nil }
  }
  @ViewBuilder private var actions: some View {
    Button(controller.text("重新计算", "Recompute")) { run(selected, delay: 0) }.disabled(!fresh || context == nil)
    Button(controller.text("复位参数", "Reset parameters")) { run(item["result"]["values"], delay: 0) }.disabled(!fresh || context == nil)
    Button(controller.text("取消", "Cancel")) { cancel() }.disabled(!busy)
  }
  private func cancel() { generation += 1; pending?.cancel(); client?.interrupt(); busy = false }
  private func run(_ next: JSONValue, delay: Int = 33) {
    guard fresh, let context, let client else { return }
    cancel(); values = next; busy = true; failure = nil
    let token = generation
    pending = Task {
      do {
        try await Task.sleep(for: .milliseconds(delay))
        guard !Task.isCancelled && alive && token == generation else { return }
        let packet = try await client.request(.object(["type": .string("run_explore_context"), "context": .string(context), "values": next, "revision": .number(Double(token))]))
        guard !Task.isCancelled && alive && token == generation && fresh else { return }
        let answer = packet.response.body["result"]
        guard packet.response.body["type"].string == "explored" && answer["view_id"] == item["view_id"] && answer["revision"].double == Double(token) else { throw KernelError.message("参数探索回复无效") }
        result = answer; busy = false
      } catch { if alive && token == generation { failure = error.localizedDescription; busy = false } }
    }
  }
  private func sample(_ request: JSONValue) async throws -> KernelPacket {
    guard fresh, let context, let client else { throw KernelError.message("参数探索输出已过期") }
    return try await client.request(.object(["type": .string("sample_explore_plot"), "context": .string(context), "values": displayed["values"], "request": request]))
  }
  private func inspect(_ request: JSONValue) async throws -> KernelPacket {
    guard fresh, let context, let client else { throw KernelError.message("参数探索输出已过期") }
    return try await client.request(.object(["type": .string("inspect_explore_expression"), "context": .string(context), "values": displayed["values"], "source": request["source"], "numeric": request["numeric"]]))
  }
}
