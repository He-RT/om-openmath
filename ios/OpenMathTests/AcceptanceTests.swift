import SwiftMath
import SwiftUI
import UIKit
import XCTest

@testable import OpenMath

@MainActor final class AcceptanceTests: XCTestCase {
  func testOriginal53RowsThroughMobileKernelAndNativeFormulaParser() async throws {
    let url = try XCTUnwrap(Bundle.main.url(forResource: "solve-corpus", withExtension: "json"))
    let rows = try JSONDecoder().decode([JSONValue].self, from: Data(contentsOf: url))
    XCTAssertEqual(rows.count, 53)
    var timings: [JSONValue] = []
    var unsupported = Set<String>()
    func inspect(_ value: JSONValue) {
      if case .string(let latex) = value, !latex.isEmpty {
        var error: NSError?
        _ = MTMathListBuilder.build(fromString: MathTypesetting.prepare(latex), error: &error)
        if error != nil { unsupported.insert(latex) }
      }
    }
    func walk(_ value: JSONValue) {
      for (key, child) in value.object {
        if key.contains("latex") {
          if case .array = child { child.array.forEach(inspect) } else { inspect(child) }
        } else {
          walk(child)
        }
      }
      value.array.forEach(walk)
    }
    for row in rows {
      let client = try KernelClient()
      let start = ContinuousClock.now
      let packet = try await client.request(
        .object([
          "type": .string("evaluate"), "cell_id": .string("case"), "source": row["input"],
          "dialect": .string("Wolfram"),
        ]))
      let duration = start.duration(to: .now)
      let ms =
        Double(duration.components.seconds) * 1000 + Double(duration.components.attoseconds) / 1e15
      let output = packet.response.body["output"]
      XCTAssertFalse(output["items"].array.isEmpty, row["id"].string)
      XCTAssertFalse(
        output["messages"].array.contains { $0["level"].string == "Error" }, row["id"].string)
      if let item = output["items"].array.last, row["check"].string == "=" {
        let expected = try await client.request(
          .object([
            "type": .string("inspect_expression"), "source": row["expected"],
            "numeric": .bool(false),
          ]))
        XCTAssertEqual(
          item["input_form"].string, expected.response.body["value"]["input_form"].string,
          row["id"].string)
      }
      XCTAssertLessThan(ms, 1000, "Mobile non-slow row \(row["id"].string)")
      walk(output)
      timings.append(.object(["id": row["id"], "ms": .number(ms), "output": output]))
      client.close()
    }
    let report = JSONValue.object([
      "rows": .array(timings),
      "unsupported_latex": .array(unsupported.sorted().map(JSONValue.string)),
      "device": .string(UIDevice.current.model), "os": .string(UIDevice.current.systemVersion),
    ])
    let attachment = XCTAttachment(data: try report.data(), uniformTypeIdentifier: "public.json")
    attachment.name = "mobile-corpus.json"
    attachment.lifetime = .keepAlways
    add(attachment)
    XCTAssertTrue(
      unsupported.isEmpty, "Unsupported corpus formula commands: \(unsupported.sorted())")
  }
  func testUTF16UTF8BoundariesAndIMEOffsets() {
    let text = "A🙂α中文B"
    XCTAssertEqual(EditorOffsets.byte(text, utf16: 1), 1)
    XCTAssertNil(EditorOffsets.byte(text, utf16: 2))
    XCTAssertEqual(EditorOffsets.byte(text, utf16: 3), 5)
    XCTAssertEqual(
      EditorOffsets.range(text, bytes: NSRange(location: 1, length: 4)),
      NSRange(location: 1, length: 2))
    XCTAssertNil(EditorOffsets.range(text, bytes: NSRange(location: 2, length: 1)))
    XCTAssertNil(EditorOffsets.range(text, bytes: NSRange(location: 99, length: 1)))
  }
  func testNativeMathFractionsMatricesAndUnknownFallback() {
    for formula in [
      "\\frac{1}{2}", "\\sqrt{x^2+1}", "\\begin{pmatrix}1&2\\\\3&4\\end{pmatrix}", "x \\in [1,2)",
    ] {
      var error: NSError?
      XCTAssertNotNil(MTMathListBuilder.build(fromString: formula, error: &error))
      XCTAssertNil(error, formula)
    }
    var error: NSError?
    _ = MTMathListBuilder.build(fromString: "\\unknowncommand{x}", error: &error)
    XCTAssertNotNil(error)
  }
  func testMarkdownMathDoesNotInterpretCodeAndKeepsSource() {
    let tokens = MathMarkdownTokens("A $x^2$ B\n```\n$x$\n```\n`$y$`\n$$\\frac{1}{2}$$")
    XCTAssertEqual(tokens.formulas.count, 2)
    XCTAssertTrue(tokens.source.contains("`$y$`"))
    XCTAssertTrue(tokens.source.contains("$x$"))
  }
  func testDocumentRoundTripAndReactiveEdits() async throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent(
      UUID().uuidString + ".omnb")
    let document = NotebookDocument(fileURL: url)
    document.notebook = NotebookFile(title: "中文🙂", cells: [CellInput(source: "α=2")])
    let created = await document.write(operation: .forCreating)
    XCTAssertTrue(created)
    _ = await document.closeFile()
    let reopened = NotebookDocument(fileURL: url)
    let opened = await reopened.openFile()
    XCTAssertTrue(opened)
    XCTAssertEqual(reopened.notebook.title, "中文🙂")
    _ = await reopened.closeFile()
    try FileManager.default.removeItem(at: url)
    let controller = NotebookController()
    await controller.start(restoreLastDocument: false)
    XCTAssertNil(controller.error, controller.error ?? "")
    controller.add(source: "let a=2")
    let a = try XCTUnwrap(controller.selected)
    controller.add(source: "a+1")
    let b = try XCTUnwrap(controller.selected)
    await controller.runAll()
    XCTAssertEqual(
      controller.cells.first { $0.id == b }?.output["items"][0]["input_form"].string, "3")
    controller.edit(a, source: "let a=5")
    await controller.run(a)
    XCTAssertEqual(
      controller.cells.first { $0.id == b }?.output["items"][0]["input_form"].string, "6")
  }
}

@MainActor final class LifecycleTests: XCTestCase {
  func testEditingDuringInterruptPreservesDraftAndIgnoresOldResult() async throws {
    let controller = NotebookController()
    await controller.start(restoreLastDocument: false)
    XCTAssertNil(controller.error, controller.error ?? "")
    controller.add(source: "factorial(1000000)")
    let id = try XCTUnwrap(controller.selected)
    let task = Task { await controller.run(id) }
    try await Task.sleep(for: .milliseconds(40))
    controller.edit(id, source: "2+3")
    controller.interrupt()
    await task.value
    XCTAssertEqual(controller.cells.first { $0.id == id }?.input.source, "2+3")
    await controller.run(id)
    XCTAssertEqual(
      controller.cells.first { $0.id == id }?.output["items"][0]["input_form"].string, "5",
      controller.error ?? controller.cells.first?.output.pretty ?? "")
    controller.kernel?.close()
  }
  func testSaveFailureBlocksDocumentSwitchAndKeepsSource() async throws {
    let controller = NotebookController()
    await controller.start(restoreLastDocument: false)
    XCTAssertNil(controller.error, controller.error ?? "")
    controller.add(source: "中文🙂")
    let id = controller.selected
    controller.document = NotebookDocument(
      fileURL: URL(fileURLWithPath: "/dev/null/unwritable.omnb"))
    await controller.save()
    XCTAssertTrue(controller.dirty)
    XCTAssertNotNil(controller.error)
    await controller.newNotebook()
    XCTAssertEqual(controller.selected, id)
    XCTAssertEqual(controller.cells.first?.input.source, "中文🙂")
    controller.kernel?.close()
  }
}

@MainActor final class CompositionTests: XCTestCase {
  func testPendingCompositionDoesNotReachKernelOrRunUntilCommitted() async throws {
    let controller = NotebookController()
    await controller.start(restoreLastDocument: false)
    controller.add(source: "2+3")
    let id = try XCTUnwrap(controller.selected)
    await controller.run(id)
    controller.setComposing(id, true)
    controller.edit(id, source: "中文🙂")
    await controller.run(id)
    await controller.runAll()
    XCTAssertEqual(controller.cells.first?.output["items"][0]["input_form"].string, "5")
    let stored = try await controller.call(.object(["type": .string("save_notebook")]))
    XCTAssertEqual(stored.response.body["file"]["cells"][0]["source"].string, "2+3")
    controller.edit(id, source: "3+4")
    controller.setComposing(id, false)
    await controller.run(id)
    XCTAssertEqual(controller.cells.first?.output["items"][0]["input_form"].string, "7")
    controller.kernel?.close()
  }
  func testMarkdownKeepsIndentedAndUnicodeInlineCodeLiteral() {
    let source = "    $literal$\n\n中文 `$code$` **$x$**\n\n  ~~~\n$inCode$\n  ~~~\n\n$$y$$"
    let tokens = MathMarkdownTokens(source)
    XCTAssertEqual(tokens.formulas.count, 2)
    for literal in ["$literal$", "$code$", "$inCode$"] {
      XCTAssertTrue(tokens.source.contains(literal))
    }
  }
}

@MainActor final class MarkdownLayoutTests: XCTestCase {
  func testInlineMathUsesOneTextLineWhenItFits() {
    let host = UIHostingController(
      rootView: NativeMarkdown(source: "A $x$ B").environment(\.dynamicTypeSize, .medium))
    let parent = UIViewController()
    parent.addChild(host)
    parent.view.addSubview(host.view)
    parent.setOverrideTraitCollection(
      UITraitCollection(preferredContentSizeCategory: .medium), forChild: host)
    host.view.frame = CGRect(x: 0, y: 0, width: 300, height: 200)
    host.view.layoutIfNeeded()
    let size = host.sizeThatFits(in: CGSize(width: 300, height: 1000))
    XCTAssertLessThan(size.height, 34, "Inline math should not force separate lines")
  }
}

@MainActor final class RestoreTests: XCTestCase {
  func testOpenedDocumentRestoresWithoutAnEditAndDoesNotExecuteCells() async throws {
    // Other tests deliberately trigger autosave; they must not overwrite this
    // document's bookmark while the restore request is suspended.
    let suite = "OpenMathRestoreTests." + UUID().uuidString
    let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let url = FileManager.default.temporaryDirectory.appendingPathComponent(
      UUID().uuidString + ".omnb")
    let file = NotebookFile(title: "恢复测试🙂", cells: [CellInput(source: "2+3")])
    try JSONEncoder().encode(file).write(to: url)
    defer { try? FileManager.default.removeItem(at: url) }
    let first = NotebookController(defaults: defaults)
    await first.start(restoreLastDocument: false)
    await first.load(url)
    XCTAssertNil(first.error)
    first.kernel?.close()
    let second = NotebookController(defaults: defaults)
    await second.start()
    XCTAssertNil(second.error)
    XCTAssertEqual(second.title, file.title)
    XCTAssertEqual(second.cells.first?.input.source, "2+3")
    XCTAssertEqual(second.cells.first?.status, .Stale)
    XCTAssertTrue(second.cells.first?.output.isNull == true)
    second.kernel?.close()
  }
}
