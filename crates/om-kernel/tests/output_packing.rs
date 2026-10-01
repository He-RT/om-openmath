//! Actual solver cards, evidence, numeric display and derivations over the protocol.
pub mod support;
use om_kernel::{Session, protocol::*};
use support::{expressions, output, same, sequential};
fn solutions(o: &CellOutput) -> (&SolutionSetView, Option<&StepsView>) {
    match &o.items[0] {
        OutputItem::Solutions { view, steps, .. } => (view, steps.as_ref()),
        other => panic!("expected Solutions: {other:?}"),
    }
}
#[test]
fn real_exact_solution_cards_preserve_forms_indices_evidence_and_actual_step_tree() {
    let mut s = sequential();
    let o = output(&mut s, "solve", "Solve[x^2==9,x]", Dialect::Wolfram);
    let (v, steps) = solutions(&o);
    assert_eq!(v.kind, SolutionKind::Finite);
    assert_eq!(v.vars, ["x"]);
    assert_eq!(v.solutions.len(), 2);
    for (sol, n) in v.solutions.iter().zip([-3.0, 3.0]) {
        assert_eq!(sol.verified, Verification::Exact);
        assert_eq!(sol.bindings[0].var, "x");
        assert!(
            (om_parse::parse_expr(
                sol.bindings[0].numeric.as_ref().unwrap(),
                om_parse::Dialect::Wolfram
            )
            .unwrap()
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap()
                - n)
                .abs()
                < 1e-9
        );
    }
    same(&expressions(&o)[0].1, "{{x->-3},{x->3}}");
    assert_eq!(expressions(&o)[0].0, 1);
    let steps = steps.unwrap();
    assert!(!steps.root.is_empty());
    fn check(raw: &om_solve::Step, view: &StepView) {
        assert_eq!(view.id, raw.id);
        assert_eq!(view.rule_id, raw.rule_id);
        assert_eq!(view.title_key, format!("step.{}", raw.rule_id));
        assert_eq!(
            view.before_latex,
            raw.before.iter().map(om_format::latex).collect::<Vec<_>>()
        );
        assert_eq!(
            view.after_latex,
            raw.after.iter().map(om_format::latex).collect::<Vec<_>>()
        );
        assert_eq!(view.children.len(), raw.children.len());
        for (a, b) in raw.children.iter().zip(&view.children) {
            check(a, b);
        }
    }
    let raw = s.notebook.cells[0].steps(1).unwrap();
    assert_eq!(raw.root.len(), steps.root.len());
    for (a, b) in raw.root.iter().zip(&steps.root) {
        check(a, b);
    }
    assert_eq!(s.notebook.cells[0].exec_count, Some(1));
    let wire = serde_json::to_value(&o).unwrap();
    assert_eq!(wire["items"][0]["type"], "solutions");
    assert!(!wire.to_string().contains("ExprNode"));
}
#[test]
fn no_steps_still_has_real_cards_and_suppressed_solver_results_keep_history() {
    let mut s = sequential();
    s.config.general.show_steps = false;
    let o = output(&mut s, "no", "Solve[x^2==4,x]", Dialect::Wolfram);
    let (v, steps) = solutions(&o);
    assert!(steps.is_none());
    assert!(
        v.solutions
            .iter()
            .all(|x| x.verified == Verification::Exact)
    );
    s.config.general.show_steps = true;
    let o = output(
        &mut s,
        "hidden",
        "solve(x^2=9,x);\n1/0;\n5",
        Dialect::Modern,
    );
    assert_eq!(expressions(&o).last().unwrap().1, "5");
    assert!(o.messages.iter().any(|m| m.tag == "infy"));
    assert!(s.notebook.cells[1].steps(2).is_some());
    assert_eq!(s.notebook.cells[1].exec_count, Some(4));
}
#[test]
fn empty_all_conditional_and_free_axis_cards_follow_real_solution_shapes() {
    let mut s = sequential();
    for (src, kind, count) in [
        ("Solve[x^2==-1,x,Reals]", SolutionKind::None, 0),
        ("Solve[x==x,x]", SolutionKind::All, 0),
        ("Solve[x/x==1,x]", SolutionKind::Finite, 1),
    ] {
        let o = output(&mut s, "shape", src, Dialect::Wolfram);
        let (v, _) = solutions(&o);
        assert_eq!(v.kind, kind, "{src}");
        assert_eq!(v.solutions.len(), count);
        if count == 1 {
            assert!(v.solutions[0].bindings.is_empty());
            assert!(
                v.solutions[0]
                    .condition_latex
                    .as_ref()
                    .unwrap()
                    .contains('x')
            );
        }
    }
    let o = output(&mut s, "free", "Solve[x+y==1,{x,y}]", Dialect::Wolfram);
    let (v, _) = solutions(&o);
    assert_eq!(v.vars, ["x", "y"]);
    assert_eq!(v.solutions[0].bindings.len(), 1);
    assert_eq!(v.solutions[0].bindings[0].var, "x");
    assert!(v.solutions[0].bindings[0].numeric.is_none());
}
#[test]
fn numerical_and_value_only_families_keep_actual_bindings_and_verification() {
    let mut s = sequential();
    for src in [
        "NSolve[x^2==2,x]",
        "FindRoot[x^2==2,{x,1}]",
        "SolveValues[x^2==2,x]",
        "NSolveValues[x^2==2,x]",
    ] {
        let o = output(&mut s, "family", src, Dialect::Wolfram);
        let (v, _) = solutions(&o);
        assert_eq!(v.kind, SolutionKind::Finite);
        assert_eq!(v.vars, ["x"]);
        assert!(v.solutions.iter().all(|x| x.bindings.len() == 1));
        if src.starts_with("SolveValues") {
            assert!(v.solutions.iter().all(|x| x.bindings[0].numeric.is_some()));
        } else {
            assert!(v.solutions.iter().all(|x| x.bindings[0].numeric.is_none()));
            assert!(
                v.solutions
                    .iter()
                    .all(|x| matches!(x.verified, Verification::Numeric { .. }))
            );
        }
    }
}
#[test]
fn plain_lists_wrappers_cached_values_and_declined_calls_stay_expressions() {
    let mut s = sequential();
    for src in [
        "{}",
        "{{}}",
        "{{x->-3},{x->3}}",
        "{x->2}",
        "Solve[x^2==9,x];{{x->-3},{x->3}}",
        "First[Solve[x^2==9,x]]",
        "Solve[Sin[x]+x==0,x]",
        "x==2",
    ] {
        let o = output(&mut s, "plain", src, Dialect::Wolfram);
        assert!(
            o.items.iter().all(|i| matches!(i, OutputItem::Expr { .. })),
            "{src}: {o:?}"
        );
    }
    output(&mut s, "cached", "s=Solve[x^2==9,x];", Dialect::Wolfram);
    let o = output(&mut s, "plain", "s", Dialect::Wolfram);
    assert!(matches!(o.items[0], OutputItem::Expr { .. }));
}
#[test]
fn complex_and_root_numeric_displays_come_from_real_n_and_do_not_consume_history() {
    let mut s = sequential();
    for src in ["Solve[x^2==-1,x]", "Solve[x^3-x-1==0,x]"] {
        let o = output(&mut s, "numeric", src, Dialect::Wolfram);
        let (v, _) = solutions(&o);
        assert!(v.solutions.iter().all(|x| x.bindings[0].numeric.is_some()));
        for sol in &v.solutions {
            let n = sol.bindings[0].numeric.as_ref().unwrap();
            assert!(
                om_parse::parse_expr(n, om_parse::Dialect::Wolfram).is_ok(),
                "{n}"
            );
        }
    }
    let o = output(&mut s, "history", "Out[2]", Dialect::Wolfram);
    assert_eq!(expressions(&o)[0].0, 3);
    assert!(matches!(o.items[0], OutputItem::Expr { .. }));
}
#[test]
fn regions_use_actual_interval_endpoints_with_no_infinite_json_values() {
    let mut s = sequential();
    let o = output(&mut s, "region", "Reduce[x^2<4,x]", Dialect::Wolfram);
    let (v, _) = solutions(&o);
    assert_eq!(v.kind, SolutionKind::Region);
    assert_eq!(v.intervals.len(), 1);
    let i = &v.intervals[0];
    assert_eq!(i.lo_value, Some(-2.0));
    assert_eq!(i.hi_value, Some(2.0));
    assert!(!i.lo_closed && !i.hi_closed);
    assert!(v.region_latex.is_some());
    let o = output(&mut s, "region", "Reduce[x>1,x]", Dialect::Wolfram);
    let (v, _) = solutions(&o);
    assert!(v.intervals[0].hi.is_none());
    assert!(v.intervals[0].hi_value.is_none());
    assert!(serde_json::to_string(&o).is_ok());
}
#[test]
fn reactive_outputs_pack_actual_rerun_results_and_preserve_raw_poles() {
    let mut s = Session::new(Default::default(), None);
    for (id, source) in [("a", "let a=2"), ("b", "solve(x^2=a,x)")] {
        s.handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: Dialect::Modern,
        });
    }
    let (r, events) = s.handle(Request::Evaluate {
        cell_id: "a".into(),
        source: "let a=9".into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { reran, .. } = r else {
        panic!()
    };
    assert_eq!(reran, ["b"]);
    let o = events
        .iter()
        .find_map(|e| {
            if let Event::CellOutput { output, .. } = e {
                Some(output)
            } else {
                None
            }
        })
        .unwrap();
    let (v, steps) = solutions(o);
    assert_eq!(v.solutions.len(), 2);
    assert!(steps.is_some());
    same(&expressions(o)[0].1, "{{x->-3},{x->3}}");
    let o = output(
        &mut sequential(),
        "poles",
        "Solve[(x^2-1)/(x-1)==0,x]",
        Dialect::Wolfram,
    );
    let (v, _) = solutions(&o);
    assert_eq!(v.solutions.len(), 1);
    same(&v.solutions[0].bindings[0].input_form, "-1");
}

#[test]
fn multiplicities_and_generated_parameter_conditions_are_not_lost_in_cards() {
    let mut s = sequential();
    let o = output(&mut s, "repeated", "Solve[(x-1)^2==0,x]", Dialect::Wolfram);
    let (v, _) = solutions(&o);
    let literal = om_parse::parse_expr(&expressions(&o)[0].1, om_parse::Dialect::Wolfram).unwrap();
    assert_eq!(v.solutions.len(), literal.args().len());
    assert!(v.solutions.iter().all(|r| r.bindings[0].input_form == "1"));
    let o = output(&mut s, "family", "Solve[Sin[x]==0,x]", Dialect::Wolfram);
    let (v, _) = solutions(&o);
    assert!(v.solutions.iter().all(|r| {
        r.condition_latex.as_ref().is_some_and(|c| {
            c.contains(&om_format::latex(&om_core::Expr::sym(
                om_core::BUILTIN::INTEGERS,
            )))
        })
    }));
    assert!(v.solutions.iter().all(|r| r.bindings[0].numeric.is_none()));
}
