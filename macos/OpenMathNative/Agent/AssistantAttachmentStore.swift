import Foundation
import PDFKit
import ImageIO
import UniformTypeIdentifiers

struct AssistantAttachmentRecord:Codable,Sendable {
  let kind:String,codecVersion:Int,attachment:AssistantAttachment
}
/// PDF parsing and image decoding stay off the main actor. Only host-selected bytes enter BlobStore.
actor AssistantAttachmentStore {
  private let storage:StorageService
  init(storage:StorageService) {self.storage=storage}
  private func retain(_ publication:BlobPublication,owner:BlobOwner,type:String)async throws->BlobDescriptor {
    do {
      _ = try await storage.attachBlob(publication,owner:owner,mediaType:type,codec:"assistant-attachment-v1")
      try await storage.releaseBlob(publication);return publication.descriptor
    } catch {try? await storage.releaseBlob(publication);throw error}
  }
  func add(file:URL,session:String,document:String)async throws->AssistantAttachment {
    let scoped=file.startAccessingSecurityScopedResource();defer {if scoped {file.stopAccessingSecurityScopedResource()}}
    return try await add(publication:storage.publishBlob(file:file),name:file.lastPathComponent,session:session,document:document)
  }
  func add(data:Data,name:String,session:String,document:String)async throws->AssistantAttachment {
    try await add(publication:storage.publishBlob(data:data),name:name,session:session,document:document)
  }
  private func add(publication:BlobPublication,name:String,session:String,document:String)async throws->AssistantAttachment {
    let id="attachment-"+UUID().uuidString.lowercased(),owner=BlobOwner(kind:.attachment,id:id)
    let original=try await retain(publication,owner:owner,type:"application/octet-stream")
    let clean=String(name.filter{!$0.isNewline && !$0.unicodeScalars.contains(where:CharacterSet.controlCharacters.contains)}.prefix(256))
    var attachment=AssistantAttachment(id:id,name:clean.isEmpty ? "附件":clean,original:original,mediaType:"application/octet-stream",kind:.other,note:"暂不能发送给模型")
    let prefix=try await Self.bytes(storage:storage,owner:owner,descriptor:original,maximum:512,partial:true)
    let pdf=prefix.starts(with:Data("%PDF-".utf8))
    let detected=CGImageSourceCreateWithData(prefix as CFData,nil).flatMap{CGImageSourceGetType($0)}
    let image=detected != nil || UTType(filenameExtension:(name as NSString).pathExtension)?.conforms(to:.image)==true
    if (pdf || image),original.byteLength>32*1024*1024 {attachment.note="文件超过 32 MiB，已保存，暂不能发送给模型"}
    else if pdf || image {
      let bytes=try await Self.bytes(storage:storage,owner:owner,descriptor:original,maximum:32*1024*1024)
      if pdf {
        attachment = .init(id:id,name:attachment.name,original:original,mediaType:"application/pdf",kind:.pdf,note:"没有可提取的文字")
        if let document=PDFDocument(data:bytes),document.pageCount>1000 {attachment.note="PDF 超过 1000 页，已保存，暂不能发送给模型"}
        else if let document=PDFDocument(data:bytes) {
          var pages:[String]=[],size=0,tooLong=false
          for index in 0..<document.pageCount {
            try Task.checkCancellation()
            guard let text=document.page(at:index)?.string,!text.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty else {continue}
            let page="〔第 \(index+1) 页〕\n"+text;size+=page.utf8.count
            if size>128*1024 {tooLong=true;break};pages.append(page)
          }
          if tooLong {attachment.note="提取文字超过 128 KiB，已保存，暂不能发送给模型"}
          else if !pages.isEmpty {
            attachment.text=try await retain(storage.publishBlob(data:Data(pages.joined(separator:"\n\n").utf8)),owner:owner,type:"text/plain; charset=utf-8")
            attachment.note=nil
          }
        } else {attachment.note="PDF 无法读取，原文件已保存，暂不能发送给模型"}
      } else if let source=CGImageSourceCreateWithData(bytes as CFData,nil),
        let prepared=Self.render(source,maxPixel:1600,jpeg:true),prepared.count<=768*1024 {
        let mediaType=CGImageSourceGetType(source).flatMap{UTType($0 as String)?.preferredMIMEType} ?? "image/*"
        attachment = .init(id:id,name:attachment.name,original:original,mediaType:mediaType,kind:.image,
          image:try await retain(storage.publishBlob(data:prepared),owner:owner,type:"image/jpeg"),thumbnail:Self.render(source,maxPixel:80,jpeg:false))
      } else {attachment.note="图片无法准备或过大，原文件已保存，暂不能发送给模型"}
    }
    _ = try await storage.createAgentSession(session,documentID:document)
    let ledger=SessionLedgerPort(storage:storage,session:session,sourceInstance:id)
    _ = try await ledger.record(AssistantAttachmentRecord(kind:"native_attachment",codecVersion:1,attachment:attachment),id:id,kind:.recovery)
    return attachment
  }
  private static func render(_ source:CGImageSource,maxPixel:Int,jpeg:Bool)->Data? {
    let options:[CFString:Any]=[kCGImageSourceCreateThumbnailFromImageAlways:true,kCGImageSourceThumbnailMaxPixelSize:maxPixel,kCGImageSourceCreateThumbnailWithTransform:true,kCGImageSourceShouldCacheImmediately:true]
    guard var image=CGImageSourceCreateThumbnailAtIndex(source,0,options as CFDictionary) else {return nil}
    if jpeg {
      guard let context=CGContext(data:nil,width:image.width,height:image.height,bitsPerComponent:8,bytesPerRow:0,space:CGColorSpaceCreateDeviceRGB(),bitmapInfo:CGImageAlphaInfo.noneSkipLast.rawValue) else {return nil}
      let bounds=CGRect(x:0,y:0,width:image.width,height:image.height)
      context.setFillColor(CGColor(gray:1,alpha:1));context.fill(bounds);context.draw(image,in:bounds)
      guard let flattened=context.makeImage() else {return nil};image=flattened
    }
    let output=NSMutableData()
    guard let writer=CGImageDestinationCreateWithData(output,(jpeg ? UTType.jpeg.identifier:UTType.png.identifier) as CFString,1,nil) else {return nil}
    CGImageDestinationAddImage(writer,image,[kCGImageDestinationLossyCompressionQuality:0.8] as CFDictionary)
    return CGImageDestinationFinalize(writer) ? output as Data:nil
  }
  static func bytes(storage:StorageService,owner:BlobOwner,descriptor:BlobDescriptor,maximum:Int,partial:Bool=false)async throws->Data {
    guard partial || descriptor.byteLength<=maximum else {throw BlobError.budgetExceeded}
    let reader=try await storage.openReferencedBlob(owner:owner,hash:descriptor.hash)
    do {
      var bytes=Data(),offset:UInt64=0;let end=min(descriptor.byteLength,UInt64(maximum))
      while offset<end {let part=try await storage.readBlob(reader,offset:offset,count:min(1024*1024,Int(end-offset)));guard !part.isEmpty else {throw BlobError.corrupt};bytes.append(part);offset+=UInt64(part.count)}
      try await storage.closeBlob(reader)
      guard partial || (bytes.count==descriptor.byteLength && KernelHashes.data(bytes)==descriptor.hash) else {throw BlobError.corrupt};return bytes
    } catch {try? await storage.closeBlob(reader);throw error}
  }
}
