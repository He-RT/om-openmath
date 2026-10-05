//! Numerical cancellation, contextual aliases, failure diagnostics and interrupts are real release contracts.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::{EvalError, Evaluator};
use om_parse::{Dialect, ParseEnv};
fn evaluate(ev: &mut Evaluator, source: &str, ctx: &Interrupt) -> Result<Expr, EvalError> {
    let parsed = om_parse::parse_with(
        source,
        Dialect::Modern,
        &ParseEnv {
            known_functions: ev.defs.known_functions(),
            ..Default::default()
        },
    );
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == om_parse::Severity::Error),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let mut value = Expr::sym(B::NULL);
    for statement in parsed.statements {
        value = ev.evaluate(&statement.expr, ctx)?;
    }
    Ok(value)
}
#[test]
fn machine_mean_preserves_small_terms_and_does_not_overflow_representable_average() {
    let mut ev = Evaluator::new();
    for source in [
        "mean([1e16,1.0,-1e16])",
        "mean([1e16,-1e16,1.0])",
        "mean([1.0,1e16,-1e16])",
    ] {
        let value = evaluate(&mut ev, source, &Interrupt::default()).unwrap();
        assert_eq!(
            value.as_number().unwrap().to_f64().unwrap(),
            1.0 / 3.0,
            "{source}"
        );
    }
    let value = evaluate(&mut ev, "mean([1e308,1e308])", &Interrupt::default()).unwrap();
    assert_eq!(value.as_number().unwrap().to_f64().unwrap(), 1e308);
}
#[test]
fn sorted_keys_and_quantiles_remain_stable_and_interrupts_recover() {
    let mut ev = Evaluator::new();
    let src = "sort_by([[1,30],[0,10],[1,20]],fn(x)=>at(x,1))";
    let value = evaluate(&mut ev, src, &Interrupt::default()).unwrap();
    assert_eq!(
        value,
        om_parse::parse_expr("{{0,10},{1,30},{1,20}}", Dialect::Wolfram).unwrap()
    );
    let data = (0..256).rev().map(Expr::int).collect::<Vec<_>>();
    for function in ["Sort", "Median"] {
        let expression = Expr::call(
            om_core::Symbol::intern(function),
            [Expr::call(B::LIST, data.clone())],
        );
        for budget in [1, 400, 1000, 2200] {
            let ctx = Interrupt::default();
            ctx.steps_left.set(budget);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                ev.evaluate(&expression, &ctx)
            }));
            assert!(result.is_ok(), "sorting must not panic on cancellation");
            assert!(
                matches!(result.unwrap(), Err(EvalError::Abort(_))),
                "budget={budget}"
            );
        }
    }
    assert_eq!(
        evaluate(&mut ev, "sort([3,1,2])", &Interrupt::default()).unwrap(),
        om_parse::parse_expr("{1,2,3}", Dialect::Wolfram).unwrap()
    );
}
#[test]
fn rationalize_chooses_zero_and_negative_symmetry_without_division_panics() {
    let mut ev = Evaluator::new();
    let zero = evaluate(
        &mut ev,
        "rationalize(0.1,tolerance:3)",
        &Interrupt::default(),
    )
    .unwrap();
    assert_eq!(zero, Expr::int(0));
    let positive = evaluate(
        &mut ev,
        "rationalize(3.2,tolerance:1.0)",
        &Interrupt::default(),
    )
    .unwrap();
    let negative = evaluate(
        &mut ev,
        "rationalize(-3.2,tolerance:1.0)",
        &Interrupt::default(),
    )
    .unwrap();
    assert_eq!(negative, om_core::neg(positive));
}
