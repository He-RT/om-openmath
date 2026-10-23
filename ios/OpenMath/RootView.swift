import SwiftUI

/// One notebook view retains its identity while the available window changes.
struct RootView: View {
  @Bindable var controller: NotebookController
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @State private var wide = false
  @State private var outline = false
  @State private var expandedOutline = false
  var body: some View {
    NavigationStack {
      GeometryReader { geometry in
        let layout = MobileWorkbenchLayout(width: geometry.size.width)
        let sidebar: CGFloat = layout.outline && outline ? 204 : 0
        let inspector: CGFloat = layout.inspector && controller.showInspector ? 340 : 0
        ZStack(alignment: .topLeading) {
          Color(.systemBackground)
          NotebookView(controller: controller, bottomClearance: layout.inspector ? 0 : 68)
            .frame(width: max(0, geometry.size.width - sidebar - inspector))
            .offset(x: sidebar)
          if sidebar > 0 {
            NotebookOutline(controller: controller) { outline = false }
              .frame(width: sidebar, height: geometry.size.height).background(.regularMaterial)
          }
          if inspector > 0 {
            InspectorView(controller: controller)
              .frame(width: inspector, height: geometry.size.height).background(.regularMaterial)
              .offset(x: geometry.size.width - inspector)
          }
        }
        .onChange(of: layout.inspector, initial: true) { _, value in wide = value }
        .onChange(of: layout.outline, initial: true) { _, value in expandedOutline = value }
      }
      .navigationTitle(controller.title)
      .navigationBarTitleDisplayMode(.inline)
      .toolbarBackground(.hidden, for: .navigationBar)
      .disabled(controller.agentCommitting)
      .toolbar {
        ToolbarItem(placement: .topBarLeading) {
          Menu {
            Button(controller.text("新建笔记本", "New notebook"), systemImage: "doc.badge.plus") {
              Task { await controller.newNotebook() }
            }.keyboardShortcut("n").accessibilityIdentifier("file.new")
            Button(controller.text("打开…", "Open…"), systemImage: "folder") {
              controller.showOpen = true
            }.keyboardShortcut("o").accessibilityIdentifier("file.open")
            Button(controller.text("保存", "Save"), systemImage: "square.and.arrow.down") {
              Task { await controller.save() }
            }.keyboardShortcut("s").accessibilityIdentifier("file.save")
            Button(controller.text("另存为…", "Save as…"), systemImage: "doc.on.doc") {
              Task {
                await controller.save()
                if !controller.dirty { controller.showSaveAs = true }
              }
            }
            Button(controller.text("分享…", "Share…"), systemImage: "square.and.arrow.up") {
              Task {
                await controller.save()
                if !controller.dirty { controller.shareURL = controller.fileURL }
              }
            }
            Button(controller.text("导出 Markdown", "Export Markdown")) {
              Task { await controller.export("markdown") }
            }
            Button(controller.text("导出 LaTeX", "Export LaTeX")) {
              Task { await controller.export("latex") }
            }
            Divider()
            Button(controller.text("设置", "Settings"), systemImage: "gearshape") {
              controller.showSettings = true
            }.keyboardShortcut(",").accessibilityIdentifier("settings.open")
          } label: {
            Image(systemName: "line.3.horizontal")
          }.accessibilityLabel(controller.text("文件和设置", "Files and settings"))
            .accessibilityIdentifier("file.menu")

        }
        ToolbarItemGroup(placement: .topBarTrailing) {
          if wide {
            Button { outline.toggle() } label: { Image(systemName: "sidebar.left") }.accessibilityLabel(controller.text("文稿目录", "Outline"))
            Menu { ForEach(CellKind.allCases, id: \.self) { kind in Button(kind.rawValue) { controller.add(kind) } } } label: { Image(systemName: "plus") }
            Button { controller.showInspector.toggle() } label: { Image(systemName: "sidebar.right") }.accessibilityLabel(controller.text("检查面板", "Inspector")).accessibilityIdentifier("inspector.toggle")
          }
          Button { controller.showCommands = true } label: { Image(systemName: "command") }
            .keyboardShortcut("k").accessibilityLabel(controller.text("命令", "Commands"))
          Button {
            if controller.busy { controller.interrupt() }
            else { Task { await controller.runAll() } }
          } label: { Image(systemName: controller.busy ? "stop.fill" : "play.fill") }
            .disabled(!controller.composingCells.isEmpty)
            .accessibilityLabel(controller.busy ? controller.text("中断", "Interrupt") : controller.text("运行全部", "Run all"))
        }
      }
      .safeAreaInset(edge: .bottom, spacing: 0) {
        if !wide {
        GlassEffectContainer(spacing: 14) {
          HStack(spacing: 14) {
            HStack(spacing: 8) {
              Button { outline.toggle() } label: { Image(systemName: "list.bullet").frame(width: 44, height: 48) }
                .accessibilityLabel(controller.text("文稿目录", "Outline"))
              Menu {
                ForEach(CellKind.allCases, id: \.self) { kind in
                  Button(kind == .Math ? controller.text("数学", "Math") : kind == .Text ? controller.text("文字", "Text") : controller.text("提问", "Ask")) { controller.add(kind) }
                }
              } label: { HStack(spacing: 7) { Image(systemName: "plus"); if !controller.notebookScrolled { Text(controller.text("单元格", "Cell")).transition(.opacity) } }.padding(.horizontal, 14).frame(height: 48) }
                .accessibilityIdentifier("cell.add")
            }.padding(.horizontal, 6).glassEffect(.regular.interactive(), in: .capsule)
            Button { controller.showInspector.toggle() } label: {
              Image(systemName: "sidebar.right").frame(width: 50, height: 50)
            }.glassEffect(.regular.tint(Color.openMathAccent.opacity(0.12)).interactive(), in: .circle)
              .accessibilityLabel(controller.text("检查面板", "Inspector")).accessibilityIdentifier("inspector.toggle")
          }.font(.system(size: 18)).buttonStyle(.plain)
        }.padding(.top, 8).padding(.bottom, 10)
          .animation(reduceMotion ? nil : .easeInOut(duration: 0.2), value: controller.notebookScrolled)
        }
      }
      .sheet(isPresented: Binding(get: { outline && !expandedOutline }, set: { if !expandedOutline { outline = $0 } })) {
        NavigationStack {
          NotebookOutline(controller: controller) { outline = false }
            .navigationTitle(controller.text("文稿目录", "Outline"))
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button(controller.text("关闭", "Close")) { outline = false } } }
        }.presentationDetents([.medium, .large]).presentationDragIndicator(.visible)
      }
      .sheet(isPresented: $controller.showOpen) {
        DocumentPicker { url in
          controller.showOpen = false
          Task { await controller.load(url) }
        }
      }
      .sheet(isPresented: $controller.showSaveAs) {
        if let url = controller.fileURL {
          SaveAsPicker(url: url) { selected in
            controller.showSaveAs = false
            Task { await controller.load(selected) }
          }
        }
      }
      .sheet(isPresented: $controller.showSettings) { SettingsView(controller: controller) }
      .sheet(isPresented: $controller.showCommands) { CommandPanel(controller: controller) }
      .sheet(
        isPresented: Binding(
          get: { controller.showInspector && !wide }, set: { if !wide { controller.showInspector = $0 } })
      ) {
        NavigationStack {
          InspectorView(controller: controller)
            .navigationTitle(controller.text("检查面板", "Inspector"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
              ToolbarItem(placement: .confirmationAction) {
                Button(controller.text("关闭", "Close")) { controller.showInspector = false }
                  .accessibilityIdentifier("inspector.close")
              }
            }
        }.presentationDetents([.medium, .large]).presentationDragIndicator(.visible)
      }
      .sheet(
        isPresented: Binding(
          get: { controller.shareURL != nil }, set: { if !$0 { controller.shareURL = nil } })
      ) { if let url = controller.shareURL { ShareSheet(urls: [url]) } }
      .alert(
        controller.text("操作失败", "Operation failed"),
        isPresented: Binding(
          get: { controller.error != nil }, set: { if !$0 { controller.error = nil } })
      ) {
        Button(controller.text("关闭", "Close")) { controller.error = nil }
      } message: {
        Text(controller.error ?? "")
      }

      .overlay {
        if controller.loading { ProgressView(controller.text("加载内核…", "Loading kernel…")) }
      }
    }
  }
}

struct NotebookView: View {
  @Bindable var controller: NotebookController
  var bottomClearance: CGFloat = 0
  var body: some View {
    ScrollView {
      LazyVStack(alignment: .leading, spacing: 28) {
        TextField(
          controller.text("笔记本标题", "Notebook title"),
          text: Binding(get: { controller.title }, set: controller.rename)
        ).font(.system(.largeTitle, design: .serif, weight: .semibold)).accessibilityIdentifier("notebook.title")
        HStack(spacing: 7) {
          Circle().fill(controller.dirty ? Color.secondary : .openMathAccent).frame(width: 5, height: 5)
          Text(controller.dirty ? controller.text("未保存", "Unsaved") : controller.notice ?? controller.text(controller.fileURL == nil ? "新笔记本" : "本地已保存", controller.fileURL == nil ? "New notebook" : "Saved locally"))
        }.font(.caption).foregroundStyle(.secondary)
        if controller.cells.isEmpty {
          VStack(alignment: .leading, spacing: 16) {
            Text(controller.text("从一个问题开始", "Start with a question")).font(.largeTitle.bold())
            Text(
              controller.text(
                "精确求解，查看推导，探索图形。计算在设备上完成。",
                "Solve exactly, inspect steps and explore plots. Computation runs on your device.")
            ).foregroundStyle(.secondary)
            Button(controller.text("一元二次方程", "Quadratic equation")) {
              controller.example("Solve(x^2+2*x-3==0,x)")
            }.buttonStyle(.borderedProminent).accessibilityIdentifier("example.quadratic")
            Button(controller.text("响应式示例 a = 2 → 5", "Reactive example a = 2 → 5")) {
              controller.add(source: "let a=2")
              controller.add(source: "a+1")
              Task { await controller.runAll() }
            }.accessibilityIdentifier("example.reactive")
            Button(controller.text("数据表与精度", "Data table and precision")) {
              controller.example("parse_csv(\"x,y\\n1,中文\\n2,emoji🙂\")")
            }.accessibilityIdentifier("example.table")
            Button(controller.text("二维数据示例", "2D data example")) {
              controller.example("data_plot([[0,0],[1,2],[2,4]])")
            }.accessibilityIdentifier("example.plot")
            Button(controller.text("三维场景", "3D scene")) {
              controller.example("scene([sphere([0,0,0],1)],dimensions:3)")
            }.accessibilityIdentifier("example.scene")
            Button(controller.text("参数探索示例", "Parameter exploration example")) {
              controller.example("explore(a^2,controls:{a:0..4},initial:{a:2})")
            }.accessibilityIdentifier("example.explore")
            Button(controller.text("新建数学单元格", "New math cell")) { controller.add() }
          }.padding(.vertical, 36)
        }
        ForEach(controller.cells) { cell in
          NotebookCellView(cell: cell, controller: controller).id(cell.id)
        }
      }.padding(.horizontal, 24).padding(.vertical, 28).padding(.bottom, bottomClearance)
        .frame(maxWidth: 800).frame(maxWidth: .infinity).scrollTargetLayout()
    }
    .scrollPosition(id: $controller.scrollTarget, anchor: .top)
    .onScrollGeometryChange(for: Bool.self) { $0.contentOffset.y > 30 } action: { _, value in controller.notebookScrolled = value }
    .scrollDismissesKeyboard(.interactively)
    .background(Color(.systemBackground))
  }
}

struct NotebookCellView: View {
  var cell: NotebookCell
  var controller: NotebookController
  @State private var editingText = false
  @State private var textSnapshot = EditorSnapshot()
  @State private var aiJob: String?
  @State private var consent = false
  private var kindLabel: String {
    cell.input.kind == .Math
      ? controller.text("数学", "Math")
      : cell.input.kind == .Text ? controller.text("文本", "Text") : controller.text("提问", "Ask")
  }
  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack {
        Label(
          kindLabel,
          systemImage: cell.input.kind == .Math
            ? "function" : cell.input.kind == .Text ? "text.alignleft" : "sparkles"
        ).font(.caption).foregroundStyle(.secondary)
        if cell.status == .Running { ProgressView() }
        Spacer()
        if cell.input.kind == .Math {
          Menu(cell.input.dialect.rawValue) {
            ForEach(Dialect.allCases, id: \.self) { dialect in
              Button(dialect.rawValue) { controller.update(cell.id, dialect: dialect) }
            }
          }.font(.caption)
        }
        Menu {
          ForEach(CellKind.allCases, id: \.self) { kind in
            Button(kind.rawValue) { controller.update(cell.id, kind: kind) }
          }
          Button(controller.text("上移", "Move up")) { controller.move(cell.id, delta: -1) }
          Button(controller.text("下移", "Move down")) { controller.move(cell.id, delta: 1) }
          Button(controller.text("在下方添加", "Add below")) { controller.add(after: cell.id) }
          Button(controller.text("删除", "Delete"), role: .destructive) { controller.remove(cell.id) }
        } label: {
          Image(systemName: "ellipsis")
        }.accessibilityLabel(controller.text("单元格操作", "Cell actions"))
      }
      if cell.input.kind == .Math {
        MathCellEditor(cell: cell, controller: controller)
      } else if cell.input.kind == .Text && !editingText {
        NativeMarkdown(source: cell.input.source).onTapGesture(count: 2) { editingText = true }
      } else {
        NativeTextEditor(
          text: Binding(
            get: { controller.cells.first { $0.id == cell.id }?.input.source ?? "" },
            set: {
              controller.setComposing(cell.id, textSnapshot.composing)
              controller.edit(cell.id, source: $0)
            }), snapshot: $textSnapshot, accessibilityID: "editor.\(cell.id)",
          onRun: { _ in
            if cell.input.kind == .Ask && !textSnapshot.composing {
              if controller.authorized("translate") { ask() } else { consent = true }
            }
          },
          onTab: { false }, onEscape: {}, onComplete: {}, onPartial: {}
        )
        .onChange(of: textSnapshot.composing) { _, composing in
          controller.setComposing(cell.id, composing)
        }
        .onDisappear { controller.setComposing(cell.id, false) }
      }
      HStack {
        if cell.input.kind == .Text {
          Button(controller.text(editingText ? "预览" : "编辑", editingText ? "Preview" : "Edit")) {
            editingText.toggle()
          }
        } else if cell.input.kind == .Ask {
          Button(controller.text("提问", "Ask")) {
            if controller.authorized("translate") { ask() } else { consent = true }
          }.disabled(textSnapshot.composing)
        } else {
          Button(controller.text("运行", "Run"), systemImage: "play.fill") {
            Task { await controller.run(cell.id) }
          }.disabled(controller.busy || !controller.composingCells.isEmpty).accessibilityIdentifier(
            "run.\(cell.id)")
          if controller.busy {
            Button(controller.text("中断", "Interrupt")) { controller.interrupt() }
          }
          if cell.status == .Error {
            Button(controller.text("AI 修复", "AI fix")) {
              if controller.authorized("fix") { ask() } else { consent = true }
            }
          }
        }
        Spacer()
        Text(
          controller.text(
            [CellStatus.Done: "完成", .Stale: "待重算", .Running: "运行中", .Queued: "排队", .Error: "错误"][
              cell.status] ?? "", cell.status.rawValue)
        ).font(.caption2).foregroundStyle(.secondary)
      }.font(.caption).buttonStyle(.borderless)
      CellOutputView(cell: cell, controller: controller)
      if let aiJob {
        NativeMarkdown(source: controller.aiText[aiJob] ?? "")
        if controller.suggestions[aiJob] != nil {
          SuggestionCard(
            id: aiJob, cell: cell.input.kind == .Math ? cell.id : nil, controller: controller)
        }
        if controller.aiStates[aiJob] == "pending" {
          Button(controller.text("取消", "Cancel")) { controller.cancelAI(aiJob) }
        }
      }
    }.padding(.leading, 16).padding(.vertical, 8)
    .overlay(alignment: .leading) {
      Rectangle().fill(railColor).frame(width: 3)
        .accessibilityHidden(true)
    }
    .simultaneousGesture(TapGesture().onEnded { controller.selected = cell.id })
    .alert(controller.text("将此单元格发送给 AI", "Send cell to AI"), isPresented: $consent) {
      Button(controller.text("发送", "Send")) {
        controller.allow(cell.input.kind == .Ask ? "translate" : "fix")
        ask()
      }
      Button(controller.text("取消", "Cancel"), role: .cancel) {}
    } message: {
      Text(controller.destination(cell.input.kind == .Ask ? "translate" : "fix"))
    }
  }
  private var railColor: Color {
    switch cell.status {
    case .Error: .red
    case .Stale, .Queued: .secondary.opacity(0.45)
    case .Running: .orange
    case .Done: .openMathAccent
    }
  }
  private func ask() {
    if let aiJob { controller.cancelAI(aiJob) }
    Task {
      aiJob = await controller.beginAI(
        feature: cell.input.kind == .Ask ? "translate" : "fix",
        body: cell.input.kind == .Ask
          ? .object(["type": .string("llm_translate"), "text": .string(cell.input.source)])
          : .object(["type": .string("llm_fix_error"), "cell_id": .string(cell.id)]), cell: cell.id)
    }
  }
}

struct InspectorView: View {
  @Bindable var controller: NotebookController
  var body: some View {
    VStack(spacing: 0) {
      Picker(controller.text("检查面板", "Inspector"), selection: $controller.inspector) {
        Text(controller.text("变量", "Variables")).tag("variables")
        Text(controller.text("步骤", "Steps")).tag("steps")
        Text(controller.text("助手", "Assistant")).tag("ai")
      }.pickerStyle(.segmented).padding()
      if controller.inspector == "steps" {
        StepsPanel(controller: controller)
      } else if controller.inspector == "ai" {
        AssistantPanel(controller: controller)
      } else if controller.inspector == "variables" {
        ScrollView {
          VStack(alignment: .leading, spacing: 12) {
            ForEach(Array(controller.variableRows.enumerated()), id: \.offset) { _, row in
              VStack(alignment: .leading) {
                Text(row[0].string).font(.system(.headline, design: .monospaced))
                Text(row[1]["value"].string).font(.system(.body, design: .monospaced))
                  .textSelection(.enabled)
                Text(row[1]["summary"].string).foregroundStyle(.secondary)
              }
            }
          }.padding()
        }.task { await controller.refreshVariables() }
      } else {
        ScrollView {
          NativeMarkdown(
            source: controller.text(
              "## 数学笔记本\n使用现代语法或 Wolfram 语法。\n\n```\nSolve(x^2+2*x-3==0,x)\nFactor(x^2-1)\nPlot(sin(x),{x,-6,6})\n```\n\n⌘/⇧ Return：运行；Control Space：补全；Tab：接受；Escape：取消。\n\n结果与步骤来自计算内核。AI 建议需明确插入或运行。",
              "## Math notebook\nUse modern or Wolfram syntax.\n\n```\nSolve(x^2+2*x-3==0,x)\nFactor(x^2-1)\nPlot(sin(x),{x,-6,6})\n```\n\n⌘/⇧ Return: run; Control Space: complete; Tab: accept; Escape: cancel.\n\nResults and steps come from the kernel. Explicitly insert or run AI suggestions."
            )
          ).padding()
        }
      }
    }
  }
}
struct CommandPanel: View {
  var controller: NotebookController
  @Environment(\.dismiss) private var dismiss
  @State private var search = ""
  var body: some View {
    NavigationStack {
      List {
        Button(controller.text("运行全部", "Run all")) {
          dismiss()
          Task { await controller.runAll() }
        }
        Button(controller.text("中断", "Interrupt")) {
          controller.interrupt()
          dismiss()
        }
        Button(controller.text("保存", "Save")) {
          dismiss()
          Task { await controller.save() }
        }
        Button(controller.text("添加数学单元格", "Add math cell")) {
          controller.add()
          dismiss()
        }
        ForEach(
          ["Solve", "Reduce", "NSolve", "Factor", "Expand", "Simplify", "Plot", "ContourPlot"]
            .filter { search.isEmpty || $0.localizedCaseInsensitiveContains(search) }, id: \.self
        ) { command in
          Button(command) {
            controller.add(source: "\(command)()")
            dismiss()
          }
        }
      }.searchable(text: $search).navigationTitle(controller.text("命令", "Commands"))
        .toolbar {
          ToolbarItem(placement: .cancellationAction) {
            Button(controller.text("关闭", "Close")) { dismiss() }
          }
        }
    }
  }
}

/// Size classes and device names never decide the notebook's identity.
struct MobileWorkbenchLayout: Equatable {
  let outline: Bool
  let inspector: Bool
  init(width: CGFloat) { outline = width >= 1120; inspector = width >= 820 }
}
struct NotebookOutline: View {
  var controller: NotebookController
  var close: () -> Void
  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 12) {
        Text(controller.text("目录", "Outline")).font(.headline).padding(.bottom, 12)
        ForEach(Array(controller.cells.enumerated()), id: \.element.id) { index, cell in
          Button {
            controller.selected = cell.id
            controller.scrollTarget = cell.id
            close()
          } label: {
            HStack(alignment: .top, spacing: 9) {
              Circle().fill(cell.status == .Done ? Color.openMathAccent : .secondary.opacity(0.5)).frame(width: 5, height: 5).padding(.top, 6)
              VStack(alignment: .leading, spacing: 4) {
                Text("\(index + 1) · " + (cell.input.kind == .Text ? controller.text("文字", "Text") : controller.text("数学", "Math"))).font(.caption).foregroundStyle(.secondary)
                Text(cell.input.source.isEmpty ? controller.text("空单元格", "Empty cell") : cell.input.source).font(.system(.caption, design: .monospaced)).lineLimit(2)
              }
            }.frame(maxWidth: .infinity, alignment: .leading).padding(8)
          }.buttonStyle(.plain)
        }
      }.padding(18)
    }
  }
}
