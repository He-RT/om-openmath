//! Symbolic primitives are checked by genuine differentiation and exact rational identities.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn value(e: &Expr) -> Expr {
    if e.is_head(B::CONDITIONAL_EXPRESSION) {
        e.args()[0].clone()
    } else {
        e.clone()
    }
}
fn check(ev: &mut Evaluator, source: &str) {
    let primitive = value(&eval(ev, &format!("integrate({source},x)")));
    assert!(
        !primitive.is_head(om_core::Symbol::intern("Integrate")),
        "{source}"
    );
    let d = ev
        .evaluate(
            &Expr::call(B::D, [primitive, Expr::symbol("x")]),
            &Interrupt::default(),
        )
        .unwrap();
    let original = eval(ev, source);
    let residual = om_core::sub(d, original);
    let zero = ev
        .evaluate(
            &Expr::call(om_core::Symbol::intern("FullSimplify"), [residual]),
            &Interrupt::default(),
        )
        .unwrap();
    assert!(zero.is_zero(), "{source}: {zero:?}");
}
#[test]
fn polynomials_linear_substitution_and_finite_parts_are_actual_primitives() {
    let mut ev = Evaluator::new();
    for f in [
        "3*x^4+2*x-7",
        "sin(3*x+2)",
        "cos(2*x)",
        "exp(2*x+1)",
        "log(2*x+1)",
        "atan(2*x)",
        "asin(x)",
        "acos(x)",
        "asinh(x)",
        "atanh(x)",
        "acoth(x)",
        "cbrt(x)",
        "x*exp(x)",
        "x^2*sin(x)",
        "x*log(x)",
        "2*x*cos(x^2)",
        "x/(x^2+1)",
    ] {
        check(&mut ev, f);
    }
    assert_eq!(eval(&mut ev, "integrate(x^2,x:0..1)"), Expr::rational(1, 3));
    assert_eq!(
        eval(&mut ev, "integrate(x^2,x:1..0)"),
        Expr::rational(-1, 3)
    );
    assert_eq!(eval(&mut ev, "integrate(cos(x),x:0..pi/2)"), Expr::int(1));
    assert_eq!(
        eval(&mut ev, "integrate((x^2-1)/(x-1),x:0..2)"),
        Expr::int(4)
    );
    let tangent = eval(&mut ev, "integrate(tan(x),x:0..pi/4)");
    let expected = eval(&mut ev, "-log(cos(pi/4))");
    assert_eq!(tangent, expected, "{:?}", ev.messages.take());
}
#[test]
fn rational_linear_quadratic_and_repeated_factors_are_verified() {
    let mut ev = Evaluator::new();
    for f in [
        "1/(x+1)",
        "1/(2*x+3)^2",
        "1/(x^2+1)",
        "(2*x+3)/(x^2+2*x+2)",
        "1/(x^2+1)^2",
        "1/((x-1)*(x+2))",
    ] {
        check(&mut ev, f);
    }
    assert_eq!(
        eval(&mut ev, "integrate(1/(x+1)^2,x:0..1)"),
        Expr::rational(1, 2)
    );
}
#[test]
fn gaussian_parameters_readonly_axes_and_unsupported_exact_are_honest() {
    let mut ev = Evaluator::new();
    check(&mut ev, "exp(-x^2)");
    check(&mut ev, "exp(-2*x^2+4*x)");
    let generic = eval(&mut ev, "integrate(exp(a*x),x)");
    assert!(generic.is_head(B::CONDITIONAL_EXPRESSION));
    assert_eq!(
        eval(&mut ev, "integrate(exp(-x^2),x:-inf..inf)"),
        eval(&mut ev, "sqrt(pi)")
    );
    assert_eq!(
        eval(&mut ev, "integrate(exp(-x^2),x:0..inf)"),
        eval(&mut ev, "sqrt(pi)/2")
    );
    eval(&mut ev, "let x=5");
    let p = eval(&mut ev, "integrate(x^2,x)");
    assert_eq!(
        value(&p),
        om_core::div(om_core::pow(Expr::symbol("x"), Expr::int(3)), Expr::int(3))
    );
    assert_eq!(eval(&mut ev, "x"), Expr::int(5));
    for f in [
        "integrate(1/x,x:-1..1)",
        "integrate(sin(x^x),x)",
        "integrate(sequence(assign(x,1),x),x)",
    ] {
        ev.messages.take();
        eval(&mut ev, f);
        assert!(!ev.messages.take().is_empty(), "{f}");
    }
}
#[test]
fn rational_domains_real_branch_paths_and_parameter_convergence_are_explicit() {
    let mut ev = Evaluator::new();
    for f in [
        "1/(x^2-1)",
        "(3*x+1)/(x^2+4)",
        "1/(x^2+2*x+2)^3",
        "x^2*exp(2*x)",
        "x*log(2*x+1)",
        "x^2/(1+x^2)",
    ] {
        check(&mut ev, f);
    }
    for source in [
        "integrate(tan(x),x:0..pi)",
        "integrate(1/(x-1)^2,x:0..2)",
        "integrate(x^a,x:0..1)",
        "integrate(1/x,x:0..1)",
        "integrate(x^i,x:0..1)",
        "integrate([x,x^2],x)",
        "integrate(sqrt(x),x:(-1-i)..(-1+i))",
        "integrate(0.1*x,x)",
        "integrate(sin(0.1*x),x)",
        "integrate(acoth(x),x:0..1/2)",
        "integrate(cbrt(i*x),x:1..2)",
        "integrate(atan(i*x),x:0..2)",
    ] {
        ev.messages.take();
        let result = eval(&mut ev, source);
        assert!(
            result.is_head(om_core::Symbol::intern("Integrate")),
            "{source}: {result:?}"
        );
        assert!(!ev.messages.take().is_empty());
    }
    let ctx = Interrupt::default();
    ctx.steps_left.set(20);
    let input = om_parse::parse_expr("integrate(x*log(x),x)", om_parse::Dialect::Modern).unwrap();
    assert!(ev.evaluate(&input, &ctx).is_err());
}
#[test]
fn independent_polynomial_coefficients_and_branch_conditions_are_preserved() {
    let mut ev = Evaluator::new();
    let mut rng = om_num::rng::SplitMix64::new(314159);
    for _ in 0..32 {
        let x = Expr::symbol("x");
        let mut terms = vec![];
        let mut expected = vec![];
        for n in 0..8 {
            let a = (rng.next_range(0, 21) as i64) - 10;
            terms.push(om_core::mul([
                Expr::int(a),
                om_core::pow(x.clone(), Expr::int(n)),
            ]));
            expected.push(om_core::mul([
                Expr::rational(a, n + 1),
                om_core::pow(x.clone(), Expr::int(n + 1)),
            ]));
        }
        let primitive = ev
            .evaluate(
                &Expr::call(
                    om_core::Symbol::intern("Integrate"),
                    [om_core::add(terms), x],
                ),
                &Interrupt::default(),
            )
            .unwrap();
        assert_eq!(primitive, om_core::add(expected));
    }
    let primitive = eval(&mut ev, "integrate(1/x,x)");
    assert!(primitive.is_head(B::CONDITIONAL_EXPRESSION));
    let condition = om_format::input_form(&primitive.args()[1]);
    assert!(
        condition.contains("Reals") && condition.contains("Re[x]"),
        "{condition}"
    );
}
#[test]
fn local_real_domain_conditions_are_proved_over_a_definite_interval() {
    let mut ev = Evaluator::new();
    assert_eq!(eval(&mut ev, "integrate(cbrt(x),x:0..8)"), Expr::int(12));
    assert_eq!(eval(&mut ev, "integrate(cbrt(x),x:-8..0)"), Expr::int(-12));
    let result = eval(&mut ev, "integrate(acoth(x),x:2..3)");
    assert!(
        !result.is_head(om_core::Symbol::intern("Integrate")),
        "{result:?}"
    );
    assert!(result.free_of(&Expr::symbol("x")));
}
