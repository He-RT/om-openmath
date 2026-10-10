//! Actual source/definition/results candidates do not mutate the parent or inherit model IO.
use om_kernel::{KernelConfig, Session, protocol::*};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
fn eval(session: &mut Session, id: &str, source: &str) -> CellOutput {
    let reply = session
        .handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: Dialect::Modern,
        })
        .0;
    let Response::Evaluated { output, .. } = reply else {
        panic!("{reply:?}")
    };
    output
}
fn value(output: &CellOutput) -> &str {
    let OutputItem::Expr { input_form, .. } = output.items.last().unwrap() else {
        panic!("{output:?}")
    };
    input_form
}
#[test]
fn working_notebook_retains_real_results_and_owners_then_can_commit_new_definitions() {
    let mut owner = Session::new(KernelConfig::default(), None);
    assert_eq!(value(&eval(&mut owner, "a", "let a=2; a")), "2");
    assert_eq!(value(&eval(&mut owner, "b", "a+1")), "3");
    let before = serde_json::to_value(owner.handle(Request::GetNotebookState).0).unwrap();
    let original_output =
        serde_json::to_value(owner.notebook.cells[1].output.as_ref().unwrap()).unwrap();
    let mut candidate = owner
        .fork_working_session(Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(
        serde_json::to_value(
            candidate.session_mut().notebook.cells[1]
                .output
                .as_ref()
                .unwrap()
        )
        .unwrap(),
        original_output
    );
    assert_eq!(
        value(&eval(candidate.session_mut(), "a", "let a=5; a")),
        "5"
    );
    assert_eq!(value(&eval(candidate.session_mut(), "b", "a+1")), "6");
    assert_eq!(
        serde_json::to_value(owner.handle(Request::GetNotebookState).0).unwrap(),
        before
    );
    let mut accepted = candidate.into_session();
    assert_eq!(value(&eval(&mut accepted, "probe", "a+1")), "6");
    assert_eq!(value(&eval(&mut owner, "probe", "a+1")), "3");
}
#[test]
fn actual_steps_and_readonly_explore_snapshots_survive_the_memory_candidate() {
    let mut owner = Session::new(Default::default(), None);
    assert!(
        owner
            .fork_working_session(owner.interrupt_handle())
            .is_err()
    );
    let solved = eval(&mut owner, "solve", "solve(x^2=2,x)");
    let out = solved.items[0].clone();
    let OutputItem::Solutions { out_index, .. } = out else {
        panic!("{solved:?}")
    };
    let step_count = owner.notebook.cells[0].steps(out_index).unwrap().root.len();
    assert!(step_count > 0);
    let explore = eval(&mut owner, "explore", "explore(a^2,controls:{a:0..4})");
    let OutputItem::Explore {
        out_index, view_id, ..
    } = &explore.items[0]
    else {
        panic!("{explore:?}")
    };
    let mut candidate = owner
        .fork_working_session(Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(
        candidate.session_mut().notebook.cells[0]
            .steps(match &solved.items[0] {
                OutputItem::Solutions { out_index, .. } => *out_index,
                _ => unreachable!(),
            })
            .unwrap()
            .root
            .len(),
        step_count
    );
    let reply = candidate
        .session_mut()
        .handle(Request::SampleExplore {
            query: ExploreQuery {
                cell_id: "explore".into(),
                out_index: *out_index,
                view_id: view_id.clone(),
            },
            values: std::collections::BTreeMap::from([("a".into(), 4.)]),
            revision: 1,
        })
        .0;
    let Response::Explored { result } = reply else {
        panic!("{reply:?}")
    };
    let OutputItem::Expr { input_form, .. } = *result.item else {
        panic!("not actual expression")
    };
    assert_eq!(input_form, "16.");
    assert_eq!(value(&eval(&mut owner, "probe", "a")), "a");
}
#[test]
fn candidate_credentials_io_and_cancellation_are_independent_from_parent() {
    let mut owner = Session::new(Default::default(), None);
    owner.config.llm.enabled = true;
    owner.config.llm.profiles[0].api_key = Some("synthetic-never-copy".into());
    owner.interrupt_handle().store(true, Ordering::Relaxed);
    let token = Arc::new(AtomicBool::new(false));
    let mut candidate = owner.fork_working_session(token.clone()).unwrap();
    assert!(!candidate.session_mut().config.llm.enabled);
    assert!(candidate.session_mut().config.llm.profiles.is_empty());
    assert_eq!(value(&eval(candidate.session_mut(), "probe", "2+2")), "4");
    assert!(owner.interrupt_handle().load(Ordering::Relaxed));
    token.store(true, Ordering::Relaxed);
    let interrupted = eval(
        candidate.session_mut(),
        "stopped",
        "map(fn(k)=>sin(k),range(1,1000))",
    );
    assert!(
        interrupted
            .messages
            .iter()
            .any(|m| m.symbol == "Kernel" && m.tag == "interrupted"),
        "{interrupted:?}"
    );
    assert!(token.load(Ordering::Relaxed));
    owner.notebook.cells.clear();
    eval(&mut owner, "state", "1");
    owner.notebook.cells[0].status = CellStatus::Running;
    assert!(
        owner
            .fork_working_session(Arc::new(AtomicBool::new(false)))
            .is_err()
    );
}
