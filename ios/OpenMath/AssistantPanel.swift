import SwiftUI
import UniformTypeIdentifiers

struct AssistantPanel: View {
  var controller: NotebookController
  @State private var snapshot = EditorSnapshot()
  @State private var consent = false
  @State private var importing = false
  @State private var showInput = false
  private var agent: MobileNotebookAgent { controller.agent }
  var body: some View {
    VStack(spacing: 0) {
      HStack {
        Text(controller.text("当前笔记本", "Current notebook")).font(.caption).foregroundStyle(.secondary)
        Spacer()
        Menu {
          ForEach(Array(agent.history().enumerated()), id: \.element) { index, name in
            Button(controller.text("历史对话 \(index + 1)", "Conversation \(index + 1)")) { agent.restoreConversation(name) }
          }
        } label: { Image(systemName: "clock.arrow.circlepath") }.disabled(agent.busy || agent.settling).accessibilityLabel(controller.text("历史对话", "Conversation history"))
        Button { agent.newConversation() } label: { Image(systemName: "square.and.pencil") }
          .disabled(agent.busy || agent.settling).accessibilityLabel(controller.text("新建对话", "New conversation"))
        if agent.fullInput != nil { Button { showInput = true } label: { Image(systemName: "doc.text.magnifyingglass") }.accessibilityLabel(controller.text("查看本轮完整输入", "Inspect model input")) }
      }.padding(.horizontal, 18).padding(.vertical, 12)
      ScrollView {
        LazyVStack(alignment: .leading, spacing: 22) {
          if agent.state.records.isEmpty {
            Text(controller.text("理解结果，或一起完成这份笔记。", "Understand a result, or work on this notebook together."))
              .font(.system(.title3, design: .serif)).foregroundStyle(.secondary).padding(.vertical, 20)
          }
          ForEach(Array(agent.state.records.enumerated()), id: \.offset) { _, record in
            let message = record["canonical"]
            if message["role"].string != "tool" {
              VStack(alignment: .leading, spacing: 8) {
                Text(message["role"].string == "user" ? controller.text("您", "You") : controller.text("助手", "Assistant"))
                  .font(.caption).foregroundStyle(message["role"].string == "user" ? Color.secondary : .indigo)
                ForEach(Array(message["parts"].array.enumerated()), id: \.offset) { _, part in
                  if part["kind"].string == "text" { NativeMarkdown(source: part["text"].string) }
                  if part["kind"].string == "tool_call" {
                    DisclosureGroup(part["name"].string) { Text(part["arguments_json"].string).font(.system(.caption, design: .monospaced)).textSelection(.enabled) }
                      .font(.caption).foregroundStyle(.secondary)
                  }
                }
              }.frame(maxWidth: .infinity, alignment: .leading)
            }
          }
          if !agent.streaming.isEmpty { NativeMarkdown(source: agent.streaming) }
          if !agent.state.operations.isEmpty {
            DisclosureGroup(controller.text("操作记录", "Operations")) {
              ForEach(agent.state.operations.keys.sorted(), id: \.self) { key in
                let operation = agent.state.operations[key] ?? .null
                DisclosureGroup(operation["name"].string + " · " + operation["status"].string) {
                  Text(operation["ok"].bool ? operation["data"].pretty : operation["error"]["message"].string)
                    .font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                  if let cell = operation["data"]["changed_cells"].array.first?.string {
                    Button(controller.text("定位单元格", "Reveal cell")) { controller.selected = cell; controller.scrollTarget = cell }
                  }
                }.font(.caption)
              }
              ForEach(agent.state.transactions.filter { $0.state == "applied" }.reversed()) { transaction in
                Button(controller.text("撤销修改", "Undo changes"), systemImage: "arrow.uturn.backward") {
                  Task { await agent.undo(transaction.id) }
                }.disabled(agent.busy || agent.settling)
              }
            }.font(.caption).tint(.indigo)
          }
          if !agent.outcome.isEmpty { Text(agent.outcome).font(.caption).foregroundStyle(.secondary) }
          if let error = agent.error { Label(error, systemImage: "exclamationmark.triangle").font(.caption).foregroundStyle(.red).textSelection(.enabled) }
        }.padding(18).frame(maxWidth: .infinity, alignment: .leading)
      }.defaultScrollAnchor(.bottom, for: .sizeChanges)
      composer
    }
    .fileImporter(isPresented: $importing, allowedContentTypes: [.item], allowsMultipleSelection: true) { result in
      if case .success(let urls) = result { Task { for url in urls { await agent.importFile(url) } } }
    }
    .alert(controller.text("发送给配置的模型服务", "Send to your model service"), isPresented: $consent) {
      Button(controller.text("发送", "Send")) { agent.allowDestination(); Task { await agent.send() } }
      Button(controller.text("取消", "Cancel"), role: .cancel) {}
    } message: { Text(agent.destination + "\n" + controller.text("发送对话、附件及工具读取的当前文稿内容。执行模式允许预览后修改与运行，可撤销。", "Sends this conversation, attachments and notebook context read by tools. Execute mode can preview, modify and run; changes can be undone.")) }
    .sheet(isPresented: $showInput) {
      NavigationStack {
        ScrollView { Text(agent.fullInput ?? "").font(.system(.caption, design: .monospaced)).textSelection(.enabled).padding() }
          .navigationTitle(controller.text("本轮完整输入", "Model input"))
          .toolbar { ToolbarItem(placement: .confirmationAction) { Button(controller.text("关闭", "Close")) { showInput = false } } }
      }
    }
  }
  private var composer: some View {
    VStack(alignment: .leading, spacing: 8) {
      if !agent.state.attachments.isEmpty {
        ScrollView(.horizontal) {
          HStack { ForEach(agent.state.attachments) { attachment in
            HStack(spacing: 6) { Image(systemName: attachment.kind == "image" ? "photo" : "doc"); Text(attachment.name).lineLimit(1); Button { agent.removeAttachment(attachment.id) } label: { Image(systemName: "xmark") }.disabled(agent.busy || agent.settling) }
              .font(.caption).padding(8).background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
          } }
        }
      }
      NativeTextEditor(text: Binding(get: { agent.draft }, set: { agent.draft = $0 }), snapshot: $snapshot,
        accessibilityID: "assistant.input", onRun: { _ in if !snapshot.composing { send() } },
        onTab: { false }, onEscape: {}, onComplete: {}, onPartial: {})
        .frame(minHeight: 60, maxHeight: 140).accessibilityLabel(controller.text("提出问题，或描述文稿任务", "Ask a question or describe a notebook task"))
      HStack(spacing: 12) {
        Button { importing = true } label: { Image(systemName: "plus") }.accessibilityLabel(controller.text("添加附件", "Add attachment")).disabled(agent.busy || agent.settling)
        Menu {
          Button(controller.text("讨论", "Discuss")) { agent.execute = false }
          Button(controller.text("执行", "Execute")) { agent.execute = true }
        } label: { Label(controller.text(agent.execute ? "执行" : "讨论", agent.execute ? "Execute" : "Discuss"), systemImage: "chevron.down").labelStyle(.titleOnly) }.disabled(agent.busy || agent.settling)
        Menu {
          ForEach(agent.profiles.map { $0["name"].string }, id: \.self) { name in Button(name) { agent.selectedProfile = name } }
          Toggle(controller.text("此模型支持图片", "Model supports images"), isOn: Binding(get: { agent.imageInput }, set: { agent.imageInput = $0 }))
        } label: { Text(agent.selectedProfile.isEmpty ? controller.text("选择模型", "Choose model") : agent.selectedProfile).lineLimit(1) }.disabled(agent.busy || agent.settling)
        Spacer(minLength: 0)
        Menu {
          Text(controller.text("本轮输入 \(agent.fullInput?.count ?? 0) 字符", "Current input: \(agent.fullInput?.count ?? 0) characters"))
          Text(agent.usage["input"].isNull ? controller.text("供应商未报告 token 用量", "Provider token usage not reported") : "\(Int(agent.usage["input"].double)) input · \(Int(agent.usage["output"].double)) output")
        } label: { Image(systemName: "info.circle") }.accessibilityLabel(controller.text("上下文与用量", "Context and usage"))
        Button { if agent.busy { agent.stop(); controller.interrupt() } else { send() } } label: {
          Image(systemName: agent.busy ? "stop.fill" : "arrow.up").frame(width: 38, height: 38)
        }.buttonStyle(.plain).foregroundStyle(Color.openMathAccent)
          .glassEffect(.regular.tint(Color.openMathAccent.opacity(0.12)).interactive(), in: .circle)
          .disabled(!agent.busy && (agent.settling || snapshot.composing || agent.profile.isNull || agent.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty))
          .accessibilityLabel(controller.text(agent.busy ? "停止" : "发送", agent.busy ? "Stop" : "Send"))
      }.font(.caption).buttonStyle(.plain)
    }.padding(12).background(Color(.systemBackground), in: RoundedRectangle(cornerRadius: 18))
      .overlay { RoundedRectangle(cornerRadius: 18).strokeBorder(Color(uiColor: .separator).opacity(0.5)) }.padding(12)
  }
  private func send() { if agent.authorized { Task { await agent.send() } } else { consent = true } }
}

struct SuggestionCard: View {
  var id: String
  var cell: String? = nil
  var controller: NotebookController
  var body: some View {
    let suggestion = controller.suggestions[id] ?? .null
    VStack(alignment: .leading, spacing: 8) {
      Label(controller.text("AI 建议 · 尚未运行", "AI suggestion · not run"), systemImage: "sparkles").font(.caption)
      MathView(latex: suggestion["latex"].string)
      Text(suggestion["modern"].string).font(.system(.body, design: .monospaced)).textSelection(.enabled)
      NativeMarkdown(source: suggestion["explanation"].string)
      HStack {
        Button(controller.text("插入为代码", "Insert code")) { controller.insertSuggestion(id, replacing: cell) }
        Button(controller.text("运行", "Run")) { controller.insertSuggestion(id, replacing: cell, run: true) }
      }.buttonStyle(.bordered)
    }.padding(12).background(.quaternary, in: RoundedRectangle(cornerRadius: 12))
  }
}
