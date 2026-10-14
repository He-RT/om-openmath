//! Plan actual parsed/live sessions without running source or advancing defs/Out/random/status.
use om_kernel::{
    CellExecutionScope as Scope, KernelConfig, Session,
    checkpoint::{CheckpointBinding, CheckpointLimits},
    protocol::*,
};
use om_num::ctx::Interrupt;
use std::{
    collections::BTreeSet,
    sync::{Arc, atomic::AtomicBool},
};
fn math(id: &str, source: &str) -> CellInput {
    CellInput {
        id: id.into(),
        source: source.into(),
        kind: CellKind::Math,
        dialect: Dialect::Modern,
    }
}
fn setup(cells: Vec<CellInput>) -> Session {
    let mut config = KernelConfig::default();
    config.general.auto_run_dependents = false;
    let mut s = Session::new(config, None);
    s.apply_source_file_without_evaluation(NotebookFile {
        version: 1,
        title: "actual planning".into(),
        cells,
    })
    .unwrap();
    s
}
fn allowed(s: &Session) -> BTreeSet<String> {
    s.notebook
        .cells
        .iter()
        .filter(|c| c.kind == CellKind::Math)
        .map(|c| c.id.clone())
        .collect()
}
fn plan(s: &Session, ids: &[&str], scope: Scope) -> om_kernel::CellExecutionPlan {
    s.plan_cell_execution(
        &ids.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
        scope,
        &allowed(s),
        &Interrupt::default(),
    )
    .unwrap()
}
fn eval(s: &mut Session, id: &str, source: &str) {
    assert!(matches!(
        s.handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: Dialect::Modern
        })
        .0,
        Response::Evaluated { .. }
    ));
}
#[test]
fn selected_never_adds_missing_prerequisites_and_required_does_not_run_unrelated_cells() {
    let mut s = setup(vec![
        math("answer", "let answer=b+1;answer"),
        math("unrelated", "seed_random(99);rand()"),
        math("b", "let b=a+1;b"),
        math("a", "let a=2;a"),
    ]);
    let selected = plan(&s, &["answer"], Scope::Selected);
    assert_eq!(selected.ordered_cell_ids, ["answer"]);
    assert_eq!(selected.unavailable_prerequisite_cell_ids, ["b"]);
    let required = plan(&s, &["answer"], Scope::Required);
    assert_eq!(required.ordered_cell_ids, ["a", "b", "answer"]);
    assert!(required.unavailable_prerequisite_cell_ids.is_empty());
    eval(&mut s, "a", "let a=2;a");
    assert_eq!(
        plan(&s, &["answer"], Scope::Required).ordered_cell_ids,
        ["b", "answer"]
    );
    eval(&mut s, "b", "let b=a+1;b");
    assert_eq!(
        plan(&s, &["answer"], Scope::Required).ordered_cell_ids,
        ["answer"]
    );
    assert!(
        plan(&s, &["answer"], Scope::Selected)
            .unavailable_prerequisite_cell_ids
            .is_empty()
    );
    assert_eq!(
        s.notebook
            .cells
            .iter()
            .find(|c| c.id == "unrelated")
            .unwrap()
            .exec_count,
        None
    );
    println!(
        "actual missing/required plans={}",
        serde_json::json!({"selected":selected,"required":required})
    );
}
#[test]
fn affected_uses_actual_graph_and_respects_native_allowed_set_without_hidden_expansion() {
    let s = setup(vec![
        math("dependent", "let b=a+1"),
        math("a", "let a=2"),
        math("tail", "b+1"),
        math("unrelated", "3+3"),
    ]);
    let affected = plan(&s, &["a"], Scope::Affected);
    assert_eq!(affected.ordered_cell_ids, ["a", "dependent", "tail"]);
    assert_eq!(
        s.plan_cell_execution(
            &["a".into()],
            Scope::Affected,
            &["a".into()].into(),
            &Interrupt::default()
        )
        .unwrap_err(),
        "PERMISSION_DENIED"
    );
    assert_eq!(
        s.plan_cell_execution(
            &["tail".into()],
            Scope::Required,
            &["tail".into()].into(),
            &Interrupt::default()
        )
        .unwrap_err(),
        "PERMISSION_DENIED"
    );
    println!(
        "actual affected plan={}",
        serde_json::to_string(&affected).unwrap()
    );
}
#[test]
fn planning_preserves_exact_checkpoint_with_actual_definitions_out_and_random() {
    let mut s = setup(vec![
        math("a", "let a=2;seed_random(31);rand()"),
        math("b", "let b=a+1;b"),
        math("tail", "b+1"),
    ]);
    eval(&mut s, "a", "let a=2;seed_random(31);rand()");
    eval(&mut s, "b", "let b=a+1;b");
    let binding = CheckpointBinding {
        document_id: "actual-plan".into(),
        document_generation: 1,
        source_revision: 1,
        execution_epoch: 1,
        kernel_state_revision: 2,
        source_snapshot_hash: "0".repeat(64),
        build: "run-plan".into(),
    };
    let before = s
        .encode_checkpoint(&binding, CheckpointLimits::default(), &Interrupt::default())
        .unwrap();
    for scope in [Scope::Selected, Scope::Required, Scope::Affected] {
        let p = plan(&s, &["a"], scope);
        assert!(!p.ordered_cell_ids.is_empty());
    }
    let after = s
        .encode_checkpoint(&binding, CheckpointLimits::default(), &Interrupt::default())
        .unwrap();
    assert_eq!(before, after);
    println!(
        "actual complete checkpoint unchanged: {} bytes",
        before.len()
    );
}
#[test]
fn live_dynamic_owner_controls_prerequisites_and_order_instead_of_syntactic_name_guesses() {
    let mut function = math("function", "f[x_]:=(a=x)");
    function.dialect = Dialect::Wolfram;
    let mut caller = math("caller", "f[2]");
    caller.dialect = Dialect::Wolfram;
    let mut s = setup(vec![math("consumer", "a+1"), caller, function]);
    for (id, source) in [("function", "f[x_]:=(a=x)"), ("caller", "f[2]")] {
        assert!(matches!(
            s.handle(Request::Evaluate {
                cell_id: id.into(),
                source: source.into(),
                dialect: Dialect::Wolfram
            })
            .0,
            Response::Evaluated { .. }
        ));
    }
    let owned = s.execution_context();
    assert!(
        owned
            .owned
            .iter()
            .any(|o| o.symbol == "a" && o.cell_id == "caller")
    );
    let ready = plan(&s, &["consumer"], Scope::Required);
    assert_eq!(ready.ordered_cell_ids, ["consumer"]);
    // Recomputing the actual owner and reader in a single explicit request must order the real
    // global-write owner first, even though its source is a call and the reader precedes it.
    let both = plan(&s, &["consumer", "caller"], Scope::Selected);
    assert_eq!(both.ordered_cell_ids, ["caller", "consumer"]);
    assert!(both.edges.contains(&("caller".into(), "consumer".into())));
    println!(
        "actual dynamic owner and plan={}",
        serde_json::json!({"owners":owned.owned.iter().map(|o|serde_json::json!({"symbol":o.symbol,"cell_id":o.cell_id})).collect::<Vec<_>>(),"plan":both})
    );
}
#[test]
fn real_cycles_blocked_descendants_invalid_inputs_and_direct_cancellation_remain_explicit() {
    let s = setup(vec![
        math("a", "let a=b+1"),
        math("b", "let b=a+1"),
        math("tail", "a+b"),
        math("free", "2+2"),
        CellInput {
            id: "text".into(),
            source: "prose".into(),
            kind: CellKind::Text,
            dialect: Dialect::Modern,
        },
    ]);
    let p = plan(&s, &["tail", "free"], Scope::Required);
    assert_eq!(p.cycle_cell_ids, ["a", "b"]);
    assert_eq!(p.blocked_cell_ids, ["tail"]);
    assert_eq!(p.ordered_cell_ids, ["free"]);
    for (ids, error) in [
        (vec![], "INVALID_ARGUMENT"),
        (vec!["a".into(), "a".into()], "INVALID_ARGUMENT"),
        (vec!["missing".into()], "INVALID_CELL_REFERENCE"),
        (vec!["text".into()], "INVALID_CELL_KIND"),
    ] {
        assert_eq!(
            s.plan_cell_execution(&ids, Scope::Selected, &allowed(&s), &Interrupt::default())
                .unwrap_err(),
            error
        );
    }
    let cancelled = Interrupt {
        flag: Arc::new(AtomicBool::new(true)),
        ..Interrupt::default()
    };
    assert_eq!(
        s.plan_cell_execution(&["free".into()], Scope::Required, &allowed(&s), &cancelled)
            .unwrap_err(),
        "CANCELLED"
    );
}
#[test]
fn long_actual_dependency_expansion_and_mid_traversal_budget_are_bounded_without_execution() {
    let mut cells = vec![math("a0", "let a0=1")];
    for i in 1..1002 {
        cells.push(math(&format!("a{i}"), &format!("let a{i}=a{}+1", i - 1)));
    }
    let s = setup(cells);
    assert_eq!(
        s.plan_cell_execution(
            &["a0".into()],
            Scope::Affected,
            &allowed(&s),
            &Interrupt::default()
        )
        .unwrap_err(),
        "LIMIT_EXCEEDED"
    );
    let budget = Interrupt {
        steps_left: std::cell::Cell::new(50),
        ..Interrupt::default()
    };
    assert_eq!(
        s.plan_cell_execution(&["a0".into()], Scope::Affected, &allowed(&s), &budget)
            .unwrap_err(),
        "LIMIT_EXCEEDED"
    );
    assert!(s.notebook.cells.iter().all(|c| c.exec_count.is_none()));
}
