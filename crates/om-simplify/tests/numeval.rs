//! Shared numeric expression evaluation, convergence and host budgets.
use om_core::{Expr, Interrupt, canonicalize};
use om_num::{BigFloat, Number, Precision, Real};
use om_parse::{Dialect, parse_expr};
use om_simplify::numeval::{approximate, evaluate};
fn e(src: &str) -> Expr {
    canonicalize(&parse_expr(src, Dialect::Wolfram).unwrap())
}

#[test]
fn repeated_numeric_subtrees_keep_directed_rounding_precision_and_cancellation() {
    use om_core::BUILTIN as B;
    use std::{
        cell::Cell,
        sync::{Arc, atomic::AtomicBool},
    };
    let term = e("Sin[Pi/7]");
    let repeated = Expr::call(B::PLUS, (0..64).map(|_| term.clone()));
    let expected = 64. * (std::f64::consts::PI / 7.).sin();
    for precision in [Precision::Machine, Precision::Bits(167)] {
        let ctx = Interrupt {
            steps_left: Cell::new(10000),
            ..Interrupt::default()
        };
        let value = approximate(&repeated, precision, &ctx).unwrap().unwrap();
        assert!((value.to_f64().unwrap() - expected).abs() < 1e-13);
        assert_eq!(value.precision(), precision);
        // Sharing reduces real computation while all visits still spend budget.
        assert!(ctx.steps_left.get() > 9000);
    }
    let cancelled = Interrupt {
        flag: Arc::new(AtomicBool::new(true)),
        ..Interrupt::default()
    };
    assert!(approximate(&repeated, Precision::Machine, &cancelled).is_err());
    let tiny = e("(1+I/10^80)^(1/3)+(1+I/10^80)^(1/3)");
    let value = approximate(&tiny, Precision::Bits(400), &Interrupt::default())
        .unwrap()
        .unwrap();
    assert!(
        matches!(value, Number::Complex(_)),
        "tiny true imaginary part must survive sharing"
    );
    let many = Expr::call(
        B::PLUS,
        (1..=300).map(|i| Expr::call(B::SQRT, [Expr::int(i)])),
    );
    let expected = (1..=300).map(|i| (i as f64).sqrt()).sum::<f64>();
    let actual = approximate(&many, Precision::Machine, &Interrupt::default())
        .unwrap()
        .unwrap()
        .to_f64()
        .unwrap();
    assert!(
        (actual - expected).abs() < 1e-10,
        "reaching the local memo cap must not discard terms"
    );
}
#[test]
fn scalar_elementary_functions_match_binary64_and_both_precision_modes() {
    for (name, reference) in [
        ("Sin", 0.5f64.sin()),
        ("Cos", 0.5f64.cos()),
        ("Tan", 0.5f64.tan()),
        ("Cot", 1.0 / 0.5f64.tan()),
        ("Sec", 1.0 / 0.5f64.cos()),
        ("Csc", 1.0 / 0.5f64.sin()),
        ("ArcSin", 0.5f64.asin()),
        ("ArcCos", 0.5f64.acos()),
        ("ArcTan", 0.5f64.atan()),
        ("ArcCot", (1.0 / 0.5f64).atan()),
        ("ArcSec", 2.0f64.acos()),
        ("ArcCsc", 2.0f64.asin()),
        ("Sinh", 0.5f64.sinh()),
        ("Cosh", 0.5f64.cosh()),
        ("Tanh", 0.5f64.tanh()),
        ("Coth", 1.0 / 0.5f64.tanh()),
        ("Sech", 1.0 / 0.5f64.cosh()),
        ("Csch", 1.0 / 0.5f64.sinh()),
        ("ArcSinh", 0.5f64.asinh()),
        ("ArcCosh", 2.0f64.acosh()),
        ("ArcTanh", 0.5f64.atanh()),
        ("Log", 0.5f64.ln()),
        ("Exp", 0.5f64.exp()),
        ("Sqrt", 0.5f64.sqrt()),
    ] {
        if !reference.is_finite() {
            continue;
        }
        let x = if name == "ArcCosh" { "2" } else { "1/2" };
        let expr = e(&format!("{name}[{x}]"));
        for precision in [Precision::Machine, Precision::Bits(167)] {
            let n = approximate(&expr, precision, &Interrupt::default())
                .unwrap()
                .unwrap();
            assert!(
                (n.to_f64().unwrap() - reference).abs() < 1e-15,
                "{name}: {n:?}"
            );
            assert_eq!(n.precision(), precision);
        }
    }
}
#[test]
fn pi_has_fifty_decimal_digits_and_results_stabilize_when_work_precision_doubles() {
    let pi = approximate(&e("Pi"), Precision::Bits(200), &Interrupt::default())
        .unwrap()
        .unwrap();
    let Number::Real(Real::Big(pi)) = pi else {
        panic!("big precision");
    };
    assert!(
        pi.to_decimal()
            .value()
            .to_string()
            .starts_with("3.14159265358979323846264338327950288419716939937510")
    );
    for src in ["1/3", "Sqrt[2]", "Sin[1/3]", "ArcSinh[1/3]", "Log[2]"] {
        let expr = e(src);
        let a = approximate(&expr, Precision::Bits(167), &Interrupt::default())
            .unwrap()
            .unwrap();
        let b = approximate(&expr, Precision::Bits(334), &Interrupt::default())
            .unwrap()
            .unwrap();
        let Number::Real(Real::Big(b)) = b else {
            panic!("big real");
        };
        assert_eq!(
            a,
            Number::Real(Real::Big(b.with_precision(167).value())),
            "{src}"
        );
    }
}
#[test]
fn complex_principal_branches_and_overflow_remain_finite() {
    for (src, re, im) in [
        ("Log[-2]", 2.0f64.ln(), std::f64::consts::PI),
        ("Sqrt[-4]", 0.0, 2.0),
        (
            "ArcSin[2]",
            std::f64::consts::FRAC_PI_2,
            -(2.0f64 + 3.0f64.sqrt()).ln(),
        ),
        (
            "ArcCosh[-2]",
            (2.0f64 + 3.0f64.sqrt()).ln(),
            std::f64::consts::PI,
        ),
        (
            "ArcTanh[2]",
            3.0f64.ln() / 2.0,
            -std::f64::consts::FRAC_PI_2,
        ),
    ] {
        let n = approximate(&e(src), Precision::Machine, &Interrupt::default())
            .unwrap()
            .unwrap();
        let (r, i) = n.to_complex_f64();
        assert!(
            (r - re).abs() < 1e-14 && (i - im).abs() < 1e-14,
            "{src}: {n:?}"
        );
    }
    assert!(matches!(
        approximate(&e("Exp[1000]"), Precision::Machine, &Interrupt::default()).unwrap(),
        Some(Number::Real(Real::Big(_)))
    ));
    assert!(
        evaluate(&e("x+1"), 128, &Interrupt::default())
            .unwrap()
            .is_none()
    );
    assert!(
        evaluate(&e("Log[0]"), 128, &Interrupt::default())
            .unwrap()
            .is_none()
    );
}
#[test]
fn nested_numeric_expression_evaluation_is_iterative_and_interruptible() {
    let expr = (0..1100).fold(Expr::int(1), |e, _| Expr::call(om_core::BUILTIN::SIN, [e]));
    assert!(
        evaluate(&expr, 128, &Interrupt::default())
            .unwrap()
            .is_some()
    );
    let ctx = Interrupt::default();
    ctx.steps_left.set(10);
    assert!(evaluate(&expr, 128, &ctx).is_err());
    assert!(
        evaluate(&e("Exp[10^100]"), 128, &Interrupt::default())
            .unwrap()
            .is_none()
    );
    assert!(
        approximate(
            &Expr::number(Number::Real(Real::Big(
                BigFloat::from(2).with_precision(100).value()
            ))),
            Precision::Machine,
            &Interrupt::default()
        )
        .unwrap()
        .is_some()
    );
}

#[test]
fn reciprocal_inverse_functions_axes_and_singularities() {
    for (src, re, im) in [
        ("ArcSec[2]", std::f64::consts::PI / 3.0, 0.0),
        ("ArcCsc[2]", std::f64::consts::PI / 6.0, 0.0),
        ("ArcSec[1/2]", 0.0, (2.0f64 + 3.0f64.sqrt()).ln()),
        (
            "ArcCsc[1/2]",
            std::f64::consts::FRAC_PI_2,
            -(2.0f64 + 3.0f64.sqrt()).ln(),
        ),
        ("ArcCot[-1]", -std::f64::consts::FRAC_PI_4, 0.0),
        ("ArcCot[0]", std::f64::consts::FRAC_PI_2, 0.0),
        ("ArcCosh[0]", 0.0, std::f64::consts::FRAC_PI_2),
        (
            "ArcSin[-2]",
            -std::f64::consts::FRAC_PI_2,
            (2.0f64 + 3.0f64.sqrt()).ln(),
        ),
        (
            "ArcTan[2 I]",
            std::f64::consts::FRAC_PI_2,
            3.0f64.ln() / 2.0,
        ),
        ("ArcTan[-1,1]", 3.0 * std::f64::consts::FRAC_PI_4, 0.0),
        ("ArcTan[0,-1]", -std::f64::consts::FRAC_PI_2, 0.0),
        ("Log[2,8]", 3.0, 0.0),
        ("Abs[3+4I]", 5.0, 0.0),
    ] {
        for p in [Precision::Machine, Precision::Bits(167)] {
            let n = approximate(&e(src), p, &Interrupt::default())
                .unwrap()
                .expect(src);
            let (r, i) = n.to_complex_f64();
            assert!(
                (r - re).abs() < 1e-14 && (i - im).abs() < 1e-14,
                "{src}: {n:?}"
            );
        }
    }
    for src in [
        "Csc[0]",
        "Cot[0]",
        "ArcTanh[1]",
        "Log[0]",
        "Log[1,2]",
        "Power[0,0]",
        "Sin[I*10^100]",
        "2^(10^100)",
    ] {
        assert!(
            evaluate(&e(src), 128, &Interrupt::default())
                .unwrap()
                .is_none(),
            "{src}"
        );
    }
}

#[test]
fn final_rounding_handles_dyadic_midpoints_and_input_precision_modes() {
    for (src, want) in [
        ("17/16", 1.0),
        ("19/16", 1.25),
        ("-17/16", -1.0),
        ("-19/16", -1.25),
    ] {
        let n = approximate(&e(src), Precision::Bits(4), &Interrupt::default())
            .unwrap()
            .unwrap();
        assert_eq!(n.to_f64().unwrap(), want);
    }
    assert!(
        approximate(&e("Pi"), Precision::Bits(0), &Interrupt::default())
            .unwrap()
            .is_none()
    );
    assert!(
        approximate(&e("Pi"), Precision::Bits(16_385), &Interrupt::default())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        approximate(&e("1/3"), Precision::Exact, &Interrupt::default()).unwrap(),
        e("1/3").as_number().cloned()
    );
}

#[test]
fn huge_binary_exponents_do_not_materialize_enormous_rationals() {
    // This value has a tiny significand but a multi-gigabit integer expansion.
    let huge = BigFloat::from_parts(om_num::Integer::ONE, 1_000_000_000)
        .with_precision(64)
        .value();
    let expr = Expr::number(Number::Real(Real::Big(huge.clone())));
    let n = approximate(&expr, Precision::Bits(64), &Interrupt::default())
        .unwrap()
        .unwrap();
    assert_eq!(n, Number::Real(Real::Big(huge)));
    for s in ["Sin[2^(10^9)]", "Exp[I*2^(10^9)]"] {
        assert!(
            evaluate(&e(s), 128, &Interrupt::default())
                .unwrap()
                .is_none(),
            "{s}"
        );
    }
}
#[test]
fn positive_integer_powers_enclose_bases_whose_balls_contain_zero() {
    let z = om_simplify::numeval::enclose(&e("Sin[Pi]^2"), 128, &Interrupt::default())
        .unwrap()
        .unwrap();
    assert!(z.re.contains_zero());
    assert!(z.re.rad.repr().is_finite() && z.im.rad.repr().is_finite());
    assert!(
        om_simplify::numeval::enclose(&e("1/Sin[Pi]"), 128, &Interrupt::default())
            .unwrap()
            .is_none()
    );
}
#[test]
fn certified_realness_keeps_formula_real_parts_and_resolves_principal_cuts() {
    let ctx = Interrupt::default();
    let base = e("(-1/2+I*Sqrt[3]/2)^(1/3)");
    let real = om_core::add([base.clone(), om_core::pow(base, Expr::int(-1))]);
    let source = om_poly::UPoly::new(vec![
        om_num::Integer::from(1),
        (-3).into(),
        0.into(),
        1.into(),
    ]);
    let value = om_poly::algebraic_root(&source, 3, &ctx).unwrap().unwrap();
    assert!(om_simplify::numeval::remember_real(&real, &value));
    assert!(!om_simplify::numeval::remember_real(&e("x"), &value));
    let imaginary = om_poly::algebraic_root(
        &om_poly::UPoly::new(vec![1.into(), 0.into(), 1.into()]),
        2,
        &ctx,
    )
    .unwrap()
    .unwrap();
    assert!(!om_simplify::numeval::remember_real(&e("I"), &imaginary));
    let expr = om_core::sqrt(om_core::sub(Expr::int(-1), real));
    let z = om_simplify::numeval::enclose(&expr, 256, &ctx)
        .unwrap()
        .unwrap();
    assert!(z.im.excludes_zero() && z.im.mid > BigFloat::ZERO);
    assert!(z.re.contains_zero());
    ctx.steps_left.set(0);
    assert!(matches!(
        om_simplify::numeval::enclose(&expr, 256, &ctx),
        Err(om_num::ctx::Abort::Budget)
    ));
}

#[test]
fn certified_real_sum_survives_rational_distribution_without_replacing_real_part() {
    let ctx = Interrupt::default();
    let base = e("(-1/2+I*Sqrt[3]/2)^(1/3)");
    let real = om_core::add([base.clone(), om_core::pow(base, Expr::int(-1))]);
    let before = om_simplify::numeval::enclose(&real, 256, &ctx)
        .unwrap()
        .unwrap();
    let source = om_poly::UPoly::new(vec![1.into(), (-3).into(), 0.into(), 1.into()]);
    let value = om_poly::algebraic_root(&source, 3, &ctx).unwrap().unwrap();
    assert!(om_simplify::numeval::remember_real(&real, &value));
    let after = om_simplify::numeval::enclose(&real, 256, &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(before.re.mid, after.re.mid);
    assert_eq!(before.re.rad, after.re.rad);
    assert_eq!(before.re.prec, after.re.prec);
    assert_eq!(after.im.mid, BigFloat::ZERO);
    assert_eq!(after.im.rad, BigFloat::ZERO);
    let distributed = om_core::sub(Expr::int(-1), real.clone());
    let principal = om_core::sqrt(distributed);
    ctx.steps_left.set(10_000);
    let z = om_simplify::numeval::enclose(&principal, 256, &ctx)
        .unwrap()
        .unwrap();
    assert!(z.im.mid > z.im.rad && z.re.contains_zero());
    // A tiny but exactly nonzero imaginary part is never projected away.
    let nonreal = om_core::add([real, e("I/2^300")]);
    let z = om_simplify::numeval::enclose(&nonreal, 1024, &Interrupt::default())
        .unwrap()
        .unwrap();
    assert!(z.im.excludes_zero());
}

#[test]
fn principal_product_log_has_certified_two_hundred_digit_real_residuals() {
    use om_num::Precision;
    use om_simplify::numeval::enclose;
    for argument in ["0", "1", "1/1000", "10", "10^100", "10^-100"] {
        let w = format!("ProductLog[{argument}]");
        let z = enclose(&e(&w), 700, &Interrupt::default())
            .unwrap()
            .expect(&w);
        assert_eq!(z.im.mid, BigFloat::ZERO);
        assert_eq!(z.im.rad, BigFloat::ZERO);
        let residual = e(&format!("({w} Exp[{w}]-({argument}))/(1+({argument}))"));
        let z = enclose(&residual, 700, &Interrupt::default())
            .unwrap()
            .unwrap();
        let limit = BigFloat::from_parts(1.into(), -400);
        assert!(
            z.re.contains_zero() && z.re.rad < limit,
            "{argument}: {z:?}"
        );
    }
    for precision in [Precision::Machine, Precision::Bits(200)] {
        let w = approximate(&e("ProductLog[1]"), precision, &Interrupt::default())
            .unwrap()
            .unwrap();
        assert!((w.to_f64().unwrap() - 0.5671432904097838).abs() < 1e-15);
        assert_eq!(w.precision(), precision);
    }
    for src in [
        "ProductLog[-1]",
        "ProductLog[I]",
        "ProductLog[1,1]",
        "ProductLog[x]",
    ] {
        assert!(
            enclose(&e(src), 128, &Interrupt::default())
                .unwrap()
                .is_none(),
            "{src}"
        );
    }
    let ctx = Interrupt::default();
    ctx.steps_left.set(20);
    assert!(enclose(&e("ProductLog[1]"), 700, &ctx).is_err());
}
