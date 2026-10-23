import XCTest
@testable import OpenMath

@MainActor final class MobileWorkbenchTests: XCTestCase {
  func testPatchRequiresExactSourceAndRejectsAmbiguousReplacementAndSelfMove() throws {
    let cell = CellInput(id: "cell-a", source: "a+a"), file = NotebookFile(cells: [cell])
    let hash = MobileAgentStore.hash(cell.source)
    let target = JSONValue.object(["cell_id": .string(cell.id)])
    let replace = JSONValue.object(["type": .string("replace_text"), "target": target, "expected_source_hash": .string(hash), "match": .string("a"), "replacement": .string("b")])
    XCTAssertThrowsError(try MobileNotebookPatch.apply([replace], to: file))
    let move = JSONValue.object(["type": .string("move_cell"), "target": target, "after": target])
    XCTAssertThrowsError(try MobileNotebookPatch.apply([move], to: file))
    let update = JSONValue.object(["type": .string("update_cell"), "target": target, "expected_source_hash": .string(hash), "source": .string("let a=5")])
    XCTAssertEqual(try MobileNotebookPatch.apply([update], to: file).cells[0].source, "let a=5")
    let changed = NotebookFile(cells: [CellInput(id: cell.id, source: "a+b")])
    XCTAssertThrowsError(try MobileNotebookPatch.apply([update], to: changed))
    XCTAssertEqual(file.cells[0].source, "a+a")
  }
  func testDraftTransactionsAndUnicodePersistWithoutTouchingNotebookSource() throws {
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: dir) }
    let store = try MobileAgentStore(document: "unit-document", root: dir)
    var state = MobileAgentState(); state.draft = "中文草稿 🧮"; state.execute = true
    let before = NotebookFile(title: "原稿", cells: [CellInput(id: "a", source: "1+1")])
    let after = NotebookFile(title: "修改", cells: [CellInput(id: "a", source: "2+2")])
    state.transactions.append(.init(id: "transaction-a", before: before, after: after, state: "pending"))
    try store.save(state)
    let restored = try store.load()
    XCTAssertEqual(restored.draft, state.draft)
    XCTAssertEqual(restored.transactions[0].before, before)
    XCTAssertEqual(restored.transactions[0].after, after)
    try store.archive(state)
    XCTAssertEqual(try store.load().draft, state.draft)
  }
  func testInvalidGeometryCannotReachNativeRenderer() {
    let invalid = JSONValue.object(["bounds": .array([]), "meshes": .array([])])
    XCTAssertThrowsError(try MobileSceneGeometry.validate(invalid))
  }
}
