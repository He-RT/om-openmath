//! Actual callback evidence follows the returned statement, including NoSteps.
use om_core::{Expr, Interrupt};
use om_eval::Evaluator;
use om_solve::{SolutionSet, Verification};
fn run(e: &mut Evaluator, s: &str) -> Expr {
    e.evaluate_statement(
        &om_parse::parse_expr(s, om_parse::Dialect::Wolfram).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn actual_outcome_retains_evidence_with_steps_disabled_and_takes_once() {
    let mut e = Evaluator::new();
    e.settings.record_steps = false;
    let value = run(&mut e, "Solve[x^2==9,x]");
    let result = e.take_solver_result().unwrap();
    assert_eq!(result.name.name(), "Solve");
    assert_eq!(result.value, value);
    assert_eq!(result.vars, [Expr::symbol("x")]);
    let SolutionSet::Finite(solutions) = result.set else {
        panic!()
    };
    assert_eq!(solutions.len(), 2);
    assert!(
        solutions
            .iter()
            .all(|s| s.verification == Verification::Exact)
    );
    assert!(e.last_steps.is_none());
    assert!(e.take_solver_result().is_none());
    assert_eq!(e.history.len(), 1);
}
#[test]
fn tail_calls_and_final_compound_children_keep_actual_provenance() {
    let mut e = Evaluator::new();
    for setup in ["s=Solve", "f[t_]:=Solve[x^2==t,x]", "g:=Solve[x^2==9,x]"] {
        run(&mut e, setup);
        e.take_solver_result();
    }
    for s in [
        "s[x^2==9,x]",
        "f[9]",
        "g",
        "0;Solve[x^2==9,x]",
        "Function[t,Solve[x^2==t,x]][9]",
    ] {
        let value = run(&mut e, s);
        let result = e.take_solver_result().unwrap();
        assert_eq!(result.value, value, "{s}");
    }
}
#[test]
fn matching_literals_nested_calls_declines_and_previous_results_do_not_forge_evidence() {
    let mut e = Evaluator::new();
    for s in [
        "{{x->-3},{x->3}}",
        "Solve[x^2==9,x];{{x->-3},{x->3}}",
        "{Solve[x^2==9,x]}",
        "First[Solve[x^2==9,x]]",
        "Solve[Sin[x]+x==0,x]",
        "Solve[x^2==9,x];Null",
    ] {
        run(&mut e, s);
        assert!(e.take_solver_result().is_none(), "{s}");
    }
    run(&mut e, "Solve[x^2==9,x]");
    run(&mut e, "{{x->-3},{x->3}}");
    assert!(e.take_solver_result().is_none());
}

#[test]
fn immediate_assignment_forwards_actual_solver_evidence_but_cached_reads_do_not() {
    let mut e = Evaluator::new();
    let value = run(&mut e, "saved=Solve[x^2==9,x]");
    assert_eq!(e.take_solver_result().unwrap().value, value);
    run(&mut e, "saved");
    assert!(e.take_solver_result().is_none());
}
