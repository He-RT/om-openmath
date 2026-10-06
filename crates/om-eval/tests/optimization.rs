//! Local numerical candidates and independently checked exact global certificates are distinct.
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
        .find(|r| r.args()[0] == Expr::string(k))
        .unwrap_or_else(|| panic!("missing {k}: {e:?}"))
        .args()[1]
}
fn real(e: &Expr) -> f64 {
    e.as_number().unwrap().to_f64().unwrap()
}
#[test]
fn actual_local_min_max_brent_bfgs_and_box_candidates_have_real_work() {
    let mut ev = Evaluator::new();
    let result = eval(&mut ev, "optimize((x-2)^2,x,initial:0)");
    assert!((real(&field(&result, "point").args()[0]) - 2.).abs() < 1e-7);
    assert_eq!(
        field(&result, "guarantee"),
        &Expr::string("numerical_stationary_candidate")
    );
    assert!(real(field(&result, "evaluations")) > 0.);
    let result = eval(&mut ev, "optimize(-(x-2)^2,x,initial:0,goal:\"max\")");
    assert!(real(field(&result, "value")).abs() < 1e-12);
    let result = eval(&mut ev, "optimize((x-4)^2,x,bounds:-2..1)");
    assert_eq!(real(&field(&result, "point").args()[0]), 1.);
    assert_eq!(real(field(&result, "value")), 9.);
    let result = eval(
        &mut ev,
        "optimize((x-2)^2+(y+3)^2,[x,y],initial:[0,0],bounds:[-1..1,-2..2])",
    );
    assert_eq!(
        field(&result, "point")
            .args()
            .iter()
            .map(real)
            .collect::<Vec<_>>(),
        vec![1., -2.]
    );
    assert_eq!(real(field(&result, "projected_gradient_norm")), 0.);
}
#[test]
fn exact_global_positive_and_semidefinite_quadratics_preserve_free_axes() {
    let mut ev = Evaluator::new();
    let result = eval(
        &mut ev,
        "optimize((x-2)^2+2*(y+1)^2+3,[x,y],scope:\"global\")",
    );
    assert_eq!(
        field(&result, "point"),
        &Expr::call(B::LIST, [Expr::int(2), Expr::int(-1)])
    );
    assert_eq!(field(&result, "value"), &Expr::int(3));
    assert_eq!(
        field(&result, "guarantee"),
        &Expr::string("certified_global")
    );
    let certificate = field(&result, "certificate");
    assert_eq!(
        field(certificate, "kkt_residual"),
        &Expr::call(B::LIST, [Expr::int(0), Expr::int(0)])
    );
    let result = eval(&mut ev, "optimize((x+y-1)^2,[x,y],scope:\"global\")");
    assert_eq!(field(&result, "value"), &Expr::int(0));
    assert_eq!(field(&result, "null_space").args().len(), 1);
    let point = field(&result, "point");
    let direction = &field(&result, "null_space").args()[0];
    for t in [-3., 0., 2.5] {
        assert!(
            (real(&point.args()[0])
                + real(&point.args()[1])
                + t * (real(&direction.args()[0]) + real(&direction.args()[1]))
                - 1.)
                .abs()
                < 1e-14
        );
    }
    let result = eval(&mut ev, "optimize(7,[x,y],scope:\"global\")");
    assert_eq!(field(&result, "null_space").args().len(), 2);
    assert_eq!(field(&result, "value"), &Expr::int(7));
}
#[test]
fn exact_box_kkt_and_concave_maximum_are_genuine_global_certificates() {
    let mut ev = Evaluator::new();
    let result = eval(
        &mut ev,
        "optimize((x-2)^2+(y+3)^2,[x,y],bounds:[-1..1,-2..2],scope:\"global\")",
    );
    assert_eq!(
        field(&result, "point"),
        &Expr::call(B::LIST, [Expr::int(1), Expr::int(-2)])
    );
    assert_eq!(field(&result, "value"), &Expr::int(2));
    let certificate = field(&result, "certificate");
    assert_eq!(
        field(certificate, "kkt_residual"),
        &Expr::call(B::LIST, [Expr::int(0), Expr::int(0)])
    );
    assert_eq!(
        field(certificate, "lower_multipliers"),
        &Expr::call(B::LIST, [Expr::int(0), Expr::int(2)])
    );
    assert_eq!(
        field(certificate, "upper_multipliers"),
        &Expr::call(B::LIST, [Expr::int(2), Expr::int(0)])
    );
    let result = eval(
        &mut ev,
        "optimize(5-(x-1)^2,x,scope:\"global\",goal:\"max\")",
    );
    assert_eq!(field(&result, "value"), &Expr::int(5));
    let result = eval(
        &mut ev,
        "optimize(x+2*y,[x,y],scope:\"global\",bounds:[0..1,2..2])",
    );
    assert_eq!(field(&result, "value"), &Expr::int(4));
    let result = eval(
        &mut ev,
        "optimize((x+y-5)^2,[x,y],scope:\"global\",bounds:[0..1,0..1])",
    );
    assert_eq!(
        field(&result, "point"),
        &Expr::call(B::LIST, [Expr::int(1), Expr::int(1)])
    );
    assert_eq!(field(&result, "value"), &Expr::int(9));
    let result = eval(
        &mut ev,
        "optimize((x+y-1)^2,[x,y],scope:\"global\",bounds:[2..3,-3..0])",
    );
    assert_eq!(field(&result, "value"), &Expr::int(0));
}
#[test]
fn unsupported_global_approximate_inputs_domains_bad_dimensions_and_writes_fail() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let x=100");
    eval(&mut ev, "let n=0");
    let result = eval(&mut ev, "optimize((x-2)^2,x,initial:0)");
    assert!((real(&field(&result, "point").args()[0]) - 2.).abs() < 1e-7);
    assert_eq!(eval(&mut ev, "x"), Expr::int(100));
    for source in [
        "optimize(x^4,x,scope:\"global\")",
        "optimize(x^2-y^2,[x,y],scope:\"global\")",
        "optimize(x^2+y,[x,y],scope:\"global\")",
        "optimize((x-0.1)^2,x,scope:\"global\")",
        "optimize(x/x,x,scope:\"global\")",
        "optimize((x/x)^0,x,scope:\"global\")",
        "optimize(sequence(assign(n,9),x^2),x,initial:0)",
        "optimize(x^2,[x,y],initial:[0])",
        "optimize(x^2,x,initial:decimal(\"1\",precision:50))",
        "optimize(log(x),x,initial:-1)",
        "optimize(-x,x,initial:0,max_iterations:1)",
        "optimize((x-4)^2,x,scope:\"global\",bounds:0..1,max_iterations:1)",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    assert_eq!(eval(&mut ev, "n"), Expr::int(0));
    let p = om_parse::parse_expr(
        "optimize(x^2,x,scope:\"global\")",
        om_parse::Dialect::Modern,
    )
    .unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(20);
    assert!(ev.evaluate(&p, &ctx).is_err());
}
#[test]
fn certificate_reconstructs_the_independent_input_hessian_and_exact_rational_minimizer() {
    use om_num::{Number, Rational};
    fn q(e: &Expr) -> Rational {
        match e.as_number().unwrap() {
            Number::Integer(n) => Rational::from(n.clone()),
            Number::Rational(q) => q.clone(),
            _ => panic!("certificate must be exact"),
        }
    }
    let mut ev = Evaluator::new();
    let result = eval(
        &mut ev,
        "optimize(3*x^2+2*x*y+2*y^2-4*x+6*y+9,[x,y],scope:\"global\")",
    );
    let point = field(&result, "point");
    assert_eq!(
        q(&point.args()[0]),
        Rational::from_parts(7.into(), 5u32.into())
    );
    assert_eq!(
        q(&point.args()[1]),
        Rational::from_parts((-11).into(), 5u32.into())
    );
    assert_eq!(
        q(field(&result, "value")),
        Rational::from_parts((-2).into(), 5u32.into())
    );
    let cert = field(&result, "certificate");
    let l = field(cert, "ldlt_l");
    let d = field(cert, "ldlt_d");
    let h = field(cert, "hessian");
    let reference = [[6, 2], [2, 4]];
    for (i, row) in reference.iter().enumerate() {
        for (j, entry) in row.iter().enumerate() {
            let reconstructed = (0..2)
                .map(|k| q(&l.args()[i].args()[k]) * q(&d.args()[k]) * q(&l.args()[j].args()[k]))
                .fold(Rational::ZERO, |a, b| a + b);
            assert_eq!(reconstructed, Rational::from(*entry));
            assert_eq!(q(&h.args()[i].args()[j]), reconstructed);
        }
    }
    assert_eq!(
        field(cert, "linear"),
        &Expr::call(B::LIST, [Expr::int(-4), Expr::int(6)])
    );
    assert_eq!(field(cert, "constant"), &Expr::int(9));
}
#[test]
fn readonly_record_bounds_method_tolerances_and_guarantees_do_not_change_meaning() {
    let mut ev = Evaluator::new();
    let result = eval(
        &mut ev,
        "optimize((x-2)^2+(y+3)^2,[x,y],initial:[0,0],bounds:{x:-1..1,y:-2..2})",
    );
    assert_eq!(real(field(&result, "value")), 2.);
    let saddle = eval(&mut ev, "optimize(-x^2,x,initial:0)");
    assert_eq!(
        field(&saddle, "guarantee"),
        &Expr::string("numerical_stationary_candidate")
    );
    assert_eq!(field(&saddle, "certificate"), &Expr::sym(B::NULL));
    for source in [
        "optimize(x^2,x,scope:\"global\",method:\"bfgs\")",
        "optimize(x^2,x,scope:\"global\",precision:\"machine\")",
        "optimize(pi*x^2,x,scope:\"global\")",
        "optimize(x^2,x,scope:\"global\",initial:0)",
        "optimize(x^2,x,initial:0,abs_tol:0.01)",
        "optimize(x^2,x,bounds:-1..1,gradient_tol:0.01)",
        "optimize(x^2,[x,x],initial:[0,0])",
        "optimize(x^2,x,initial:0,bounds:{y:-1..1})",
        "optimize(0*(x/x),x,scope:\"global\")",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
}
