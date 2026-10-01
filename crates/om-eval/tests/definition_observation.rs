//! Definition observations describe real global mutations without executing queries.
use om_core::{Expr, Interrupt, Symbol};
use om_eval::Evaluator;
use std::collections::BTreeSet;
fn names(symbols: BTreeSet<Symbol>) -> Vec<&'static str> {
    let mut n = symbols.into_iter().map(Symbol::name).collect::<Vec<_>>();
    n.sort();
    n
}
fn run(e: &mut Evaluator, source: &str) {
    e.evaluate_statement(
        &om_parse::parse_expr(source, om_parse::Dialect::Wolfram).unwrap(),
        &Interrupt::default(),
    )
    .unwrap();
}
#[test]
fn observation_handles_same_value_writes_downvalues_unset_and_direct_clear_without_history() {
    let mut e = Evaluator::new();
    assert!(e.defs.defined_symbols().is_empty());
    assert!(e.defs.take_changed_symbols().is_empty());
    run(&mut e, "a=2");
    run(&mut e, "a=2");
    run(&mut e, "f[x_]:=x+a");
    assert_eq!(names(e.defs.defined_symbols()), ["a", "f"]);
    assert_eq!(names(e.defs.take_changed_symbols()), ["a", "f"]);
    assert!(e.defs.take_changed_symbols().is_empty());
    run(&mut e, "Unset[f[x_]]");
    assert_eq!(names(e.defs.defined_symbols()), ["a"]);
    assert_eq!(names(e.defs.take_changed_symbols()), ["f"]);
    let history = e.history.clone();
    e.defs.clear(Symbol::intern("a"));
    assert!(e.defs.defined_symbols().is_empty());
    assert_eq!(names(e.defs.take_changed_symbols()), ["a"]);
    assert_eq!(e.history, history);
    assert!(e.messages.take().is_empty());
}
#[test]
fn iterator_scopes_do_not_report_global_parameter_writes() {
    let mut e = Evaluator::new();
    run(&mut e, "x=9");
    e.defs.take_changed_symbols();
    run(&mut e, "Table[x=2,{x,1,3}]");
    assert!(e.defs.take_changed_symbols().is_empty());
    assert_eq!(
        e.evaluate(&Expr::sym(Symbol::intern("x")), &Interrupt::default())
            .unwrap(),
        Expr::int(9)
    );
}

#[test]
fn compound_statement_reports_all_real_rhs_side_effects() {
    let mut e = Evaluator::new();
    run(&mut e, "x=9");
    e.defs.take_changed_symbols();
    run(&mut e, "x=2;f[y_]:=y");
    assert_eq!(names(e.defs.take_changed_symbols()), ["f", "x"]);
    assert_eq!(
        e.evaluate(&Expr::sym(Symbol::intern("x")), &Interrupt::default())
            .unwrap(),
        Expr::int(2)
    );
}
