import Combine
import SwiftUI

/// Both outline and paper subscribe to the same retained result metadata.
struct CellPresentationScope<Content:View>:View {
  let session:NativeDocumentSession,cell:NativeSourceCell
  @ObservedObject var workspace:NativeFileWorkspace
  @ObservedObject private var runtime:NotebookRuntime
  @ObservedObject private var assistant:NativeAssistant
  @StateObject private var metadata=CellMetadataObservation()
  let content:(CellPresentationState)->Content
  init(session:NativeDocumentSession,cell:NativeSourceCell,workspace:NativeFileWorkspace,@ViewBuilder content:@escaping (CellPresentationState)->Content) {
    self.session=session;self.cell=cell;self.workspace=workspace;self.content=content
    _runtime=ObservedObject(wrappedValue:session.runtime)
    _assistant=ObservedObject(wrappedValue:session.assistant)
  }
  var body:some View {
    let binding=runtime.results[cell.id]?.bindings.last
    let presenter=binding.map{runtime.presenter(for:$0)}
    content(runtime.presentation(cell,session:session,presenter:presenter))
      .task(id:binding?.result_ref) {metadata.observe(presenter)}
  }
}
@MainActor private final class CellMetadataObservation:ObservableObject {
  private var subscription:AnyCancellable?
  func observe(_ presenter:ResultPresenter?) {
    subscription=presenter?.objectWillChange.sink { [weak self] in
      Task { @MainActor in self?.objectWillChange.send() }
    }
  }
}
extension NotebookRuntime {
  func presentation(_ cell:NativeSourceCell,session:NativeDocumentSession,presenter:ResultPresenter?)->CellPresentationState {
    let result=results[cell.id]
    let draft=session.drafts.overlay(cell.id)
    let changed=draft.map{!$0.source.utf8.elementsEqual(cell.source.utf8)} ?? false
    var facts=CellPresentationState.Facts()
    facts.running=operationCell==cell.id || session.assistant.busy && session.assistant.messages.flatMap(\.actions).contains{$0.tool=="run_cells" && $0.status == .running && $0.cells.contains(cell.id)}
    facts.stopped=stoppedCells.contains(cell.id)
    facts.hasResult=result != nil
    facts.byAgent=agentModifiedCells.contains(cell.id) && !changed
    facts.sourceChanged=changed
    if let result {
      let currentSource=draft?.source ?? cell.source
      facts.sourceChanged=result.receipt.producer.cell_source_hash.value != KernelHashes.data(Data(currentSource.utf8))
      if let reason=staleReasons[cell.id] {
        facts.staleReason=reason
      } else if !hasResultSource(cell.id) && result.receipt.producer.execution_epoch.value != session.drafts.confirmed.execution_epoch.value {
        facts.staleReason="计算所用的源码或定义已改变"
      }
      facts.error=result.receipt.producer.terminal_status == .error
    }
    if let summary=presenter?.summary {
      facts.hasMetadata=true
      facts.nature=summary.statement.value?.nature ?? ""
      facts.outputKind=summary.statement.value?.output_kind ?? ""
      if let page=presenter?.presentation?.solutions {
        facts.conditional=page.solutions.contains{$0.condition_latex.value != nil}
        facts.verified=page.complete && !page.solutions.isEmpty && page.solutions.allSatisfy{
          if case .exact=$0.verified {return true};return false
        }
      }
      if facts.nature=="symbolic",presenter?.presentation?.kind=="expression",let value=result?.values.last {
        let input=cell.source.trimmingCharacters(in:.whitespacesAndNewlines)
        facts.unevaluated=input==value.modern?.trimmingCharacters(in:.whitespacesAndNewlines) || input==value.source.trimmingCharacters(in:.whitespacesAndNewlines)
      }
    }
    return .derive(facts)
  }
}
extension CellPresentationState {
  var color:Color {
    switch status {
    case .error:return Theme.Color.red
    case .conditional:return Theme.Color.conditional
    default:return muted ? Theme.Color.ink3:Theme.Color.verified
    }
  }
}
