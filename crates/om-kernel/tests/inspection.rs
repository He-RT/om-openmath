//! Output inspection uses the real evaluator in an isolated read-only fork.
pub mod support;
use om_kernel::{Session, protocol::*};
use std::sync::atomic::Ordering;
use support::*;

fn inspect(s: &mut Session, source: &str, numeric: bool) -> ExpressionView {
    let (r, events) = s.handle(Request::InspectExpression {
        source: source.into(),
        numeric,
    });
    assert!(events.is_empty());
    let Response::Expression { value } = r else {
        panic!("{r:?}")
    };
    value
}

#[test]
fn numeric_inspection_has_twenty_digits_without_notebook_history_or_flag_changes() {
    let mut s = sequential();
    output(&mut s, "definition", "a=7", Dialect::Wolfram);
    let before = serde_json::to_value(s.handle(Request::GetNotebookState).0).unwrap();
    s.interrupt_handle().store(true, Ordering::Relaxed);
    let v = inspect(&mut s, "Sqrt[2]", true);
    assert!(
        v.input_form.starts_with("1.4142135623730950488"),
        "{}",
        v.input_form
    );
    assert!(!v.latex.contains("N"));
    assert!(s.interrupt_handle().load(Ordering::Relaxed));
    assert_eq!(
        before,
        serde_json::to_value(s.handle(Request::GetNotebookState).0).unwrap()
    );
    let next = output(&mut s, "next", "a+1", Dialect::Wolfram);
    assert_eq!(expressions(&next)[0].0, 2);
    same(&expressions(&next)[0].1, "8");
    for src in ["a=99", "Clear[a]", "1;2", "", "N[Set[a,99]]"] {
        assert!(
            matches!(
                s.handle(Request::InspectExpression {
                    source: src.into(),
                    numeric: false
                })
                .0,
                Response::Error { .. }
            ),
            "{src}"
        );
    }
    output(&mut s, "f", "f[x_]:=(a=x)", Dialect::Wolfram);
    let _ = s.handle(Request::InspectExpression {
        source: "f[99]".into(),
        numeric: false,
    });
    same(
        &expressions(&output(&mut s, "check", "a", Dialect::Wolfram))[0].1,
        "7",
    );
}

#[test]
fn actual_solution_display_keeps_copy_source_and_reports_roots_and_precision() {
    let mut s = sequential();
    let o = output(&mut s, "irrational", "Solve[x^2==2,x]", Dialect::Wolfram);
    let OutputItem::Solutions { view, .. } = &o.items[0] else {
        panic!()
    };
    assert!(view.solutions.iter().all(|r| {
        r.bindings[0]
            .numeric
            .as_ref()
            .unwrap()
            .contains("4142135623730950488")
    }));
    let o = output(&mut s, "family", "Solve[Sin[x]==1/2,x]", Dialect::Wolfram);
    let OutputItem::Solutions {
        view, input_form, ..
    } = &o.items[0]
    else {
        panic!()
    };
    assert!(input_form.contains("C["));
    assert!(view.solutions.iter().all(|r| {
        let condition = r.condition_display_latex.as_ref().unwrap();
        condition.contains("\\in") && condition.contains("\\mathbb{Z}")
    }));
    assert!(
        view.solutions
            .iter()
            .any(|r| r.bindings[0].latex.contains('k'))
    );
    assert!(
        view.solutions
            .iter()
            .all(|r| !r.bindings[0].latex.contains("C\\"))
    );
    let o = output(
        &mut s,
        "root",
        "Solve[x^5-x+1==0,x,Reals]",
        Dialect::Wolfram,
    );
    let OutputItem::Solutions { view, .. } = &o.items[0] else {
        panic!()
    };
    let b = &view.solutions[0].bindings[0];
    assert_eq!(b.root_index, Some(1));
    assert!(b.numeric.is_some());
    assert!(b.radicals.is_none());
    assert_eq!(b.var_latex.as_deref(), Some("x"));
    let o = output(
        &mut s,
        "cubic",
        "Solve[x^3-x-1==0,x,Reals,Cubics->False]",
        Dialect::Wolfram,
    );
    let OutputItem::Solutions { view, .. } = &o.items[0] else {
        panic!()
    };
    let b = &view.solutions[0].bindings[0];
    assert_eq!(b.root_index, Some(1));
    assert!(!b.radicals.as_ref().unwrap().input_form.contains("Root"));
    let original = inspect(&mut s, &b.input_form, true);
    let converted = inspect(&mut s, &b.radicals.as_ref().unwrap().input_form, true);
    let a = om_parse::parse_expr(&original.input_form, om_parse::Dialect::Wolfram)
        .unwrap()
        .as_number()
        .unwrap()
        .to_f64()
        .unwrap();
    let b = om_parse::parse_expr(&converted.input_form, om_parse::Dialect::Wolfram)
        .unwrap()
        .as_number()
        .unwrap()
        .to_f64()
        .unwrap();
    assert!((a - b).abs() < 1e-14);
    let o = output(
        &mut s,
        "collision",
        "Solve[Sin[k]==1/2,k]",
        Dialect::Wolfram,
    );
    let OutputItem::Solutions { view, .. } = &o.items[0] else {
        panic!()
    };
    assert!(
        view.solutions
            .iter()
            .any(|r| r.condition_latex.as_ref().unwrap().contains('m'))
    );
}

#[test]
fn readonly_inspection_enforces_injected_deadline_limits_and_real_definitions() {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    struct Clock(AtomicU64);
    impl om_num::ctx::Clock for Clock {
        fn now_ms(&self) -> f64 {
            self.0.fetch_add(6000, Ordering::Relaxed) as f64
        }
    }
    let mut s = Session::new(Default::default(), Some(Arc::new(Clock(AtomicU64::new(0)))));
    assert!(matches!(
        s.handle(Request::InspectExpression {
            source: "Sqrt[2]".into(),
            numeric: true
        })
        .0,
        Response::Error { .. }
    ));
    let mut s = sequential();
    assert!(matches!(
        s.handle(Request::InspectExpression {
            source: "x".repeat(65_537),
            numeric: false
        })
        .0,
        Response::Error { .. }
    ));
    output(&mut s, "a", "a=2", Dialect::Wolfram);
    assert!(inspect(&mut s, "a+1", true).input_form.starts_with("3."));
}
