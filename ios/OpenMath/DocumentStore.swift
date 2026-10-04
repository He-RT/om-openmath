import SwiftUI
import UIKit
import UniformTypeIdentifiers

extension UTType {
  static let openMathNotebook = UTType(exportedAs: "org.openmath.notebook", conformingTo: .json)
}
private final class DocumentSnapshot: @unchecked Sendable {
  private let lock = NSLock()
  private var value = NotebookFile()
  func read() -> NotebookFile {
    lock.lock()
    defer { lock.unlock() }
    return value
  }
  func replace(_ file: NotebookFile) {
    lock.lock()
    defer { lock.unlock() }
    value = file
  }
}
@MainActor final class NotebookDocument: UIDocument {
  private nonisolated let snapshot = DocumentSnapshot()
  var notebook: NotebookFile {
    get { snapshot.read() }
    set { snapshot.replace(newValue) }
  }
  override func contents(forType typeName: String) throws -> Any {
    try JSONEncoder().encode(snapshot.read())
  }
  override func load(fromContents contents: Any, ofType typeName: String?) throws {
    guard let data = contents as? Data else { throw KernelError.message("笔记本数据无效") }
    let file = try JSONDecoder().decode(NotebookFile.self, from: data)
    try file.validate()
    snapshot.replace(file)
  }
  func openFile() async -> Bool {
    await withCheckedContinuation { c in open { c.resume(returning: $0) } }
  }
  func write(operation: UIDocument.SaveOperation) async -> Bool {
    await withCheckedContinuation { c in
      save(to: fileURL, for: operation) { c.resume(returning: $0) }
    }
  }
  func closeFile() async -> Bool {
    await withCheckedContinuation { c in close { c.resume(returning: $0) } }
  }
}

struct DocumentPicker: UIViewControllerRepresentable {
  var completion: (URL) -> Void
  func makeUIViewController(context: Context) -> UIDocumentPickerViewController {
    let vc = UIDocumentPickerViewController(
      forOpeningContentTypes: [.openMathNotebook, .json], asCopy: false)
    vc.delegate = context.coordinator
    return vc
  }
  func updateUIViewController(_ vc: UIDocumentPickerViewController, context: Context) {}
  func makeCoordinator() -> Coordinator { Coordinator(completion) }
  final class Coordinator: NSObject, UIDocumentPickerDelegate {
    let completion: (URL) -> Void
    init(_ completion: @escaping (URL) -> Void) { self.completion = completion }
    func documentPicker(
      _ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]
    ) { if let url = urls.first { completion(url) } }
  }
}
struct ShareSheet: UIViewControllerRepresentable {
  var urls: [URL]
  func makeUIViewController(context: Context) -> UIActivityViewController {
    let controller = UIActivityViewController(activityItems: urls, applicationActivities: nil)
    controller.modalPresentationStyle = .pageSheet
    controller.popoverPresentationController?.sourceView = controller.view
    controller.popoverPresentationController?.sourceRect = CGRect(x: 0, y: 0, width: 1, height: 1)
    controller.popoverPresentationController?.permittedArrowDirections = []
    return controller
  }
  func updateUIViewController(_ vc: UIActivityViewController, context: Context) {}
}
struct SaveAsPicker: UIViewControllerRepresentable {
  var url: URL
  var completion: (URL) -> Void
  func makeUIViewController(context: Context) -> UIDocumentPickerViewController {
    let picker = UIDocumentPickerViewController(forExporting: [url], asCopy: true)
    picker.delegate = context.coordinator
    return picker
  }
  func updateUIViewController(_ picker: UIDocumentPickerViewController, context: Context) {}
  func makeCoordinator() -> DocumentPicker.Coordinator { DocumentPicker.Coordinator(completion) }
}
