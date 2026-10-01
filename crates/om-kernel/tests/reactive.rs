//! Reactive protocol acceptance uses the real parser, evaluator and solver.
pub mod support;
use om_core::Symbol;
use om_kernel::{Session, protocol::*};
use support::{expressions, same};

fn session() -> Session {
    Session::new(Default::default(), None)
}
fn evaluate(s: &mut Session, id: &str, source: &str) -> (CellOutput, Vec<String>, Vec<Event>) {
    let (r, e) = s.handle(Request::Evaluate {
        cell_id: id.into(),
        source: source.into(),
        dialect: Dialect::Modern,
    });
    match r {
        Response::Evaluated { output, reran, .. } => (output, reran, e),
        other => panic!("{other:?}"),
    }
}
fn upsert(s: &mut Session, id: &str, source: &str, kind: CellKind) -> Vec<Event> {
    let (r, e) = s.handle(Request::UpsertCell {
        cell: CellInput {
            id: id.into(),
            kind,
            source: source.into(),
            dialect: Dialect::Modern,
        },
    });
    assert!(matches!(r, Response::Ok), "{r:?}");
    e
}
fn value(s: &Session, id: &str) -> String {
    expressions(
        s.notebook
            .cells
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .output
            .as_ref()
            .unwrap(),
    )
    .last()
    .unwrap()
    .1
    .clone()
}
fn status(s: &Session, id: &str) -> CellStatus {
    s.notebook.cells.iter().find(|c| c.id == id).unwrap().status
}
fn ids(s: &Session) -> Vec<&str> {
    s.notebook.cells.iter().map(|c| c.id.as_str()).collect()
}
fn event_order(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            Event::CellStatus { cell_id, status } => format!("{cell_id}:{status:?}"),
            Event::CellOutput { cell_id, .. } => format!("{cell_id}:output"),
            _ => panic!("{e:?}"),
        })
        .collect()
}

#[test]
fn authority_edit_recalculates_real_solutions_and_keeps_actual_history_steps() {
    let mut s = session();
    evaluate(&mut s, "a", "let a = 2");
    evaluate(&mut s, "b", "solve(x^2 = a, x)");
    let (_, reran, events) = evaluate(&mut s, "a", "let a = 9");
    assert_eq!(reran, ["b"]);
    same(&value(&s, "b"), "{{x->-3},{x->3}}");
    assert_eq!(
        event_order(&events),
        ["b:Queued", "b:Running", "b:Done", "b:output"]
    );
    let b = &s.notebook.cells[1];
    assert_eq!(b.exec_count, Some(4));
    assert!(b.input(4).is_some());
    assert!(b.steps(4).is_some());
    let (r, _) = s.handle(Request::Evaluate {
        cell_id: "history".into(),
        source: "Out[3]".into(),
        dialect: Dialect::Wolfram,
    });
    let Response::Evaluated { output: o, .. } = r else {
        panic!()
    };
    assert_eq!(expressions(&o)[0], (5, "9".into()));
}

#[test]
fn conflicts_do_not_execute_or_clear_prior_successful_definitions() {
    let mut s = session();
    evaluate(&mut s, "a", "let a=2");
    evaluate(&mut s, "b", "let b=4");
    let (o, reran, _) = evaluate(&mut s, "b", "let a=9;\nlet b=99");
    assert!(o.items.iter().any(|i| matches!(i,OutputItem::Error{message,..} if message.contains("err.multiple_definitions") && message.contains("a") && message.contains('1'))));
    assert!(reran.is_empty());
    assert_eq!(status(&s, "b"), CellStatus::Error);
    assert_eq!(expressions(&evaluate(&mut s, "probe", "a+b").0)[0].1, "6");
}

#[test]
fn diamond_and_layer_order_are_stable_and_unrelated_cells_are_untouched() {
    let mut s = session();
    for (id, src) in [
        ("a", "let a=1"),
        ("end", "let z=b+c"),
        ("c", "let c=a+2"),
        ("b", "let b=a+1"),
        ("ind", "123"),
    ] {
        evaluate(&mut s, id, src);
    }
    let count = s.notebook.cells[4].exec_count;
    let (_, reran, events) = evaluate(&mut s, "a", "let a=10");
    assert_eq!(reran, ["c", "b", "end"]);
    assert_eq!(value(&s, "end"), "23");
    assert_eq!(s.notebook.cells[4].exec_count, count);
    assert_eq!(
        event_order(&events)
            .iter()
            .filter(|e| e.ends_with(":output"))
            .cloned()
            .collect::<Vec<_>>(),
        ["c:output", "b:output", "end:output"]
    );
}

#[test]
fn genuine_cycles_error_and_downstream_is_stale_then_recovers() {
    let mut s = session();
    evaluate(&mut s, "a", "let a=b+1");
    evaluate(&mut s, "b", "let b=2");
    evaluate(&mut s, "c", "a+10");
    let (_, reran, events) = evaluate(&mut s, "b", "let b=a+1");
    assert!(reran.is_empty());
    for id in ["a", "b"] {
        assert_eq!(status(&s, id), CellStatus::Error);
        assert!(
            s.notebook
                .cells
                .iter()
                .find(|c| c.id == id)
                .unwrap()
                .output
                .as_ref()
                .unwrap()
                .items
                .iter()
                .any(
                    |i| matches!(i,OutputItem::Error{message,..} if message.contains("err.cycle"))
                )
        );
    }
    assert_eq!(status(&s, "c"), CellStatus::Stale);
    assert!(event_order(&events).contains(&"c:Stale".into()));
    let (_, reran, _) = evaluate(&mut s, "b", "let b=5");
    assert_eq!(reran, ["a", "c"]);
    assert_eq!(value(&s, "c"), "16");
}

#[test]
fn switches_keep_stale_outputs_and_sequential_redefinitions() {
    let mut s = session();
    evaluate(&mut s, "a", "let a=2");
    evaluate(&mut s, "b", "a+1");
    s.config.general.auto_run_dependents = false;
    let (_, reran, events) = evaluate(&mut s, "a", "let a=9");
    assert!(reran.is_empty());
    assert_eq!(status(&s, "b"), CellStatus::Stale);
    assert_eq!(value(&s, "b"), "3");
    assert_eq!(event_order(&events), ["b:Stale"]);
    s.config.general.reactive = false;
    evaluate(&mut s, "other", "let a=20");
    let (_, reran, events) = evaluate(&mut s, "a", "a+1");
    assert_eq!(value(&s, "a"), "21");
    assert!(reran.is_empty() && events.is_empty());
    assert!(matches!(
        s.handle(Request::DeleteCell {
            cell_id: "a".into()
        })
        .0,
        Response::Ok
    ));
    assert_eq!(expressions(&evaluate(&mut s, "probe", "a").0)[0].1, "20");
}

#[test]
fn raw_user_function_heads_and_scoped_patterns_drive_dependencies() {
    let mut s = session();
    evaluate(&mut s, "f", "let f(t)=t+1");
    evaluate(&mut s, "use", "f(2)");
    let (_, reran, _) = evaluate(&mut s, "f", "let f(t)=t+3");
    assert_eq!(reran, ["use"]);
    assert_eq!(value(&s, "use"), "5");
    assert_eq!(s.notebook.cells[0].defines, [Symbol::intern("f")].into());
    assert!(s.notebook.cells[0].uses.is_empty());
    assert!(s.notebook.cells[1].uses.contains(&Symbol::intern("f")));
    for (src, defs, uses) in [
        ("f[x_]/;x>limit:=g[x]+a", vec!["f"], vec!["a", "g", "limit"]),
        ("Function[{x},x+a][b]+x", vec![], vec!["a", "b", "x"]),
        (
            "Table[f[i,j],{i,n},{j,i,m}]+i",
            vec![],
            vec!["f", "i", "m", "n"],
        ),
        (
            "Sum[a*i,{i,i,n}]+Product[b*j,{j,m}]",
            vec![],
            vec!["a", "b", "i", "m", "n"],
        ),
        ("x/.{t_:>t+a}", vec![], vec!["a", "x"]),
        ("letIgnored", vec![], vec!["letIgnored"]),
    ] {
        let (r, _) = s.handle(Request::UpsertCell {
            cell: CellInput {
                id: "syntax".into(),
                kind: CellKind::Math,
                source: src.into(),
                dialect: Dialect::Wolfram,
            },
        });
        assert!(matches!(r, Response::Ok));
        let cell = s.notebook.cells.iter().find(|c| c.id == "syntax").unwrap();
        let names = |set: &std::collections::BTreeSet<Symbol>| {
            let mut n = set.iter().map(|x| x.name()).collect::<Vec<_>>();
            n.sort();
            n
        };
        assert_eq!(names(&cell.defines), defs, "{src}");
        assert_eq!(names(&cell.uses), uses, "{src}");
    }
}

#[test]
fn edit_parse_failure_delete_and_kind_change_preserve_correct_live_ownership() {
    let mut s = session();
    evaluate(&mut s, "a", "let a=2");
    evaluate(&mut s, "use", "a+1");
    upsert(&mut s, "a", "let b=9", CellKind::Math);
    assert_eq!(status(&s, "a"), CellStatus::Stale);
    assert_eq!(status(&s, "use"), CellStatus::Stale);
    assert_eq!(expressions(&evaluate(&mut s, "probe", "a").0)[0].1, "2");
    let (o, _, _) = evaluate(&mut s, "a", "let b = (");
    assert!(
        o.items
            .iter()
            .any(|i| matches!(i, OutputItem::Error { .. }))
    );
    assert_eq!(expressions(&evaluate(&mut s, "probe", "a").0)[0].1, "2");
    evaluate(&mut s, "a", "let b=9");
    same(&value(&s, "use"), "a+1");
    evaluate(&mut s, "new", "let a=7");
    assert!(matches!(
        s.handle(Request::DeleteCell {
            cell_id: "a".into()
        })
        .0,
        Response::Ok
    ));
    same(
        &expressions(&evaluate(&mut s, "probe", "a+b").0)[0].1,
        "7+b",
    );
    upsert(&mut s, "new", "prose", CellKind::Text);
    assert_eq!(status(&s, "new"), CellStatus::Done);
    assert!(
        s.notebook
            .cells
            .iter()
            .find(|c| c.id == "new")
            .unwrap()
            .defines
            .is_empty()
    );
    assert_eq!(status(&s, "use"), CellStatus::Stale);
    assert_eq!(value(&s, "use"), "8");
}

#[test]
fn editing_is_source_only_move_is_atomic_and_run_all_uses_document_order() {
    let mut s = session();
    upsert(&mut s, "b", "a+1", CellKind::Math);
    upsert(&mut s, "a", "let a=3", CellKind::Math);
    upsert(&mut s, "ask", "let a=100", CellKind::Ask);
    assert!(s.notebook.cells.iter().all(|c| c.exec_count.is_none()));
    assert!(matches!(
        s.handle(Request::MoveCell {
            cell_id: "a".into(),
            to_index: 0
        })
        .0,
        Response::Ok
    ));
    assert_eq!(ids(&s), ["a", "b", "ask"]);
    for req in [
        Request::MoveCell {
            cell_id: "a".into(),
            to_index: 3,
        },
        Request::MoveCell {
            cell_id: "missing".into(),
            to_index: 0,
        },
        Request::DeleteCell {
            cell_id: "missing".into(),
        },
        Request::UpsertCell {
            cell: CellInput {
                id: "".into(),
                kind: CellKind::Math,
                source: "1".into(),
                dialect: Dialect::Modern,
            },
        },
    ] {
        assert!(matches!(s.handle(req).0, Response::Error { .. }));
        assert_eq!(ids(&s), ["a", "b", "ask"]);
    }
    let (r, events) = s.handle(Request::RunAll);
    assert!(matches!(r, Response::Ok));
    assert_eq!(value(&s, "b"), "4");
    assert_eq!(s.notebook.cells[0].exec_count, Some(1));
    assert_eq!(s.notebook.cells[1].exec_count, Some(2));
    assert_eq!(
        event_order(&events),
        [
            "a:Queued",
            "a:Running",
            "a:Done",
            "a:output",
            "b:Queued",
            "b:Running",
            "b:Done",
            "b:output"
        ]
    );
    let (r, _) = s.handle(Request::SaveNotebook);
    let Response::Notebook { file } = r else {
        panic!()
    };
    assert_eq!(
        file.cells.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["a", "b", "ask"]
    );
    assert!(matches!(
        s.handle(Request::LoadNotebook { file }).0,
        Response::Ok
    ));
    assert!(s.notebook.cells[1].uses.contains(&Symbol::intern("a")));
    assert!(s.notebook.cells[2].defines.is_empty());
}

#[test]
fn failed_prerequisites_block_descendants_but_independent_branches_run() {
    let mut s = session();
    for (id, src) in [
        ("a", "let a=1"),
        ("bad", "let b=1/a"),
        ("child", "b+1"),
        ("good", "a+10"),
    ] {
        evaluate(&mut s, id, src);
    }
    // A parser failure is atomic but still makes its dependency closure stale.
    let (_, reran, _) = evaluate(&mut s, "a", "let a = (");
    assert!(reran.is_empty());
    for id in ["bad", "child", "good"] {
        assert_eq!(status(&s, id), CellStatus::Stale);
    }
    evaluate(&mut s, "a", "let a=2");
    upsert(
        &mut s,
        "bad",
        "let b=4;\nlet loop=loop;\nloop",
        CellKind::Math,
    );
    evaluate(&mut s, "a", "let a=3");
    // Restore a dependency while exercising a genuine evaluator recursion failure.
    upsert(&mut s, "bad", "let b=a;\nloop:=loop\nloop", CellKind::Math);
    let (_, reran, _) = evaluate(&mut s, "a", "let a=4");
    assert_eq!(reran, ["bad", "good"]);
    assert_eq!(status(&s, "bad"), CellStatus::Error);
    assert_eq!(status(&s, "child"), CellStatus::Stale);
    assert_eq!(value(&s, "good"), "14");
}
