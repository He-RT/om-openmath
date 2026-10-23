import Foundation

/// Pure source transaction preview. It neither evaluates source nor writes files.
enum MobileNotebookPatch {
  static func apply(_ operations: [JSONValue], to original: NotebookFile) throws -> NotebookFile {
    guard !operations.isEmpty, operations.count <= 64 else { throw KernelError.message("修改数量超过限制") }
    var file = original, inserted: [String: String] = [:]
    func id(_ selector: JSONValue) -> String { selector["cell_id"].string.isEmpty ? inserted[selector["client_key"].string] ?? "" : selector["cell_id"].string }
    for operation in operations {
      let type = operation["type"].string
      let target = id(operation["target"])
      let index = file.cells.firstIndex { $0.id == target }
      if ["update_cell", "replace_text", "delete_cell"].contains(type) {
        guard let index, MobileAgentStore.hash(file.cells[index].source) == operation["expected_source_hash"].string else { throw KernelError.message("源码已改变，请重新读取文稿") }
      }
      switch type {
      case "insert_cell":
        let key = operation["client_key"].string
        guard !key.isEmpty, inserted[key] == nil, let kind = CellKind(rawValue: operation["kind"].string), kind != .Ask,
          let dialect = Dialect(rawValue: operation["dialect"].string) else { throw KernelError.message("无效的新单元格") }
        let cell = CellInput(kind: kind, source: operation["source"].string, dialect: dialect)
        let after = operation["after"]
        let position: Int
        if after.isNull { position = 0 }
        else { guard let i = file.cells.firstIndex(where: { $0.id == id(after) }) else { throw KernelError.message("找不到插入位置") }; position = i + 1 }
        file.cells.insert(cell, at: position); inserted[key] = cell.id
      case "update_cell":
        guard let index else { throw KernelError.message("找不到单元格") }
        file.cells[index].source = operation["source"].string
        if !operation["kind"].isNull { guard let kind = CellKind(rawValue: operation["kind"].string), kind != .Ask else { throw KernelError.message("无效单元格类型") }; file.cells[index].kind = kind }
        if !operation["dialect"].isNull { guard let dialect = Dialect(rawValue: operation["dialect"].string) else { throw KernelError.message("无效方言") }; file.cells[index].dialect = dialect }
      case "replace_text":
        guard let index else { throw KernelError.message("找不到单元格") }
        let match = operation["match"].string, source = file.cells[index].source
        guard !match.isEmpty, source.components(separatedBy: match).count == 2 else { throw KernelError.message("替换目标必须唯一") }
        file.cells[index].source = source.replacingOccurrences(of: match, with: operation["replacement"].string)
      case "delete_cell":
        guard let index else { throw KernelError.message("找不到单元格") }; file.cells.remove(at: index)
      case "move_cell":
        guard let index else { throw KernelError.message("找不到单元格") }
        let after = operation["after"]
        guard after.isNull || id(after) != target else { throw KernelError.message("单元格不能移动到自身之后") }
        let cell = file.cells.remove(at: index)
        let position: Int
        if after.isNull { position = 0 }
        else { guard let i = file.cells.firstIndex(where: { $0.id == id(after) }) else { throw KernelError.message("找不到移动位置") }; position = i + 1 }
        file.cells.insert(cell, at: position)
      case "rename_notebook":
        let title = operation["title"].string
        guard !title.isEmpty, title.count <= 500 else { throw KernelError.message("无效文稿标题") }; file.title = title
      default: throw KernelError.message("不支持此源码操作")
      }
      guard file.cells.count <= 2000, file.cells.allSatisfy({ $0.source.utf8.count <= 65536 }) else { throw KernelError.message("源码超过预算") }
    }
    try file.validate(); return file
  }
}
