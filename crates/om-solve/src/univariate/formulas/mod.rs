//! Exact low-degree formulas; higher formulas operate on depressed rational coefficients.
mod cubic;
mod quartic;
use super::{Candidate, extract};
use crate::{Formula, Level, SolveError, Step, StepKind, StepSink};
pub(super) use cubic::cardano;
use om_core::{Expr, add, div, mul, neg, pow, sqrt, sub};
use om_num::{Rational, ctx::Interrupt};
use om_simplify::{
    algebra::cancel_with,
    zero::{Tri, is_zero_with},
};
pub(super) use quartic::ferrari;
fn simplified(e: Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    let canceled = cancel_with(&e, &[], ctx)?
        .ok_or_else(|| SolveError::Unsupported("formula simplification failed".into()))?;
    // Generic formulas retain their canonical numerator/denominator presentation.
    // Cancel is still useful when it proves a numeric value or reduces a numeric radical.
    Ok(
        if canceled.as_number().is_some() || e.free_symbols().is_empty() {
            canceled
        } else {
            e
        },
    )
}
pub(super) fn formula(
    c: &[Expr],
    polynomial: &Expr,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    let (family, bindings, values) = match c.len() {
        2 => {
            let value = simplified(div(neg(c[0].clone()), c[1].clone()), ctx)?;
            (
                Formula::Linear,
                vec![("a".into(), c[1].clone()), ("b".into(), c[0].clone())],
                vec![(value, 1)],
            )
        }
        3 => {
            let (a, b, constant) = (&c[2], &c[1], &c[0]);
            let d = simplified(
                sub(
                    pow(b.clone(), Expr::int(2)),
                    mul([Expr::int(4), a.clone(), constant.clone()]),
                ),
                ctx,
            )?;
            let sign = extract::exact(&d).map(|q| {
                if q < Rational::ZERO {
                    crate::Sign::Negative
                } else if q == Rational::ZERO {
                    crate::Sign::Zero
                } else {
                    crate::Sign::Positive
                }
            });
            sink.record(|| {
                Step::new(
                    StepKind::Discriminant {
                        value: d.clone(),
                        sign,
                    },
                    vec![polynomial.clone()],
                    vec![d.clone()],
                    Level::Minor,
                )
            });
            let denominator = mul([Expr::int(2), a.clone()]);
            let values = if is_zero_with(&d, ctx)? == Tri::Zero {
                vec![(simplified(div(neg(b.clone()), denominator), ctx)?, 2)]
            } else if b.is_zero()
                && extract::exact(a).is_some()
                && extract::exact(constant).is_some()
            {
                // Taking the square root of the exact ratio retains the compact
                // reciprocal radical instead of expanding its denominator.
                let s = sqrt(div(neg(constant.clone()), a.clone()));
                vec![(neg(s.clone()), 1), (s, 1)]
            } else {
                let s = sqrt(d);
                vec![
                    (
                        simplified(
                            div(sub(neg(b.clone()), s.clone()), denominator.clone()),
                            ctx,
                        )?,
                        1,
                    ),
                    (
                        simplified(div(add([neg(b.clone()), s]), denominator), ctx)?,
                        1,
                    ),
                ]
            };
            (
                Formula::Quadratic,
                vec![
                    ("a".into(), a.clone()),
                    ("b".into(), b.clone()),
                    ("c".into(), constant.clone()),
                ],
                values,
            )
        }
        _ => return Ok(None),
    };
    sink.record(|| {
        Step::new(
            StepKind::ApplyFormula {
                formula: family,
                bindings,
                results: values.iter().map(|(e, _)| e.clone()).collect(),
            },
            vec![polynomial.clone()],
            values.iter().map(|(e, _)| e.clone()).collect(),
            Level::Major,
        )
    });
    Ok(Some(
        values
            .into_iter()
            .map(|(value, multiplicity)| Candidate {
                value,
                multiplicity,
                key: None,
            })
            .collect(),
    ))
}
