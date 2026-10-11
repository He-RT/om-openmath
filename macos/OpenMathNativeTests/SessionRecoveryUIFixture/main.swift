import AppKit
import SwiftUI

/// Reuses the actual production recovery view and storage adapter with an already generated,
/// disposable fixture root. Preview/public roots cannot be redirected by product arguments.
@main struct SessionRecoveryFixtureApp:App {
  @StateObject private var workspace:NativeFileWorkspace
  @State private var visible=true
  init() {
    guard let file=Bundle.main.url(forResource:"FixtureRoot",withExtension:"txt"),
      let root=try? String(contentsOf:file,encoding:.utf8) else {fatalError("fixture root missing")}
    _workspace=StateObject(wrappedValue:NativeFileWorkspace(storage:NativeAppStorage(paths:.init(root:URL(fileURLWithPath:root.trimmingCharacters(in:.whitespacesAndNewlines),isDirectory:true),channel:.preview))))
  }
  var body:some Scene {
    Window("OpenMath Session Recovery Fixture",id:"fixture") {
      NativeSessionRecoveryList(workspace:workspace,isPresented:$visible)
        .task {await workspace.refreshRecoverableSessions()}
    }
  }
}
