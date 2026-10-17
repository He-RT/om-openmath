import Foundation
@main struct CellPresentationFixtures {
  static func main() {
    var f=CellPresentationState.Facts()
    precondition(CellPresentationState.derive(f).status == .pending)
    f.running=true;precondition(CellPresentationState.derive(f).status == .running)
    f.running=false;f.stopped=true;precondition(CellPresentationState.derive(f).status == .stopped)
    f.stopped=false;f.hasResult=true;precondition(CellPresentationState.derive(f).status == .unconfirmed)
    f.hasMetadata=true;f.nature="exact";precondition(CellPresentationState.derive(f).status == .exact)
    for nature in ["machine","high_precision"] {f.nature=nature;precondition(CellPresentationState.derive(f).status == .numeric)}
    f.nature="symbolic";f.unevaluated=true;precondition(CellPresentationState.derive(f).status == .unevaluated)
    f.unevaluated=false;f.verified=true;precondition(CellPresentationState.derive(f).status == .verified)
    f.conditional=true;precondition(CellPresentationState.derive(f).status == .conditional)
    f.error=true;precondition(CellPresentationState.derive(f).status == .error)
    f.sourceChanged=true;precondition(CellPresentationState.derive(f).status == .stale("源码已修改"))
    f.sourceChanged=false;f.staleReason="a 已改变";precondition(CellPresentationState.derive(f).status == .stale("a 已改变"))
    f.byAgent=true;precondition(CellPresentationState.derive(f).byAgent)
    f.running=true;precondition(CellPresentationState.derive(f).status == .running)
    precondition(CellPresentationState.derive(f).dashed)
    f.running=false;f.stopped=true;precondition(CellPresentationState.derive(f).status == .stopped)
    print("Cell presentation: all states, precedence, missing metadata and agent provenance passed")
  }
}
