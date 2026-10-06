import SwiftUI

/// Native pages of retained kernel values. No source is re-executed to draw or navigate a result.
struct ValueOutputView: View {
  private struct Occurrence: Identifiable { var value: JSONValue; var id: String { value["id"].string } }
  var item: JSONValue
  var cell: NotebookCell
  var controller: NotebookController
  @State private var loaded: JSONValue = .null
  @State private var trail: [JSONValue] = []
  @State private var busy = false
  @State private var error: String?
  @State private var showSource = false
  @State private var generation = 0
  @State private var alive = true
  @State private var work: Task<Void, Never>?
  private var page: JSONValue { loaded.isNull ? item["presentation"] : loaded }
  private var fresh: Bool { cell.status == .Done }
  private var root: Bool { page["path"].array.isEmpty }
  private var authoritative: Bool { root && !page["origin"].isNull }
  private var source: JSONValue { root ? item : page["source"] }
  private var rowCount: Int { Int(page["row_count"].double) }
  private var columnCount: Int { Int(page["column_count"].double) }
  private var rowEnd: Int { Int(page["offset"].double) + page["rows"].array.count }
  private var columnEnd: Int { Int(page["column_offset"].double) + page["columns"].array.count }
  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      ViewThatFits(in: .horizontal) {
        HStack(alignment: .firstTextBaseline, spacing: 12) { heading }
        VStack(alignment: .leading, spacing: 6) { heading }
      }
      if !page["rows"].array.isEmpty {
        ScrollView([.horizontal,.vertical]) {
          Grid(alignment: .topLeading, horizontalSpacing: 16, verticalSpacing: 12) {
            GridRow {
              Text(page["kind"].string == "record" ? controller.text("字段", "Field") : "#").font(.caption.bold())
              ForEach(page["columns"].array.map(\.string), id: \.self) { column in
                Text(["record", "list"].contains(page["kind"].string) ? controller.text("值", "Value") : column).font(.caption.bold())
              }
            }
            ForEach(page["rows"].array.map { Occurrence(value: $0) }) { occurrence in
              let row=occurrence.value
              GridRow {
                Text(fieldLabel(row["label"].string)).font(.caption).foregroundStyle(.secondary)
                  .frame(minWidth: 80, maxWidth: 240, alignment: .leading).textSelection(.enabled)
                ForEach(row["cells"].array.map { Occurrence(value: $0) }) { occurrence in
                  entryView(occurrence.value).frame(minWidth: 100, maxWidth: 400, alignment: .leading)
                }
              }.accessibilityElement(children: .contain).accessibilityIdentifier("value.row.\(row["id"].string)")
            }
          }.padding(12)
        }.frame(maxHeight:420).background(Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 8))
      } else if rowCount == 0 && ["list", "table", "record"].contains(page["kind"].string) {
        Text(controller.text("没有数据行", "No data rows")).foregroundStyle(.secondary)
        if !page["columns"].array.isEmpty { Text(page["columns"].array.map(\.string).joined(separator: " · ")).font(.caption).textSelection(.enabled) }
      } else if !page["source"].isNull {
        Text(page["source"]["modern_form"].string).font(.system(.body, design: .monospaced)).textSelection(.enabled)
      }
      if rowCount > page["rows"].array.count || columnCount > page["columns"].array.count {
        VStack(alignment: .leading, spacing: 8) {
          HStack {
            Button(controller.text("上一页", "Previous")) { load(offset: max(0, Int(page["offset"].double) - 32)) }
              .disabled(!fresh || busy || page["offset"].double == 0)
            Text("\(rowCount == 0 ? 0 : Int(page["offset"].double) + 1)–\(rowEnd) / \(rowCount)").monospacedDigit()
            Button(controller.text("下一页", "Next")) { load(offset: rowEnd) }.disabled(!fresh || busy || rowEnd >= rowCount)
          }
          if columnCount > page["columns"].array.count {
            HStack {
              Button(controller.text("前列", "Previous columns")) { load(columnOffset: max(0, Int(page["column_offset"].double) - 8)) }
                .disabled(!fresh || busy || page["column_offset"].double == 0)
              Text("\(columnCount == 0 ? 0 : Int(page["column_offset"].double) + 1)–\(columnEnd) / \(columnCount)").monospacedDigit()
              Button(controller.text("后列", "Next columns")) { load(columnOffset: columnEnd) }.disabled(!fresh || busy || columnEnd >= columnCount)
            }
          }
        }.font(.caption).buttonStyle(.borderless)
      }
      ViewThatFits(in: .horizontal) {
        HStack(spacing: 12) { actions }
        VStack(alignment: .leading, spacing: 8) { actions }
      }.font(.caption).buttonStyle(.borderless)
      if showSource && !source.isNull {
        Text(source["modern_form"].string).font(.system(.body, design: .monospaced)).textSelection(.enabled)
      }
      if busy { ProgressView(controller.text("读取结果…", "Reading result…")) }
      if let error { Text(error).foregroundStyle(.red).textSelection(.enabled).accessibilityAddTraits(.updatesFrequently) }
    }.accessibilityIdentifier("value.output")
      .onAppear { alive = true }
      .onDisappear { alive = false; generation += 1; work?.cancel() }
      .onChange(of: fresh) { _, _ in generation += 1; work?.cancel(); busy = false; error = nil }
  }
  @ViewBuilder private var heading: some View {
    Text(kindLabel(page["kind"].string)).font(.headline)
    if root && !page["origin"].isNull { Text(page["origin"]["name"].string + " · " + controller.text("实际计算", "Actual computation")).font(.caption).foregroundStyle(.secondary) }
    if rowCount > 0 { Text("\(rowCount) × \(columnCount)").font(.caption).foregroundStyle(.secondary) }
    if !trail.isEmpty {
      Button(controller.text("返回上层", "Back")) {
        guard let parent = trail.popLast() else { return }
        generation += 1; work?.cancel(); loaded = parent; error = nil; showSource = false
      }.disabled(busy)
    }
  }
  @ViewBuilder private var actions: some View {
    if !source.isNull {
      Menu(controller.text("复制", "Copy")) {
        Button("Wolfram") { UIPasteboard.general.string = source["input_form"].string }
        Button(controller.text("现代语法", "Modern")) { UIPasteboard.general.string = source["modern_form"].string }
        Button("LaTeX") { UIPasteboard.general.string = source["latex"].string }
      }
      Button(controller.text("插入", "Insert")) { controller.add(source: source["modern_form"].string, after: cell.id) }
    }
    Button(controller.text("完整源码", "Full source")) {
      if source.isNull { load(includeSource: true) } else { showSource.toggle() }
    }.disabled(!root && (!fresh || busy))
  }
  @ViewBuilder private func entryView(_ entry: JSONValue) -> some View {
    if !entry["source"].isNull {
      VStack(alignment: .leading, spacing: 4) {
        let value = entry["source"]
        if let label = statusLabel(value["input_form"].string) { Text(label).textSelection(.enabled) }
        else if ["text", "boolean", "null"].contains(entry["nature"].string) { Text(value["input_form"].string).font(.system(.body, design: .monospaced)).textSelection(.enabled) }
        else { MathView(latex: value["latex"].string, source: value["input_form"].string, fontSize: 17) }
        Text(natureLabel(entry["nature"].string)).font(.caption2).foregroundStyle(.secondary)
        Menu(controller.text("值操作", "Value actions")) {
          Button("Wolfram") { UIPasteboard.general.string = value["input_form"].string }
          Button(controller.text("现代语法", "Modern")) { UIPasteboard.general.string = value["modern_form"].string }
          Button("LaTeX") { UIPasteboard.general.string = value["latex"].string }
          Button(controller.text("插入", "Insert")) { controller.add(source: value["modern_form"].string, after: cell.id) }
        }.font(.caption).buttonStyle(.borderless)
      }
    } else {
      Button { load(path: entry["path"], offset: 0, columnOffset: 0, descend: true) } label: {
        Text(kindLabel(entry["kind"].string) + " · \(Int(entry["count"].double)) " + controller.text("项", "items"))
          .frame(minHeight: 44, alignment: .leading)
      }.disabled(!fresh || busy)
    }
  }
  private func load(path: JSONValue? = nil, offset: Int? = nil, columnOffset: Int? = nil, includeSource: Bool = false, descend: Bool = false) {
    guard alive && fresh && !busy else { return }
    generation += 1; let token = generation; let parent = page; let viewID = item["presentation"]["view_id"].string
    busy = true; error = nil
    work = Task {
      do {
        let result = try await controller.call(.object(["type": .string("inspect_value"), "query": .object([
          "cell_id": .string(cell.id), "out_index": item["out_index"], "view_id": .string(viewID), "path": path ?? parent["path"],
          "offset": .number(Double(offset ?? Int(parent["offset"].double))), "limit": .number(32),
          "column_offset": .number(Double(columnOffset ?? Int(parent["column_offset"].double))), "column_limit": .number(8), "include_source": .bool(includeSource),
        ])]))
        guard alive && generation == token && fresh && !Task.isCancelled else { return }
        let body = result.response.body
        guard body["type"].string == "value_page" && body["page"]["view_id"].string == viewID else {
          error = body["message"].string.isEmpty ? controller.text("结果读取失败", "Cannot read result") : body["message"].string
          busy = false; return
        }
        if descend { trail.append(parent) }
        loaded = body["page"]; showSource = includeSource; busy = false
      } catch let failure {
        if alive && generation == token && fresh { error = failure.localizedDescription; busy = false }
      }
    }
  }
  private func kindLabel(_ kind: String) -> String {
    let labels = ["scalar": ["表达式", "Expression"], "list": ["列表", "List"], "matrix": ["矩阵", "Matrix"], "record": ["记录", "Record"], "table": ["表格", "Table"], "model": ["拟合模型", "Fitted model"], "interpolation": ["插值数据", "Interpolation"], "series": ["有限级数", "Finite series"], "quantity": ["量与单位", "Quantity"]]
    return labels[kind].map { controller.text($0[0], $0[1]) } ?? kind
  }
  private func natureLabel(_ nature: String) -> String {
    let labels = ["exact": ["精确数", "Exact number"], "machine": ["机器数", "Machine number"], "high_precision": ["高精度数", "High precision"], "symbolic": ["符号", "Symbolic"], "text": ["文本", "Text"], "boolean": ["布尔", "Boolean"], "null": ["空值", "Null"]]
    return labels[nature].map { controller.text($0[0], $0[1]) } ?? nature
  }
  private func fieldLabel(_ label: String) -> String {
    guard authoritative else { return label }
    let labels = ["value": ["结果值", "Value"], "point": ["参数点", "Point"], "parameters": ["参数", "Parameters"], "converged": ["计算状态", "Computation state"], "guarantee": ["数学保证", "Guarantee"], "method": ["方法", "Method"], "scope": ["模式", "Scope"], "goal": ["目标", "Goal"], "termination": ["停止原因", "Termination"], "residual_norm": ["残差范数", "Residual norm"], "error_estimate": ["误差估计（非证书）", "Estimated error (not a certificate)"], "sum_squares": ["残差平方和", "Residual sum of squares"], "data": ["样本与预测", "Samples and predictions"], "certificate": ["证书数据", "Certificate data"], "null_space": ["自由方向", "Free directions"], "iterations": ["实际迭代", "Iterations"], "evaluations": ["实际调用", "Evaluations"], "variables": ["输入变量", "Variables"], "parameter_names": ["参数名", "Parameter names"], "faces_examined": ["检查的约束面", "Faces examined"], "projected_gradient_norm": ["投影梯度范数", "Projected gradient norm"], "bracket_width": ["剩余区间宽度", "Remaining bracket width"], "optimal_set": ["最优集合形式", "Optimal set"], "sample_count": ["样本数", "Samples"], "parameter_count": ["参数数", "Parameters"], "degrees_of_freedom": ["自由度", "Degrees of freedom"], "rank_tolerance": ["数值秩阈值", "Rank threshold"], "damping": ["阻尼", "Damping"], "gradient_cosine": ["梯度夹角余弦", "Gradient cosine"], "model_evaluations": ["模型调用", "Model evaluations"], "jacobian_evaluations": ["Jacobian调用", "Jacobian evaluations"], "rms": ["均方根残差", "RMS residual"], "sum_squares_status": ["平方和表示状态", "Sum of squares status"], "rms_status": ["RMS表示状态", "RMS status"], "precision": ["精度模式", "Precision"], "residuals": ["实际残差", "Actual residuals"]]
    return labels[label].map { controller.text($0[0], $0[1]) } ?? label
  }
  private func statusLabel(_ source: String) -> String? {
    guard authoritative, let data = source.data(using: .utf8), let value = try? JSONDecoder().decode(String.self, from: data) else { return nil }
    let labels = ["certified_global": ["精确全局认证", "Certified global"], "numerical_stationary_candidate": ["数值驻点候选", "Numerical stationary candidate"], "numerical_bounded_candidate": ["数值有界候选", "Numerical bounded candidate"], "numerical_local_fit": ["数值局部拟合", "Numerical local fit"], "numerical_linear_least_squares": ["数值线性最小二乘", "Numerical linear least squares"], "overflow": ["超出机器表示范围", "Outside machine range"], "underflow": ["低于机器表示范围", "Below machine range"], "residual_tolerance": ["达到残差容差", "Residual tolerance met"], "gradient_tolerance": ["达到梯度容差", "Gradient tolerance met"], "exact_ldlt": ["精确LDLᵀ", "Exact LDLᵀ"], "linear_qr": ["线性QR", "Linear QR"], "machine": ["机器精度", "Machine precision"], "finite": ["有限可表示", "Finite"], "local": ["局部", "Local"], "global": ["全局", "Global"], "min": ["最小化", "Minimize"], "max": ["最大化", "Maximize"], "affine": ["仿射自由方向", "Affine free directions"], "one_certified_box_optimum": ["一个盒约束认证点", "One certified box optimum"]]
    return labels[value].map { controller.text($0[0], $0[1]) }
  }
}
