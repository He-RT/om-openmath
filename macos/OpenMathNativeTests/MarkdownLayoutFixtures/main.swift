import AppKit
import Foundation

@main struct MarkdownLayoutFixtures {
  @MainActor static func main() async throws {
    let app=NSApplication.shared;app.setActivationPolicy(.accessory)
    let data=try Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[1]))
    let raw=try JSONSerialization.jsonObject(with:data) as! [String:Any]
    let cells=raw["cells"] as! [[String:Any]],source=cells[0]["source"] as! String
    let document=MarkdownRenderDocument.parse(source)
    let sections=document.blocks
    let table=sections.compactMap(\.table).first!
    precondition(table.rows.count==3 && sections.contains{$0.code != nil})
    let first=MarkdownRenderDocument(source:document.source,runs:sections.prefix(while:{$0.table==nil}).compactMap(\.text).flatMap(\.runs),fallback:false)
    let paper=MarkdownTextView(usingTextLayoutManager:true);paper.configure();paper.install(first,dark:false,fontSize:16.5,paper:true)
    let headline=paper.textStorage!.attribute(.font,at:0,effectiveRange:nil) as! NSFont
    precondition(headline.pointSize==30,"paper heading did not use 30 pt title")
    precondition(headline.fontDescriptor.object(forKey:.cascadeList) != nil,"Chinese paper font cascade missing")
    let view=MarkdownTextView(usingTextLayoutManager:true);view.configure();view.install(first,dark:true)
    guard view.textLayoutManager != nil else {fatalError("TextKit 2 unavailable")}
    let height=view.measuredHeight(width:820)
    let window=NSWindow(contentRect:NSRect(x:0,y:0,width:820,height:700),styleMask:[.titled,.resizable],backing:.buffered,defer:false)
    window.contentView=view;window.makeKeyAndOrderFront(nil);window.displayIfNeeded()
    try await Task.sleep(for:.milliseconds(100))
    precondition(view.textLayoutManager != nil,"reading surface unexpectedly fell back to TextKit 1")
    let attached=view.measuredHeight(width:820),narrow=view.measuredHeight(width:300)
    precondition(attached>200 && abs(attached-height)<2 && narrow>=attached,"native Markdown height lost paragraphs")
    let board=NSPasteboard.general,old=board.pasteboardItems?.map {item ->[NSPasteboard.PasteboardType:Data] in Dictionary(uniqueKeysWithValues:item.types.compactMap{type in item.data(forType:type).map{(type,$0)}})} ?? []
    defer {
      let restored=old.map {values ->NSPasteboardItem in let item=NSPasteboardItem();for (type,data) in values {item.setData(data,forType:type)};return item}
      board.clearContents();board.writeObjects(restored)
    }
    view.setSelectedRange(NSRange(location:0,length:view.textStorage!.length));view.copy(nil)
    let copied=board.string(forType:.string)!
    precondition(copied.contains("$x^2 - 5x + 6 = 0$") && copied.contains(#"\(x=2\)"#) && copied.contains("普通文字"),"formula selection copy lost original delimiters")
    let original=view.string
    let changed=MarkdownRenderDocument.parse("replacement")
    view.install(changed,dark:true);precondition(view.string==original,"selected old projection was replaced")
    view.setSelectedRange(NSRange(location:0,length:0))
    try await Task.sleep(for:.milliseconds(20))
    precondition(view.string.contains("replacement"),"new projection was never installed after selection ended")
    precondition(table.tsv.contains("第一个解\t$2$"))
    let code=NativeCodeScrollView(frame:NSRect(x:0,y:0,width:250,height:100))
    let ascii="    ____________________________\n   /    o    o    o    o        \\\n  /_____________________________\\\n"+String(repeating:"wide_",count:80)
    code.install(ascii);code.layoutSubtreeIfNeeded()
    precondition(code.code.string==ascii && code.code.frame.width>code.contentSize.width)
    precondition(code.code.textContainer?.widthTracksTextView==false && code.hasHorizontalScroller)
    code.code.selectAll(nil);code.code.copy(nil)
    precondition(board.string(forType:.string)==ascii,"code copy changed whitespace or wrapped lines")
    print("Actual TextKit 2 height \(attached), narrow \(narrow); continuous copy with original formulas, pending selection update and independent table segmentation passed.")
    window.orderOut(nil)
  }
}
