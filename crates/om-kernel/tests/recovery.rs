//! Worker recovery uses the real dependency graph and never computes nondefinition cells.
use om_kernel::{KernelConfig, Session, protocol::*};
fn cell(id: &str, source: &str) -> CellInput {
    CellInput {
        id: id.into(),
        kind: CellKind::Math,
        source: source.into(),
        dialect: Dialect::Modern,
    }
}
#[test]
fn restart_restores_definitions_in_dependency_order_and_retains_source_and_stale_computations() {
    let mut s = Session::new(KernelConfig::default(), None);
    let file = NotebookFile {
        version: 1,
        title: "source".into(),
        cells: vec![
            cell("b", "let b=a+1"),
            cell("a", "let a=9"),
            cell("slow", "factorial(10000000)"),
            cell("query", "b+1"),
        ],
    };
    s.handle(Request::LoadNotebook { file: file.clone() });
    let Response::NotebookState { state } = s.handle(Request::GetNotebookState).0 else {
        panic!()
    };
    assert_eq!(state.definition_order, ["a", "b"]);
    assert_eq!(state.cells[0].uses, ["a"]);
    assert_eq!(state.cells[1].defines, ["a"]);
    let (response, events) = s.handle(Request::RestoreDefinitions);
    let Response::NotebookState { state } = response else {
        panic!()
    };
    assert_eq!(
        serde_json::to_value(&state.file).unwrap(),
        serde_json::to_value(&file).unwrap()
    );
    assert_eq!(state.cells[0].status, CellStatus::Done);
    assert_eq!(state.cells[1].status, CellStatus::Done);
    assert_eq!(state.cells[2].status, CellStatus::Stale);
    assert_eq!(state.cells[3].status, CellStatus::Stale);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::KernelRestarted { .. }))
    );
    let Response::Evaluated { output, .. } = s
        .handle(Request::Evaluate {
            cell_id: "query".into(),
            source: "b+1".into(),
            dialect: Dialect::Modern,
        })
        .0
    else {
        panic!()
    };
    assert!(matches!(&output.items[0],OutputItem::Expr{input_form,..}if input_form=="11"));
    assert!(s.notebook.cells[2].output.is_none());
}
#[test]
fn restore_reports_real_cycles_and_does_not_run_their_dependents() {
    let mut s = Session::new(KernelConfig::default(), None);
    s.handle(Request::LoadNotebook {
        file: NotebookFile {
            version: 1,
            title: "".into(),
            cells: vec![
                cell("a", "let a=b+1"),
                cell("b", "let b=a+1"),
                cell("c", "let c=a+2"),
                cell("plain", "a+1"),
            ],
        },
    });
    let Response::NotebookState { state } = s.handle(Request::RestoreDefinitions).0 else {
        panic!()
    };
    assert_eq!(state.cycles.len(), 3);
    assert!(
        state.cells[..3]
            .iter()
            .all(|c| c.status == CellStatus::Error)
    );
    assert_eq!(state.cells[3].status, CellStatus::Stale);
}
