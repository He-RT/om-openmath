//! State, resource limits and evaluation contracts without kernel fixtures.
use om_core::{Abort, BUILTIN as B, Expr, Interrupt};
use om_eval::{EvalError, Evaluator};
use om_parse::{Dialect, parse_expr};
fn src(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn run(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(&src(s), &Interrupt::default()).unwrap()
}
#[test]
fn atoms_heads_and_recursive_arguments_evaluate_without_history_side_effects() {
    let mut ev = Evaluator::new();
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
    assert_eq!(run(&mut ev, "x"), Expr::symbol("x"));
    assert_eq!(run(&mut ev, "\"hello\""), Expr::string("hello"));
    assert_eq!(run(&mut ev, "1/3+1/6"), Expr::rational(1, 2));
    assert_eq!(run(&mut ev, "x=3"), Expr::int(3));
    assert_eq!(
        run(&mut ev, "f[x+1]"),
        Expr::call(om_core::Symbol::intern("f"), [Expr::int(4)])
    );
    assert_eq!(run(&mut ev, "h=f"), Expr::symbol("f"));
    assert_eq!(
        run(&mut ev, "h[2+3]"),
        Expr::call(om_core::Symbol::intern("f"), [Expr::int(5)])
    );
    assert!(ev.history.is_empty());
}
#[test]
fn immediate_delayed_unset_and_clear_respect_assignment_targets() {
    let mut ev = Evaluator::new();
    assert_eq!(run(&mut ev, "a=1"), Expr::int(1));
    run(&mut ev, "immediate=a+1");
    assert_eq!(run(&mut ev, "delayed:=a+1"), Expr::sym(B::NULL));
    run(&mut ev, "a=5");
    assert_eq!(run(&mut ev, "immediate"), Expr::int(2));
    assert_eq!(run(&mut ev, "delayed"), Expr::int(6));
    assert_eq!(run(&mut ev, "a=7"), Expr::int(7));
    assert_eq!(run(&mut ev, "a"), Expr::int(7));
    assert_eq!(run(&mut ev, "a=."), Expr::sym(B::NULL));
    assert_eq!(run(&mut ev, "a"), Expr::symbol("a"));
    run(&mut ev, "f[1]=2");
    assert_eq!(run(&mut ev, "f[1]"), Expr::int(2));
    run(&mut ev, "f[1]=3");
    assert_eq!(run(&mut ev, "f[1]"), Expr::int(3));
    assert_eq!(run(&mut ev, "f[2]"), src("f[2]"));
    assert_eq!(
        run(&mut ev, "Clear[f,immediate,delayed]"),
        Expr::sym(B::NULL)
    );
    assert_eq!(run(&mut ev, "f[1]"), src("f[1]"));
    assert_eq!(run(&mut ev, "immediate"), Expr::symbol("immediate"));
    assert_eq!(run(&mut ev, "delayed"), Expr::symbol("delayed"));
}

#[test]
fn recursion_limits_and_abort_restore_the_evaluators_depth() {
    let mut ev = Evaluator::new();
    ev.settings.recursion_limit = 8;
    run(&mut ev, "x:=x");
    assert!(matches!(
        ev.evaluate(&src("x"), &Interrupt::default()),
        Err(EvalError::Recursion(8))
    ));
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
    let expr = (0..12).fold(Expr::int(1), |e, _| {
        Expr::call(om_core::Symbol::intern("f"), [e])
    });
    assert!(matches!(
        ev.evaluate(&expr, &Interrupt::default()),
        Err(EvalError::Recursion(8))
    ));
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        ev.evaluate(&Expr::int(1), &ctx),
        Err(EvalError::Abort(Abort::Budget))
    ));
    assert_eq!(run(&mut ev, "3+3"), Expr::int(6));
    let mut ev = Evaluator::new();
    run(&mut ev, "x:=x");
    assert!(matches!(
        ev.evaluate(&src("x"), &Interrupt::default()),
        Err(EvalError::Recursion(1024))
    ));
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}
#[test]
fn literal_downvalues_stop_at_the_iteration_limit_with_a_held_result() {
    let mut ev = Evaluator::new();
    ev.settings.iteration_limit = 3;
    run(&mut ev, "f[1]:=f[2]");
    run(&mut ev, "f[2]:=f[1]");
    let result = run(&mut ev, "f[1]");
    assert!(result.is_head(B::HOLD));
    let messages = ev.messages.take();
    assert!(
        messages
            .iter()
            .any(|m| m.symbol == "$IterationLimit" && m.tag == "itlim")
    );
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}
#[test]
fn statements_record_successful_input_output_and_out_reads_previous_results() {
    let mut ev = Evaluator::new();
    let ctx = Interrupt::default();
    for (s, n) in [("2+2", 4), ("3+3", 6)] {
        let input = src(s);
        assert_eq!(ev.evaluate_statement(&input, &ctx).unwrap(), Expr::int(n));
        assert_eq!(ev.history.last().unwrap(), &(input, Expr::int(n)));
    }
    assert_eq!(run(&mut ev, "%"), Expr::int(6));
    assert_eq!(run(&mut ev, "%1"), Expr::int(4));
    assert_eq!(run(&mut ev, "Out[-2]"), Expr::int(4));
    assert_eq!(run(&mut ev, "Out[0]"), src("Out[0]"));
    assert_eq!(ev.history.len(), 2);
    ctx.steps_left.set(0);
    assert!(ev.evaluate_statement(&src("1"), &ctx).is_err());
    assert_eq!(ev.history.len(), 2);
    assert_eq!(run(&mut ev, "a=1;a=2;a"), Expr::int(2));
}
#[test]
fn readonly_forks_are_snapshots_and_block_all_definition_mutations() {
    let mut ev = Evaluator::new();
    run(&mut ev, "x=2");
    let mut fork = ev.fork_readonly();
    run(&mut ev, "x=3");
    assert_eq!(run(&mut fork, "x+1"), Expr::int(3));
    for s in ["x=9", "x:=9", "x=.", "Clear[x]"] {
        assert!(
            fork.evaluate(&src(s), &Interrupt::default()).is_err(),
            "{s}"
        );
    }
    assert_eq!(run(&mut fork, "x"), Expr::int(2));
    assert_eq!(run(&mut ev, "x"), Expr::int(3));
}
#[test]
fn arity_protection_docs_and_constructor_messages_are_real() {
    let mut ev = Evaluator::new();
    assert_eq!(run(&mut ev, "Set[x]"), src("Set[x]"));
    assert_eq!(run(&mut ev, "Pi=3"), src("Pi=3"));
    assert_eq!(run(&mut ev, "Pi"), Expr::sym(B::PI));
    let msgs = ev.messages.take();
    assert!(msgs.iter().any(|m| m.symbol == "Set" && m.tag == "argx"));
    assert!(msgs.iter().any(|m| m.symbol == "Set" && m.tag == "wrsym"));
    assert_eq!(run(&mut ev, "1/0"), Expr::call(B::DIRECTED_INFINITY, []));
    let msgs = ev.messages.take();
    assert_eq!(
        msgs.iter()
            .filter(|m| m.symbol == "Power" && m.tag == "infy")
            .count(),
        1
    );
    let doc = Evaluator::doc(B::SET).unwrap();
    assert_eq!(doc.name, "Set");
    assert!(!doc.summary_zh.is_empty());
    let names: Vec<_> = Evaluator::all_docs().map(|d| d.name).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    assert!(names.contains(&"Set") && names.contains(&"Clear"));
}

#[test]
fn default_recursion_limit_bounds_deep_syntax_and_nested_ownvalues() {
    let mut ev = Evaluator::new();
    for head in [om_core::Symbol::intern("f"), B::COMPOUND_EXPRESSION] {
        let deep = (0..1100).fold(Expr::int(1), |e, _| Expr::call(head, [e]));
        assert!(matches!(
            ev.evaluate(&deep, &Interrupt::default()),
            Err(EvalError::Recursion(1024))
        ));
    }
    run(&mut ev, "x:=f[x]");
    assert!(matches!(
        ev.evaluate(&src("x"), &Interrupt::default()),
        Err(EvalError::Recursion(1024))
    ));
    assert_eq!(run(&mut ev, "2+2"), Expr::int(4));
}
