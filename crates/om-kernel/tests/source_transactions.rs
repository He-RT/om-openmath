//! Actual non-evaluating whole-source changes retain history and retire old real definitions.
use om_kernel::{
    KernelConfig, Session,
    config::{ConfigDialect, Constants, Language},
    protocol::{CellInput, CellKind, CellStatus, Dialect, NotebookFile, Request, Response},
    source,
};
fn math(id: &str, source: &str) -> CellInput {
    CellInput {
        id: id.into(),
        kind: CellKind::Math,
        source: source.into(),
        dialect: Dialect::Modern,
    }
}
fn file(cells: Vec<CellInput>) -> NotebookFile {
    NotebookFile {
        version: 1,
        title: "".into(),
        cells,
    }
}
fn evaluate(session: &mut Session, id: &str, source: &str) -> String {
    let (r, _) = session.handle(Request::Evaluate {
        cell_id: id.into(),
        source: source.into(),
        dialect: Dialect::Modern,
    });
    let value = serde_json::to_value(r).unwrap();
    value["output"]["items"].as_array().unwrap().last().unwrap()["input_form"]
        .as_str()
        .unwrap()
        .to_owned()
}
#[test]
fn whole_delete_does_not_cascade_and_old_owner_values_are_actually_removed() {
    let mut session = Session::new(KernelConfig::default(), None);
    assert_eq!(evaluate(&mut session, "a", "let a=2; a"), "2");
    assert_eq!(evaluate(&mut session, "b", "let b=a+1; b"), "3");
    let plan = session
        .apply_source_file_without_evaluation(file(vec![math("b", "let b=a+1; b")]))
        .unwrap();
    assert!(plan.retire_owner_cells.contains(&"a".into()));
    assert!(plan.retire_owner_cells.contains(&"b".into()));
    let (response, events) = session.handle(Request::GetNotebookState);
    assert!(events.is_empty());
    let state = serde_json::to_value(response).unwrap();
    assert_eq!(state["state"]["cells"][0]["status"], "Stale");
    assert_eq!(evaluate(&mut session, "probe", "b"), "b");
    assert_eq!(evaluate(&mut session, "probe-a", "a"), "a");
}
#[test]
fn old_caller_owned_dynamic_side_effects_contribute_real_invalidation() {
    let mut session = Session::new(KernelConfig::default(), None);
    evaluate(&mut session, "f", "let f(x)=x+1");
    evaluate(&mut session, "caller", "let value=f(2);value");
    let p = session
        .apply_source_file_without_evaluation(file(vec![
            math("f", "let f(x)=x+2"),
            math("caller", "let value=f(2);value"),
        ]))
        .unwrap();
    assert!(p.affected_cells.contains(&"caller".into()));
    assert!(p.changed_symbols.contains(&"value".into()));
    assert_eq!(evaluate(&mut session, "probe", "value"), "value");
}
#[test]
fn text_title_and_prose_reorder_keep_live_math_and_no_evaluation_occurs() {
    let mut session = Session::new(KernelConfig::default(), None);
    assert_eq!(evaluate(&mut session, "a", "let a=2; a"), "2");
    let mut text = math("text", "paragraph");
    text.kind = CellKind::Text;
    let mut updated = file(vec![text, math("a", "let a=2; a")]);
    updated.title = "new".into();
    let p = session
        .apply_source_file_without_evaluation(updated)
        .unwrap();
    assert!(!p.math_changed);
    assert!(p.retire_owner_cells.is_empty());
    let (r, _) = session.handle(Request::GetNotebookState);
    if let Response::NotebookState { state } = r {
        assert_eq!(
            state.cells.iter().find(|c| c.id == "a").unwrap().status,
            CellStatus::Done
        );
    } else {
        panic!()
    }
    assert_eq!(evaluate(&mut session, "probe", "a"), "2");
}
#[test]
fn real_cycle_conflict_and_lexical_function_scope_are_analyzed_without_execution() {
    let f = file(vec![
        math("a", "let a=b"),
        math("b", "let b=a"),
        math("dependent", "a+1"),
        math("duplicate", "let a=7"),
    ]);
    let p = source::analyze_source(&f, ConfigDialect::Modern, Constants::Math, &[]).unwrap();
    assert_eq!(p.cycles, vec!["a", "b"]);
    assert!(p.blocked.contains(&"dependent".into()));
    assert_eq!(p.conflicts[0].symbol, "a");
    let local = source::analyze_source(
        &file(vec![
            math("f", "let f(x)=sin(x)+parameter"),
            math("use", "f (2)"),
        ]),
        ConfigDialect::Modern,
        Constants::Math,
        &[],
    )
    .unwrap();
    assert_eq!(local.cells[0].uses, vec!["parameter"]);
    assert!(local.cells[1].uses.contains(&"f".into()));
}
#[test]
fn invalid_batch_does_not_modify_definitions_or_source_and_settings_are_not_evaluated() {
    let mut session = Session::new(KernelConfig::default(), None);
    evaluate(&mut session, "a", "let a=2;a");
    assert!(
        session
            .apply_source_file_without_evaluation(file(vec![
                math("a", "new"),
                math("a", "invalid")
            ]))
            .is_err()
    );
    assert_eq!(evaluate(&mut session, "probe", "a"), "2");
    let mut cfg = KernelConfig::default().general;
    cfg.language = Language::En;
    let p = session
        .apply_calculation_settings_without_evaluation(cfg.clone())
        .unwrap();
    assert!(!p.math_changed);
    cfg.constants = Constants::Strict;
    let p = session
        .apply_calculation_settings_without_evaluation(cfg)
        .unwrap();
    assert!(p.math_changed);
    assert_eq!(evaluate(&mut session, "post-settings", "a"), "a");
}
