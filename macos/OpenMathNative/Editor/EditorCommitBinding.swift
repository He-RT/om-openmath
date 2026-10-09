import AppKit

/// Binds actual native input identity to confirmed source transactions; the receipt owns success.
/// The workbench uses this barrier before run/save/agent preview, never an editor echo as a commit.
@MainActor final class EditorCommitBinding {
  let editor:DraftTextViewAdapter
  let undo:UndoCoordinator
  private let port:DocCommitPort
  init(editor:DraftTextViewAdapter,port:DocCommitPort,undo:UndoCoordinator) {
    self.editor=editor;self.port=port;self.undo=undo;editor.bindDocumentUndo(undo.manager)
  }
  @discardableResult func flush(faults:StorageFaults = .init()) async throws ->NativeCommitState? {
    let group=editor.inputGroup
    let actual=try await port.synchronizeDrafts(faults:faults,inputGroup:group)
    if let actual {try undo.registerCommitted(actual,group:group,name:"编辑数学单元格")}
    editor.reflectAcknowledgedSource()
    return actual
  }
  func reconcile(_ operation:String,group:String) async throws {
    let actual=try await port.reconcile(operation)
    guard actual.phase == .completed else {throw CommitPortError.sourceConflict}
    try undo.registerCommitted(actual,group:group,name:"编辑数学单元格");editor.reflectAcknowledgedSource()
  }
}
