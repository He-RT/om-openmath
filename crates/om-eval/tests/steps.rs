//! Real step type is isolated between statements and readonly forks.
use om_core::Expr;
use om_eval::Evaluator;
use om_num::ctx::{Abort, Interrupt};
use om_solve::{Level, Step, StepKind, Steps};
fn steps() -> Steps {
    Steps {
        root: vec![Step::new(StepKind::Normalize, vec![], vec![], Level::Major)],
    }
}
#[test]
fn steps_start_empty_and_do_not_leak_between_statements_or_forks() {
    let mut eval = Evaluator::new();
    assert!(eval.last_steps.is_none());
    eval.last_steps = Some(steps());
    assert!(eval.fork_readonly().last_steps.is_none());
    assert!(eval.last_steps.is_some());
    eval.evaluate_statement(&Expr::int(1), &Interrupt::default())
        .unwrap();
    assert!(eval.last_steps.is_none());
    eval.last_steps = Some(steps());
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        eval.evaluate_statement(&Expr::int(1), &ctx),
        Err(om_eval::EvalError::Abort(Abort::Budget))
    ));
    assert!(eval.last_steps.is_none());
}
