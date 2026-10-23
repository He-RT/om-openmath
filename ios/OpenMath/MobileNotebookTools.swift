import Foundation

@MainActor final class MobileNotebookTools {
  private weak var controller: NotebookController?
  private var snapshots: [String: NotebookFile] = [:]
  private var previews: [String: (before: NotebookFile, after: NotebookFile)] = [:]
  private var results: [String: (cell: String, revision: Int, output: JSONValue)] = [:]
  init(controller: NotebookController) { self.controller = controller }
  func reset() { snapshots = [:]; previews = [:]; results = [:] }
  private func snapshot(_ file: NotebookFile) throws -> String {
    let ref = "snapshot-" + MobileAgentStore.hash(String(decoding: try JSONEncoder().encode(file), as: UTF8.self))
    snapshots[ref] = file; return ref
  }
  private func current(_ ref: String, _ controller: NotebookController) throws -> NotebookFile {
    guard let file = snapshots[ref], file == controller.notebook else { throw KernelError.message("文稿快照已过期，请重新读取") }
    return file
  }
  func invoke(_ name: String, args: JSONValue, execute: Bool, agent: MobileNotebookAgent) async -> JSONValue {
    let request = "request-" + UUID().uuidString.lowercased(), operation = "operation-" + UUID().uuidString.lowercased()
    var data: JSONValue = .null, failure: String?, status = "completed"
    do {
      guard let controller, !controller.loading, !controller.agentCommitting else { throw KernelError.message("文稿尚未就绪") }
      if ["apply_notebook_patch", "run_cells", "undo_transaction"].contains(name) {
        guard execute else { throw KernelError.message("讨论模式不允许修改或运行文稿") }
        guard !controller.busy, controller.composingCells.isEmpty else { throw KernelError.message("请先完成当前编辑或计算") }
      }
      switch name {
      case "read_notebook":
        let file = controller.notebook, ref = try snapshot(file)
        if !args["snapshot_ref"].isNull { _ = try current(args["snapshot_ref"].string, controller) }
        let ids = args["cell_ids"].array.map(\.string)
        guard ids.allSatisfy({ id in controller.cells.contains { $0.id == id } }) else { throw KernelError.message("找不到单元格引用") }
        let chosen = controller.cells.filter { ids.isEmpty || ids.contains($0.id) }
        let limit = min(50, max(1, args["limit"].isNull ? 20 : Int(args["limit"].double)))
        let offset = Int(args["cursor"].string.split(separator: ":").last ?? "0") ?? 0
        guard offset >= 0, offset <= chosen.count, args["cursor"].isNull || args["cursor"].string.hasPrefix(ref + ":") else { throw KernelError.message("无效分页位置") }
        let rows = chosen.dropFirst(offset).prefix(limit).map { cell -> JSONValue in
          var row = cell.input.json
          row["source_hash"] = .string(MobileAgentStore.hash(cell.input.source)); row["status"] = .string(cell.status.rawValue)
          if args["view"].string != "cells" { row["source"] = .string(String(cell.input.source.prefix(180))) }
          if cell.status == .Done, !cell.output.isNull {
            let resultRef = "result-" + UUID().uuidString.lowercased()
            results[resultRef] = (cell.id, cell.revision, cell.output); row["result_ref"] = .string(resultRef)
            if args["include_outputs"].bool { row["output"] = cell.output }
          }
          return row
        }
        if args["view"].string == "symbols" {
          await controller.refreshVariables(); data = .object(["snapshot_ref": .string(ref), "symbols": .array(controller.variableRows)])
        } else {
          data = .object(["snapshot_ref": .string(ref), "title": .string(file.title), "cells": .array(rows), "next_cursor": offset + limit < chosen.count ? .string(ref + ":\(offset + limit)") : .null])
        }
      case "search_functions", "get_function_docs":
        let catalog = try await controller.call(.object(["type": .string("mobile_function_docs")])).response.body["documents"]
        let functions = catalog["functions"].array
        if !args["metadata_version"].isNull, args["metadata_version"] != catalog["metadata_version"] { throw KernelError.message("函数目录版本已改变") }
        let query = name == "search_functions" ? args["query"].string.lowercased() : args["name"].string.lowercased()
        let matches = functions.filter { row in
          if name == "get_function_docs" { return !args["function_id"].string.isEmpty ? row["descriptor"]["id"] == args["function_id"] : row["descriptor"]["name"].string.lowercased() == query }
          return row.pretty.lowercased().contains(query) && (args["category"].string.isEmpty || row["category"] == args["category"])
        }
        data = .object(["metadata_version": catalog["metadata_version"], "functions": .array(Array(matches.prefix(name == "search_functions" ? min(20, max(1, args["limit"].isNull ? 8 : Int(args["limit"].double))) : 1)))])
      case "preview_source":
        let before = try current(args["snapshot_ref"].string, controller), input = args["input"]
        var after = before, diagnostics: [JSONValue] = []
        if input["kind"].string == "patch" { after = try MobileNotebookPatch.apply(input["operations"].array, to: before) }
        let sources: [(String, String)] = input["kind"].string == "source" ? [(input["source"].string, input["dialect"].string)] : after.cells.filter { cell in cell.kind == .Math && !before.cells.contains(cell) }.map { ($0.source, $0.dialect.rawValue) }
        for (source, dialect) in sources {
          let reply = try await controller.call(.object(["type": .string("preview"), "source": .string(source), "dialect": .string(dialect)]))
          diagnostics += reply.response.body["diagnostics"].array
        }
        let valid = !diagnostics.contains { $0["severity"].string.lowercased() == "error" }
        let ref = "preview-" + UUID().uuidString.lowercased()
        if valid, input["kind"].string == "patch" { previews[ref] = (before, after) }
        data = .object(["valid": .bool(valid), "preview_ref": valid ? .string(ref) : .null, "diagnostics": .array(diagnostics)])
      case "apply_notebook_patch":
        guard let preview = previews.removeValue(forKey: args["preview_ref"].string), preview.before == controller.notebook else { throw KernelError.message("修改预览已过期") }
        let transaction = MobileSourceTransaction(id: "transaction-" + UUID().uuidString.lowercased(), before: preview.before, after: preview.after, state: "pending")
        try agent.appendTransaction(transaction)
        try await controller.commitAgentNotebook(preview.after, expected: preview.before)
        try agent.finishTransaction(transaction.id, state: "applied")
        data = .object(["transaction_ref": .string(transaction.id), "snapshot_ref": .string(try snapshot(controller.notebook)), "changed_cells": .array(preview.after.cells.filter { !preview.before.cells.contains($0) }.map { .string($0.id) })])
      case "run_cells":
        let file = try current(args["snapshot_ref"].string, controller)
        let metadata = try await controller.call(.object(["type": .string("get_notebook_state")])).response.body["state"]
        let ids = try MobileRunPlanner.plan(state: metadata, requested: args["cell_ids"].array.map(\.string), scope: args["scope"].string.isEmpty ? "required" : args["scope"].string)
        var rows: [JSONValue] = []
        for id in ids {
          try Task.checkCancellation()
          guard controller.notebook == file else { throw KernelError.message("运行期间文稿已改变，请重新读取") }
          guard controller.cells.contains(where: { $0.id == id && $0.input.kind == .Math }) else { throw KernelError.message("找不到数学单元格") }
          await controller.run(id)
          try Task.checkCancellation()
          guard let cell = controller.cells.first(where: { $0.id == id }) else { throw KernelError.message("运行期间文稿已改变") }
          let ref = "result-" + UUID().uuidString.lowercased(); results[ref] = (id, cell.revision, cell.output)
          rows.append(.object(["cell_id": .string(id), "status": .string(cell.status.rawValue), "result_ref": .string(ref)]))
          if cell.status == .Error { status = "partial"; if !args["continue_on_error"].bool { break } }
        }
        data = .object(["cells": .array(rows), "snapshot_ref": .string(try snapshot(controller.notebook))])
      case "inspect_result":
        guard let value = results[args["result_ref"].string], controller.cells.contains(where: { $0.id == value.cell && $0.revision == value.revision && $0.status == .Done }) else { throw KernelError.message("结果引用已过期") }
        let part = args["part"].string
        if part == "diagnostics" { data = value.output["messages"] }
        else if part == "summary" || part.isEmpty {
          data = .object(["cell_id": .string(value.cell), "messages": value.output["messages"], "items": .array(value.output["items"].array.map { item in
            if item["type"].string == "scene3_d" {
              return .object(["type": item["type"], "meshes": .number(Double(item["data"]["meshes"].array.count)), "triangles": .number(Double(item["data"]["meshes"].array.reduce(0) { $0 + $1["triangles"].array.count })), "bounds": item["data"]["bounds"]])
            }
            return .object(["type": item["type"], "input_form": item["input_form"], "latex": item["latex"], "view": item["view"]])
          })])
        } else if part == "geometry" {
          let items = value.output["items"].array
          let limit = max(1, min(100, args["limit"].isNull ? 20 : Int(args["limit"].double)))
          let cursor = args["cursor"].string, prefix = args["result_ref"].string + ":"
          let offset = Int(cursor.split(separator: ":").last ?? "0") ?? 0
          guard cursor.isEmpty || cursor.hasPrefix(prefix), offset >= 0 else { throw KernelError.message("无效结果分页") }
          var rows: [JSONValue] = [], total = 0
          for item in items {
            if item["type"].string == "scene3_d" {
              for (index, mesh) in item["data"]["meshes"].array.enumerated() {
                let points = mesh["positions"].array, start = max(0, offset - total)
                let count = max(0, min(points.count - start, limit - rows.count))
                if count > 0 { for vertex in start..<(start + count) {
                  rows.append(.object(["mesh": .number(Double(index)), "vertex": .number(Double(vertex)), "position": points[vertex], "normal": mesh["normals"][vertex], "color": mesh["colors"][vertex]]))
                } }
                total += points.count
              }
            } else if item["type"].string == "plot" {
              for curve in item["data"]["curves"].array { for segment in curve["segments"].array {
                let points = segment.array, start = max(0, offset - total)
                let count = max(0, min(points.count - start, limit - rows.count))
                if count > 0 { rows += points[start..<(start + count)] }
                total += points.count
              } }
            }
          }
          guard offset <= total else { throw KernelError.message("结果分页超过范围") }
          data = .object(["geometry": .array(rows), "total": .number(Double(total)), "next_cursor": offset + rows.count < total ? .string(prefix + "\(offset + rows.count)") : .null])
        } else if part == "steps" {
          data = .object(["steps": .array(value.output["items"].array.filter { !$0["steps"].isNull }.map { $0["steps"] })])
        } else if part == "value" {
          data = .object(["items": .array(value.output["items"].array.filter { ["expr", "solutions"].contains($0["type"].string) })])
        } else if part == "preview" {
          data = .object(["preview": .string("data_only"), "message": .string("工具返回几何数据；没有注册渲染截图。")])
        } else { data = value.output }
      case "evaluate_scratch":
        _ = try current(args["snapshot_ref"].string, controller)
        guard let client = controller.kernel else { throw KernelError.message("内核尚未就绪") }
        let output = try await client.scratch(source: args["source"].string, dialect: args["dialect"].string, definitions: args["use_notebook_definitions"].isNull || args["use_notebook_definitions"].bool)
        data = .object(["output": output.response.body["output"], "evaluation_status": .string(output.response.body["output"]["messages"].array.contains { $0["level"].string == "Error" } ? "error" : "completed")])
      case "undo_transaction":
        _ = try current(args["snapshot_ref"].string, controller)
        let requested = args["transaction_refs"].array.map(\.string)
        let refs = agent.state.transactions.filter { requested.contains($0.id) }.map(\.id)
        guard Set(refs) == Set(requested), !refs.isEmpty else { throw KernelError.message("撤销事务引用无效") }
        var file = controller.notebook
        for ref in refs.reversed() {
          guard let transaction = agent.state.transactions.first(where: { $0.id == ref && $0.state == "applied" }), transaction.after == file else { throw KernelError.message("撤销与后续编辑冲突") }
          file = transaction.before
        }
        try await controller.commitAgentNotebook(file, expected: controller.notebook)
        for ref in refs { try agent.finishTransaction(ref, state: "undone") }
        data = .object(["snapshot_ref": .string(try snapshot(controller.notebook)), "undone": .array(refs.map(JSONValue.string))])
      case "get_operation_status":
        guard let result = agent.state.operations[args["operation_ref"].string] else { throw KernelError.message("操作引用无效") }; return result
      case "read_attachment", "prepare_attachment":
        data = try agent.attachment(args, prepare: name == "prepare_attachment")
      default: throw KernelError.message("未注册此工具")
      }
      guard (try data.data()).count <= 512 * 1024 else { throw KernelError.message("结果过大，请缩小查询或分页") }
    } catch { failure = error.localizedDescription; status = error is CancellationError ? "cancelled" : "failed" }
    let reply = JSONValue.object(["ok": .bool(failure == nil), "status": .string(status), "request_ref": .string(request), "operation_ref": .string(operation),
      "document": .null, "data": failure == nil ? data : .null,
      "error": failure.map { .object(["code": .string("EXECUTION_FAILED"), "message": .string($0), "recovery": .string("refresh_snapshot"), "details": .object([:])]) } ?? .null,
      "pagination": .object(["truncated": .bool(false), "next_cursor": data["next_cursor"]])])
    do { try agent.recordOperation(operation, name: name, reply: reply) }
    catch { return .object(["ok": .bool(false), "status": .string("unknown"), "request_ref": .string(request), "operation_ref": .string(operation), "data": .null, "error": .object(["message": .string("操作记录未能保存，请检查文稿")])]) }
    return reply
  }
}
