//! Real cube roots and principal nth roots stay mathematically distinct; old Root/ root syntax remains intact.
use om_core::{Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn real_cubes_and_principal_nth_roots_preserve_their_branch_contracts() {
    let mut ev = Evaluator::new();
    for (source, expected) in [
        ("cbrt(-8)", "-2"),
        ("cbrt(27/8)", "3/2"),
        ("nth_root(81,4)", "3"),
        ("nth_root(-4,2)", "2*i"),
        ("root(-4,2)", "2*i"),
        ("nth_root(x,3)", "x^(1/3)"),
    ] {
        assert_eq!(eval(&mut ev, source), eval(&mut ev, expected), "{source}");
    }
    assert_ne!(eval(&mut ev, "nth_root(-8,3)"), Expr::int(-2));
    assert_eq!(
        eval(&mut ev, "cbrt(-8.0)").as_number().unwrap().to_f64(),
        Some(-2.)
    );
    assert_eq!(eval(&mut ev, "cuberoot(-8)"), Expr::int(-2));
    for source in ["nth_root(1,0)", "nth_root(1,-2)", "cbrt(i)"] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty());
    }
}
#[test]
fn root_precision_listability_and_interrupts_use_actual_existing_math() {
    let mut ev = Evaluator::new();
    assert_eq!(eval(&mut ev, "cbrt([-8,0,27])"), eval(&mut ev, "[-2,0,3]"));
    let cube = eval(&mut ev, "cbrt(decimal(\"-2\",precision:50))");
    let n = cube.as_number().unwrap();
    assert!(matches!(n.precision(),om_num::Precision::Bits(bits) if bits>=166));
    assert!((n.to_f64().unwrap() + 1.2599210498948732).abs() < 1e-15);
    let e = om_parse::parse_expr("nth_root(2,3)", om_parse::Dialect::Modern).unwrap();
    let ctx = Interrupt::default();
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(ev.evaluate(&e, &ctx).is_err());
}
#[test]
fn diag_extracts_real_rectangular_matrix_diagonals_and_still_constructs_vectors() {
    let mut ev = Evaluator::new();
    assert_eq!(
        eval(&mut ev, "diag([[1,2,3],[4,5,6]])"),
        eval(&mut ev, "[1,5]")
    );
    assert_eq!(eval(&mut ev, "diag([2,3])"), eval(&mut ev, "[[2,0],[0,3]]"));
    ev.messages.take();
    eval(&mut ev, "diag([[1,2],[3]])");
    assert!(!ev.messages.take().is_empty());
}
#[test]
fn nth_root_held_resolution_preserves_raw_poles_and_lowers_for_real_sampling() {
    let mut ev = Evaluator::new();
    assert_eq!(
        eval(&mut ev, "solve(nth_root((x^2-1)/(x-1),2)=sqrt(2),x)"),
        eval(&mut ev, "[]")
    );
    let ctx = Interrupt::default();
    let x = om_core::Symbol::intern("x");
    let expression = om_parse::parse_expr("nth_root(x^2,2)", om_parse::Dialect::Modern).unwrap();
    let prepared = ev.prepare_numeric(&expression, &[(x, None)], &ctx).unwrap();
    let program = om_eval::numeric::compile_f64(&prepared, &[x]).unwrap();
    assert!((program.eval(&[-3.]) - 3.).abs() < 1e-14);
}
