//! A write candidate owns cloned math state; it is neither the active owner nor a readonly fork.
use om_core::{Expr, Interrupt, Symbol};
use om_eval::Evaluator;
use om_parse::{Dialect, parse_expr};
fn source(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn run(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate_statement(&source(s), &Interrupt::default())
        .unwrap()
}
#[test]
fn staged_definitions_rules_and_out_are_isolated_until_consumed() {
    let mut owner = Evaluator::new();
    run(&mut owner, "a=2");
    run(&mut owner, "f[x_]:=x+a");
    let owner_history = owner.history.clone();
    let mut stage = owner.fork_working_stage().unwrap();
    run(stage.evaluator_mut(), "a=5");
    assert_eq!(run(stage.evaluator_mut(), "f[{1,2}]"), source("{6,7}"));
    assert_eq!(
        stage
            .evaluator_mut()
            .evaluate(&source("Out[-1]"), &Interrupt::default())
            .unwrap(),
        source("{6,7}")
    );
    assert_eq!(
        owner.defs.own_value(Symbol::intern("a")),
        Some(&Expr::int(2))
    );
    assert_eq!(owner.history, owner_history);
    let accepted = stage.into_evaluator();
    assert_eq!(
        accepted.defs.own_value(Symbol::intern("a")),
        Some(&Expr::int(5))
    );
    assert_eq!(accepted.history.last().unwrap().1, source("{6,7}"));
}
#[test]
fn discarded_random_stage_does_not_advance_owner_and_consumed_state_keeps_the_stream() {
    let mut owner = Evaluator::new();
    run(&mut owner, "SeedRandom[71]");
    let mut first = owner.fork_working_stage().unwrap();
    let mut second = owner.fork_working_stage().unwrap();
    let a = run(first.evaluator_mut(), "RandomUniform[]");
    let b = run(second.evaluator_mut(), "RandomUniform[]");
    assert_eq!(a, b);
    drop(second);
    assert_eq!(run(&mut owner, "RandomUniform[]"), a);
    let next = run(first.evaluator_mut(), "RandomUniform[]");
    let mut accepted = first.into_evaluator();
    assert_eq!(run(&mut owner, "RandomUniform[]"), next);
    assert_eq!(
        run(&mut accepted, "RandomUniform[]"),
        run(&mut owner, "RandomUniform[]")
    );
}
#[test]
fn readonly_projection_cannot_be_promoted_to_a_writable_owner() {
    let owner = Evaluator::new();
    let readonly = owner.fork_readonly();
    assert!(readonly.fork_working_stage().is_err());
}

#[test]
fn exact_numeric_atoms_and_settings_are_cloned_without_machine_projection() {
    let mut owner = Evaluator::new();
    let value = run(
        &mut owner,
        "exact=123456789012345678901234567890123456789/100000000000000000000000000000000000003",
    );
    owner.settings.recursion_limit = 512;
    let mut stage = owner.fork_working_stage().unwrap();
    assert_eq!(run(stage.evaluator_mut(), "exact"), value);
    stage.evaluator_mut().settings.recursion_limit = 128;
    assert_eq!(owner.settings.recursion_limit, 512);
    assert_eq!(stage.evaluator_mut().settings.recursion_limit, 128);
    assert_eq!(owner.defs.own_value(Symbol::intern("exact")), Some(&value));
}
