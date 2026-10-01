//! Real stack programs agree with independent native f64 references.
use om_core::{Interrupt, Symbol};
use om_eval::numeric::{compile_f64, compile_f64_with_ctx};
fn src(s: &str) -> om_core::Expr {
    om_parse::parse_expr(s, om_parse::Dialect::Wolfram).unwrap()
}
#[test]
fn arithmetic_constants_and_closed_roots_compile_once_with_checked_dimensions() {
    let vars = [Symbol::intern("x"), Symbol::intern("y")];
    let f = compile_f64(&src("Sin[x]+x^2/(y+2)+Pi/3"), &vars).unwrap();
    for i in 0..200 {
        let x = i as f64 / 25.0 - 4.0;
        let y = i as f64 / 100.0;
        let expected = x.sin() + x * x / (y + 2.0) + std::f64::consts::PI / 3.0;
        assert!((f.eval(&[x, y]) - expected).abs() < 1e-13);
    }
    assert!(f.eval(&[1.0]).is_nan());
    assert!(f.eval(&[f64::NAN, 1.0]).is_nan());
    let root = compile_f64(&src("Root[#^2-2&,2]"), &[]).unwrap();
    assert!((root.eval(&[]) - 2.0_f64.sqrt()).abs() < 1e-14);
}
#[test]
fn common_real_functions_match_native_methods_and_branch_domains() {
    let x = Symbol::intern("x");
    for (name, reference) in [
        ("Sin", f64::sin as fn(f64) -> f64),
        ("Cos", f64::cos),
        ("Tan", f64::tan),
        ("Exp", f64::exp),
        ("Log", f64::ln),
        ("Sqrt", f64::sqrt),
        ("Abs", f64::abs),
        ("ArcSin", f64::asin),
        ("ArcCos", f64::acos),
        ("ArcTan", f64::atan),
        ("Sinh", f64::sinh),
        ("Cosh", f64::cosh),
        ("Tanh", f64::tanh),
        ("ArcSinh", f64::asinh),
        ("ArcCosh", f64::acosh),
        ("ArcTanh", f64::atanh),
        ("Floor", f64::floor),
        ("Ceiling", f64::ceil),
        ("Round", f64::round_ties_even),
    ] {
        let f = compile_f64(&src(&format!("{name}[x]")), &[x]).unwrap();
        for value in [-3.5, -0.5, 0.0, 0.5, 2.5] {
            let expected = reference(value);
            let actual = f.eval(&[value]);
            if expected.is_finite() {
                assert!(
                    (actual - expected).abs() < 1e-12,
                    "{name} {value}: {actual} {expected}"
                );
            } else {
                assert!(actual.is_nan());
            }
        }
    }
    let log = compile_f64(&src("Log[2,x]"), &[x]).unwrap();
    assert_eq!(log.eval(&[8.0]), 3.0);
    assert!(
        compile_f64(&src("1/x"), &[x])
            .unwrap()
            .eval(&[0.0])
            .is_nan()
    );
    assert!(
        compile_f64(&src("x^x"), &[x])
            .unwrap()
            .eval(&[0.0])
            .is_nan()
    );
    assert!(
        compile_f64(&src("I*x"), &[x])
            .unwrap()
            .eval(&[1.0])
            .is_nan()
    );
}
#[test]
fn unsupported_heads_unknown_axes_arity_and_interrupt_are_checked() {
    let x = Symbol::intern("x");
    for s in ["unknown[x]", "x+y", "Sin[x,x]", "x[1]", "Hold[x]"] {
        assert!(compile_f64(&src(s), &[x]).is_err(), "{s}");
    }
    assert!(compile_f64(&src("x"), &[x, x]).is_err());
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(compile_f64_with_ctx(&src("x+1"), &[x], &ctx).is_err());
}
#[test]
fn actual_readonly_preparation_masks_axes_resolves_functions_and_preserves_poles() {
    let mut ev = om_eval::Evaluator::new();
    let ctx = Interrupt::default();
    for s in ["x=99", "a=2", "f[t_]:=(t-1)/(t-1)+a"] {
        ev.evaluate_statement(&src(s), &ctx).unwrap();
    }
    let history = ev.history.clone();
    let x = Symbol::intern("x");
    let p = ev
        .prepare_numeric(
            &src("f[x]"),
            &[
                (x, None),
                (Symbol::intern("a"), Some(om_core::Expr::int(4))),
            ],
            &ctx,
        )
        .unwrap();
    let f = compile_f64(&p, &[x]).unwrap();
    assert_eq!(f.eval(&[0.0]), 5.0);
    assert!(f.eval(&[1.0]).is_nan());
    assert_eq!(ev.history, history);
    assert_eq!(
        ev.evaluate(&src("x+a"), &ctx).unwrap(),
        om_core::Expr::int(101)
    );
    assert!(ev.prepare_numeric(&src("a=9"), &[(x, None)], &ctx).is_err());
}

#[test]
fn runtime_instructions_share_real_budget_and_recover_reused_storage() {
    let f = compile_f64(&src("x^2+1"), &[Symbol::intern("x")]).unwrap();
    let mut work = vec![];
    let ctx = Interrupt::default();
    ctx.steps_left.set(2);
    assert!(f.eval_with_ctx(&[2.0], &mut work, &ctx).is_err());
    assert_eq!(
        f.eval_with_ctx(&[2.0], &mut work, &Interrupt::default())
            .unwrap(),
        5.0
    );
}

#[test]
fn compiler_respects_actual_round_ties_and_two_argument_real_domains() {
    let mut ev = om_eval::Evaluator::new();
    let x = Symbol::intern("x");
    let f = compile_f64(&src("Round[x]"), &[x]).unwrap();
    for value in [-3.5, -2.5, 2.5, 3.5] {
        let e = om_core::Expr::call(
            om_core::BUILTIN::ROUND,
            [om_core::Expr::number(om_num::Number::Real(
                om_num::Real::Machine(value),
            ))],
        );
        let expected = ev
            .evaluate(&e, &Interrupt::default())
            .unwrap()
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap();
        assert_eq!(f.eval(&[value]), expected);
    }
    let f = compile_f64(&src("Log[x,2]"), &[x]).unwrap();
    for value in [0.0, -1.0, 1.0] {
        assert!(f.eval(&[value]).is_nan());
    }
}
