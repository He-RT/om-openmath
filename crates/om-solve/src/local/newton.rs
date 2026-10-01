//! Newton proposals must satisfy a checked Armijo decrease before becoming iterates.
use super::{arithmetic as a, derivative};
use crate::{FindRootOptions, SolveError, StepSink};
use om_core::Expr;
use om_num::{Number, ctx::Interrupt};
pub(super) fn iterate(
    eqs: &[Expr],
    vars: &[Expr],
    mut values: Vec<Number>,
    opts: &FindRootOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Number>, SolveError> {
    let target = crate::numeric::bits(opts.precision)?;
    let bits = target + 32;
    let tolerance = a::power(-2 * i64::from(target + 8), bits);
    let jacobian = eqs
        .iter()
        .map(|e| {
            vars.iter()
                .map(|x| derivative::derivative(e, x, ctx))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let residuals = |v: &[Number]| {
        eqs.iter()
            .map(|e| a::eval(e, vars, v, bits, ctx))
            .collect::<Result<Vec<_>, SolveError>>()
    };
    for iteration in 0..=opts.max_iterations {
        ctx.tick()?;
        let residual = residuals(&values)?;
        let old_merit = a::merit(&residual);
        if old_merit.is_zero() {
            return Ok(values);
        }
        let mut matrix = vec![];
        for (i, e) in eqs.iter().enumerate() {
            let mut row = vec![];
            for (j, d) in jacobian[i].iter().enumerate() {
                let value = if let Some(d) = d {
                    a::eval(d, vars, &values, bits, ctx)?
                } else {
                    let magnitude = om_simplify::numeval::evaluate(
                        &om_core::func(
                            om_core::BUILTIN::ABS,
                            vec![Expr::number(values[j].clone())],
                        ),
                        bits,
                        ctx,
                    )?
                    .ok_or_else(|| {
                        SolveError::Unsupported("local difference scale unavailable".into())
                    })?;
                    let h = a::power(-i64::from(bits / 3), bits)
                        .mul(&magnitude.add(&Number::Integer(1.into())));
                    let mut plus = values.clone();
                    let mut minus = values.clone();
                    plus[j] = plus[j].add(&h);
                    minus[j] = a::sub(&minus[j], &h);
                    a::div(
                        &a::sub(
                            &a::eval(e, vars, &plus, bits, ctx)?,
                            &a::eval(e, vars, &minus, bits, ctx)?,
                        ),
                        &Number::Integer(2.into()).mul(&h),
                    )?
                };
                row.push(value);
            }
            matrix.push(row);
        }
        let direction = a::linear(matrix, residual.iter().map(Number::neg).collect(), ctx)?;
        if a::less(&old_merit, &tolerance) && a::less(&a::merit(&direction), &tolerance) {
            return Ok(values);
        }
        if iteration == opts.max_iterations {
            break;
        }
        let mut scale = Number::Integer(1.into());
        let mut chosen = None;
        for _ in 0..64 {
            ctx.tick()?;
            let next = values
                .iter()
                .zip(&direction)
                .map(|(v, d)| v.add(&scale.mul(d)))
                .collect::<Vec<_>>();
            match residuals(&next) {
                Ok(r) => {
                    let limit = old_merit.mul(&a::sub(
                        &Number::Integer(1.into()),
                        &scale.mul(&Number::Rational(
                            om_num::Rational::from(1) / om_num::Rational::from(10_000),
                        )),
                    ));
                    if a::less(&a::merit(&r), &limit) {
                        chosen = Some((next, r));
                        break;
                    }
                }
                Err(e @ SolveError::Abort(_)) => return Err(e),
                _ => {}
            }
            scale = scale.mul(&Number::Rational(
                om_num::Rational::from(1) / om_num::Rational::from(2),
            ));
        }
        let (next, residual) = chosen.ok_or_else(|| {
            SolveError::Unsupported("Newton line search could not reduce the residual".into())
        })?;
        super::record(eqs, vars, &values, &next, &residual, sink);
        values = next;
    }
    Err(SolveError::Unsupported(
        "local iteration limit reached".into(),
    ))
}
