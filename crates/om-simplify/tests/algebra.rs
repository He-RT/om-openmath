//! Rational transforms preserve exact identities and requested expansion/factor structure.
use om_core::{Expr, canonicalize};
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
use om_parse::{Dialect, parse_expr};
use om_simplify::algebra::{
    cancel, cancel_with, expand, expand_with, factor, factor_with, together, together_with,
};
use om_simplify::convert::to_rational_function_with;
use proptest::prelude::*;
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
fn equal(a: &Expr, b: &Expr, ctx: &Interrupt) {
    let d = om_core::sub(a.clone(), b.clone());
    let view = to_rational_function_with(&d, &[], ctx).unwrap().unwrap();
    assert!(view.num.is_zero(), "{a:?} != {b:?}");
}
#[test]
fn authority_together_cancel_and_factor_vectors() {
    for (input, output) in [
        ("1/x+1/y", "(x+y)/(x*y)"),
        ("1/(x+1)+1/(x-1)", "2*x/((x+1)*(x-1))"),
    ] {
        let got = together(&e(input));
        equal(&got, &e(output), &Interrupt::default());
    }
    for (input, output) in [
        ("(x^2-1)/(x-1)", "x+1"),
        ("(x*y+x)/(x+1)", "x*(y+1)/(x+1)"),
        ("(x^2-y^2)/(x-y)", "x+y"),
        ("(x^2/3-x/3)/(x/2-1/2)", "2*x/3"),
    ] {
        let got = cancel(&e(input));
        if input == "(x*y+x)/(x+1)" {
            equal(&got, &e(output), &Interrupt::default());
        } else {
            assert_eq!(got, e(output));
        }
    }
    for (input, output) in [
        ("x^2-1", "(x-1)*(x+1)"),
        ("x^4-1", "(x-1)*(x+1)*(x^2+1)"),
        ("x^2-2*x*y+y^2", "(x-y)^2"),
        ("(x^2-1)/(y^2-1)", "(x-1)*(x+1)/((y-1)*(y+1))"),
        ("x^2/6-1/6", "(x-1)*(x+1)/6"),
    ] {
        assert_eq!(factor(&e(input)), e(output));
    }
}
#[test]
fn expand_distributes_positive_powers_and_keeps_negative_power_kernels() {
    for (input, output) in [
        ("(x+y)^3", "x^3+3*x^2*y+3*x*y^2+y^3"),
        ("(x+1)^2/(y+1)", "x^2/(y+1)+2*x/(y+1)+1/(y+1)"),
        ("1/(x+y)^2", "1/(x+y)^2"),
        ("Sin[(x+1)^2]", "Sin[(x+1)^2]"),
        ("(Sqrt[x]+x^(1/3))^2", "x+2*x^(5/6)+x^(2/3)"),
    ] {
        assert_eq!(expand(&e(input)), e(output));
    }
}
#[test]
fn normalized_generators_support_cancellation_and_opaque_functions() {
    for (input, output) in [
        ("(x^(1/2)-x^(1/3))/(x^(1/6)-1)", "x^(1/3)"),
        ("(Sin[x]^2-1)/(Sin[x]-1)", "1+Sin[x]"),
        ("(E^(2*x)-1)/(E^x-1)", "(E^(2*x)-1)/(E^x-1)"),
    ] {
        equal(&cancel(&e(input)), &e(output), &Interrupt::default());
    }
    let got = factor(&e("Sin[x]^2-1"));
    assert_eq!(got, e("(Sin[x]-1)*(Sin[x]+1)"));
}
#[test]
fn constants_zeros_and_checked_resource_failures_are_honest() {
    let ctx = Interrupt::default();
    for input in ["0", "-7/3", "1.25", "I"] {
        let input = e(input);
        assert_eq!(cancel(&input), input);
        assert_eq!(factor(&input), input);
        assert_eq!(expand(&input), input);
        assert_eq!(together(&input), input);
    }
    for budget in [0, 10, 100] {
        ctx.steps_left.set(budget);
        assert!(matches!(
            expand_with(&e("(x+y+z)^50"), &ctx),
            Err(Abort::Budget)
        ));
    }
    ctx.steps_left.set(0);
    assert!(matches!(
        together_with(&e("1/x+1/y"), &[], &ctx),
        Err(Abort::Budget)
    ));
    assert!(matches!(
        cancel_with(&e("x"), &[], &ctx),
        Err(Abort::Budget)
    ));
    assert!(matches!(
        factor_with(&e("x^2-1"), &[], &ctx),
        Err(Abort::Budget)
    ));
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        expand_with(&Expr::int(0), &ctx),
        Err(Abort::Interrupted)
    ));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:64,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d382e32),..ProptestConfig::default()})]
 #[test]
 fn rational_transformations_and_idempotence_preserve_exact_values(a in -5i64..=5,b in -5i64..=5,c in 1i64..=5,x in 6i64..=10){let ctx=Interrupt::default();let input=e(&format!("(x+({a}))*(x+({b}))/((x+({c}))*(x+({a})))"));
 for operation in [together as fn(&Expr)->Expr,cancel,factor,expand]{let got=operation(&input);equal(&got,&input,&ctx);prop_assert_eq!(operation(&got),got.clone());let at=got.replace_all(&[(e("x"),Expr::int(x))]);let expected=Rational::from(x+b)/Rational::from(x+c);prop_assert_eq!(at,Expr::number(om_num::Number::Rational(expected)));}
 }
}
