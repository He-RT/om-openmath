import SwiftUI

struct AssistantPanel: View {
  var controller: NotebookController
  private var input: String {
    get { controller.assistantDraft }
    nonmutating set { controller.assistantDraft = newValue }
  }
  @State private var snapshot = EditorSnapshot()
  private var job: String? {
    get { controller.chatJob }
    nonmutating set { controller.chatJob = newValue }
  }
  @State private var consent = false
  @State private var submitted = ""
  var body: some View {
    VStack {
      ScrollView {
        VStack(alignment: .leading, spacing: 12) {
          ForEach(Array(controller.chatTurns.enumerated()), id: \.offset) { _, turn in
            VStack(alignment: .leading) {
              Text(turn.role == "user" ? controller.text("您", "You") : "AI").font(.caption)
                .foregroundStyle(.secondary)
              NativeMarkdown(source: turn.content)
            }.padding(10).background(.quaternary, in: RoundedRectangle(cornerRadius: 12))
          }
          if let job {
            NativeMarkdown(source: controller.aiText[job] ?? "")
            ForEach(Array((controller.tools[job] ?? []).enumerated()), id: \.offset) { _, tool in
              DisclosureGroup("CAS · " + tool["name"].string) {
                Text(tool["result_summary"].string).font(.system(.caption, design: .monospaced))
                  .textSelection(.enabled)
              }
            }
            if controller.suggestions[job] != nil {
              SuggestionCard(id: job, controller: controller)
            }
            if controller.aiStates[job] == "pending" {
              HStack {
                ProgressView()
                Button(controller.text("取消", "Cancel")) { controller.cancelAI(job) }
              }
            }
          }
        }.padding()
      }
      Divider()
      HStack(alignment: .bottom) {
        NativeTextEditor(
          text: Binding(get: { input }, set: { input = $0 }), snapshot: $snapshot,
          accessibilityID: "assistant.input",
          onRun: { _ in
            if !snapshot.composing {
              submitted = input
              if controller.authorized("chat") { send() } else { consent = true }
            }
          }, onTab: { false }, onEscape: {}, onComplete: {}, onPartial: {}
        ).accessibilityLabel(controller.text("提出问题，或引用 @cell1", "Ask, or reference @cell1"))
        Button {
          submitted = input
          if controller.authorized("chat") { send() } else { consent = true }
        } label: {
          Image(systemName: "arrow.up.circle.fill").font(.title2)
        }.disabled(
          snapshot.composing || input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
      }.padding()
    }
    .alert(controller.text("将对话发送给 AI", "Send conversation to AI"), isPresented: $consent) {
      Button(controller.text("发送", "Send")) {
        controller.allow("chat")
        send()
      }
      Button(controller.text("取消", "Cancel"), role: .cancel) {}
    } message: {
      Text(
        controller.destination("chat") + "\n"
          + controller.text(
            "发送对话和明确引用单元格的源码及当前结果；工具只读 CAS。",
            "Sends the conversation and explicitly referenced source/current results; tools read the CAS."
          ))
    }

  }
  private func send() {
    if let job, controller.aiStates[job] == "pending" { controller.cancelAI(job) }
    if let job, controller.aiStates[job] == "done", let reply = controller.aiText[job] {
      controller.chatTurns.append(("assistant", reply))
    }
    var text = submitted
    let regex = try? NSRegularExpression(pattern: "@cell([0-9]+)")
    let ns = text as NSString
    for match in (regex?.matches(in: text, range: NSRange(location: 0, length: ns.length)) ?? [])
      .reversed()
    {
      let index = (Int(ns.substring(with: match.range(at: 1))) ?? 0) - 1
      guard controller.cells.indices.contains(index) else {
        controller.error = controller.text("找不到引用的单元格", "Unknown cell reference")
        return
      }
      let cell = controller.cells[index]
      let result =
        cell.status == .Done
        ? cell.output["items"].array.map { $0["input_form"].string }.joined(separator: "\n") : ""
      text = (text as NSString).replacingCharacters(
        in: match.range,
        with: "@cell\(index+1) source:\n\(cell.input.source)\ncurrent result:\n\(result)")
    }
    controller.chatTurns.append(("user", text))
    input = ""
    let messages = controller.chatTurns.map {
      JSONValue.object([
        "role": .string($0.role), "content": .string($0.content), "tool_calls": .array([]),
        "tool_call_id": .null,
      ])
    }
    Task {
      job = await controller.beginAI(
        feature: "chat", body: .object(["type": .string("llm_chat"), "messages": .array(messages)]))
    }
  }
}
struct SuggestionCard: View {
  var id: String
  var cell: String? = nil
  var controller: NotebookController
  var body: some View {
    let suggestion = controller.suggestions[id] ?? .null
    VStack(alignment: .leading, spacing: 8) {
      Label(controller.text("AI 建议 · 尚未运行", "AI suggestion · not run"), systemImage: "sparkles")
        .font(.caption).foregroundStyle(.secondary)
      MathView(latex: suggestion["latex"].string)
      Text(suggestion["modern"].string).font(.system(.body, design: .monospaced)).textSelection(
        .enabled)
      NativeMarkdown(source: suggestion["explanation"].string)
      HStack {
        Button(controller.text("插入为代码", "Insert code")) {
          controller.insertSuggestion(id, replacing: cell)
        }
        Button(controller.text("运行", "Run")) {
          controller.insertSuggestion(id, replacing: cell, run: true)
        }
        Button(controller.text("编辑", "Edit")) { controller.insertSuggestion(id, replacing: cell) }
      }.buttonStyle(.bordered)
    }.padding(12).background(.quaternary, in: RoundedRectangle(cornerRadius: 12))
  }
}
