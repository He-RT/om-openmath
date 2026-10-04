import Markdown
import SwiftUI

struct MathMarkdownTokens {
  var source: String
  var formulas: [String: (text: String, display: Bool)]
  init(_ input: String) {
    let chars = Array(input)
    let bytes = Array(input.utf8)
    var lineStarts = [0]
    for (offset, byte) in bytes.enumerated() where byte == 10 { lineStarts.append(offset + 1) }
    var byteOffsets = [0]
    for character in chars { byteOffsets.append(byteOffsets.last! + String(character).utf8.count) }
    var protected: [Range<Int>] = []
    var nodes: [any Markup] = [Markdown.Document(parsing: input)]
    while let node = nodes.popLast() {
      if node is CodeBlock || node is InlineCode, let range = node.range {
        let startLine = range.lowerBound.line - 1
        let endLine = range.upperBound.line - 1
        if lineStarts.indices.contains(startLine), lineStarts.indices.contains(endLine) {
          let start = lineStarts[startLine] + (node is CodeBlock ? 0 : range.lowerBound.column - 1)
          let end =
            node is CodeBlock
            ? (endLine + 1 < lineStarts.count ? lineStarts[endLine + 1] : bytes.count)
            : lineStarts[endLine] + range.upperBound.column - 1
          if start >= 0 && end <= bytes.count && start < end { protected.append(start..<end) }
        }
      } else {
        nodes.append(contentsOf: node.children)
      }
    }
    protected.sort { $0.lowerBound < $1.lowerBound }
    var protectedIndex = 0
    var result = ""
    var math: [String: (String, Bool)] = [:]
    var i = 0
    var lineStart = true
    var fence: Character?
    var fenceCount = 0
    var inlineTicks = 0
    func matches(_ pattern: String, _ index: Int) -> Bool {
      let p = Array(pattern)
      return index + p.count <= chars.count && Array(chars[index..<index + p.count]) == p
    }
    while i < chars.count {
      while protectedIndex < protected.count
        && byteOffsets[i] >= protected[protectedIndex].upperBound
      { protectedIndex += 1 }
      if protectedIndex < protected.count && protected[protectedIndex].contains(byteOffsets[i]) {
        result.append(chars[i])
        lineStart = chars[i] == "\n"
        i += 1
        continue
      }
      if lineStart && (chars[i] == "`" || chars[i] == "~") {
        let c = chars[i]
        var count = 0
        while i + count < chars.count && chars[i + count] == c { count += 1 }
        if count >= 3 {
          if fence == c && count >= fenceCount {
            fence = nil
            fenceCount = 0
          } else if fence == nil {
            fence = c
            fenceCount = count
          }
          while i < chars.count && chars[i] != "\n" {
            result.append(chars[i])
            i += 1
          }
          continue
        }
      }
      if fence != nil {
        result.append(chars[i])
        lineStart = chars[i] == "\n"
        i += 1
        continue
      }
      if chars[i] == "`" {
        var count = 0
        while i + count < chars.count && chars[i + count] == "`" { count += 1 }
        inlineTicks = inlineTicks == count ? 0 : (inlineTicks == 0 ? count : inlineTicks)
        result += String(repeating: "`", count: count)
        i += count
        lineStart = false
        continue
      }
      var delimiter: (String, String, Bool)?
      if inlineTicks == 0 {
        if matches("$$", i) {
          delimiter = ("$$", "$$", true)
        } else if matches("\\[", i) {
          delimiter = ("\\[", "\\]", true)
        } else if matches("\\(", i) {
          delimiter = ("\\(", "\\)", false)
        } else if chars[i] == "$" && (i == 0 || chars[i - 1] != "\\") {
          delimiter = ("$", "$", false)
        }
      }
      if let (open, close, display) = delimiter {
        let start = i + open.count
        var end = start
        while end < chars.count && !matches(close, end) {
          if !display && chars[end] == "\n" { break }
          end += 1
        }
        if end < chars.count && matches(close, end) && end > start {
          let key = "OMMATHPLACEHOLDER\(math.count)END"
          math[key] = (String(chars[start..<end]), display)
          result += key
          i = end + close.count
          lineStart = false
          continue
        }
      }
      result.append(chars[i])
      lineStart = chars[i] == "\n"
      i += 1
    }
    source = result
    formulas = math
  }
}
struct NativeMarkdown: View {
  var source: String
  var onReference: ((String) -> Void)? = nil
  var body: some View {
    let tokens = MathMarkdownTokens(source)
    let document = Markdown.Document(parsing: tokens.source)
    VStack(alignment: .leading, spacing: 12) {
      ForEach(Array(document.children.enumerated()), id: \.offset) { _, node in
        block(node, tokens: tokens)
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
  private func safeURL(_ destination: String?) -> URL? {
    guard let destination, let url = URL(string: destination),
      ["https", "http", "mailto"].contains(url.scheme?.lowercased() ?? "")
    else { return nil }
    return url
  }
  private func plain(_ node: any Markup) -> String {
    if let text = node as? Markdown.Text { return text.string }
    if let code = node as? InlineCode { return code.code }
    if node is SoftBreak || node is LineBreak { return "\n" }
    return node.children.map { plain($0) }.joined()
  }
  @ViewBuilder private func inline(_ node: any Markup, tokens: MathMarkdownTokens) -> some View {
    if let code = node as? InlineCode {
      SwiftUI.Text(code.code).font(.system(.body, design: .monospaced)).padding(3).background(
        .quaternary
      ).textSelection(.enabled)
    } else if let link = node as? Markdown.Link, let url = safeURL(link.destination) {
      Link(destination: url) { textWithMath(plain(link), tokens: tokens) }
    } else if node is Markdown.Strong {
      textWithMath(plain(node), tokens: tokens).bold()
    } else if node is Emphasis {
      textWithMath(plain(node), tokens: tokens).italic()
    } else if node is Strikethrough {
      textWithMath(plain(node), tokens: tokens).strikethrough()
    } else {
      textWithMath(plain(node), tokens: tokens)
    }
  }
  private func textWithMath(_ source: String, tokens: MathMarkdownTokens) -> some View {
    let pattern = "OMMATHPLACEHOLDER[0-9]+END|\\[S[0-9]+(?:\\.[0-9]+)*\\]"
    let regex = try? NSRegularExpression(pattern: pattern)
    let string = source as NSString
    let matches =
      regex?.matches(in: source, range: NSRange(location: 0, length: string.length)) ?? []
    var pieces: [(String, Bool)] = []
    var position = 0
    for match in matches {
      if match.range.location > position {
        pieces.append(
          (
            string.substring(
              with: NSRange(location: position, length: match.range.location - position)), false
          ))
      }
      pieces.append((string.substring(with: match.range), true))
      position = NSMaxRange(match.range)
    }
    if position < string.length { pieces.append((string.substring(from: position), false)) }
    return FlowLayout(spacing: 3) {
      ForEach(Array(pieces.enumerated()), id: \.offset) { _, piece in
        if let formula = tokens.formulas[piece.0] {
          MathView(latex: formula.text, fontSize: 18, inline: !formula.display)
        } else if piece.1, let onReference {
          Button(piece.0) { onReference(String(piece.0.dropFirst().dropLast())) }.buttonStyle(
            .borderless)
        } else {
          SwiftUI.Text(piece.0).textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
        }
      }
    }
  }
  private func block(_ node: any Markup, tokens: MathMarkdownTokens) -> AnyView {
    if let heading = node as? Heading {
      return AnyView(
        textWithMath(plain(heading), tokens: tokens).font(heading.level <= 2 ? .title2 : .headline)
          .fontWeight(
            .semibold
          ).textSelection(.enabled))
    }
    if let code = node as? CodeBlock {
      return AnyView(
        ScrollView(.horizontal) {
          SwiftUI.Text(code.code).font(.system(.body, design: .monospaced)).textSelection(.enabled)
            .padding(12)
        }.background(.quaternary, in: RoundedRectangle(cornerRadius: 10)))
    }
    if node is ThematicBreak { return AnyView(Divider()) }
    if node is BlockQuote {
      return AnyView(
        HStack(alignment: .top) {
          Rectangle().fill(.secondary).frame(width: 3)
          VStack(alignment: .leading, spacing: 8) { children(node, tokens: tokens) }
        }.padding(.leading, 8))
    }
    if node is UnorderedList || node is OrderedList {
      return AnyView(
        VStack(alignment: .leading, spacing: 8) {
          ForEach(Array(node.children.enumerated()), id: \.offset) { index, child in
            HStack(alignment: .top) {
              SwiftUI.Text(node is OrderedList ? "\(index+1)." : "•")
              block(child, tokens: tokens)
            }
          }
        })
    }
    if let table = node as? Markdown.Table {
      return AnyView(
        ScrollView(.horizontal) {
          VStack(alignment: .leading, spacing: 0) {
            tableRow(table.head, tokens: tokens).fontWeight(.semibold)
            Divider()
            ForEach(Array(table.body.children.enumerated()), id: \.offset) { _, row in
              tableRow(row, tokens: tokens)
              Divider()
            }
          }
        }.background(.quaternary.opacity(0.5)))
    }
    if node is Markdown.Image {
      return AnyView(SwiftUI.Text("[\(plain(node))]").foregroundStyle(.secondary))
    }
    if node is Paragraph {
      return AnyView(
        FlowLayout(spacing: 3) {
          ForEach(Array(node.children.enumerated()), id: \.offset) { _, child in
            inline(child, tokens: tokens)
          }
        })
    }
    if let html = node as? HTMLBlock {
      return AnyView(SwiftUI.Text(html.rawHTML).textSelection(.enabled))
    }
    if let html = node as? InlineHTML {
      return AnyView(SwiftUI.Text(html.rawHTML).textSelection(.enabled))
    }
    return AnyView(VStack(alignment: .leading, spacing: 8) { children(node, tokens: tokens) })
  }
  private func tableRow(_ row: any Markup, tokens: MathMarkdownTokens) -> some View {
    HStack(alignment: .top, spacing: 12) {
      ForEach(Array(row.children.enumerated()), id: \.offset) { _, cell in
        FlowLayout(spacing: 3) {
          ForEach(Array(cell.children.enumerated()), id: \.offset) { _, child in
            inline(child, tokens: tokens)
          }
        }
        .frame(width: 180, alignment: .leading).padding(8)
      }
    }
  }
  private func children(_ node: any Markup, tokens: MathMarkdownTokens) -> some View {
    ForEach(Array(node.children.enumerated()), id: \.offset) { _, child in
      block(child, tokens: tokens)
    }
  }
}
struct FlowLayout: Layout {
  var spacing: CGFloat = 5
  func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
    layout(proposal: proposal, subviews: subviews).size
  }
  func placeSubviews(
    in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()
  ) {
    let positions = layout(
      proposal: ProposedViewSize(width: bounds.width, height: nil), subviews: subviews
    ).points
    for (index, view) in subviews.enumerated() {
      view.place(
        at: CGPoint(x: bounds.minX + positions[index].x, y: bounds.minY + positions[index].y),
        proposal: ProposedViewSize(width: bounds.width, height: nil))
    }
  }
  private func layout(proposal: ProposedViewSize, subviews: Subviews) -> (
    size: CGSize, points: [CGPoint]
  ) {
    let width = max(1, proposal.width ?? 1000)
    var x: CGFloat = 0
    var y: CGFloat = 0
    var height: CGFloat = 0
    var points: [CGPoint] = []
    var occupiedWidth: CGFloat = 0
    for view in subviews {
      let size = view.sizeThatFits(ProposedViewSize(width: width, height: nil))
      if x > 0 && x + size.width > width {
        x = 0
        y += height + spacing
        height = 0
      }
      points.append(CGPoint(x: x, y: y))
      x += min(width, size.width) + spacing
      occupiedWidth = max(occupiedWidth, x - spacing)
      height = max(height, size.height)
    }
    return (CGSize(width: occupiedWidth, height: y + height), points)
  }
}
