//! Independent Cartesian derivatives verify actual dimensions, local axes and calculus identities.
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
fn gradient_jacobian_hessian_and_laplacian_use_real_symbolic_derivatives() {
    let mut ev = Evaluator::new();
    for (source, want) in [
        ("grad(x^2*y,[x,y])", "[2*x*y,x^2]"),
        ("jacobian([x*y,sin(x)],[x,y])", "[[y,x],[cos(x),0]]"),
        ("hessian(x^2*y+y^3,[x,y])", "[[2*y,2*x],[2*x,6*y]]"),
        ("laplacian(x^2+y^2+z^2,[x,y,z])", "6"),
        ("div([x^2,y^2,z^2],[x,y,z])", "2*x+2*y+2*z"),
        ("curl([-y,x,0],[x,y,z])", "[0,0,2]"),
    ] {
        assert_eq!(eval(&mut ev, source), eval(&mut ev, want), "{source}");
    }
}
#[test]
fn local_variables_and_readonly_source_leave_global_definitions_intact() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let x=7");
    eval(&mut ev, "let y=8");
    let gradient = eval(&mut ev, "grad(x^2*y,[x,y])");
    let expected = om_parse::parse_expr("{2*x*y,x^2}", om_parse::Dialect::Wolfram).unwrap();
    assert_eq!(
        om_core::canonicalize(&gradient),
        om_core::canonicalize(&expected)
    );
    assert_eq!(eval(&mut ev, "x+y"), Expr::int(15));
    eval(&mut ev, "let n=0");
    ev.messages.take();
    eval(&mut ev, "grad(sequence(assign(n,1),x^2),[x])");
    assert_eq!(eval(&mut ev, "n"), Expr::int(0));
    assert!(!ev.messages.take().is_empty());
}
#[test]
fn malformed_vector_dimensions_and_budget_do_not_produce_fake_results() {
    let mut ev = Evaluator::new();
    for source in [
        "grad([x,y],[x,y])",
        "jacobian(x,[x])",
        "hessian(x,[x,x])",
        "div([x,y],[x])",
        "curl([x,y],[x,y])",
        "laplacian(x,[])",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    let e = om_parse::parse_expr("hessian((x+y+z)^8,[x,y,z])", om_parse::Dialect::Modern).unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(20);
    assert!(ev.evaluate(&e, &ctx).is_err());
}
#[test]
fn curl_grad_and_div_curl_are_identically_zero() {
    let mut ev = Evaluator::new();
    assert_eq!(
        eval(&mut ev, "curl(grad(x^3*y+sin(z)*y,[x,y,z]),[x,y,z])"),
        eval(&mut ev, "[0,0,0]")
    );
    assert_eq!(
        eval(&mut ev, "div(curl([x*y,sin(z)*x,y*z],[x,y,z]),[x,y,z])"),
        Expr::int(0)
    );
}
#[test]
fn new_elementary_derivatives_match_independent_centered_values() {
    let mut ev = Evaluator::new();
    let x = om_core::Symbol::intern("x");
    for (source, point, function) in [
        (
            "diff(erf(x),x)",
            0.7,
            om_analysis::special::erf as fn(f64, &Interrupt) -> Result<f64, om_analysis::Error>,
        ),
        (
            "diff(erfc(x),x)",
            0.7,
            om_analysis::special::erfc as fn(f64, &Interrupt) -> Result<f64, om_analysis::Error>,
        ),
    ] {
        let derivative = eval(&mut ev, source);
        let p = om_eval::numeric::compile_f64(&derivative, &[x]).unwrap();
        let h = 1e-5;
        let ctx = Interrupt::default();
        let reference =
            (function(point + h, &ctx).unwrap() - function(point - h, &ctx).unwrap()) / (2. * h);
        assert!((p.eval(&[point]) - reference).abs() < 2e-10);
    }
    assert_eq!(
        eval(&mut ev, "diff(cbrt(x),x)"),
        eval(&mut ev, "1/(3*cbrt(x)^2)")
    );
    assert_eq!(
        eval(&mut ev, "diff(acoth(x),x)"),
        eval(&mut ev, "1/(1-x^2)")
    );
}
