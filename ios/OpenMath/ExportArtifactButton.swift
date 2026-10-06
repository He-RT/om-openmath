import SwiftUI

/// A generated artifact becomes shareable only after exact local bytes are written and read back.
struct ExportArtifactButton: View {
  var title: String
  var name: String
  var version: String
  var enabled: Bool
  var controller: NotebookController
  var request: () -> JSONValue
  var operation: ((JSONValue) async throws -> KernelPacket)? = nil
  @State private var busy = false
  @State private var alive = true
  @State private var revision = 0
  @State private var failure: String?
  @State private var task: Task<Void, Never>?
  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      Button(busy ? controller.text("正在导出…", "Exporting…") : title) {
        guard enabled && !busy else { return }
        revision += 1; let token = revision; busy = true; failure = nil
        let body = request()
        task = Task {
          do {
            let packet: KernelPacket
            if let operation { packet = try await operation(body) }
            else { packet = try await controller.call(body) }
            guard alive && token == revision && enabled && !Task.isCancelled else { return }
            guard packet.response.body["type"].string == "artifact" else { throw KernelError.message("导出未返回真实字节") }
            try controller.shareArtifact(packet.response.body["artifact"], name: name)
            busy = false
          } catch { if alive && token == revision { failure = error.localizedDescription; busy = false } }
        }
      }.disabled(!enabled || busy).frame(minHeight: 44)
      if let failure { Text(failure).font(.caption).foregroundStyle(.red).textSelection(.enabled) }
    }.onAppear { alive = true }
      .onChange(of: version) { _, _ in revision += 1; task?.cancel(); busy = false; failure = nil }
      .onChange(of: enabled) { _, value in if !value { revision += 1; task?.cancel(); busy = false } }
      .onDisappear { alive = false; revision += 1; task?.cancel() }
  }
}
