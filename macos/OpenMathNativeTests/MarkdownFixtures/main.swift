import Foundation

@main struct MarkdownFixtures {
  static func text(_ document:MarkdownRenderDocument)->String {document.runs.map{if case .text(let value)=$0.content {return value};return "[MATH]"}.joined()}
  static func math(_ document:MarkdownRenderDocument)->[String] {document.runs.compactMap{if case .math(let value,_,_)=$0.content {return value};return nil}}
  static func main() {
    let source=#"""
    # 中文🙂 heading
    **bold _both_** and ~~strike~~, $x^2$ and \(\frac{1}{2}\).
    `code $literal$` and [link](https://example.com/$literal$).
    <span>$literal$</span>
    ![alt $literal$](https://example.com/p.png)

    ```math
    $literal$ + \(literal\)
    ```

    $$
    x = 2
    $$

    | 名称 | 数值 |
    |---|---|
    | a | $y$ |

    OMMATHPLACEHOLDER0END costs $5 and $10. Escaped \$x\$; unfinished \(x.
    """#
    let actual=MarkdownRenderDocument.parse(source)
    let prefix="首段 $x^2$。\n\n"
    let streamingBefore=MarkdownRenderDocument.parse(prefix+"正在写"),streamingAfter=MarkdownRenderDocument.parse(prefix+"正在写下一段，还没有结束")
    precondition(streamingBefore.blocks[0].text!.sourceDigest==streamingAfter.blocks[0].text!.sourceDigest,"appending reinstalled a stable Markdown block")
    precondition(streamingBefore.blocks[1].text!.sourceDigest != streamingAfter.blocks[1].text!.sourceDigest,"changed tail did not invalidate its presentation")
    let unresolved=MarkdownRenderDocument.parse("[说明][link]\n\n正文"),resolved=MarkdownRenderDocument.parse("[说明][link]\n\n正文\n\n[link]: https://example.com\n")
    precondition(unresolved.blocks[0].text!.sourceDigest != resolved.blocks[0].text!.sourceDigest,"resolved link did not invalidate earlier semantics")
    precondition(actual.firstOutlineHeading=="中文🙂 heading")
    let fenced=actual.blocks.compactMap(\.code)
    precondition(fenced.count==1 && fenced[0].language=="math" && fenced[0].code.contains("$literal$"))
    if let span=fenced[0].sourceRange {
      precondition(String(decoding:Array(source.utf8)[span],as:UTF8.self)==fenced[0].original)
      precondition(fenced[0].original.contains("```math"),"code source mapping lost original fence")
    } else {preconditionFailure("code source span missing")}
    precondition(MarkdownRenderDocument.parse("```openmath\n2+2\n```\n").blocks.compactMap(\.code).first?.isOpenMath==true)
    precondition(MarkdownRenderDocument.parse("```\n# hidden\n```\n\n## **图形** 与 $x$\n正文\n# second").firstOutlineHeading=="图形 与 $x$")
    precondition(MarkdownRenderDocument.parse("### not a group\nplain").firstOutlineHeading==nil)
    precondition(!actual.fallback)
    precondition(math(actual)==["x^2",#"\frac{1}{2}"#,"\nx = 2\n","y"],"math delimiter/code/URL/image protection mismatch: \(math(actual))")
    precondition(actual.runs.contains{if case .text("both")=$0.content {return $0.style.bold && $0.style.italic};return false})
    precondition(actual.runs.contains{$0.style.strike})
    precondition(actual.runs.contains{$0.style.table?.row==1 && $0.style.table?.column==1})
    precondition(text(actual).contains("OMMATHPLACEHOLDER0END costs $5 and $10."))
    precondition(text(actual).contains(#"unfinished \(x."#))
    precondition(text(actual).contains("code $literal$"))
    precondition(text(actual).contains("[图片未加载：alt $literal$]"))
    let original=Array(source.utf8)
    for run in actual.runs {
      if case .math(_,let raw,_) = run.content {
        let span=run.source!;precondition(String(decoding:original[span],as:UTF8.self)==raw)
      }
    }
    for partial in [#"$x"#,#"\(x"#,"$$\nx",#"`$x$`"#,"A $5 and $10",#"\$x\$"#] {precondition(math(.parse(partial)).isEmpty,partial)}
    precondition(math(.parse(#"before $$x$$ after"#)).isEmpty,"display math must be a separate block")
    let nested=MarkdownRenderDocument.parse("- **outer _inner_**\n  - nested `code`\n\n&gt; &amp; repeated 中文 中文")
    precondition(text(nested).contains("outer inner") && text(nested).contains("> & repeated 中文 中文"))
    precondition(MarkdownRenderDocument.parse(String(repeating:"x",count:1024*1024+1)).fallback)
    print("Native Markdown parser: nested styles, exact Unicode formula spans, protected code/URLs/images, inert HTML, tables, currency/escape/unclosed tails and source fallback passed.")
  }
}
