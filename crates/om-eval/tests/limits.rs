//! Limits use exact rational orders and analytic identities, never nearby numeric samples.
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
fn rational_finite_one_sided_and_infinite_limits_match_exact_orders() {
    let mut ev = Evaluator::new();
    for (source, want) in [
        ("limit((x^2-1)/(x-1),x,at:1)", "2"),
        ("limit(1/x,x,at:0,direction:\"right\")", "inf"),
        ("limit(1/x,x,at:0,direction:\"left\")", "-inf"),
        ("limit(1/x^2,x,at:0)", "inf"),
        ("limit((3*x^2+1)/(2*x^2-1),x,at:inf)", "3/2"),
        ("limit(1/x,x,at:-inf)", "0"),
    ] {
        assert_eq!(eval(&mut ev, source), eval(&mut ev, want), "{source}");
    }
}
#[test]
fn elementary_zero_over_zero_and_essential_sides_use_real_math() {
    let mut ev = Evaluator::new();
    for (source, want) in [
        ("limit(sin(x)/x,x,at:0)", "1"),
        ("limit((exp(x)-1)/x,x,at:0)", "1"),
        ("limit((1-cos(x))/x^2,x,at:0)", "1/2"),
        ("limit(log(x),x,at:0,direction:\"right\")", "-inf"),
        ("limit(exp(-x),x,at:inf)", "0"),
        ("limit(exp(-1/x^2),x,at:0)", "0"),
        ("limit(exp(1/x),x,at:0,direction:\"left\")", "0"),
    ] {
        assert_eq!(eval(&mut ev, source), eval(&mut ev, want), "{source}");
    }
    for (source, want) in [
        ("limit(sin(x)/x,x,at:inf)", "0"),
        ("limit(x*sin(1/x),x,at:0)", "0"),
        ("limit(sqrt(x),x,at:inf)", "inf"),
        ("limit(sqrt(x),x,at:0,direction:\"right\")", "0"),
        ("limit(log(sin(x)/x),x,at:0)", "0"),
        ("limit((1+1/x)^x,x,at:inf)", "e"),
        ("limit((1+x)^(1/x),x,at:0)", "e"),
    ] {
        assert_eq!(eval(&mut ev, source), eval(&mut ev, want), "{source}");
    }
}
#[test]
fn nonexistent_oscillatory_unknown_and_mutating_limits_remain_diagnosed() {
    let mut ev = Evaluator::new();
    for source in [
        "limit(1/x,x,at:0)",
        "limit(sin(1/x),x,at:0)",
        "limit(sin(x),x,at:inf)",
        "limit(f(x),x,at:0)",
        "limit(exp(1/x),x,at:0)",
        "limit(x,at:0)",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    eval(&mut ev, "let x=7");
    assert_eq!(eval(&mut ev, "limit(sin(x)/x,x,at:0)"), Expr::int(1));
    assert_eq!(eval(&mut ev, "x"), Expr::int(7));
    let p = om_parse::parse_expr("limit(sin(x)/x,x,at:0)", om_parse::Dialect::Modern).unwrap();
    let ctx = Interrupt::default();
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(ev.evaluate(&p, &ctx).is_err());
}
#[test]
fn shifted_high_order_holes_and_standard_wolfram_rule_syntax_are_exact() {
    let mut ev = Evaluator::new();
    for (source, want) in [
        ("limit((x-3)^20/(x-3)^19,x,at:3)", "0"),
        ("limit((x-3)^19/(x-3)^20,x,at:3,direction:\"left\")", "-inf"),
        ("limit((2*x+1)/(x-4),x,at:-inf)", "2"),
        ("limit(cbrt(x),x,at:0)", "0"),
        ("limit(atan(x),x,at:-inf)", "-pi/2"),
    ] {
        assert_eq!(eval(&mut ev, source), eval(&mut ev, want), "{source}");
    }
    let p = om_parse::parse_expr("Limit[Sin[x]/x,x->0]", om_parse::Dialect::Wolfram).unwrap();
    assert_eq!(
        ev.evaluate(&p, &Interrupt::default()).unwrap(),
        Expr::int(1)
    );
    for source in [
        "limit(sin(i*x)/x,x,at:inf)",
        "limit(1/x,x,at:0,direction:\"right\",direction:\"left\")",
        "limit(sequence(assign(x,1),x),x,at:0)",
    ] {
        ev.messages.take();
        if let Ok(p) = om_parse::parse_expr(source, om_parse::Dialect::Modern) {
            ev.evaluate(&p, &Interrupt::default()).unwrap();
            assert!(!ev.messages.take().is_empty(), "{source}");
        }
    }
}
