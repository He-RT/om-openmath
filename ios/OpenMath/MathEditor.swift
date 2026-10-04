import SwiftUI
import UIKit

struct EditorSnapshot: Equatable {
  var selection = NSRange(location: 0, length: 0)
  var composing = false
}
enum EditorOffsets {
  static func byte(_ text: String, utf16: Int) -> Int? {
    let utf16View = text.utf16
    guard utf16 >= 0, utf16 <= utf16View.count,
      let index = String.Index(utf16View.index(utf16View.startIndex, offsetBy: utf16), within: text)
    else { return nil }
    return text[..<index].utf8.count
  }
  static func range(_ text: String, bytes: NSRange) -> NSRange? {
    let utf8 = text.utf8
    guard bytes.location >= 0, NSMaxRange(bytes) <= utf8.count,
      let start = String.Index(utf8.index(utf8.startIndex, offsetBy: bytes.location), within: text),
      let end = String.Index(utf8.index(utf8.startIndex, offsetBy: NSMaxRange(bytes)), within: text)
    else { return nil }
    return NSRange(start..<end, in: text)
  }
}
struct NativeTextEditor: UIViewRepresentable {
  @Binding var text: String
  @Binding var snapshot: EditorSnapshot
  var accessibilityID: String
  var tokens: JSONValue = .null
  var focus = false
  var onFocused: () -> Void = {}
  var onRun: (Bool) -> Void
  var onTab: () -> Bool
  var onEscape: () -> Void
  var onComplete: () -> Void
  var onPartial: () -> Void
  func makeUIView(context: Context) -> EditorTextView {
    let view = EditorTextView(usingTextLayoutManager: true)
    view.delegate = context.coordinator
    view.font = .preferredFont(forTextStyle: .body).withDesign(.monospaced)
    view.adjustsFontForContentSizeCategory = true
    view.autocorrectionType = .no
    view.autocapitalizationType = .none
    view.smartQuotesType = .no
    view.smartDashesType = .no
    view.backgroundColor = .clear
    view.textContainerInset = UIEdgeInsets(top: 10, left: 6, bottom: 10, right: 6)
    view.accessibilityLabel = "数学源码 / Math source"
    view.keyboardDismissMode = .interactive
    view.accessibilityIdentifier = accessibilityID
    view.onRun = onRun
    view.onTab = onTab
    view.onEscape = onEscape
    view.onComplete = onComplete
    view.onPartial = onPartial
    return view
  }
  func updateUIView(_ view: EditorTextView, context: Context) {
    context.coordinator.parent = self
    context.coordinator.updating = true
    defer { context.coordinator.updating = false }
    if view.text != text && view.markedTextRange == nil {
      if let range = view.textRange(from: view.beginningOfDocument, to: view.endOfDocument) {
        view.replace(range, withText: text)
      }
      view.selectedRange = NSRange(
        location: min(snapshot.selection.location, text.utf16.count),
        length: min(
          snapshot.selection.length, max(0, text.utf16.count - snapshot.selection.location)))
    }
    view.onRun = onRun
    view.onTab = onTab
    view.onEscape = onEscape
    view.onComplete = onComplete
    view.onPartial = onPartial
    if focus { view.requestFocus(onFocused) }
    guard view.markedTextRange == nil else { return }
    guard
      context.coordinator.styledText != text || context.coordinator.styledTokens != tokens
        || context.coordinator.styledSize != view.traitCollection.preferredContentSizeCategory
    else { return }
    context.coordinator.styledText = text
    context.coordinator.styledTokens = tokens
    context.coordinator.styledSize = view.traitCollection.preferredContentSizeCategory
    let selection = view.selectedRange
    view.textStorage.beginEditing()
    view.textStorage.addAttributes(
      [
        .foregroundColor: UIColor.label,
        .font: UIFont.preferredFont(forTextStyle: .body).withDesign(.monospaced),
      ], range: NSRange(location: 0, length: view.text.utf16.count))
    for token in tokens.array {
      let span = token[0]
      let bytes = NSRange(
        location: Int(span["start"].double),
        length: max(0, Int(span["end"].double - span["start"].double)))
      if let range = EditorOffsets.range(view.text, bytes: bytes) {
        let name = token[1].string
        let color: UIColor =
          name == "Number"
          ? .systemBlue
          : name == "String"
            ? .systemOrange
            : name == "Comment" ? .secondaryLabel : name == "Builtin" ? .systemPurple : .label
        view.textStorage.addAttribute(.foregroundColor, value: color, range: range)
      }
    }
    view.textStorage.endEditing()
    view.selectedRange = selection
  }
  func makeCoordinator() -> Coordinator { Coordinator(self) }
  func sizeThatFits(_ proposal: ProposedViewSize, uiView: EditorTextView, context: Context)
    -> CGSize?
  {
    let width = proposal.width ?? 320
    let size = uiView.sizeThatFits(CGSize(width: width, height: .greatestFiniteMagnitude))
    return CGSize(width: width, height: min(280, max(80, size.height)))
  }
  final class Coordinator: NSObject, UITextViewDelegate {
    var parent: NativeTextEditor
    var updating = false
    var styledText: String?
    var styledTokens: JSONValue = .null
    var styledSize: UIContentSizeCategory?
    init(_ parent: NativeTextEditor) { self.parent = parent }
    func textViewDidChange(_ textView: UITextView) {
      guard !updating else { return }
      let next = EditorSnapshot(
        selection: textView.selectedRange, composing: textView.markedTextRange != nil)
      if parent.snapshot != next { parent.snapshot = next }
      if parent.text != textView.text { parent.text = textView.text }
    }
    func textViewDidChangeSelection(_ textView: UITextView) {
      guard !updating else { return }
      let next = EditorSnapshot(
        selection: textView.selectedRange, composing: textView.markedTextRange != nil)
      if parent.snapshot != next { parent.snapshot = next }
    }
  }
}
final class EditorTextView: UITextView {
  var onRun: ((Bool) -> Void)?
  var onTab: (() -> Bool)?
  var onEscape: (() -> Void)?
  var onComplete: (() -> Void)?
  var onPartial: (() -> Void)?
  private var pendingFocus: (() -> Void)?
  func requestFocus(_ completion: @escaping () -> Void) {
    pendingFocus = completion
    applyFocus()
  }
  private func applyFocus() {
    guard window != nil, let completion = pendingFocus else { return }
    pendingFocus = nil
    becomeFirstResponder()
    DispatchQueue.main.async(execute: completion)
  }
  override func didMoveToWindow() {
    super.didMoveToWindow()
    applyFocus()
  }
  override var keyCommands: [UIKeyCommand]? {
    [
      UIKeyCommand(input: "\r", modifierFlags: .command, action: #selector(run)),
      UIKeyCommand(input: "\r", modifierFlags: .shift, action: #selector(run)),
      UIKeyCommand(input: "\t", modifierFlags: [], action: #selector(tab)),
      UIKeyCommand(input: UIKeyCommand.inputEscape, modifierFlags: [], action: #selector(escape)),
      UIKeyCommand(input: " ", modifierFlags: .control, action: #selector(complete)),
      UIKeyCommand(
        input: UIKeyCommand.inputRightArrow, modifierFlags: .command, action: #selector(partial)),
    ]
  }
  @objc private func run(_ command: UIKeyCommand) {
    if markedTextRange == nil { onRun?(command.modifierFlags.contains(.shift)) }
  }
  @objc private func tab() { if markedTextRange == nil, onTab?() != true { insertText("\t") } }
  @objc private func escape() { onEscape?() }
  @objc private func complete() { if markedTextRange == nil { onComplete?() } }
  @objc private func partial() { if markedTextRange == nil { onPartial?() } }
}
extension UIFont {
  func withDesign(_ design: UIFontDescriptor.SystemDesign) -> UIFont {
    UIFont(descriptor: fontDescriptor.withDesign(design) ?? fontDescriptor, size: pointSize)
  }
}

struct MathCellEditor: View {
  var cell: NotebookCell
  var controller: NotebookController
  private var snapshot: EditorSnapshot {
    get { controller.editorSnapshots[cell.id] ?? EditorSnapshot() }
    nonmutating set {
      if controller.editorSnapshots[cell.id] != newValue {
        controller.editorSnapshots[cell.id] = newValue
      }
    }
  }
  @State private var preview: JSONValue = .null
  @State private var local: JSONValue = .null
  @State private var ghost = ""
  @State private var ghostID: String?
  @State private var ghostAnchor: Int?
  @State private var preservedGhostSource: String?
  @State private var ghostGeneration = 0
  @State private var requestTask: Task<Void, Never>?
  @State private var previewRevision = 0
  @State private var showConsent = false
  @State private var hover: JSONValue = .null
  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      NativeTextEditor(
        text: Binding(
          get: { controller.cells.first { $0.id == cell.id }?.input.source ?? cell.input.source },
          set: { value in
            controller.setComposing(cell.id, snapshot.composing)
            controller.edit(cell.id, source: value)
            changed(value)
          }), snapshot: Binding(get: { snapshot }, set: { snapshot = $0 }),
        accessibilityID: "editor.\(cell.id)", tokens: preview["tokens"],
        focus: controller.focusCell == cell.id,
        onFocused: { if controller.focusCell == cell.id { controller.focusCell = nil } },
        onRun: { next in Task { await controller.run(cell.id, next: next) } }, onTab: accept,
        onEscape: clearGhost,
        onComplete: complete, onPartial: { insertGhost(partial: true) })
      if !ghost.isEmpty {
        HStack {
          Text(ghost).font(.system(.body, design: .monospaced)).foregroundStyle(.secondary)
            .accessibilityHidden(true)
          Button(controller.text("接受", "Accept")) { insertGhost() }
          Button(controller.text("一段", "Part")) { insertGhost(partial: true) }
          Button {
            clearGhost()
          } label: {
            Image(systemName: "xmark")
          }
        }.buttonStyle(.borderless)
      }
      if !local["items"].array.isEmpty {
        ScrollView(.horizontal) {
          HStack {
            ForEach(Array(local["items"].array.enumerated()), id: \.offset) { _, item in
              Button(item["label"].string) { applyLocal(item) }.buttonStyle(.bordered)
            }
          }
        }
      }
      if !preview["latex"].string.isEmpty {
        MathView(latex: preview["latex"].string, source: cell.input.source, fontSize: 18)
          .foregroundStyle(.secondary)
      }
      ForEach(Array(preview["diagnostics"].array.enumerated()), id: \.offset) { _, diagnostic in
        HStack {
          Text(diagnostic["message"].string).font(.caption).foregroundStyle(.red)
          if !diagnostic["fix"].isNull {
            Button(controller.text("快速修复", "Quick fix")) { applyFix(diagnostic["fix"]) }.font(
              .caption)
          }
        }
      }
      if !hover.isNull {
        VStack(alignment: .leading) {
          Text(hover["signature"].string).font(.system(.caption, design: .monospaced))
          Text(hover["summary"].string).font(.caption)
          Text(hover["value"].string).font(.system(.caption, design: .monospaced))
        }.textSelection(.enabled)
      }
      FlowLayout(spacing: 8) {
        ForEach(Array(preview["actions"].array.enumerated()), id: \.offset) { _, action in
          Button(action["source"].string) {
            controller.add(source: action["source"].string, after: cell.id)
          }.font(.caption).buttonStyle(.bordered)
        }
      }
      FlowLayout(spacing: 10) {
        Button(controller.text("补全", "Complete"), action: complete)
        Button("AI") {
          if controller.authorized("complete") { requestGhost() } else { showConsent = true }
        }.disabled(snapshot.composing)
        Button(controller.text("帮助", "Help")) {
          Task {
            if let cursor = cursorBytes {
              hover =
                (try? await controller.call(
                  .object([
                    "type": .string("hover"), "source": .string(cell.input.source),
                    "dialect": .string(cell.input.dialect.rawValue),
                    "cursor": .number(Double(cursor)),
                  ])))?.response.body["info"] ?? .null
            }
          }
        }
        if snapshot.selection.length > 0 && !snapshot.composing
          && NSMaxRange(snapshot.selection) <= cell.input.source.utf16.count
        {
          Menu(controller.text("选区操作", "Selection")) {
            ForEach(["Factor", "Expand", "Simplify", "Solve", "Plot"], id: \.self) { action in
              Button(action) {
                let selection = (cell.input.source as NSString).substring(with: snapshot.selection)
                controller.add(source: "\(action)(\(selection))", after: cell.id)
                if let selected = controller.selected { Task { await controller.run(selected) } }
              }
            }
          }
        }
        ForEach(["α", "π", "√", "^", "=", "(", ")"], id: \.self) { token in
          Button(token) { insert(token == "√" ? "sqrt()" : token) }
        }
      }.font(.caption).buttonStyle(.borderless)
    }
    .task { changed(cell.input.source) }
    .onChange(of: cell.revision) { _, _ in changed(cell.input.source) }
    .onChange(of: snapshot) { old, new in
      controller.setComposing(cell.id, new.composing)
      if old.composing && !new.composing { changed(cell.input.source) }
      if snapshot.composing || snapshot.selection.length != 0
        || (ghostAnchor != nil && snapshot.selection.location != ghostAnchor)
      {
        clearGhost()
      }
    }
    .onChange(of: controller.aiText[ghostID ?? ""]) { _, _ in
      guard let id = ghostID, controller.aiStates[id] == "done" else { return }
      ghost = controller.aiText[id] ?? ""
    }
    .onChange(of: controller.aiStates[ghostID ?? ""]) { _, value in
      if value == "done", let id = ghostID { ghost = controller.aiText[id] ?? "" }
    }
    .onDisappear {
      requestTask?.cancel()
      snapshot.composing = false
      controller.setComposing(cell.id, false)
      clearGhost()
    }
    .alert(controller.text("将代码发送给 AI", "Send code to AI"), isPresented: $showConsent) {
      Button(controller.text("发送", "Send")) {
        controller.allow("complete")
        requestGhost()
      }
      Button(controller.text("取消", "Cancel"), role: .cancel) {}
    } message: {
      Text(
        controller.destination("complete") + "\n"
          + controller.text(
            "发送光标附近代码；启用上下文时包括前三个数学源码。只有接受才写入源码。",
            "Sends code around the cursor and, when enabled, up to three preceding math sources. Acceptance inserts source only."
          ))
    }
  }
  private var cursorBytes: Int? {
    EditorOffsets.byte(cell.input.source, utf16: snapshot.selection.location)
  }
  private func changed(_ source: String) {
    previewRevision += 1
    let token = previewRevision
    requestTask?.cancel()
    if !ghost.isEmpty, snapshot.selection.length == 0, source != preservedGhostSource {
      clearGhost()
    }
    requestTask = Task {
      try? await Task.sleep(for: .milliseconds(80))
      guard !Task.isCancelled, !snapshot.composing else { return }
      if let packet = try? await controller.call(
        .object([
          "type": .string("preview"), "source": .string(source),
          "dialect": .string(cell.input.dialect.rawValue),
          "cursor": cursorBytes.map { .number(Double($0)) } ?? .null,
        ])), token == previewRevision
      {
        preview = packet.response.body
      }
      try? await Task.sleep(for: .milliseconds(270))
      guard !Task.isCancelled, token == previewRevision, !controller.busy, ghost.isEmpty,
        snapshot.selection.length == 0,
        source.trimmingCharacters(in: .whitespacesAndNewlines).count >= 3,
        snapshot.selection.location == source.utf16.count, !snapshot.composing,
        controller.selected == cell.id
      else { return }
      if controller.authorized("complete") { requestGhost() }
    }
  }
  private func complete() {
    guard !snapshot.composing else { return }
    clearGhost()
    Task {
      guard let cursor = cursorBytes else { return }
      if let packet = try? await controller.call(
        .object([
          "type": .string("complete"), "source": .string(cell.input.source),
          "dialect": .string(cell.input.dialect.rawValue), "cursor": .number(Double(cursor)),
        ]))
      {
        local = packet.response.body
      }
    }
  }
  private func requestGhost() {
    guard !snapshot.composing, snapshot.selection.length == 0 else { return }
    Task {
      clearGhost()
      let generation = ghostGeneration
      let cursor = snapshot.selection.location
      ghostAnchor = cursor
      let started = await controller.beginAI(
        feature: "complete",
        body: .object([
          "type": .string("llm_complete"),
          "prefix": .string(
            (cell.input.source as NSString).substring(
              to: min(snapshot.selection.location, cell.input.source.utf16.count))),
          "suffix": .string(
            (cell.input.source as NSString).substring(
              from: min(snapshot.selection.location, cell.input.source.utf16.count))),
          "dialect": .string(cell.input.dialect.rawValue),
        ]), cell: cell.id)
      guard generation == ghostGeneration, cursor == snapshot.selection.location,
        !snapshot.composing
      else {
        if let started { controller.cancelAI(started) }
        return
      }
      ghostID = started
    }
  }
  private func accept() -> Bool {
    if !ghost.isEmpty {
      insertGhost()
      return true
    }
    let shortcuts = [
      "\\alpha": "α", "\\beta": "β", "\\gamma": "γ", "\\pi": "π", "\\theta": "θ", "\\lambda": "λ",
      "\\sigma": "σ", "\\omega": "ω",
    ]
    let prefix = (cell.input.source as NSString).substring(
      to: min(snapshot.selection.location, cell.input.source.utf16.count))
    if let (word, value) = shortcuts.first(where: { prefix.hasSuffix($0.key) }) {
      let range = NSRange(
        location: snapshot.selection.location - word.utf16.count, length: word.utf16.count)
      controller.edit(
        cell.id, source: (cell.input.source as NSString).replacingCharacters(in: range, with: value)
      )
      snapshot.selection = NSRange(location: range.location + value.utf16.count, length: 0)
      return true
    }
    if let first = local["items"].array.first {
      applyLocal(first)
      return true
    }
    return false
  }
  private func insert(_ text: String) {
    guard !snapshot.composing else { return }
    let range = snapshot.selection
    guard NSMaxRange(range) <= cell.input.source.utf16.count else { return }
    controller.edit(
      cell.id, source: (cell.input.source as NSString).replacingCharacters(in: range, with: text))
    snapshot.selection = NSRange(location: range.location + text.utf16.count, length: 0)
    clearGhost()
  }
  private func insertGhost(partial: Bool = false) {
    let value = partial ? String(ghost.prefix(while: { !$0.isWhitespace })) : ghost
    let remaining = partial ? String(ghost.dropFirst(value.count)) : ""
    let anchor = snapshot.selection.location + value.utf16.count
    insert(value)
    ghost = remaining
    if !remaining.isEmpty {
      ghostAnchor = anchor
      preservedGhostSource = controller.cells.first(where: { $0.id == cell.id })?.input.source
    }
  }
  private func clearGhost() {
    ghostGeneration += 1
    ghostAnchor = nil
    preservedGhostSource = nil
    if let id = ghostID { controller.cancelAI(id) }
    ghostID = nil
    ghost = ""
  }
  private func applyLocal(_ item: JSONValue) {
    guard !snapshot.composing else { return }
    if let native = EditorOffsets.range(
      cell.input.source,
      bytes: NSRange(
        location: Int(local["from"].double), length: Int(local["to"].double - local["from"].double))
    ) {
      controller.edit(
        cell.id,
        source: (cell.input.source as NSString).replacingCharacters(
          in: native, with: item["insert_text"].string))
      snapshot.selection = NSRange(
        location: native.location + item["insert_text"].string.utf16.count, length: 0)
    }
    local = .null
  }
  private func applyFix(_ fix: JSONValue) {
    guard !snapshot.composing else { return }
    let range = fix["span"]
    if let native = EditorOffsets.range(
      cell.input.source,
      bytes: NSRange(
        location: Int(range["start"].double),
        length: Int(range["end"].double - range["start"].double)))
    {
      controller.edit(
        cell.id,
        source: (cell.input.source as NSString).replacingCharacters(
          in: native, with: fix["replacement"].string))
    }
  }
}
