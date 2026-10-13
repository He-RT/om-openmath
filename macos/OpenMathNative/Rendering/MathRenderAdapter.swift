import AppKit
import CryptoKit
import SwiftMath

struct NativeMathRequest:Sendable,Equatable {
  let identity:String
  let originalLatex:String
  let wolfram:String
  let modern:String?
  var inline=false
  var fontSize:CGFloat=22
  var dark=false
  static func ==(a:Self,b:Self)->Bool {
    a.identity.utf8.elementsEqual(b.identity.utf8) && a.originalLatex.utf8.elementsEqual(b.originalLatex.utf8)
      && a.wolfram.utf8.elementsEqual(b.wolfram.utf8) && a.modern.map{Data($0.utf8)}==b.modern.map{Data($0.utf8)}
      && a.inline==b.inline && (a.fontSize==b.fontSize || (a.fontSize.isNaN && b.fontSize.isNaN)) && a.dark==b.dark
  }
}
struct NativeMathCacheKey:Hashable,Sendable {
  let latexHash:String,transformVersion:UInt32,inline:Bool,fontName:String,fontHash:String,fontSize:CGFloat,dark:Bool
}
enum NativeMathState:String,Sendable {
  case ready,byteBudget,depthBudget,atomBudget,parseFailure,fontUnavailable,invalidSize
  var reason:String {
    switch self {
    case .ready:""
    case .byteBudget:"公式较长，已保留原式"
    case .depthBudget:"公式嵌套超出排版预算，已保留原式"
    case .atomBudget:"公式元素超出排版预算，已保留原式"
    case .parseFailure:"此公式暂不支持排版，已保留原式"
    case .fontUnavailable:"数学字体不可用，已保留原式"
    case .invalidSize:"字号不可用，已保留原式"
    }
  }
}
@MainActor struct NativeMathPlan {
  let request:NativeMathRequest
  let key:NativeMathCacheKey
  let displayLatex:String
  let transformedOperators:Int
  let state:NativeMathState
  let atoms:Int
  let list:MTMathList?
  let font:MTFont?
}
enum NativeMathBudgetError:Error {case bytes,depth,atoms,syntax}

/// Package resource discovery is deliberately local to the current executable/App. A missing
/// installed font cannot borrow a checkout, user's font directory or developer build path.
enum NativeMathFonts {
  static let face="latinmodern-math"
  static let fontSHA="6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f"
  static let tableSHA="201e6a483783415f335328f2d02b356fe55cd478eb0cd052898236589c8cb946"
  static func directory(in bundle:Bundle = .main)->URL? {
    for parent in [bundle.resourceURL,bundle.bundleURL].compactMap({$0}) {
      let path=parent.appendingPathComponent("SwiftMath_SwiftMath.bundle/mathFonts.bundle")
      if FileManager.default.fileExists(atPath:path.path) {return path}
      let mac=parent.appendingPathComponent("SwiftMath_SwiftMath.bundle/Contents/Resources/mathFonts.bundle")
      if FileManager.default.fileExists(atPath:mac.path) {return mac}
    }
    return nil
  }
  static func digest(in directory:URL?) ->String? {
    guard let directory,let font=try? Data(contentsOf:directory.appendingPathComponent(face+".otf")),
      let table=try? Data(contentsOf:directory.appendingPathComponent(face+".plist")),!font.isEmpty,!table.isEmpty else {return nil}
    func sha(_ data:Data)->String {SHA256.hash(data:data).map {String(format:"%02x",$0)}.joined()}
    guard sha(font)==fontSHA,sha(table)==tableSHA else {return nil}
    var bytes=font;bytes.append(table);return SHA256.hash(data:bytes).map {String(format:"%02x",$0)}.joined()
  }
}

/// This preflight follows the fixed builder's recursive argument shapes before that builder is
/// invoked. It handles unbraced arguments, scripts, font/accent commands, left/right, tables and
/// infix fractions; a brace-only depth check would miss deeply nested unbraced commands.
@MainActor private struct NativeLatexPreflight {
  enum Token:Equatable {case character(UInt32),command(String),open,close}
  enum Stop {case eof,brace,bracket,right,environment}
  let tokens:[Token]
  let maxDepth:Int,maxAtoms:Int
  var index=0,lowerBoundAtoms=0
  init(_ source:String,maxDepth:Int,maxAtoms:Int) {
    self.maxDepth=maxDepth;self.maxAtoms=maxAtoms
    let scalars=Array(source.unicodeScalars);var result:[Token]=[],i=0
    while i<scalars.count {
      let value=scalars[i].value;i+=1
      if value==92 {
        let start=i
        while i<scalars.count && ((65...90).contains(scalars[i].value) || (97...122).contains(scalars[i].value)) {i+=1}
        if i==start,i<scalars.count {result.append(.command(String(scalars[i])));i+=1}
        else {result.append(.command(String(String.UnicodeScalarView(scalars[start..<i]))))}
      } else if value==123 {result.append(.open)}
      else if value==125 {result.append(.close)}
      else {result.append(.character(value))}
    }
    tokens=result
  }
  mutating func check() throws {try list(depth:0,stop:.eof,table:false,textSpaces:false)}
  private func whitespace(_ token:Token)->Bool {if case .character(let code)=token {return code==32 || code==9 || code==10 || code==13};return false}
  private mutating func charge()throws {lowerBoundAtoms+=1;if lowerBoundAtoms>maxAtoms {throw NativeMathBudgetError.atoms}}
  private func depth(_ value:Int)throws {if value>maxDepth {throw NativeMathBudgetError.depth}}
  private mutating func skipSpace() {while index<tokens.count && whitespace(tokens[index]) {index+=1}}
  private mutating func literalGroup() {
    skipSpace();guard index<tokens.count,tokens[index] == .open else {return};index+=1
    while index<tokens.count {let token=tokens[index];index+=1;if token == .close {return}}
  }
  private mutating func boundary() {skipSpace();if index<tokens.count {index+=1}}
  private mutating func argument(depth level:Int,table:Bool,textSpaces:Bool)throws {
    try depth(level);skipSpace();guard index<tokens.count else {throw NativeMathBudgetError.syntax}
    let token=tokens[index]
    if token == .close || token == .character(94) || token == .character(95) || token == .character(38) {throw NativeMathBudgetError.syntax}
    if token == .open {index+=1;try list(depth:level,stop:.brace,table:table,textSpaces:textSpaces)}
    else {index+=1;try atom(token,depth:level,table:table,textSpaces:textSpaces)}
  }
  private mutating func list(depth level:Int,stop:Stop,table:Bool,textSpaces:Bool)throws {
    try depth(level)
    while index<tokens.count {
      let token=tokens[index];index+=1
      switch (stop,token) {
      case (.brace,.close),(.bracket,.character(93)):return
      case (.right,.command("right")):boundary();return
      case (.environment,.command("end")):literalGroup();return
      default:break
      }
      if token == .open {try list(depth:level+1,stop:.brace,table:table,textSpaces:textSpaces)}
      else if token == .character(94) || token == .character(95) {try argument(depth:level+1,table:table,textSpaces:textSpaces)}
      else if case .command(let name)=token,["over","atop","choose","brack","brace"].contains(name) {
        try charge();try list(depth:level+1,stop:stop,table:table,textSpaces:textSpaces);return
      } else if !table && (token == .character(38) || token == .command("\\") || token == .command("cr")) {
        try charge();try list(depth:level+1,stop:stop,table:true,textSpaces:textSpaces);return
      } else {try atom(token,depth:level,table:table,textSpaces:textSpaces)}
    }
  }
  private mutating func atom(_ token:Token,depth level:Int,table:Bool,textSpaces:Bool)throws {
    switch token {
    case .open:try list(depth:level+1,stop:.brace,table:table,textSpaces:textSpaces)
    case .close:break // The actual parser reports mismatched syntax; no substitute parse is returned.
    case .character(let value):
      if value != 38 && (!whitespace(token) || textSpaces) {try charge()}
    case .command(let word):
      if MTMathAtomFactory.fontStyleWithName(word) != nil {
        try argument(depth:level+1,table:table,textSpaces:word=="text")
      } else if MTMathAtomFactory.accent(withName:word) != nil || word=="overline" || word=="underline" {
        try charge();try argument(depth:level+1,table:table,textSpaces:textSpaces)
      } else if word=="frac" || word=="binom" {
        try charge();try argument(depth:level+1,table:table,textSpaces:textSpaces);try argument(depth:level+1,table:table,textSpaces:textSpaces)
      } else if word=="sqrt" {
        try charge()
        if index<tokens.count,tokens[index] == .character(91) {index+=1;try list(depth:level+1,stop:.bracket,table:table,textSpaces:textSpaces)}
        try argument(depth:level+1,table:table,textSpaces:textSpaces)
      } else if word=="left" {try charge();boundary();try list(depth:level+1,stop:.right,table:table,textSpaces:textSpaces)}
      else if word=="begin" {try charge();literalGroup();try list(depth:level+1,stop:.environment,table:true,textSpaces:textSpaces)}
      else if ["color","textcolor","colorbox"].contains(word) {try charge();literalGroup();try argument(depth:level+1,table:table,textSpaces:textSpaces)}
      else if word=="right" {boundary()}
      else if word=="end" {literalGroup()}
      else if MTMathAtomFactory.atom(forLatexSymbol:word) != nil {try charge()}
    }
  }
}

@MainActor enum MathRenderAdapter {
  static let maximumBytes=64*1024,maximumDepth=128,maximumAtoms=20000
  /// A lexical control-word scan, not a global regex: an escaped slash cannot become a command.
  static func displayTransform(_ original:String)->(source:String,count:Int) {
    let bytes=Array(original.utf8);var result:[UInt8]=[],i=0,count=0
    while i<bytes.count {
      guard bytes[i]==92 else {result.append(bytes[i]);i+=1;continue}
      let start=i;i+=1
      let wordStart=i
      while i<bytes.count && ((65...90).contains(bytes[i]) || (97...122).contains(bytes[i])) {i+=1}
      if i==wordStart {if i<bytes.count {i+=1};result.append(contentsOf:bytes[start..<i]);continue}
      let word=String(decoding:bytes[wordStart..<i],as:UTF8.self)
      if word=="operatorname",i<bytes.count,bytes[i]==123 {
        var end=i+1
        while end<bytes.count && bytes[end] != 125 {end+=1}
        let name=bytes[(i+1)..<end]
        if end<bytes.count,let first=name.first,((65...90).contains(first) || (97...122).contains(first)),name.allSatisfy({(65...90).contains($0) || (97...122).contains($0) || (48...57).contains($0) || $0==32}) {
          result.append(contentsOf:Array("\\mathrm".utf8));count+=1;continue
        }
      }
      result.append(contentsOf:bytes[start..<i])
    }
    return (String(decoding:result,as:UTF8.self),count)
  }
  static func preflight(_ display:String,maxDepth:Int=maximumDepth,maxAtoms:Int=maximumAtoms)throws {
    guard display.utf8.count<=maximumBytes else {throw NativeMathBudgetError.bytes}
    var scanner=NativeLatexPreflight(display,maxDepth:maxDepth,maxAtoms:maxAtoms);try scanner.check()
  }
  /// Count actual parsed nodes and all recursive data routes before any native typesetting.
  static func atomCount(_ root:MTMathList,maxDepth:Int=maximumDepth,maxAtoms:Int=maximumAtoms)throws->Int {
    var count=0,pending:[(MTMathList,Int)]=[(root,0)]
    while let (list,depth)=pending.popLast() {
      guard depth<=maxDepth else {throw NativeMathBudgetError.depth}
      for atom in list.atoms {
        count+=1;guard count<=maxAtoms else {throw NativeMathBudgetError.atoms}
        var children=[atom.subScript,atom.superScript].compactMap{$0}
        if let value=atom as? MTFraction {children += [value.numerator,value.denominator].compactMap{$0}}
        if let value=atom as? MTRadical {children += [value.degree,value.radicand].compactMap{$0}}
        if let value=atom as? MTInner {children += [value.innerList].compactMap{$0};count += (value.leftBoundary==nil ? 0 : 1)+(value.rightBoundary==nil ? 0 : 1)}
        if let value=atom as? MTOverLine {children += [value.innerList].compactMap{$0}}
        if let value=atom as? MTUnderLine {children += [value.innerList].compactMap{$0}}
        if let value=atom as? MTAccent {children += [value.innerList].compactMap{$0}}
        if let value=atom as? MTMathColor {children += [value.innerList].compactMap{$0}}
        if let value=atom as? MTMathTextColor {children += [value.innerList].compactMap{$0}}
        if let value=atom as? MTMathColorbox {children += [value.innerList].compactMap{$0}}
        if let value=atom as? MTMathTable {children += value.cells.flatMap{$0}}
        guard count<=maxAtoms else {throw NativeMathBudgetError.atoms}
        pending.append(contentsOf:children.map{($0,depth+1)})
      }
    }
    return count
  }
  static func prepare(_ request:NativeMathRequest,fontDirectory:URL?=NativeMathFonts.directory())->NativeMathPlan {
    let hash=SHA256.hash(data:Data(request.originalLatex.utf8)).map {String(format:"%02x",$0)}.joined()
    let fontHash=NativeMathFonts.digest(in:fontDirectory)
    let key=NativeMathCacheKey(latexHash:hash,transformVersion:1,inline:request.inline,fontName:NativeMathFonts.face,fontHash:fontHash ?? "missing",fontSize:request.fontSize,dark:request.dark)
    var adapted=(source:request.originalLatex,count:0)
    func failed(_ state:NativeMathState)->NativeMathPlan {.init(request:request,key:key,displayLatex:adapted.source,transformedOperators:adapted.count,state:state,atoms:0,list:nil,font:nil)}
    guard request.fontSize.isFinite,request.fontSize>=8,request.fontSize<=96 else {return failed(.invalidSize)}
    guard request.originalLatex.utf8.count<=maximumBytes else {return failed(.byteBudget)}
    adapted=displayTransform(request.originalLatex)
    do {try preflight(adapted.source)} catch NativeMathBudgetError.depth {return failed(.depthBudget)} catch NativeMathBudgetError.atoms {return failed(.atomBudget)} catch NativeMathBudgetError.syntax {return failed(.parseFailure)} catch {return failed(.byteBudget)}
    var error:NSError?
    guard let list=MTMathListBuilder.build(fromString:adapted.source,error:&error),error==nil else {return failed(.parseFailure)}
    let atoms:Int
    do {atoms=try atomCount(list)} catch NativeMathBudgetError.depth {return failed(.depthBudget)} catch {return failed(.atomBudget)}
    guard fontHash != nil,let font=MTFontManager().latinModernFont(withSize:request.fontSize) else {return failed(.fontUnavailable)}
    guard atoms>0 else {return failed(.parseFailure)}
    let measure=MTMathUILabel();measure.font=font;measure.fontSize=request.fontSize;measure.mathList=list
    measure.labelMode=request.inline ? .text : .display;measure.displayErrorInline=false
    let size=measure.fittingSize
    guard size.width.isFinite,size.height.isFinite,size.width>0,size.height>0 else {return failed(.parseFailure)}
    return .init(request:request,key:key,displayLatex:adapted.source,transformedOperators:adapted.count,state:.ready,atoms:atoms,list:list,font:font)
  }
}
