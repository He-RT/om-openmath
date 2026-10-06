//! Ownership, failure and cancellation regressions through the shared Session.
pub mod support;
use om_core::Symbol;
use om_kernel::{Session, protocol::*};
use om_num::ctx::Clock;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use support::{expressions, same};

fn eval(s: &mut Session, id: &str, src: &str) -> (CellOutput, Vec<String>, Vec<Event>) {
    let (r, e) = s.handle(Request::Evaluate {
        cell_id: id.into(),
        source: src.into(),
        dialect: Dialect::Wolfram,
    });
    let Response::Evaluated { output, reran, .. } = r else {
        panic!("{r:?}")
    };
    (output, reran, e)
}
fn upsert(s: &mut Session, id: &str, src: &str) {
    assert!(matches!(
        s.handle(Request::UpsertCell {
            cell: CellInput {
                id: id.into(),
                source: src.into(),
                dialect: Dialect::Wolfram,
                kind: CellKind::Math
            }
        })
        .0,
        Response::Ok
    ));
}
fn cell<'a>(s: &'a Session, id: &str) -> &'a om_kernel::Cell {
    s.notebook.cells.iter().find(|c| c.id == id).unwrap()
}
fn value(s: &Session, id: &str) -> String {
    expressions(cell(s, id).output.as_ref().unwrap())
        .last()
        .unwrap()
        .1
        .clone()
}

#[test]
fn partial_effects_clear_unset_and_identical_reassignments_have_actual_ownership() {
    let mut s = Session::new(Default::default(), None);
    let (o, _, _) = eval(&mut s, "partial", "a=2; loop:=loop; loop; never=5");
    assert!(
        o.items
            .iter()
            .any(|i| matches!(i, OutputItem::Error { .. }))
    );
    assert_eq!(expressions(&eval(&mut s, "probe", "a").0)[0].1, "2");
    assert!(matches!(
        s.handle(Request::DeleteCell {
            cell_id: "partial".into()
        })
        .0,
        Response::Ok
    ));
    same(
        &expressions(&eval(&mut s, "probe", "a+never").0)[0].1,
        "a+never",
    );
    eval(&mut s, "a", "a=2");
    eval(&mut s, "clear", "Clear[a]");
    s.handle(Request::DeleteCell {
        cell_id: "clear".into(),
    });
    eval(&mut s, "new", "a=2");
    assert!(matches!(
        s.handle(Request::DeleteCell {
            cell_id: "a".into()
        })
        .0,
        Response::Ok
    ));
    assert_eq!(expressions(&eval(&mut s, "probe", "a").0)[0].1, "2");
    eval(&mut s, "unset", "Unset[a]");
    s.handle(Request::DeleteCell {
        cell_id: "unset".into(),
    });
    eval(&mut s, "last", "a=9");
    assert!(matches!(
        s.handle(Request::DeleteCell {
            cell_id: "new".into()
        })
        .0,
        Response::Ok
    ));
    assert_eq!(expressions(&eval(&mut s, "probe", "a").0)[0].1, "9");
}

#[test]
fn deferred_global_sets_and_lexical_local_sets_do_not_claim_unexecuted_values() {
    let mut s = Session::new(Default::default(), None);
    eval(&mut s, "deferred", "f[x_]:=(a=x)");
    assert_eq!(
        cell(&s, "deferred").defines,
        [Symbol::intern("f"), Symbol::intern("a")].into()
    );
    // a is only potential in deferred source and is not live yet.
    let (o, _, _) = eval(&mut s, "a", "a=7");
    assert_eq!(expressions(&o)[0].1, "7");
    eval(&mut s, "deferred", "f[x_]:=(x=3)");
    assert_eq!(cell(&s, "deferred").defines, [Symbol::intern("f")].into());
    eval(&mut s, "local", "f[8]");
    same(&expressions(&eval(&mut s, "probe", "a+x").0)[0].1, "7+x");
    assert!(matches!(
        s.handle(Request::DeleteCell {
            cell_id: "deferred".into()
        })
        .0,
        Response::Ok
    ));
    assert_eq!(expressions(&eval(&mut s, "probe", "a").0)[0].1, "7");
}

#[test]
fn delete_auto_recalculates_the_transitive_closure_and_disabled_mode_only_stales() {
    for auto in [true, false] {
        let mut s = Session::new(Default::default(), None);
        s.config.general.auto_run_dependents = auto;
        for (id, src) in [("a", "a=2"), ("b", "b=a+1"), ("c", "b+1"), ("ind", "10")] {
            eval(&mut s, id, src);
        }
        let old = cell(&s, "c").exec_count;
        let (r, e) = s.handle(Request::DeleteCell {
            cell_id: "a".into(),
        });
        assert!(matches!(r, Response::Ok));
        if auto {
            same(&value(&s, "c"), "a+2");
            assert_eq!(cell(&s, "c").status, CellStatus::Done);
            assert_eq!(
                e.iter()
                    .filter(|x| matches!(x, Event::CellOutput { .. }))
                    .count(),
                2
            );
        } else {
            assert_eq!(cell(&s, "b").status, CellStatus::Stale);
            assert_eq!(cell(&s, "c").status, CellStatus::Stale);
            assert_eq!(cell(&s, "c").exec_count, old);
        }
        assert_eq!(cell(&s, "ind").exec_count, Some(4));
    }
}

#[test]
fn stale_prerequisite_outside_the_affected_closure_blocks_execution_until_repaired() {
    let mut s = Session::new(Default::default(), None);
    for (id, src) in [("a", "a=1"), ("c", "c=2"), ("b", "a+c")] {
        eval(&mut s, id, src);
    }
    let old = cell(&s, "b").exec_count;
    upsert(&mut s, "c", "c=8");
    let (_, reran, _) = eval(&mut s, "a", "a=3");
    assert!(reran.is_empty());
    assert_eq!(cell(&s, "b").status, CellStatus::Stale);
    assert_eq!(cell(&s, "b").exec_count, old);
    let (_, reran, _) = eval(&mut s, "c", "c=8");
    assert_eq!(reran, ["b"]);
    assert_eq!(value(&s, "b"), "11");
}

struct CancellingClock {
    enabled: AtomicBool,
    handle: Mutex<Option<Arc<AtomicBool>>>,
}
impl Clock for CancellingClock {
    fn now_ms(&self) -> f64 {
        if self.enabled.load(Ordering::Relaxed)
            && let Some(flag) = self.handle.lock().unwrap().as_ref()
        {
            flag.store(true, Ordering::Relaxed);
        }
        0.0
    }
}
#[test]
fn one_cascade_preserves_interrupt_and_a_new_request_recovers() {
    let clock = Arc::new(CancellingClock {
        enabled: AtomicBool::new(false),
        handle: Mutex::new(None),
    });
    let mut s = Session::new(Default::default(), Some(clock.clone()));
    *clock.handle.lock().unwrap() = Some(s.interrupt_handle());
    for (id, src) in [("a", "a=1"), ("b", "b=a+1"), ("c", "b+1")] {
        eval(&mut s, id, src);
    }
    clock.enabled.store(true, Ordering::Relaxed);
    let (o, reran, _) = eval(&mut s, "a", "a=3");
    assert!(o.messages.iter().any(|m| m.tag == "interrupted"));
    assert!(reran.is_empty());
    assert_eq!(cell(&s, "b").status, CellStatus::Stale);
    assert_eq!(cell(&s, "c").status, CellStatus::Stale);
    clock.enabled.store(false, Ordering::Relaxed);
    let (_, reran, _) = eval(&mut s, "a", "a=4");
    assert_eq!(reran, ["b", "c"]);
    assert_eq!(value(&s, "c"), "6");
    s.handle(Request::Interrupt);
    let (_, events) = s.handle(Request::DeleteCell {
        cell_id: "a".into(),
    });
    assert_eq!(cell(&s, "b").status, CellStatus::Done);
    assert!(
        events
            .iter()
            .any(|e| matches!(e,Event::CellOutput{cell_id,..} if cell_id=="c"))
    );
}

#[test]
fn scopes_do_not_hide_symbols_used_outside_the_binding_expression() {
    let mut s = Session::new(Default::default(), None);
    for (src, defs, uses) in [
        ("f[x_]:=x+a; x", vec!["f"], vec!["a", "x"]),
        (
            "Function[x,Function[y,x+y+g[z]]]+y",
            vec![],
            vec!["g", "y", "z"],
        ),
        ("Table[f[i],{i,n}]+i", vec![], vec!["f", "i", "n"]),
        ("f[x_]:=(g[y_]:=x+y+a)", vec!["f", "g"], vec!["a"]),
        ("Function[x,x=2]+x", vec![], vec!["x"]),
        ("f[x_]:=g[x]/;x>limit", vec!["f"], vec!["g", "limit"]),
    ] {
        upsert(&mut s, "syntax", src);
        let names = |symbols: &std::collections::BTreeSet<Symbol>| {
            let mut n = symbols.iter().map(|s| s.name()).collect::<Vec<_>>();
            n.sort();
            n
        };
        assert_eq!(names(&cell(&s, "syntax").defines), defs, "{src}");
        assert_eq!(names(&cell(&s, "syntax").uses), uses, "{src}");
    }
}

struct MidCascadeClock {
    armed: AtomicBool,
    calls: std::sync::atomic::AtomicUsize,
    handle: Mutex<Option<Arc<AtomicBool>>>,
}
impl Clock for MidCascadeClock {
    fn now_ms(&self) -> f64 {
        if self.armed.load(Ordering::Relaxed)
            && self.calls.fetch_add(1, Ordering::Relaxed) == 3
            && let Some(flag) = self.handle.lock().unwrap().as_ref()
        {
            flag.store(true, Ordering::Relaxed);
        }
        0.0
    }
}
#[test]
fn cancellation_during_auto_execution_keeps_the_completed_root_and_blocks_descendants() {
    let clock = Arc::new(MidCascadeClock {
        armed: AtomicBool::new(false),
        calls: Default::default(),
        handle: Mutex::new(None),
    });
    let mut s = Session::new(Default::default(), Some(clock.clone()));
    *clock.handle.lock().unwrap() = Some(s.interrupt_handle());
    for (id, src) in [("a", "a=1"), ("b", "b=a+1"), ("c", "b+1")] {
        eval(&mut s, id, src);
    }
    clock.armed.store(true, Ordering::Relaxed);
    let (o, reran, events) = eval(&mut s, "a", "a=4");
    assert_eq!(expressions(&o)[0].1, "4");
    assert_eq!(reran, ["b"]);
    assert_eq!(cell(&s, "a").status, CellStatus::Done);
    assert_eq!(cell(&s, "b").status, CellStatus::Error);
    assert_eq!(cell(&s, "c").status, CellStatus::Stale);
    assert!(
        cell(&s, "b")
            .output
            .as_ref()
            .unwrap()
            .messages
            .iter()
            .any(|m| m.tag == "interrupted")
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::CellOutput { .. }))
            .count(),
        1
    );
    clock.armed.store(false, Ordering::Relaxed);
    let (_, reran, _) = eval(&mut s, "a", "a=5");
    assert_eq!(reran, ["b", "c"]);
    assert_eq!(value(&s, "c"), "7");
}

#[test]
fn run_all_cycle_errors_preserve_real_history_of_earlier_evaluations() {
    let mut s = Session::new(Default::default(), None);
    eval(&mut s, "a", "a=b+1");
    eval(&mut s, "b", "b=2");
    eval(&mut s, "c", "a+10");
    upsert(&mut s, "b", "b=a+1");
    let a_count = cell(&s, "a").exec_count.unwrap();
    let (r, events) = s.handle(Request::RunAll);
    assert!(matches!(r, Response::Ok));
    assert_eq!(cell(&s, "a").status, CellStatus::Error);
    assert_eq!(cell(&s, "b").status, CellStatus::Error);
    assert_eq!(cell(&s, "c").status, CellStatus::Stale);
    assert_eq!(cell(&s, "a").exec_count, Some(a_count));
    assert!(cell(&s, "a").input(a_count).is_some());
    assert!(!events.iter().any(|e| matches!(
        e,
        Event::CellStatus {
            status: CellStatus::Running,
            ..
        }
    )));
}

#[test]
fn dependency_analysis_excludes_registered_builtins_beyond_core_symbol_table() {
    let mut s = Session::new(Default::default(), None);
    upsert(
        &mut s,
        "syntax",
        "First[data]+Rest[data]+Append[data,a]+Divide[a,b]+Subtract[a,b]+Length[data]",
    );
    assert_eq!(
        cell(&s, "syntax").uses,
        [
            Symbol::intern("a"),
            Symbol::intern("b"),
            Symbol::intern("data")
        ]
        .into()
    );
    upsert(&mut s, "syntax", "Sum=2;First=3;Unprotect=4");
    assert!(
        cell(&s, "syntax")
            .defines
            .contains(&Symbol::intern("Unprotect"))
    ); // no such builtin is implemented
    assert!(
        !cell(&s, "syntax")
            .defines
            .contains(&Symbol::intern("First"))
    );
}
#[test]
fn calculus_coordinates_are_local_but_parameters_and_bounds_remain_real_dependencies() {
    let mut s = Session::new(Default::default(), None);
    for (src, uses) in [
        ("Series[a*x,{x,b,3}]", vec!["a", "b"]),
        ("Limit[a*x,x->b]", vec!["a", "b"]),
        ("Integrate[a*x,{x,b,c}]", vec!["a", "b", "c"]),
        ("Grad[a*x*y,{x,y}]", vec!["a"]),
        ("Grad[x=2,{x}]", vec![]),
        (
            "Ode[Function[{t,y},a*y],{t,b,c},Initial->d]",
            vec!["a", "b", "c", "d"],
        ),
        (
            "Sample[Function[x,a*x],{x,b,c},Count->5]",
            vec!["a", "b", "c"],
        ),
    ] {
        upsert(&mut s, "calculus", src);
        let mut actual = cell(&s, "calculus")
            .uses
            .iter()
            .map(|s| s.name())
            .collect::<Vec<_>>();
        actual.sort();
        assert_eq!(actual, uses, "{src}");
        assert!(cell(&s, "calculus").defines.is_empty());
    }
}

#[test]
fn edited_away_live_definitions_do_not_create_false_source_cycles() {
    let mut s = Session::new(Default::default(), None);
    eval(&mut s, "a", "a=2");
    eval(&mut s, "b", "b=a+1");
    upsert(&mut s, "a", "c=b");
    let (r, events) = s.handle(Request::RunAll);
    assert!(matches!(r, Response::Ok));
    assert_eq!(cell(&s, "a").status, CellStatus::Done);
    assert_eq!(cell(&s, "b").status, CellStatus::Done);
    assert!(!events.iter().any(|e|matches!(e,Event::CellOutput{output,..} if output.messages.iter().any(|m|m.tag=="err.cycle"))));
    assert_eq!(cell(&s, "a").defines, [Symbol::intern("c")].into());
    same(&value(&s, "b"), "a+1");
}

#[test]
fn rejected_conflicting_evaluate_does_not_invalidate_the_other_owners_dependents() {
    let mut s = Session::new(Default::default(), None);
    eval(&mut s, "owner", "a=2");
    eval(&mut s, "use", "a+1");
    let old = cell(&s, "use").exec_count;
    let (o, reran, events) = eval(&mut s, "conflict", "a=9");
    assert!(
        o.messages
            .iter()
            .any(|m| m.tag == "err.multiple_definitions")
    );
    assert!(reran.is_empty());
    assert!(events.is_empty());
    assert_eq!(cell(&s, "use").status, CellStatus::Done);
    assert_eq!(cell(&s, "use").exec_count, old);
}
