import Foundation

enum NotebookExporter {
  static func escapeTex(_ text: String) -> String {
    text.map { c -> String in
      switch c {
      case "\\": "\\textbackslash{}"
      case "{": "\\{"
      case "}": "\\}"
      case "&": "\\&"
      case "%": "\\%"
      case "$": "\\$"
      case "#": "\\#"
      case "_": "\\_"
      case "^": "\\textasciicircum{}"
      case "~": "\\textasciitilde{}"
      default: String(c)
      }
    }.joined()
  }
  static func formulas(_ cell: NotebookCell) -> [String] {
    guard cell.status == .Done else { return [] }
    return cell.output["items"].array.flatMap { item -> [String] in
      if item["type"].string == "expr" { return [item["latex"].string] }
      guard item["type"].string == "solutions" else { return [] }
      let view = item["view"]
      if !view["region_latex"].string.isEmpty { return [view["region_latex"].string] }
      if view["kind"].string == "none" { return ["\\varnothing"] }
      if view["kind"].string == "all" { return ["\\text{All values}"] }
      return view["solutions"].array.map { s in
        let body = s["bindings"].array.map {
          "\($0["var_latex"].string.isEmpty ? escapeTex($0["var"].string) : $0["var_latex"].string) = \($0["latex"].string)"
        }.joined(separator: ",\\quad ")
        let condition =
          s["condition_display_latex"].string.isEmpty
          ? s["condition_latex"].string : s["condition_display_latex"].string
        return body + (condition.isEmpty ? "" : "\\quad\\text{if } " + condition)
      }
    }
  }
  static func render(title: String, cells: [NotebookCell], format: String) -> String {
    if format == "markdown" {
      return "# \(title)\n\n"
        + cells.map { cell in
          if cell.input.kind == .Text { return cell.input.source + "\n\n" }
          let longest =
            cell.input.source.split(whereSeparator: { $0 != "`" }).map(\.count).max() ?? 0
          let fence = String(repeating: "`", count: max(3, longest + 1))
          return
            "\(fence)\(cell.input.kind == .Math ? "om" : "text")\n\(cell.input.source)\n\(fence)\n\n"
            + formulas(cell).map { "$$\n\($0)\n$$\n\n" }.joined()
        }.joined()
    }
    let body = cells.map {
      "\\begin{quote}\\ttfamily\n\(escapeTex($0.input.source).replacingOccurrences(of: "\n", with: "\n\\par\n"))\n\\end{quote}\n"
        + formulas($0).map { "\\[\($0)\\]\n" }.joined()
    }.joined(separator: "\n")
    return
      "\\documentclass{article}\n\\usepackage{amsmath,amssymb}\n\\title{\(escapeTex(title))}\n\\date{}\n\\begin{document}\n\\maketitle\n\(body)\\end{document}\n"
  }
}
