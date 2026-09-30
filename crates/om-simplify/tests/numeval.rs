//! Shared numeric expression evaluation, convergence and host budgets.
use om_core::{Expr, Interrupt, canonicalize};
use om_num::{BigFloat, Number, Precision, Real};
use om_parse::{Dialect, parse_expr};
use om_simplify::numeval::{approximate, evaluate};
fn e(src: &str) -> Expr {
    canonicalize(&parse_expr(src, Dialect::Wolfram).unwrap())
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
