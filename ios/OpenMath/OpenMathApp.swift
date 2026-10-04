import SwiftUI

@main struct OpenMathApp: App {
  @State private var controller = NotebookController()
  @Environment(\.scenePhase) private var phase
  var body: some Scene {
    WindowGroup {
      RootView(controller: controller)
        .tint(Color.openMathAccent)
        .preferredColorScheme(
          controller.scheme == "dark" ? .dark : controller.scheme == "light" ? .light : nil
        )
        .task { if controller.kernel == nil { await controller.start() } }
        .onOpenURL { url in Task { await controller.load(url) } }
        .onChange(of: phase) { _, value in
          if value == .background { Task { await controller.background() } }
        }
    }
  }
}
