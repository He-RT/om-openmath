import Foundation

/// Plan against actual kernel-owned definition/use metadata, never source-text guesses.
enum MobileRunPlanner {
  static func plan(state: JSONValue, requested: [String], scope: String) throws -> [String] {
    let cells = state["cells"].array
    let byID = Dictionary(uniqueKeysWithValues: cells.map { ($0["id"].string, $0) })
    guard !requested.isEmpty, requested.count <= 50, requested.allSatisfy({ byID[$0] != nil }) else { throw KernelError.message("运行目标无效") }
    var selected = Set(requested), owners: [String: String] = [:]
    for cell in cells { for symbol in cell["defines"].array.map(\.string) {
      if let owner = owners[symbol], owner != cell["id"].string { throw KernelError.message("存在重复定义，请先修复文稿") }
      owners[symbol] = cell["id"].string
    } }
    if scope == "affected" {
      var changed = true
      while changed {
        changed = false
        let symbols = Set(selected.flatMap { byID[$0]?["defines"].array.map(\.string) ?? [] })
        for cell in cells where !selected.contains(cell["id"].string) {
          if !symbols.isDisjoint(with: cell["uses"].array.map(\.string)) { selected.insert(cell["id"].string); changed = true }
        }
      }
    }
    if scope != "selected" {
      var changed = true
      while changed {
        changed = false
        for id in Array(selected) { for symbol in byID[id]?["uses"].array.map(\.string) ?? [] {
          if let owner = owners[symbol], !selected.contains(owner), byID[owner]?["status"].string != "Done" {
            selected.insert(owner); changed = true
          }
        } }
      }
    }
    let cycles = Set(state["cycles"].array.flatMap { $0.array.isEmpty ? [$0.string] : $0.array.map(\.string) })
    guard cycles.isDisjoint(with: selected) else { throw KernelError.message("运行目标依赖循环，请先修复文稿") }
    var order = state["definition_order"].array.map(\.string).filter { selected.contains($0) }
    order += cells.map { $0["id"].string }.filter { selected.contains($0) && !order.contains($0) }
    return order
  }
}
