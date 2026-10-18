import AppKit

/// Only native clipboard/drag/file-picker input can provide local URLs to the importer.
@MainActor enum AssistantAttachmentInput {
  static func accepts(_ board:NSPasteboard)->Bool {
    board.canReadObject(forClasses:[NSURL.self],options:[.urlReadingFileURLsOnly:true]) || board.types?.contains(.png)==true || board.types?.contains(.tiff)==true
  }
  static func read(_ board:NSPasteboard,assistant:NativeAssistant)->Bool {
    if let urls=board.readObjects(forClasses:[NSURL.self],options:[.urlReadingFileURLsOnly:true]) as? [URL],!urls.isEmpty {
      Task {await assistant.addAttachmentFiles(urls)};return true
    }
    for (type,name) in [(NSPasteboard.PasteboardType.png,"粘贴图片.png"),(.tiff,"粘贴图片.tiff")] {
      if let data=board.data(forType:type) {Task {await assistant.addAttachmentData(data,name:name)};return true}
    }
    return false
  }
  static func choose(window:NSWindow?,assistant:NativeAssistant) {
    let panel=NSOpenPanel();panel.canChooseFiles=true;panel.canChooseDirectories=false;panel.allowsMultipleSelection=true
    panel.message="图片将发送给支持图片的模型；PDF 提取文字；其他格式保存但暂不发送。";panel.prompt="加入附件"
    if let window {panel.beginSheetModal(for:window) {response in if response == .OK {Task {await assistant.addAttachmentFiles(panel.urls)}}}}
    else {panel.begin {response in if response == .OK {Task {await assistant.addAttachmentFiles(panel.urls)}}}}
  }
}
