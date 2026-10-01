//! Exact zero certificates and deliberately inconclusive numerical identities.
use om_core::{Expr, canonicalize};
use om_num::{
    BigFloat, Number, Rational,
    ctx::{Abort, Interrupt},
};
use om_parse::{Dialect, parse_expr};
use om_simplify::{
    numeval,
    root_reduce::to_algebraic,
    zero::{Tri, UnknownReason, is_zero, is_zero_with},
};
use proptest::prelude::*;

fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
const PROBABLY: Tri = Tri::Unknown(UnknownReason::ProbablyZero);
const NO_INFO: Tri = Tri::Unknown(UnknownReason::NoInfo);

#[test]
fn all_five_authority_vectors() {
    for (input, expected) in [
        ("Sqrt[2]*Sqrt[3]-Sqrt[6]", Tri::Zero),
        ("Sqrt[3+2*Sqrt[2]]-1-Sqrt[2]", Tri::Zero),
        ("Sin[x]^2+Cos[x]^2-1", PROBABLY),
        ("Pi-355/113", Tri::NonZero),
        ("(x+1)^2-x^2-2*x-1", Tri::Zero),
    ] {
        assert_eq!(is_zero(&e(input)), expected, "{input}");
    }
}
#[test]
fn structural_and_rational_levels_decide_formal_identities() {
    for input in ["0", "0.", "0.+0.*I", "1/x+1/y-(x+y)/(x*y)"] {
        assert_eq!(is_zero(&e(input)), Tri::Zero, "{input}");
    }
    for input in ["-3/7", "1.25", "I", "x", "x*y-1", "(x^2-1)/(x-1)"] {
        assert_eq!(is_zero(&e(input)), Tri::NonZero, "{input}");
    }
    for input in ["Indeterminate", "Infinity", "f[x]", "f[1]", "\"text\""] {
        assert_eq!(is_zero(&e(input)), NO_INFO, "{input}");
    }
}
#[test]
fn denesting_and_radical_relations_preserve_principal_values() {
    for input in [
        "Sqrt[3-2*Sqrt[2]]-Sqrt[2]+1",
        "Sqrt[5+2*Sqrt[6]]-Sqrt[2]-Sqrt[3]",
        "(2^(1/3)+3^(1/3))^3-5-3*6^(1/3)*(2^(1/3)+3^(1/3))",
        "1/(1+Sqrt[2])+1-Sqrt[2]",
        "(Sqrt[2]+Sqrt[3])^2-5-2*Sqrt[6]",
    ] {
        assert_eq!(is_zero(&e(input)), Tri::Zero, "{input}");
    }
    for input in [
        "Sqrt[3-2*Sqrt[2]]+Sqrt[2]-1",
        "(-8)^(1/3)+2",
        "Sqrt[-2]-Sqrt[2]",
    ] {
        assert_eq!(is_zero(&e(input)), Tri::NonZero, "{input}");
    }
}
#[test]
fn root_expression_bridge_certifies_minpoly_and_root_number() {
    let ctx = Interrupt::default();
    let a = to_algebraic(&e("Sqrt[2]+Sqrt[3]"), &ctx).unwrap().unwrap();
    assert_eq!(
        a.minimal_polynomial(&ctx).unwrap().coeffs,
        vec![1.into(), 0.into(), (-10).into(), 0.into(), 1.into()]
    );
    let a = to_algebraic(&e("Sqrt[Root[#^3-2&,1]]"), &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(
        a.minimal_polynomial(&ctx).unwrap().coeffs,
        vec![
            (-2).into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            1.into()
        ]
    );
    for input in [
        "Root[#^3-2&,1]^3-2",
        "Root[#^2-2&,1]+Sqrt[2]",
        "Root[#^2+1&,1]+I",
        "Sqrt[Root[#^3-2&,1]]^6-2",
    ] {
        assert_eq!(is_zero(&e(input)), Tri::Zero, "{input}");
    }
    for input in [
        "Root[#^2-2&,3]",
        "Root[#^2-2&,0]",
        "Root[#^2+a&,1]",
        "2^(1/65)",
    ] {
        assert!(to_algebraic(&e(input), &ctx).unwrap().is_none(), "{input}");
    }
}
#[test]
fn algebraic_zero_denominators_and_negative_radicands_are_not_false_certificates() {
    assert_eq!(is_zero(&e("1/(Sqrt[3+2*Sqrt[2]]-1-Sqrt[2])")), NO_INFO);
    assert_eq!(is_zero(&e("Sqrt[1-2*Sqrt[2]]")), Tri::NonZero);
    assert_eq!(is_zero(&e("Sqrt[3-2*Sqrt[2]]-(1-Sqrt[2])")), Tri::NonZero);
    assert!(
        to_algebraic(&e("Sin[1]"), &Interrupt::default())
            .unwrap()
            .is_none()
    );
    assert!(
        to_algebraic(&e("1.25"), &Interrupt::default())
            .unwrap()
            .is_none()
    );
}
#[test]
fn numeric_level_refines_before_classifying_and_never_proves_zero() {
    for input in [
        "Sin[Pi]",
        "Sin[Pi/7]^2+Cos[Pi/7]^2-1",
        "Sin[x]^2+Cos[x]^2-1",
    ] {
        assert_eq!(is_zero(&e(input)), PROBABLY, "{input}");
        assert_eq!(is_zero(&e(input)), is_zero(&e(input)));
    }
    for input in [
        "Sin[Pi+1/10^100]",
        "Exp[x]-1",
        "Sin[x]+Cos[y]",
        "Pi-355/113",
    ] {
        assert_eq!(is_zero(&e(input)), Tri::NonZero, "{input}");
    }
    assert_eq!(is_zero(&e("Sin[Pi+1/10^400]")), PROBABLY);
}
#[test]
fn enclosure_adapter_exposes_certificates_and_rejects_invalid_work() {
    let ctx = Interrupt::default();
    for input in ["Pi-355/113", "Root[#^3-2&,1]"] {
        let z = numeval::enclose(&e(input), 256, &ctx).unwrap().unwrap();
        assert!(z.re.excludes_zero() || z.im.excludes_zero());
    }
    let z = numeval::enclose(&e("Sin[Pi]"), 256, &ctx).unwrap().unwrap();
    assert!(z.re.contains_zero() && z.im.contains_zero());
    assert!(z.re.rad < BigFloat::from_parts(1.into(), -236));
    for (input, bits) in [("x", 64), ("1", 0), ("1", 16385), ("1/0", 64)] {
        assert!(numeval::enclose(&e(input), bits, &ctx).unwrap().is_none());
    }
}
#[test]
fn checked_interfaces_propagate_budget_and_cancellation() {
    let ctx = Interrupt::default();
    for budget in [0, 10, 100] {
        ctx.steps_left.set(budget);
        assert_eq!(is_zero_with(&e("(x+y+z)^100-1"), &ctx), Err(Abort::Budget));
    }
    ctx.steps_left.set(0);
    assert_eq!(
        numeval::enclose(&Expr::int(1), 64, &ctx).unwrap_err(),
        Abort::Budget
    );
    assert_eq!(
        to_algebraic(&Expr::int(1), &ctx).unwrap_err(),
        Abort::Budget
    );
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(is_zero_with(&Expr::int(0), &ctx), Err(Abort::Interrupted));
}
proptest! {
    #![proptest_config(ProptestConfig{cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d382e33),..ProptestConfig::default()})]
    #[test]
    fn rational_expressions_are_decided_exactly(a in -10i64..=10,b in -10i64..=10,c in 1i64..=10) {
        let p=e(&format!("((x+({a}))*(y+({b}))-(x*y+({b})*x+({a})*y+({a})*({b})))/(x^2+({c}))"));
        prop_assert_eq!(is_zero(&p),Tri::Zero);
        let p=e(&format!("(x+({a}))*(y+({b}))/(x^2+({c}))"));
        prop_assert_eq!(is_zero(&p),Tri::NonZero);
        let at=p.replace_all(&[(e("x"),Expr::int(20)),(e("y"),Expr::int(20))]);
        prop_assert_eq!(at,Expr::number(Number::Rational(Rational::from((20+a)*(20+b))/Rational::from(400+c))));
    }
}
proptest! {
    #![proptest_config(ProptestConfig{cases:32,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d382e3302),..ProptestConfig::default()})]
    #[test]
    fn principal_cubic_bridge_has_independent_exact_endpoint_certificate(n in 2i64..=30) {
        let ctx=Interrupt::default();
        let a=to_algebraic(&e(&format!("({n})^(1/3)")),&ctx).unwrap().unwrap();
        let z=a.enclosure(128,&ctx).unwrap().unwrap();
        let q=|x: &BigFloat| {
            let r=x.repr(); let q=Rational::from(r.significand().clone());
            if r.exponent()>=0 { q*Rational::from(om_num::Integer::ONE<<r.exponent() as usize) } else { q/Rational::from(om_num::Integer::ONE<<r.exponent().unsigned_abs()) }
        };
        let lo=q(&z.re.mid)-q(&z.re.rad); let hi=q(&z.re.mid)+q(&z.re.rad);
        prop_assert!(&lo*&lo*&lo<=Rational::from(n));
        prop_assert!(&hi*&hi*&hi>=Rational::from(n));
        prop_assert!(lo>Rational::ZERO);
        prop_assert_eq!(z.im.mid,BigFloat::ZERO);
        prop_assert_eq!(z.im.rad,BigFloat::ZERO);
    }
}
