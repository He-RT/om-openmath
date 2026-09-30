//! N's precision contract, symbolic traversal and resource recovery.
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol};
use om_eval::Evaluator;
use om_num::{Number, Precision, Rational, Real};
use om_parse::{Dialect, parse_expr};

fn src(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn eval(s: &str) -> Expr {
    Evaluator::new()
        .evaluate(&src(s), &Interrupt::default())
        .unwrap()
}
#[test]
fn n_has_fifty_correct_digits_and_machine_rationals() {
    let p = eval("N[Pi,50]");
    let Some(Number::Real(Real::Big(pi))) = p.as_number() else {
        panic!("N must return Big");
    };
    assert_eq!(p.as_number().unwrap().precision(), Precision::Bits(167));
    let reference: Rational = "3141592653589793238462643383279502884197169399375105820974944/1000000000000000000000000000000000000000000000000000000000000".parse().unwrap();
    let error = Rational::try_from(pi.clone()).unwrap() - reference;
    let bound: Rational = "1/100000000000000000000000000000000000000000000000000"
        .parse()
        .unwrap();
    assert!(error > -&bound && error < bound);
    assert_eq!(
        eval("N[1/3]"),
        Expr::number(Number::Real(Real::Machine(1.0 / 3.0)))
    );
    assert_eq!(eval("N[0]"), Expr::number(Number::Real(Real::Machine(0.0))));
}
#[test]
fn n_approximates_numeric_subtrees_and_preserves_holds() {
    let p = eval("N[{Pi,1/3,Sin[1/2]},30]");
    assert!(p.is_head(B::LIST));
    assert!(
        p.args()
            .iter()
            .all(|e| e.as_number().unwrap().precision() == Precision::Bits(100))
    );
    assert_eq!(
        eval("N[x+1]"),
        om_core::add([
            Expr::symbol("x"),
            Expr::number(Number::Real(Real::Machine(1.0)))
        ])
    );
    assert_eq!(
        eval("N[f[Pi]]").args()[0]
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap(),
        std::f64::consts::PI
    );
    assert_eq!(eval("N[Hold[1/3]]"), src("Hold[1/3]"));
    assert_eq!(eval("N[HoldForm[Pi]]"), src("HoldForm[Pi]"));
    assert_eq!(
        eval("x=2;N[Sin[x],30]").as_number().unwrap().precision(),
        Precision::Bits(100)
    );
}
#[test]
fn n_refines_cancellation_and_preserves_overflow_and_underflow() {
    let out = eval("N[Exp[1000]]");
    assert!(matches!(out.as_number(), Some(Number::Real(Real::Big(_)))));
    assert_eq!(out.as_number().unwrap().precision(), Precision::Bits(64));
    assert_eq!(
        eval("N[Exp[1000+I]]").as_number().unwrap().precision(),
        Precision::Bits(64)
    );
    assert!(matches!(
        eval("N[Exp[-1000]]").as_number(),
        Some(Number::Real(Real::Big(_)))
    ));
    let out = eval("N[10^80*(Sqrt[1+10^-80]-1),50]");
    assert!((out.as_number().unwrap().to_f64().unwrap() - 0.5).abs() < 1e-15);
    let high = eval("N[Sin[1/3],100]");
    let low = eval("N[Sin[1/3],50]");
    let Some(Number::Real(Real::Big(high))) = high.as_number() else {
        panic!("big");
    };
    assert_eq!(
        low.as_number().unwrap(),
        &Number::Real(Real::Big(high.clone().with_precision(167).value()))
    );
}
#[test]
fn n_docs_invalid_precision_arity_and_abort_are_real() {
    let doc = Evaluator::doc(B::N).unwrap();
    assert!(!doc.summary_zh.is_empty() && !doc.summary_en.is_empty() && doc.examples.len() >= 2);
    let mut ev = Evaluator::new();
    for s in ["N[Pi,0]", "N[Pi,-1]", "N[Pi,x]", "N[Pi,1000000000]"] {
        assert_eq!(ev.evaluate(&src(s), &Interrupt::default()).unwrap(), src(s));
        assert!(
            ev.messages
                .take()
                .iter()
                .any(|m| m.symbol == "N" && m.tag == "precbd")
        );
    }
    let e = src("N[]");
    assert_eq!(ev.evaluate(&e, &Interrupt::default()).unwrap(), e);
    assert!(
        ev.messages
            .take()
            .iter()
            .any(|m| m.symbol == "N" && m.tag == "argrx")
    );
    let ctx = Interrupt::default();
    ctx.steps_left.set(8);
    assert!(ev.evaluate(&src("N[Sin[1/3],50]"), &ctx).is_err());
    assert_eq!(
        ev.evaluate(&src("2+2"), &Interrupt::default()).unwrap(),
        Expr::int(4)
    );
    assert!(Evaluator::all_docs().any(|d| d.name == Symbol::intern("N").name()));
}
