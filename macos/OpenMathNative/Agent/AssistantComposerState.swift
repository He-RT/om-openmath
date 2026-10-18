import Foundation
import CoreFoundation

struct AssistantCellReference:Identifiable,Codable,Equatable,Sendable {
  let id:String,ordinal:Int,summary:String,sourceHash:String
  var label:String {"单元格 \(ordinal)"}
}
enum AssistantSendShortcut:String,CaseIterable {case enter,commandReturn}
struct AssistantAttachment:Identifiable,Codable,Equatable,Sendable {
  enum Kind:String,Codable,Sendable {case image,pdf,other}
  let id:String,name:String,original:BlobDescriptor,mediaType:String,kind:Kind
  var image:BlobDescriptor?,text:BlobDescriptor?,thumbnail:Data?,note:String?
  var owner:BlobOwner {.init(kind:.attachment,id:id)}
  var sendable:Bool {image != nil || text != nil}
  func explanation(images:Bool)->String? {
    if kind == .image,image != nil,!images {return "当前模型不能读取图片"}
    return note
  }
}
struct AssistantUsage:Sendable {
  let input:Int?,output:Int?,cacheRead:Int?,cacheWrite:Int?
  init?(_ bytes:Data) {
    guard let value=try? JSONSerialization.jsonObject(with:bytes) as? [String:Any] else {return nil}
    func number(_ key:String)->Int? {guard let n=value[key] as? NSNumber,CFGetTypeID(n) != CFBooleanGetTypeID(),n.doubleValue>=0,n.doubleValue.rounded()==n.doubleValue,n.doubleValue<1e12 else {return nil};return n.intValue}
    input=number("input");output=number("output");cacheRead=number("cache_read");cacheWrite=number("cache_write")
    if input==nil && output==nil && cacheRead==nil && cacheWrite==nil {return nil}
  }
}
struct AssistantInputSnapshot:Sendable {
  let systemCharacters:Int,conversationCharacters:Int,attachmentCharacters:Int,fullInput:String,actual:Bool
  var estimatedTokens:Int {Int(ceil(Double(systemCharacters+conversationCharacters+attachmentCharacters)/1.6))}
}
enum AssistantInputPresentation {
  static func readable(_ source:String)->String {
    guard let root=try? JSONSerialization.jsonObject(with:Data(source.utf8)) as? [String:Any],let messages=root["messages"] as? [[String:Any]] else {return source}
    var sections:[String]=[]
    for (index,entry) in messages.enumerated() {
      let message=entry["canonical"] as? [String:Any] ?? entry,role=message["role"] as? String ?? "message"
      var text=message["content"] as? String ?? ""
      if let parts=message["parts"] as? [[String:Any]] {
        text=parts.compactMap {part in
          if let text=part["text"] as? String {return text}
          if let media=part["media_ref"] as? String {return "〔图片引用 \(media)：原始请求 JSON 包含完整编码〕"}
          return nil
        }.joined(separator:"\n")
      }
      if let content=message["content"] as? [[String:Any]] {
        text=content.map {part in
          if let text=part["text"] as? String {return text}
          return "〔图片数据：可切换原始请求 JSON 查看完整编码〕"
        }.joined(separator:"\n")
      }
      if text.isEmpty {text=String(decoding:(try? JSONSerialization.data(withJSONObject:message,options:[.prettyPrinted,.sortedKeys,.withoutEscapingSlashes])) ?? Data(),as:UTF8.self)}
      sections.append("〔\(index+1) · \(role)〕\n"+text)
    }
    if let tools=root["tools"],let bytes=try? JSONSerialization.data(withJSONObject:tools,options:[.prettyPrinted,.sortedKeys,.withoutEscapingSlashes]) {sections.append("〔工具说明〕\n"+String(decoding:bytes,as:UTF8.self))}
    if let results=root["results"],let bytes=try? JSONSerialization.data(withJSONObject:results,options:[.prettyPrinted,.sortedKeys,.withoutEscapingSlashes]) {sections.append("〔工具结果〕\n"+String(decoding:bytes,as:UTF8.self))}
    return sections.joined(separator:"\n\n")
  }
}
enum AssistantComposerRules {
  static func shouldSend(marked:Bool,command:Bool,shift:Bool,otherModifiers:Bool,shortcut:AssistantSendShortcut)->Bool {
    !marked && !shift && !otherModifiers && (shortcut == .enter ? !command:command)
  }
  static func referencePart(_ references:[AssistantCellReference])->String {
    references.map{"引用单元格：\($0.label)，cell_id=\($0.id)，source_hash=\($0.sourceHash)"}.joined(separator:"\n")
  }
}
