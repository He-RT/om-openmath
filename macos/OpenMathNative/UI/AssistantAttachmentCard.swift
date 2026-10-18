import AppKit
import SwiftUI

struct AssistantAttachmentCard:View {
  let attachment:AssistantAttachment
  var images=true
  var remove:(()->Void)?
  var body:some View {
    HStack(spacing:7) {
      if let thumbnail=attachment.thumbnail,let image=NSImage(data:thumbnail) {
        Image(nsImage:image).resizable().scaledToFit().frame(width:40,height:32).background(.white,in:RoundedRectangle(cornerRadius:4))
      } else {Image(systemName:attachment.kind == .pdf ? "doc.richtext":"doc").frame(width:32,height:32).foregroundStyle(Theme.Color.ink2)}
      VStack(alignment:.leading,spacing:2) {
        Text(attachment.name).lineLimit(1).truncationMode(.middle).frame(maxWidth:150,alignment:.leading)
        Text(ByteCountFormatter.string(fromByteCount:Int64(attachment.original.byteLength),countStyle:.file)+" · "+(attachment.kind == .image ? "图片":attachment.kind == .pdf ? "PDF":"文件")).foregroundStyle(Theme.Color.ink2)
        if let note=attachment.explanation(images:images) {Text(note).foregroundStyle(Theme.Color.conditional).lineLimit(2).frame(maxWidth:200,alignment:.leading)}
      }.font(.system(size:10.5))
      if let remove {Button("移除 "+attachment.name,systemImage:"xmark",action:remove).labelStyle(.iconOnly).font(.system(size:9)).buttonStyle(.plain)}
    }.padding(6).background(Theme.Color.well,in:RoundedRectangle(cornerRadius:6)).accessibilityElement(children:.contain)
  }
}
