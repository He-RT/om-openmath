//! Exact sample statistics and documented type-7 quantiles; no cancellation-prone raw moments.
use super::*;
use om_num::{Integer, Number, Rational};
fn mean(values: &[Number], ctx: &Interrupt) -> Result<Number, EvalError> {
    if values.is_empty() {
        return Err(error("样本不能为空"));
    }
    let mut sum = Rational::ZERO;
    let mut precision = om_num::Precision::Exact;
    for value in values {
        ctx.tick()?;
        sum += crate::scalar::rational(value).ok_or_else(|| error("样本需要有限实数"))?;
        precision = match (precision, value.precision()) {
            (om_num::Precision::Machine, _) | (_, om_num::Precision::Machine) => {
                om_num::Precision::Machine
            }
            (om_num::Precision::Exact, p) | (p, om_num::Precision::Exact) => p,
            (om_num::Precision::Bits(a), om_num::Precision::Bits(b)) => {
                om_num::Precision::Bits(a.min(b))
            }
        };
    }
    let value = Number::Rational(sum / Rational::from(Integer::from(values.len()))).normalize();
    if precision == om_num::Precision::Exact {
        return Ok(value);
    }
    om_simplify::numeval::approximate(&Expr::number(value), precision, ctx)?
        .ok_or_else(|| error("均值超出所选数值表示范围"))
}

fn covariance(
    a: &[Number],
    b: &[Number],
    sample: bool,
    ctx: &Interrupt,
) -> Result<Number, EvalError> {
    if a.len() != b.len() || a.len() <= usize::from(sample) {
        return Err(error("样本长度须相同且分母须为正"));
    }
    let (ma, mb) = (mean(a, ctx)?, mean(b, ctx)?);
    let mut sum = Number::Integer(Integer::ZERO);
    for (x, y) in a.iter().zip(b) {
        ctx.tick()?;
        sum = sum.add(&x.add(&ma.neg()).mul(&y.add(&mb.neg())));
    }
    Ok(sum.mul(&Number::Rational(Rational::from_parts(
        Integer::ONE,
        Integer::from(a.len() - usize::from(sample)).into_parts().1,
    ))))
}
fn quantile(mut values: Vec<Number>, p: Rational, ctx: &Interrupt) -> Result<Number, EvalError> {
    if values.is_empty() || p < Rational::ZERO || p > Rational::ONE {
        return Err(error("分位数需要非空样本及0..1概率"));
    }
    let mut keyed = vec![];
    for value in values.drain(..) {
        ctx.tick()?;
        let q = crate::scalar::rational(&value).ok_or_else(|| error("样本不可比较"))?;
        keyed.push((q, value));
    }
    let order = super::ordering::indices(keyed.len(), ctx, |a, b| keyed[a].0.cmp(&keyed[b].0))?;
    let rank = p * Rational::from(Integer::from(keyed.len() - 1));
    let index = usize::try_from(rank.numerator() / Integer::from(rank.denominator().clone()))
        .map_err(|_| error("分位数位置溢出"))?;
    let fraction = rank - Rational::from(Integer::from(index));
    if index + 1 == keyed.len() {
        return Ok(keyed[order[index]].1.clone());
    }
    Ok(keyed[order[index]]
        .1
        .mul(&Number::Rational(Rational::ONE - &fraction))
        .add(&keyed[order[index + 1]].1.mul(&Number::Rational(fraction))))
}
pub(super) fn dispatch(
    ev: &mut Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let result = match name {
        "Mean" => Expr::number(mean(&vector(args.values[0], ctx)?, ctx)?),
        "Median" => Expr::number(quantile(
            vector(args.values[0], ctx)?,
            Rational::from_parts(Integer::ONE, 2u32.into()),
            ctx,
        )?),
        "Quantile" | "Percentile" => {
            if args.values.len() != 2 {
                return Err(error("需要样本和概率"));
            }
            let mut p = rational(args.values[1])?;
            if name == "Percentile" {
                p /= Rational::from(100);
            }
            Expr::number(quantile(vector(args.values[0], ctx)?, p, ctx)?)
        }
        "Variance" | "StandardDeviation" => {
            let values = vector(args.values[0], ctx)?;
            let sample = args.boolean("Sample", true)?;
            let variance = Expr::number(covariance(&values, &values, sample, ctx)?);
            if name == "StandardDeviation" {
                ev.evaluate(&om_core::sqrt(variance), ctx)?
            } else {
                variance
            }
        }
        "Covariance" | "Correlation" => {
            if args.values.len() != 2 {
                return Err(error("需要两份长度相同的样本"));
            }
            let a = vector(args.values[0], ctx)?;
            let b = vector(args.values[1], ctx)?;
            let sample = args.boolean("Sample", true)?;
            let cov = covariance(&a, &b, sample, ctx)?;
            if name == "Covariance" {
                Expr::number(cov)
            } else {
                let va = covariance(&a, &a, sample, ctx)?;
                let vb = covariance(&b, &b, sample, ctx)?;
                if va.is_zero() || vb.is_zero() {
                    return Err(error("零方差样本的相关系数未定义"));
                }
                ev.evaluate(
                    &om_core::div(Expr::number(cov), om_core::sqrt(Expr::number(va.mul(&vb)))),
                    ctx,
                )?
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
