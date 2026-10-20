import Foundation
import CryptoKit

enum SourceIndexError:Error {case invalidUTF8,invalidRange,notScalarBoundary,staleSource,tooLarge}
struct SourceLineStart:Sendable {let utf8:Int;let utf16:Int}
/// Exact scalar boundaries in the original bytes. Grapheme/visual columns never define an edit.
struct SourceIndexMap:Sendable {
  let source:String
  let sourceHash:String
  private(set) var draftSequence:UInt64
  let utf8Count:Int,utf16Count:Int
  let lineStarts:[SourceLineStart]
  let preferredNewline:String
  private let bytes:[Int],units:[Int]
  init(utf8:Data,draftSequence:UInt64) throws {
    guard let source=String(data:utf8,encoding:.utf8),Data(source.utf8)==utf8 else {throw SourceIndexError.invalidUTF8}
    try self.init(source:source,draftSequence:draftSequence)
  }
  init(source:String,draftSequence:UInt64) throws {
    let byteCount=source.utf8.count,unitCount=source.utf16.count
    guard byteCount<=Int(UInt32.max),unitCount<=Int(UInt32.max) else {throw SourceIndexError.tooLarge}
    self.source=source;self.draftSequence=draftSequence
    sourceHash=SHA256.hash(data:Data(source.utf8)).map {String(format:"%02x",$0)}.joined()
    var byte=0,unit=0,bytes=[0],units=[0],lines=[SourceLineStart(utf8:0,utf16:0)]
    bytes.reserveCapacity(unitCount+1);units.reserveCapacity(unitCount+1)
    var wasCR=false,preference:String?
    for scalar in source.unicodeScalars {
      byte+=scalar.utf8.count;unit+=scalar.value>0xffff ? 2 : 1
      bytes.append(byte);units.append(unit)
      if scalar.value==10 {
        if wasCR {
          lines[lines.count-1] = .init(utf8:byte,utf16:unit)
          if preference=="\r" && lines.count==2 {preference="\r\n"}
        } else {lines.append(.init(utf8:byte,utf16:unit));if preference==nil {preference="\n"}}
      } else if scalar.value==13 {lines.append(.init(utf8:byte,utf16:unit));if preference==nil {preference="\r"}}
      wasCR=scalar.value==13
    }
    self.bytes=bytes;self.units=units;lineStarts=lines;preferredNewline=preference ?? "\n"
    utf8Count=byte;utf16Count=unit
  }
  private func boundary(_ value:Int,in table:[Int])throws->Int {
    guard value>=0,value<=table.last! else {throw SourceIndexError.invalidRange}
    var lo=0,hi=table.count
    while lo<hi {let mid=lo+(hi-lo)/2;if table[mid]<value {lo=mid+1} else {hi=mid}}
    guard lo<table.count,table[lo]==value else {throw SourceIndexError.notScalarBoundary};return lo
  }
  func utf8Offset(forUTF16 offset:Int)throws->Int {bytes[try boundary(offset,in:units)]}
  func utf16Offset(forUTF8 offset:Int)throws->Int {units[try boundary(offset,in:bytes)]}
  func byteRange(for range:NSRange)throws->Range<Int> {
    guard range.location>=0,range.length>=0,range.location<=utf16Count,range.length<=utf16Count-range.location else {throw SourceIndexError.invalidRange}
    return try utf8Offset(forUTF16:range.location)..<utf8Offset(forUTF16:range.location+range.length)
  }
  func nativeRange(start:Int,end:Int)throws->NSRange {
    guard start>=0,end>=start,end<=utf8Count else {throw SourceIndexError.invalidRange}
    let first=try utf16Offset(forUTF8:start),last=try utf16Offset(forUTF8:end)
    return NSRange(location:first,length:last-first)
  }
  func validate(sourceHash:String,draftSequence:UInt64)throws {
    guard self.sourceHash==sourceHash,self.draftSequence==draftSequence else {throw SourceIndexError.staleSource}
  }
  mutating func bindSequence(_ sequence:UInt64) {draftSequence=sequence}
  /// One-based logical line and native UTF16 column, for presentation only. Tabs stay one unit.
  func position(forUTF8 offset:Int)throws->(line:Int,column:Int) {
    let unit=try utf16Offset(forUTF8:offset)
    var lo=0,hi=lineStarts.count
    while lo<hi {let mid=lo+(hi-lo)/2;if lineStarts[mid].utf8<=offset {lo=mid+1} else {hi=mid}}
    let row=max(0,lo-1);return (row+1,unit-lineStarts[row].utf16+1)
  }
  /// A later fix must refuse a grapheme-cutting edit if native text storage cannot preserve it.
  /// This does not change any scalar byte boundary or expand the requested replacement.
  func isComposedRange(_ range:NSRange)throws->Bool {
    _ = try byteRange(for:range)
    let ns=source as NSString
    if range.length==0 {
      if range.location==0 || range.location==utf16Count {return true}
      return ns.rangeOfComposedCharacterSequence(at:range.location).location==range.location
    }
    return ns.rangeOfComposedCharacterSequences(for:range)==range
  }
}
