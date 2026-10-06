//! InterpolationData is held pure data, not a public executable placeholder or live session handle.
use super::*;
use om_analysis::interpolation::{Interpolation, Method};
pub(super) fn method(m: Method) -> &'static str {
    match m {
        Method::Linear => "linear",
        Method::Hermite => "hermite",
        Method::DormandPrince => "dormand_prince_5_4",
    }
}
fn numbers(values: &[f64], ctx: &Interrupt) -> Result<Expr, EvalError> {
    Ok(list(
        values
            .iter()
            .map(|x| {
                ctx.tick()?;
                real(*x)
            })
            .collect::<Result<Vec<_>, EvalError>>()?,
    ))
}
pub(super) fn encode(
    value: &Interpolation,
    scalar: bool,
    slopes: &str,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    let times = numbers(value.times(), ctx)?;
    let mut rows = vec![];
    for row in value.values() {
        rows.push(numbers(row, ctx)?);
    }
    let mut coefficients = vec![];
    for segment in value.coefficients() {
        let mut row = vec![];
        for c in segment {
            row.push(numbers(c, ctx)?);
        }
        coefficients.push(list(row));
    }
    Ok(Expr::call(
        B::INTERPOLATION_DATA,
        [
            times,
            list(rows),
            list(coefficients),
            Expr::string(method(value.method())),
            Expr::sym(if scalar { B::TRUE } else { B::FALSE }),
            Expr::string(slopes),
        ],
    ))
}
fn vector(e: &Expr, ctx: &Interrupt) -> Result<Vec<f64>, EvalError> {
    if !e.is_head(B::LIST) || e.args().len() > 100000 {
        return Err(error("插值数据需要有限数列表"));
    }
    e.args()
        .iter()
        .map(|e| {
            ctx.tick()?;
            machine(e)
        })
        .collect()
}
pub(super) fn decode(object: &Expr, ctx: &Interrupt) -> Result<(Interpolation, bool), EvalError> {
    if !object.is_head(B::INTERPOLATION_DATA) || object.args().len() != 6 {
        return Err(error("插值对象形状无效"));
    }
    let a = object.args();
    let times = vector(&a[0], ctx)?;
    if !a[1].is_head(B::LIST)
        || a[1].args().len() != times.len()
        || !a[2].is_head(B::LIST)
        || a[2].args().len() != times.len().saturating_sub(1)
    {
        return Err(error("插值节点/段数不符"));
    }
    let d = a[1].args().first().map_or(0, |r| r.args().len());
    if !(1..=64).contains(&d) || times.len().saturating_mul(5 * d + 1) > 100000 {
        return Err(error("插值对象超出维度/标量限额"));
    }
    let scalar = match a[4].as_symbol() {
        Some(B::TRUE) if d == 1 => true,
        Some(B::FALSE) => false,
        _ => return Err(error("插值返回形状无效")),
    };
    let method = match string(&a[3])? {
        "linear" => Method::Linear,
        "hermite" => Method::Hermite,
        "dormand_prince_5_4" => Method::DormandPrince,
        _ => return Err(error("插值方法无效")),
    };
    let slope_source = string(&a[5])?;
    if !matches!(
        (method, slope_source),
        (Method::Linear, "none")
            | (Method::Hermite, "supplied" | "estimated")
            | (Method::DormandPrince, "rhs")
    ) {
        return Err(error("插值导数来源无效"));
    }
    let mut rows = vec![];
    for e in a[1].args() {
        if e.args().len() != d {
            return Err(error("插值节点维度不符"));
        }
        rows.push(vector(e, ctx)?);
    }
    let mut coefficients = vec![];
    for s in a[2].args() {
        if !s.is_head(B::LIST) || s.args().len() != d {
            return Err(error("插值段维度不符"));
        }
        let mut segment = vec![];
        for c in s.args() {
            segment.push(
                vector(c, ctx)?
                    .try_into()
                    .map_err(|_| error("插值系数需要四项"))?,
            );
        }
        coefficients.push(segment);
    }
    let data =
        Interpolation::new(times, rows, coefficients, method, ctx).map_err(analysis_failure)?;
    Ok((data, scalar))
}
pub(super) fn value(values: Vec<f64>, scalar: bool) -> Result<Expr, EvalError> {
    if scalar {
        real(values[0])
    } else {
        Ok(list(
            values
                .into_iter()
                .map(real)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
pub(super) fn call(object: &Expr, args: &[Expr], ctx: &Interrupt) -> Result<Expr, EvalError> {
    if args.len() != 1 {
        return Err(error("插值取值需要一个有限实数参数"));
    }
    let (data, scalar) = decode(object, ctx)?;
    value(
        data.evaluate(machine(&args[0])?, ctx)
            .map_err(analysis_failure)?,
        scalar,
    )
}
