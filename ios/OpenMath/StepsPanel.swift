import SwiftUI

struct StepsPanel: View {
  var controller: NotebookController
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var details: Bool {
    get { controller.stepDetails }
    nonmutating set { controller.stepDetails = newValue }
  }
  private var expanded: [String: Bool] {
    get { controller.stepExpanded }
    nonmutating set { controller.stepExpanded = newValue }
  }
  private var job: String? {
    get { controller.stepJob }
    nonmutating set { controller.stepJob = newValue }
  }
  @State private var consent = false
  @State private var chosenStep: String?
  private var highlight: String? {
    get { controller.stepHighlight }
    nonmutating set { controller.stepHighlight = newValue }
  }
  private var item: JSONValue {
    guard let cell = controller.cells.first(where: { $0.id == controller.stepCell }) else {
      return .null
    }
    return cell.output["items"].array.first {
      $0["out_index"].double == Double(controller.stepIndex ?? -1) && !$0["steps"].isNull
    } ?? .null
  }
  var body: some View {
    ScrollViewReader { scroll in
      ScrollView {
        VStack(alignment: .leading, spacing: 14) {
          Text(controller.text("步骤", "Steps")).font(.title2).accessibilityIdentifier("steps.title")
          Toggle(
            controller.text("显示全部细节", "Show all details"),
            isOn: Binding(get: { details }, set: { details = $0 }))
          if item.isNull {
            Text(controller.text("选择一个记录了推导步骤的结果。", "Select a result with recorded steps."))
              .foregroundStyle(.secondary)
          } else {
            Button(controller.text("AI 讲解全部", "Explain all")) { explain(nil) }
            ForEach(Array(item["steps"]["root"].array.enumerated()), id: \.offset) { _, step in
              node(step)
            }
          }
          if let job {
            if controller.aiStates[job] == "pending" {
              HStack {
                ProgressView()
                Button(controller.text("取消讲解", "Cancel explanation")) { controller.cancelAI(job) }
              }
            }
            NativeMarkdown(source: controller.aiText[job] ?? "") { id in
              highlight = id
              if reduceMotion {
                scroll.scrollTo(id, anchor: .center)
              } else {
                withAnimation { scroll.scrollTo(id, anchor: .center) }
              }
            }
          }
        }.padding().scrollTargetLayout()
      }.scrollPosition(
        id: Binding(get: { controller.stepScroll }, set: { controller.stepScroll = $0 }))
    }
    .alert(controller.text("将计算内容发送给 AI", "Send computation to AI"), isPresented: $consent) {
      Button(controller.text("发送", "Send")) {
        controller.allow("explain")
        start(chosenStep)
      }
      Button(controller.text("取消", "Cancel"), role: .cancel) {}
    } message: {
      Text(
        controller.destination("explain") + "\n"
          + controller.text(
            "只发送选中计算的输入、结果和记录步骤。", "Sends only the selected input, result and recorded steps."))
    }
    .onDisappear { if controller.inspector != "steps", let job { controller.cancelAI(job) } }
    .onChange(of: controller.stepCell) { _, _ in
      if let job { controller.cancelAI(job) }
      job = nil
    }
  }
  private func node(_ step: JSONValue) -> AnyView {
    let id = step["id"].string
    return AnyView(
      DisclosureGroup(
        isExpanded: Binding(
          get: {
            details || highlight == id || (expanded[id] ?? (step["level"].string != "Minor"))
          }, set: { expanded[id] = $0 })
      ) {
        VStack(alignment: .leading, spacing: 8) {
          ForEach(Array(step["before_latex"].array.enumerated()), id: \.offset) { _, latex in
            MathView(latex: latex.string, fontSize: 18)
          }
          if !step["after_latex"].array.isEmpty {
            Image(systemName: "arrow.down").foregroundStyle(.secondary)
          }
          ForEach(Array(step["after_latex"].array.enumerated()), id: \.offset) { _, latex in
            MathView(latex: latex.string, fontSize: 18)
          }
          ForEach(step["params"].object.keys.sorted(), id: \.self) { key in
            if !["before", "after"].contains(key) {
              VStack(alignment: .leading) {
                Text(key).font(.caption).foregroundStyle(.secondary)
                MathView(latex: step["params"][key].string, fontSize: 17)
              }
            }
          }
          Button(controller.text("为什么？", "Why?")) { explain(id) }.font(.caption)
          ForEach(Array(step["children"].array.enumerated()), id: \.offset) { _, child in
            node(child).padding(.leading, 10)
          }
        }
      } label: {
        HStack(alignment: .firstTextBaseline) {
          Text(id).font(.system(.caption, design: .monospaced)).foregroundStyle(
            Color.openMathAccent)
          Text(StepTitles.title(step["rule_id"].string, english: controller.isEnglish)).font(
            .subheadline)
        }
      }.id(id).padding(8).background(
        highlight == id ? Color.openMathAccent.opacity(0.12) : .clear,
        in: RoundedRectangle(cornerRadius: 8)))
  }
  private func explain(_ step: String?) {
    chosenStep = step
    if controller.authorized("explain") { start(step) } else { consent = true }
  }
  private func start(_ step: String?) {
    guard let cell = controller.stepCell else { return }
    if let job { controller.cancelAI(job) }
    Task {
      job = await controller.beginAI(
        feature: "explain",
        body: .object([
          "type": .string("llm_explain"), "cell_id": .string(cell),
          "step_id": step.map(JSONValue.string) ?? .null,
          "out_index": controller.stepIndex.map { .number(Double($0)) } ?? .null,
        ]), cell: cell)
    }
  }
}
enum StepTitles {
  static func title(_ rule: String, english: Bool) -> String {
    let name = "steps." + (english ? "en" : "zh-CN")
    guard let url = Bundle.main.url(forResource: name, withExtension: "json"),
      let data = try? Data(contentsOf: url),
      let templates = try? JSONDecoder().decode([String: String].self, from: data)
    else { return rule }
    let template = templates["step." + rule] ?? rule
    return template.replacingOccurrences(
      of: #"\{[a-z_]+\}"#, with: "…", options: .regularExpression)
  }
}
