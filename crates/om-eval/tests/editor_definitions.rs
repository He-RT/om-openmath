//! Editor reads expose actual stored definitions and never execute queries.
use om_core::{Expr, Interrupt, Symbol};
use om_eval::Evaluator;
#[test]
fn read_only_own_and_downvalues_preserve_storage_order_and_effectful_rhs() {
    let mut ev = Evaluator::new();
    let ctx = Interrupt::default();
    for source in ["a:=counter=counter+1", "f[x_]:=x^2", "f[0]=7"] {
        ev.evaluate_statement(
            &om_parse::parse_expr(source, om_parse::Dialect::Wolfram).unwrap(),
            &ctx,
        )
        .unwrap();
    }
    let a = Symbol::intern("a");
    let f = Symbol::intern("f");
    assert!(ev.defs.own_value(a).unwrap().is_head(om_core::BUILTIN::SET));
    assert_eq!(ev.defs.down_values(f).len(), 2);
    assert!(ev.defs.down_values(f)[0].delayed);
    assert!(ev.defs.own_value(Symbol::intern("counter")).is_none());
    assert_eq!(ev.history.len(), 3);
    ev.defs.clear(f);
    assert!(ev.defs.down_values(f).is_empty());
    assert_eq!(ev.defs.own_value(Symbol::intern("missing")), None::<&Expr>);
}
