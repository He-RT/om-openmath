import Foundation

@main struct MobileContractsFixture {
  static func main() throws {
    let original = NotebookFile(title: "数学🙂", cells: [CellInput(id: "cell-a", source: "a+a")])
    let target = JSONValue.object(["cell_id": .string("cell-a")]), hash = MobileAgentStore.hash("a+a")
    let update = JSONValue.object(["type": .string("update_cell"), "target": target, "source": .string("let a=5"), "expected_source_hash": .string(hash)])
    let after = try MobileNotebookPatch.apply([update], to: original)
    precondition(after.cells[0].source == "let a=5" && original.cells[0].source == "a+a")
    do { _ = try MobileNotebookPatch.apply([update], to: after); fatalError("stale source accepted") } catch {}
    let ambiguous = JSONValue.object(["type": .string("replace_text"), "target": target, "expected_source_hash": .string(hash), "match": .string("a"), "replacement": .string("b")])
    do { _ = try MobileNotebookPatch.apply([ambiguous], to: original); fatalError("ambiguous replacement accepted") } catch {}
    let move = JSONValue.object(["type": .string("move_cell"), "target": target, "after": target])
    do { _ = try MobileNotebookPatch.apply([move], to: original); fatalError("self move accepted") } catch {}
    let root = FileManager.default.temporaryDirectory.appendingPathComponent("OpenMath-mobile-contracts-" + UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let store = try MobileAgentStore(document: "own-fixture", root: root)
    var state = MobileAgentState(); state.draft = "中文草稿 🧮"
    state.transactions.append(.init(id: "tx-1", before: original, after: after, state: "pending"))
    let attachment = Data("exact attachment bytes 中文".utf8)
    let firstHash = try store.retainAttachment(attachment)
    let secondHash = try store.retainAttachment(attachment)
    precondition(firstHash == secondHash)
    try store.save(state); try store.archive(state)
    let loaded = try store.load()
    let restored = try store.restore(store.history()[0].lastPathComponent)
    precondition(loaded.draft == state.draft)
    precondition(restored.transactions[0].before == original)
    do { _ = try store.restore("../../not-a-conversation.json"); fatalError("foreign path accepted") } catch {}
    let proposal = JSONValue.object(["canonical": .object(["role": .string("assistant"), "parts": .array([
      .object(["kind": .string("tool_call"), "tool_call_id": .string("finished")]),
      .object(["kind": .string("tool_call"), "tool_call_id": .string("stopped")])])]), "replay": .object(["binding": .object([:])])])
    let result = JSONValue.object(["canonical": .object(["role": .string("tool"), "parts": .array([.object(["kind": .string("tool_result"), "tool_call_id": .string("finished")])])])])
    let replay = MobileAgentReplay.history([proposal, result], binding: .object([:]))
    precondition(replay.count == 2 && replay[0]["canonical"]["parts"].array.count == 1 && replay[0]["canonical"]["parts"][0]["tool_call_id"].string == "finished" && replay[0]["replay"].isNull)
    precondition(proposal["canonical"]["parts"].array.count == 2)
    let valid = JSONValue.object(["bounds": .array([.array([.number(0), .number(0), .number(0)]), .array([.number(1), .number(1), .number(1)])]),
      "meshes": .array([.object(["positions": .array([.array([.number(0), .number(0), .number(0)])]), "normals": .array([.array([.number(0), .number(0), .number(1)])]), "colors": .array([.array([.number(1), .number(0), .number(0), .number(1)])]), "triangles": .array([.array([.number(0), .number(0), .number(0)])])])]), "lines": .array([])])
    try MobileSceneGeometry.validate(valid)
    var invalid = valid
    invalid["meshes"] = .array([.object(["positions": .array([]), "normals": .array([]), "colors": .array([]), "triangles": .array([.array([.number(0), .number(0), .number(0)])])])])
    do { try MobileSceneGeometry.validate(invalid); fatalError("invalid index accepted") } catch {}
    var badBounds = valid; badBounds["bounds"] = .array([.array([.string("0"), .number(0), .number(0)]), .array([.number(1), .number(1), .number(1)])])
    do { try MobileSceneGeometry.validate(badBounds); fatalError("nonnumeric bounds accepted") } catch {}
    let metadata = JSONValue.object(["cells": .array([
      .object(["id": .string("a"), "defines": .array([.string("a")]), "uses": .array([]), "status": .string("Stale")]),
      .object(["id": .string("b"), "defines": .array([]), "uses": .array([.string("a")]), "status": .string("Stale")]),
      .object(["id": .string("unrelated"), "defines": .array([.string("z")]), "uses": .array([]), "status": .string("Stale")])]), "definition_order": .array([.string("a"), .string("unrelated")]), "cycles": .array([])])
    let required = try MobileRunPlanner.plan(state: metadata, requested: ["b"], scope: "required")
    let selected = try MobileRunPlanner.plan(state: metadata, requested: ["b"], scope: "selected")
    let affected = try MobileRunPlanner.plan(state: metadata, requested: ["a"], scope: "affected")
    precondition(required == ["a", "b"] && selected == ["b"] && affected == ["a", "b"])
    print("Native source patch, stale-source and ambiguity rejection, persisted Unicode draft/pending transaction/archive, partial-turn replay pairs, run dependencies, foreign-path rejection and geometry bounds passed; no simulator")
  }
}
