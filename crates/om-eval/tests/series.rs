//! Exact Taylor coefficients and singular-domain checks run through real source/derivative paths.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn true_derivatives_produce_exact_taylor_coefficients_and_normal_polynomials() {
    let mut ev = Evaluator::new();
    for (source, want) in [
        (
            "normal(series(sin(x),x,order:7))",
            "x-x^3/6+x^5/120-x^7/5040",
        ),
        ("normal(series(exp(x),x,order:4))", "1+x+x^2/2+x^3/6+x^4/24"),
        ("normal(series(e^x,x,order:4))", "1+x+x^2/2+x^3/6+x^4/24"),
        ("normal(series(1/(1-x),x,order:4))", "1+x+x^2+x^3+x^4"),
        (
            "normal(series(log(x),x,at:1,order:3))",
            "(x-1)-(x-1)^2/2+(x-1)^3/3",
        ),
    ] {
        let result = eval(&mut ev, source);
        assert_eq!(result, eval(&mut ev, want), "{source}: {result:?}");
    }
    assert_eq!(
        eval(&mut ev, "series_coefficient(series(sin(x),x,order:7),5)"),
        Expr::rational(1, 120)
    );
    let s = eval(&mut ev, "series(exp(x),x,order:2)");
    assert_eq!(s.head_symbol().unwrap().name(), "SeriesData");
    assert_eq!(s.args()[4], Expr::int(3));
}
#[test]
fn truncation_range_singularities_branch_points_and_formal_derivatives_are_not_faked() {
    let mut ev = Evaluator::new();
    for source in [
        "series(1/x,x)",
        "series(log(x),x)",
        "series(sqrt(x),x)",
        "series(abs(x),x)",
        "series((x^2-1)/(x-1),x,at:1)",
        "series(f(x),x)",
        "series(0.1*x,x)",
        "series_coefficient(series(exp(x),x,order:2),3)",
        "series(acsch(x),x,at:2,order:3)",
        "normal(SeriesData(x,0,[1],0,2,1))",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    assert_eq!(eval(&mut ev, "normal([1,2])"), eval(&mut ev, "[1,2]"));
    eval(&mut ev, "let n=0");
    ev.messages.take();
    eval(&mut ev, "normal(SeriesData(x,0,[assign(n,1)],0,1,1))");
    assert!(!ev.messages.take().is_empty());
    assert_eq!(eval(&mut ev, "n"), Expr::int(0));
}
#[test]
fn taylor_axes_are_local_readonly_and_cancelled_work_retains_session_state() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let x=5");
    let result = eval(&mut ev, "normal(series(x^2,x,order:2))");
    assert_eq!(result, om_core::pow(Expr::symbol("x"), Expr::int(2)));
    assert_eq!(eval(&mut ev, "x"), Expr::int(5));
    eval(&mut ev, "let n=0");
    ev.messages.take();
    eval(&mut ev, "series(sequence(assign(n,1),x),x)");
    assert!(!ev.messages.take().is_empty());
    assert_eq!(eval(&mut ev, "n"), Expr::int(0));
    let p = om_parse::parse_expr("series(exp(x),x,order:64)", om_parse::Dialect::Modern).unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(20);
    assert!(ev.evaluate(&p, &ctx).is_err());
    assert_eq!(B::LENGTH.name(), "Length");
}
#[test]
fn independent_binomial_coefficients_high_orders_and_wolfram_specs_are_real() {
    let mut ev = Evaluator::new();
    for n in 0..=8 {
        let source = format!("normal(series((1+x)^{n},x,order:{n}))");
        let mut coefficient = 1i64;
        let mut terms = vec![];
        for k in 0..=n {
            terms.push(om_core::mul([
                Expr::int(coefficient),
                om_core::pow(Expr::symbol("x"), Expr::int(k)),
            ]));
            if k < n {
                coefficient = coefficient * (n - k) / (k + 1);
            }
        }
        assert_eq!(eval(&mut ev, &source), om_core::add(terms));
    }
    assert_eq!(
        eval(&mut ev, "normal(series(x^x,x,at:1,order:3))"),
        eval(&mut ev, "1+(x-1)+(x-1)^2+(x-1)^3/2")
    );
    assert_eq!(
        eval(&mut ev, "series_coefficient(series(exp(x),x,order:64),64)"),
        eval(&mut ev, "1/factorial(64)")
    );
    let expr =
        om_parse::parse_expr("Normal[Series[Sin[x],{x,0,3}]]", om_parse::Dialect::Wolfram).unwrap();
    assert_eq!(
        ev.evaluate(&expr, &Interrupt::default()).unwrap(),
        eval(&mut ev, "x-x^3/6")
    );
    for source in ["series(x^x,x)", "series(x,x,order:65)"] {
        ev.messages.take();
        if let Ok(p) = om_parse::parse_expr(source, om_parse::Dialect::Modern) {
            ev.evaluate(&p, &Interrupt::default()).unwrap();
            assert!(!ev.messages.take().is_empty());
        }
    }
}
