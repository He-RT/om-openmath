//! Finite ordinary Taylor coefficients are genuine derivatives; no Laurent or fake zero extension.
use super::calculus_source as source;
use super::*;
use om_core::{Symbol, add, mul, pow};
use om_num::Integer;
fn uint(e: &Expr) -> Result<usize, EvalError> {
    if let Some(Number::Integer(n)) = e.as_number() {
        usize::try_from(n).map_err(|_| error("Taylor阶数需要非负整数"))
    } else {
        Err(error("Taylor阶数需要整数"))
    }
}
fn validate(e: &Expr, ctx: &Interrupt) -> Result<Symbol, EvalError> {
    ctx.tick()?;
    if !e.is_head(B::SERIES_DATA)
        || e.args().len() != 6
        || !e.args()[2].is_head(B::LIST)
        || e.args()[3] != Expr::int(0)
        || e.args()[5] != Expr::int(1)
    {
        return Err(error(
            "需要有效的普通Taylor SeriesData，不支持Laurent/分数幂系列",
        ));
    }
    let x = source::axis(&e.args()[0])?;
    let end = uint(&e.args()[4])?;
    if !(1..=65).contains(&end)
        || e.args()[2].args().len() != end
        || !e.args()[1].free_of(&Expr::sym(x))
    {
        return Err(error("Taylor截断或系数形状无效"));
    }
    for c in e.args()[2].args() {
        ctx.tick()?;
        if !c.free_of(&Expr::sym(x)) || !source::finite(c, ctx)? {
            return Err(error("Taylor系数不能含局部坐标或未求值导数"));
        }
    }
    Ok(x)
}
pub(super) fn dispatch(
    ev: &Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if !matches!(name, "Series" | "SeriesCoefficient" | "Normal") {
        return Ok(None);
    }
    let mut fork = ev.fork_readonly();
    if name == "Series" {
        let spec = args.values[1];
        let (axis, point, degree) = if spec.is_head(B::LIST) && spec.args().len() == 3 {
            if args.options.contains_key("At") || args.options.contains_key("Order") {
                return Err(error("Series的展开点/阶数不能重复提供"));
            }
            (
                &spec.args()[0],
                Some(&spec.args()[1]),
                Some(&spec.args()[2]),
            )
        } else {
            (
                spec,
                args.options.get("At").copied(),
                args.options.get("Order").copied(),
            )
        };
        let variable = source::axis(axis)?;
        let x = Expr::sym(variable);
        let center = if let Some(p) = point {
            fork.evaluate(p, ctx)?
        } else {
            Expr::int(0)
        };
        source::exact_source(&center, ctx)?;
        if !center.free_of(&x) || source::sign(&center, ctx)?.is_none() {
            return Err(error("Taylor展开点需要闭合有限精确实数"));
        }
        let order = if let Some(p) = degree {
            uint(&fork.evaluate(p, ctx)?)?
        } else {
            6
        };
        if order > 64 {
            return Err(error("Taylor首版阶数限0..64"));
        }
        let raw = fork.prepare_numeric(args.values[0], &[(variable, None)], ctx)?;
        source::exact_source(&raw, ctx)?;
        fork.scopes.push([(variable, None)].into_iter().collect());
        if !source::regular(&mut fork, &raw, &x, &center, ctx)? {
            return Err(error(
                "展开点处原式存在奇点/分支点或无法证明解析，不能伪装普通Taylor",
            ));
        }
        let mut derivative = fork.evaluate(&raw, ctx)?;
        let mut coefficients = vec![];
        let mut factorial = Integer::ONE;
        for n in 0..=order {
            ctx.tick()?;
            let coefficient = source::at(&mut fork, &derivative, &x, &center, ctx)?;
            if !source::finite(&coefficient, ctx)? || !coefficient.free_of(&x) {
                return Err(error("当前求导器不能得到该Taylor系数，保留原式"));
            }
            coefficients.push(om_core::div(coefficient, Expr::integer(factorial.clone())));
            if n < order {
                factorial *= Integer::from(n + 1);
                derivative = crate::algebra::differentiate(&derivative, &x, ctx)?;
            }
        }
        return Ok(Some(Expr::call(
            B::SERIES_DATA,
            [
                x,
                center,
                list(coefficients),
                Expr::int(0),
                Expr::int((order + 1) as i64),
                Expr::int(1),
            ],
        )));
    }
    let series = fork.evaluate(args.values[0], ctx)?;
    if name == "Normal" && !series.is_head(B::SERIES_DATA) {
        return Ok(Some(series));
    }
    let variable = validate(&series, ctx)?;
    fork.scopes.push([(variable, None)].into_iter().collect());
    if name == "SeriesCoefficient" {
        let n = uint(&fork.evaluate(args.values[1], ctx)?)?;
        if n >= series.args()[2].args().len() {
            return Err(error("请求阶数超出截断已知范围，不能把未知系数返回0"));
        }
        return Ok(Some(fork.evaluate(&series.args()[2].args()[n], ctx)?));
    }
    let x = om_core::sub(Expr::sym(variable), series.args()[1].clone());
    let mut terms = vec![];
    for (n, c) in series.args()[2].args().iter().enumerate() {
        ctx.tick()?;
        terms.push(mul([
            fork.evaluate(c, ctx)?,
            pow(x.clone(), Expr::int(n as i64)),
        ]));
    }
    Ok(Some(add(terms)))
}
