import Foundation
import CryptoKit
import Markdown

struct MarkdownSourceMapping:Sendable {
  let source:Range<Int>?
  let rendered:NSRange
  let mapping:MarkdownSpanMapping
}
struct MarkdownTextStyle:Sendable {
  var bold=false,italic=false,strike=false,code=false
  var heading=0,indent=0,quote=0
  var link:String?
  var table:MarkdownTableCell?
}
struct MarkdownTableCell:Sendable {let id:Int,row:Int,column:Int,columns:Int;let header:Bool}
struct MarkdownRenderRun:Sendable {
  enum Content:Sendable {case text(String),math(latex:String,original:String,display:Bool)}
  let content:Content,style:MarkdownTextStyle,source:Range<Int>?
}
struct MarkdownRenderDocument:Sendable {
  let source:String,runs:[MarkdownRenderRun],fallback:Bool
  var projectionID=""
  var sourceDigest=""
  /// Use parsed heading runs, so fenced code, escaped hashes and inline markup stay correct.
  var firstOutlineHeading:String? {
    var title="",level:Int?
    for run in runs {
      if level==nil {
        guard run.style.heading==1 || run.style.heading==2 else {continue}
        level=run.style.heading
      }
      guard run.style.heading==level else {break}
      switch run.content {
      case .text(let text):title+=text
      case .math(_,let original,_):title+=original
      }
      if title.contains("\n") {break}
    }
    let trimmed=title.trimmingCharacters(in:.whitespacesAndNewlines)
    return trimmed.isEmpty ? nil:trimmed
  }
  static func parse(_ source:String)->Self {
    var result:Self
    if source.utf8.count>1024*1024 {result = .init(source:source,runs:[.init(content:.text(source),style:.init(code:true),source:0..<source.utf8.count)],fallback:true)}
    else {var parser=MarkdownDocumentParser(source:source);result=parser.render()}
    result.sourceDigest=SHA256.hash(data:Data(source.utf8)).map{String(format:"%02x",$0)}.joined()
    return result

  }
}

/// Unique owned tokens carry exact original byte spans. No user-supplied placeholder or regex
/// can select a formula; AST source coordinates map through the explicit replacement segments.
private struct MarkdownMathToken {
  let key:String,latex:String,original:String,display:Bool,source:Range<Int>,literal:Bool
}
private struct MarkdownReplacement {let masked:Range<Int>,source:Range<Int>;let atomic:Bool}
private struct MarkdownDocumentParser {
  let source:String
  private var bytes:[UInt8],lineStarts:[Int]
  private var protected:[Range<Int>]=[]
  private var tokens:[String:MarkdownMathToken]=[:]
  private var htmlOpen:[String:[Int]]=[:]
  private var replacements:[MarkdownReplacement]=[]
  private var masked="",maskedLines:[Int]=[],tokenPrefix=""
  private var runs:[MarkdownRenderRun]=[]
  private var nodes=0,tableID=0,exceeded=false
  init(source:String) {
    self.source=source;bytes=Array(source.utf8);lineStarts=[0]
    for (i,byte) in bytes.enumerated() where byte==10 {lineStarts.append(i+1)}
  }
  mutating func render()->MarkdownRenderDocument {
    let first=Markdown.Document(parsing:source)
    protect(first,depth:0)
    for starts in htmlOpen.values {for start in starts {protected.append(start..<bytes.count)}}
    guard !exceeded else {return fallback()}
    scanMath()
    maskedLines=[0]
    for (i,b) in masked.utf8.enumerated() where b==10 {maskedLines.append(i+1)}
    nodes=0
    renderNode(Markdown.Document(parsing:masked),style:.init(),depth:0)
    if exceeded {return fallback()}
    return .init(source:source,runs:runs,fallback:false)
  }
  private func fallback()->MarkdownRenderDocument {.init(source:source,runs:[.init(content:.text(source),style:.init(code:true),source:0..<bytes.count)],fallback:true)}
  private func range(_ node:any Markup,lines:[Int],length:Int)->Range<Int>? {
    guard let r=node.range,lines.indices.contains(r.lowerBound.line-1),lines.indices.contains(r.upperBound.line-1) else {return nil}
    let a=lines[r.lowerBound.line-1]+r.lowerBound.column-1,b=lines[r.upperBound.line-1]+r.upperBound.column-1
    guard a>=0,b>=a,b<=length else {return nil};return a..<b
  }
  private mutating func protect(_ node:any Markup,depth:Int) {
    nodes+=1;guard depth<128,nodes<=20000 else {exceeded=true;return}
    // Protect complete link/image syntax as well as code/HTML. Labels still render their AST
    // styles, while dollars inside destinations can never turn into an attachment.
    if node is CodeBlock || node is InlineCode || node is Markdown.Link || node is Markdown.Image || node is HTMLBlock || node is InlineHTML {
      if var span=range(node,lines:lineStarts,length:bytes.count) {
        if node is CodeBlock {let start=bytes[..<span.lowerBound].lastIndex(of:10).map{$0+1} ?? 0;span=start..<span.upperBound}
        protected.append(span)
        if let html=node as? InlineHTML {
          let raw=html.rawHTML.lowercased(),closing=raw.hasPrefix("</")
          let name=String(raw.dropFirst(closing ? 2:1).prefix { $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "-" || $0 == ":") })
          if !name.isEmpty {
            if closing,let start=htmlOpen[name]?.popLast() {protected.append(start..<span.upperBound)}
            else if !closing,!raw.hasSuffix("/>"),!["br","hr","img","input","meta","link","wbr"].contains(name) {htmlOpen[name,default:[]].append(span.lowerBound)}
          }
        }
      }
    } else {for child in node.children {protect(child,depth:depth+1)}}
  }
  private func escaped(_ i:Int)->Bool {
    var j=i,count=0;while j>0,bytes[j-1]==92 {count+=1;j-=1};return count%2==1
  }
  private func white(_ b:UInt8)->Bool {[9,10,13,32].contains(b)}
  private func standalone(_ start:Int,_ end:Int)->Bool {
    let a=bytes[..<start].lastIndex(of:10).map{$0+1} ?? 0
    let b=bytes[end...].firstIndex(of:10) ?? bytes.count
    return bytes[a..<start].allSatisfy(white) && bytes[end..<b].allSatisfy(white)
  }
  private mutating func scanMath() {
    protected.sort{$0.lowerBound<$1.lowerBound}
    var prefix="OMNATIVEMATH"+UUID().uuidString.replacingOccurrences(of:"-",with:"")+"X"
    while source.contains(prefix) {prefix+="X"}
    tokenPrefix=prefix
    var index=0,copied=0,protectedIndex=0
    func matches(_ pattern:[UInt8],_ i:Int)->Bool {i+pattern.count<=bytes.count && bytes[i..<i+pattern.count].elementsEqual(pattern)}
    while index<bytes.count,tokens.count<2048 {
      while protectedIndex<protected.count,protected[protectedIndex].upperBound<=index {protectedIndex+=1}
      if protectedIndex<protected.count,protected[protectedIndex].contains(index) {index=protected[protectedIndex].upperBound;continue}
      guard !escaped(index) else {index+=1;continue}
      let open:[UInt8],close:[UInt8],display:Bool
      if matches([36,36],index) {open=[36,36];close=open;display=true}
      else if matches([92,91],index) {open=[92,91];close=[92,93];display=true}
      else if matches([92,40],index) {open=[92,40];close=[92,41];display=false}
      else if bytes[index]==36 {open=[36];close=open;display=false}
      else {index+=1;continue}
      let begin=index+open.count
      guard begin<bytes.count,!white(bytes[begin]) || display else {index+=open.count;continue}
      let nextProtected=protectedIndex<protected.count ? protected[protectedIndex].lowerBound:bytes.count
      var end=begin
      while end<bytes.count {
        if !display,[10,13].contains(bytes[end]) {break}
        if end>=nextProtected {break}
        if matches(close,end),!escaped(end) {break}
        end+=1
      }
      let finish=end+close.count
      let valid=end>begin && matches(close,end) && end-begin<=16384 &&
        (!white(bytes[end-1]) || display) &&
        !(open==[36] && finish<bytes.count && (48...57).contains(bytes[finish])) &&
        (!display || standalone(index,finish))
      if !valid {
        if open.first==92 {
          let stop=min(bytes.count,max(begin,end))
          appendUnchanged(copied..<index)
          let token=MarkdownMathToken(key:prefix+String(tokens.count)+"END",latex:"",original:String(decoding:bytes[index..<stop],as:UTF8.self),display:false,source:index..<stop,literal:true)
          let offset=masked.utf8.count;masked+=token.key
          replacements.append(.init(masked:offset..<offset+token.key.utf8.count,source:token.source,atomic:true));tokens[token.key]=token
          index=stop;copied=stop;continue
        }
        index+=open.count;continue
      }
      appendUnchanged(copied..<index)
      let token=MarkdownMathToken(key:prefix+String(tokens.count)+"END",latex:String(decoding:bytes[begin..<end],as:UTF8.self),original:String(decoding:bytes[index..<finish],as:UTF8.self),display:display,source:index..<finish,literal:false)
      let offset=masked.utf8.count;masked+=token.key
      replacements.append(.init(masked:offset..<offset+token.key.utf8.count,source:token.source,atomic:true));tokens[token.key]=token
      index=finish;copied=finish
    }
    appendUnchanged(copied..<bytes.count)
  }
  private mutating func appendUnchanged(_ span:Range<Int>) {
    guard !span.isEmpty else {return}
    let offset=masked.utf8.count;masked+=String(decoding:bytes[span],as:UTF8.self)
    replacements.append(.init(masked:offset..<offset+span.count,source:span,atomic:false))
  }
  private func originalRange(_ node:any Markup)->Range<Int>? {
    guard let span=range(node,lines:maskedLines,length:masked.utf8.count) else {return nil}
    func boundary(_ offset:Int,end:Bool)->Int? {
      if offset==masked.utf8.count {return bytes.count}
      guard let segment=replacements.first(where:{$0.masked.contains(offset) || end && $0.masked.upperBound==offset}) else {return nil}
      if segment.atomic {
        if offset==segment.masked.lowerBound {return segment.source.lowerBound}
        if offset==segment.masked.upperBound {return segment.source.upperBound}
        return nil
      }
      return segment.source.lowerBound+offset-segment.masked.lowerBound
    }
    guard let a=boundary(span.lowerBound,end:false),let b=boundary(span.upperBound,end:true),a<=b else {return nil}
    return a..<b
  }
  private mutating func text(_ value:String,style:MarkdownTextStyle,source:Range<Int>?) {if !value.isEmpty {runs.append(.init(content:.text(value),style:style,source:source))}}
  private mutating func leaf(_ value:String,node:any Markup,style:MarkdownTextStyle) {
    let span=originalRange(node)
    var cursor=value.startIndex
    while cursor<value.endIndex {
      guard let start=value.range(of:tokenPrefix,range:cursor..<value.endIndex),
        let suffix=value.range(of:"END",range:start.upperBound..<value.endIndex),
        let token=tokens[String(value[start.lowerBound..<suffix.upperBound])] else {
        text(String(value[cursor...]),style:style,source:cursor==value.startIndex ? span:nil);return
      }
      let r=start.lowerBound..<suffix.upperBound
      if cursor<r.lowerBound {text(String(value[cursor..<r.lowerBound]),style:style,source:nil)}
      if token.literal {text(token.original,style:style,source:token.source)}
      else {runs.append(.init(content:.math(latex:token.latex,original:token.original,display:token.display),style:style,source:token.source))}
      cursor=r.upperBound
    }
  }
  private mutating func renderNode(_ node:any Markup,style:MarkdownTextStyle,depth:Int) {
    nodes+=1;guard depth<128,nodes<=20000 else {exceeded=true;return}
    var next=style
    if let heading=node as? Heading {next.heading=heading.level;next.bold=true}
    if node is Strong {next.bold=true}
    if node is Emphasis {next.italic=true}
    if node is Strikethrough {next.strike=true}
    if node is BlockQuote {next.quote+=1;next.indent+=1}
    if let link=node as? Markdown.Link {next.link=link.destination}
    if let value=node as? Markdown.Text {leaf(value.string,node:node,style:next);return}
    if let value=node as? InlineCode {next.code=true;text(value.code,style:next,source:originalRange(node));return}
    if let value=node as? CodeBlock {next.code=true;text(value.code,style:next,source:originalRange(node));text("\n",style:next,source:nil);return}
    if let value=node as? InlineHTML {next.code=true;text(value.rawHTML,style:next,source:originalRange(node));return}
    if let value=node as? HTMLBlock {next.code=true;text(value.rawHTML+"\n",style:next,source:originalRange(node));return}
    if node is SoftBreak {text("\n",style:next,source:originalRange(node));return}
    if node is LineBreak {text("\n",style:next,source:originalRange(node));return}
    if node is ThematicBreak {text("────────────\n",style:next,source:originalRange(node));return}
    if let image=node as? Markdown.Image {
      text("[图片未加载："+image.plainText+"]"+(image.source.map{" ("+$0+")"} ?? ""),style:next,source:originalRange(node));return
    }
    if node is UnorderedList || node is OrderedList {
      next.indent+=1
      let first=(node as? OrderedList)?.startIndex ?? 1
      for (index,child) in node.children.enumerated() {
        text(node is OrderedList ? "\(first+UInt(index)).\t":"•\t",style:next,source:nil)
        renderNode(child,style:next,depth:depth+1)
      }
      return
    }
    if let table=node as? Markdown.Table {
      tableID+=1;let current=tableID
      let rows:[any Markup]=[table.head]+Array(table.body.children)
      for (rowIndex,row) in rows.enumerated() {
        for (column,cell) in row.children.enumerated() {
          var cellStyle=next;cellStyle.table = .init(id:current,row:rowIndex,column:column,columns:table.maxColumnCount,header:rowIndex==0)
          if rowIndex==0 {cellStyle.bold=true}
          for child in cell.children {renderNode(child,style:cellStyle,depth:depth+1)}
          text("\n",style:cellStyle,source:nil)
        }
      }
      return
    }
    for child in node.children {renderNode(child,style:next,depth:depth+1)}
    if node is Paragraph || node is Heading {text(next.indent>0 ? "\n":"\n\n",style:next,source:nil)}
  }
}
