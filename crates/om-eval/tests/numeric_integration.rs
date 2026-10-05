//! The modern API consumes raw readonly source and returns real error/work diagnostics.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn field<'a>(e: &'a Expr, k: &str) -> &'a Expr {
    &e.args()
        .iter()
        .find(|r| r.args().first() == Some(&Expr::string(k)))
        .unwrap_or_else(|| panic!("missing {k}: {e:?}"))
        .args()[1]
}
#[test]
fn modern_numeric_integration_returns_real_machine_diagnostics() {
    let mut ev = Evaluator::new();
    for (source, want) in [
        ("integrate(x^2,x:0..1,mode:\"numeric\")", 1. / 3.),
        (
            "integrate(exp(-x^2),x:-inf..inf,mode:\"numeric\")",
            std::f64::consts::PI.sqrt(),
        ),
        ("integrate(1/sqrt(x),x:0..1,mode:\"numeric\")", 2.),
    ] {
        let value = eval(&mut ev, source);
        assert!(
            (field(&value, "value")
                .as_number()
                .unwrap()
                .to_f64()
                .unwrap()
                - want)
                .abs()
                < 1e-7
        );
        assert_eq!(field(&value, "converged"), &Expr::sym(B::TRUE));
        assert!(
            field(&value, "error_estimate")
                .as_number()
                .unwrap()
                .to_f64()
                .unwrap()
                >= 0.
        );
        assert_eq!(field(&value, "precision"), &Expr::string("machine"));
    }
    let v = om_parse::parse_expr("NIntegrate[x^2,{x,0,1}]", om_parse::Dialect::Wolfram).unwrap();
    let r = ev.evaluate(&v, &Interrupt::default()).unwrap();
    assert!((field(&r, "value").as_number().unwrap().to_f64().unwrap() - 1. / 3.).abs() < 1e-12);
}
#[test]
fn raw_holes_split_points_global_axes_and_readonly_state_are_preserved() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let x=9");
    eval(&mut ev, "let n=0");
    let value = eval(&mut ev, "integrate(x^2,x:0..1,mode:\"numeric\")");
    assert!(
        (field(&value, "value")
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap()
            - 1. / 3.)
            .abs()
            < 1e-12
    );
    assert_eq!(eval(&mut ev, "x"), Expr::int(9));
    ev.messages.take();
    eval(&mut ev, "integrate((x^2-1)/(x-1),x:0..2,mode:\"numeric\")");
    assert!(!ev.messages.take().is_empty());
    let value = eval(
        &mut ev,
        "integrate((x^2-1)/(x-1),x:0..2,mode:\"numeric\",breakpoints:[1])",
    );
    assert!(
        (field(&value, "value")
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap()
            - 4.)
            .abs()
            < 1e-12
    );
    ev.messages.take();
    eval(
        &mut ev,
        "integrate(sequence(assign(n,1),x),x:0..1,mode:\"numeric\")",
    );
    assert!(!ev.messages.take().is_empty());
    assert_eq!(eval(&mut ev, "n"), Expr::int(0));
}
#[test]
fn unsupported_exact_precision_singularities_and_budget_are_real_failures() {
    let mut ev = Evaluator::new();
    for source in [
        "integrate(sin(x^x),x)",
        "integrate(1/x,x:-1..1,mode:\"numeric\")",
        "integrate(x,x:0..1,mode:\"numeric\",precision:50)",
        "integrate(x,x:0..1,mode:\"numeric\",breakpoints:[2])",
    ] {
        ev.messages.take();
        let p = om_parse::parse_expr(source, om_parse::Dialect::Modern);
        if p.is_err() {
            continue;
        }
        ev.evaluate(&p.unwrap(), &Interrupt::default()).unwrap();
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    let p = om_parse::parse_expr(
        "integrate(exp(-x^2),x:0..1,mode:\"numeric\")",
        om_parse::Dialect::Modern,
    )
    .unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(10);
    assert!(ev.evaluate(&p, &ctx).is_err());
}
