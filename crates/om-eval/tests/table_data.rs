//! Real table rows, headers and functional side effects share the original data-operation contracts.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn setup() -> Evaluator {
    let mut ev = Evaluator::new();
    eval(
        &mut ev,
        "let t=DataTable([\"a\",\"b\"],[{a:3,b:\"z\"},{b:\"y\",a:1},{a:2,b:\"x\"}])",
    );
    ev
}
#[test]
fn table_subset_sort_and_structural_operations_preserve_headers() {
    let mut ev = setup();
    assert_eq!(eval(&mut ev, "t.a"), eval(&mut ev, "[3,1,2]"));
    assert_eq!(eval(&mut ev, "t.columns"), eval(&mut ev, "[\"a\",\"b\"]"));
    assert_eq!(eval(&mut ev, "length(t.rows)"), Expr::int(3));
    assert_eq!(eval(&mut ev, "slice([1,2,3],3..2)"), eval(&mut ev, "[3,2]"));
    for (source, expected) in [
        ("length(t)", "3"),
        ("first(t).a", "3"),
        ("last(t).a", "2"),
        ("map(fn(r)=>r.a,t)", "[3,1,2]"),
        ("length(rest(t))", "2"),
        ("length(filter(t,fn(r)=>r.a>9))", "0"),
        ("map(fn(r)=>r.a,sort(t))", "[1,2,3]"),
        (
            "map(fn(r)=>r.b,sort_by(t,fn(r)=>r.b))",
            "[\"x\",\"y\",\"z\"]",
        ),
        ("map(fn(r)=>r.a,take(t,-2))", "[1,2]"),
        ("map(fn(r)=>r.a,drop(t,1))", "[1,2]"),
        ("map(fn(r)=>r.a,slice(t,3..2))", "[2,1]"),
    ] {
        assert_eq!(eval(&mut ev, source), eval(&mut ev, expected), "{source}");
    }
    assert_eq!(
        eval(&mut ev, "to_csv(filter(t,fn(r)=>r.a>9))"),
        Expr::string("a,b\r\n")
    );
    assert_eq!(
        eval(&mut ev, "length(append(t,{a:4,b:\"q\"}))"),
        Expr::int(4)
    );
}
#[test]
fn fold_group_counts_zip_and_record_sort_use_actual_rows() {
    let mut ev = setup();
    assert_eq!(
        eval(&mut ev, "fold(fn(total,r)=>total+r.a,0,t)"),
        Expr::int(6)
    );
    let groups = eval(&mut ev, "group_by(t,fn(r)=>mod(r.a,2))");
    assert_eq!(groups.args().len(), 2);
    let subgroup = &groups.args()[0].args()[1].args()[1];
    assert!(subgroup.is_head(B::DATA_TABLE));
    assert_eq!(subgroup.args()[1].args().len(), 2);
    assert_eq!(eval(&mut ev, "length(zip(t,t))"), Expr::int(3));
    assert_eq!(
        eval(&mut ev, "map(fn(r)=>r.a,sort([{a:3},{a:1},{a:2}]))"),
        eval(&mut ev, "[1,2,3]")
    );
    eval(
        &mut ev,
        "let d=DataTable([\"a\",\"b\"],[{a:1,b:2},{b:2,a:1}])",
    );
    assert_eq!(eval(&mut ev, "length(unique(d))"), Expr::int(1));
    let counts = eval(&mut ev, "counts(d)");
    assert_eq!(counts.args().len(), 1);
    assert_eq!(counts.args()[0].args()[1].args()[1], Expr::int(2));
}
#[test]
fn table_keys_evaluate_once_and_bad_shapes_do_not_claim_success() {
    let mut ev = setup();
    eval(&mut ev, "let n=0");
    eval(&mut ev, "sort_by(t,fn(r)=>sequence(assign(n,n+1),r.a))");
    assert_eq!(eval(&mut ev, "n"), Expr::int(3));
    for source in [
        "append(t,{a:4})",
        "filter(t,fn(r)=>r.a)",
        "sort([{a:1},{b:2}])",
        "take(DataTable([\"a\"],[{b:1}]),1)",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    let ctx = Interrupt::default();
    ctx.steps_left.set(20);
    let e = om_parse::parse_expr("sort_by(t,fn(r)=>r.a)", om_parse::Dialect::Modern).unwrap();
    assert!(ev.evaluate(&e, &ctx).is_err());
}
