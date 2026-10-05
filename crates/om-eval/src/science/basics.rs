//! Numeric ranges, exact truncation and nominal precision; no invented high precision.
use super::*;
use om_num::{Integer, Number, Precision, Rational};
fn floor(q: &Rational) -> Integer {
    let d = Integer::from(q.denominator().clone());
    let mut n = q.numerator() / &d;
    if q.numerator() < &Integer::ZERO && q.numerator() % d != Integer::ZERO {
        n -= 1;
    }
    n
}
fn simplest(mut lo: Rational, mut hi: Rational, ctx: &Interrupt) -> Result<Rational, EvalError> {
    if lo <= Rational::ZERO && hi >= Rational::ZERO {
        return Ok(Rational::ZERO);
    }
    if hi < Rational::ZERO {
        return simplest(-hi, -lo, ctx).map(|value| -value);
    }
    let mut coefficients = vec![];
    for _ in 0..4096 {
        ctx.tick()?;
        let ceil = -floor(&(-&lo));
        if Rational::from(ceil.clone()) <= hi {
            let mut result = Rational::from(ceil);
            for n in coefficients.into_iter().rev() {
                ctx.tick()?;
                result = Rational::from(n) + Rational::ONE / result;
            }
            return Ok(result);
        }
        let n = floor(&lo);
        lo -= Rational::from(n.clone());
        hi -= Rational::from(n.clone());
        (lo, hi) = (Rational::ONE / hi, Rational::ONE / lo);
        coefficients.push(n);
    }
    Err(error("有理化未在预算内完成"))
}
fn extremes(values: &[&Expr], ctx: &Interrupt) -> Result<(Expr, Expr), EvalError> {
    let mut work = values.to_vec();
    let mut lo: Option<(Rational, Expr)> = None;
    let mut hi: Option<(Rational, Expr)> = None;
    while let Some(value) = work.pop() {
        ctx.tick()?;
        if value.is_head(B::LIST) {
            work.extend(value.args());
            continue;
        }
        let q = rational(value)?;
        if lo.as_ref().is_none_or(|a| q < a.0) {
            lo = Some((q.clone(), value.clone()));
        }
        if hi.as_ref().is_none_or(|a| q > a.0) {
            hi = Some((q, value.clone()));
        }
    }
    lo.zip(hi)
        .map(|(a, b)| (a.1, b.1))
        .ok_or_else(|| error("极值需要非空实数数据"))
}
fn chop_number(n: &Number, tolerance: f64) -> Number {
    match n {
        Number::Real(_) if n.to_f64().is_some_and(|v| v.abs() <= tolerance) => {
            Number::Integer(Integer::ZERO)
        }
        Number::Complex(c) => Number::Complex(Box::new(om_num::Complex {
            re: chop_number(&c.re, tolerance),
            im: chop_number(&c.im, tolerance),
        }))
        .normalize(),
        _ => n.clone(),
    }
}
fn chop(expr: &Expr, tolerance: f64, ctx: &Interrupt) -> Result<Expr, EvalError> {
    enum Task<'a> {
        Enter(&'a Expr),
        Build(usize),
    }
    let mut tasks = vec![Task::Enter(expr)];
    let mut values = vec![];
    while let Some(task) = tasks.pop() {
        ctx.tick()?;
        match task {
            Task::Enter(e) => match e.kind() {
                ExprKind::Number(n) => values.push(Expr::number(chop_number(n, tolerance))),
                ExprKind::Normal(n) => {
                    tasks.push(Task::Build(n.args.len()));
                    tasks.extend(n.args.iter().rev().map(Task::Enter));
                    tasks.push(Task::Enter(&n.head));
                }
                _ => values.push(e.clone()),
            },
            Task::Build(count) => {
                let args = values.split_off(values.len() - count);
                let head = values.pop().expect("invariant: chop emitted head");
                values.push(Expr::normal(head, args));
            }
        }
    }
    Ok(values.pop().expect("invariant: chop emitted root"))
}
pub(super) fn dispatch(
    _: &mut Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let result = match name {
        "Min" | "Max" | "MinMax" => {
            let (lo, hi) = extremes(&args.values, ctx)?;
            match name {
                "Min" => lo,
                "Max" => hi,
                _ => list([lo, hi]),
            }
        }
        "IntegerPart" | "FractionalPart" => {
            let q = rational(args.values[0])?;
            let integral = q.numerator() / Integer::from(q.denominator().clone());
            if name == "IntegerPart" {
                Expr::integer(integral)
            } else {
                Expr::number(number(args.values[0])?.add(&Number::Integer(integral).neg()))
            }
        }
        "Precision" | "Accuracy" => {
            let n = number(args.values[0])?;
            let digits = match n.precision() {
                Precision::Exact => return Ok(Some(Expr::sym(B::INFINITY))),
                Precision::Machine => 53.0 * std::f64::consts::LOG10_2,
                Precision::Bits(bits) => f64::from(bits) * std::f64::consts::LOG10_2,
            };
            if name == "Accuracy" {
                let value = n
                    .to_f64()
                    .filter(|v| v.is_finite() && *v != 0.0)
                    .ok_or_else(|| error("近似零或机器范围外的值缺少可推断的绝对尺度"))?;
                real(digits - value.abs().log10())?
            } else {
                real(digits)?
            }
        }
        "Rationalize" => {
            let q = rational(args.values[0])?;
            let tolerance = args
                .options
                .get("Tolerance")
                .map(|e| rational(e))
                .transpose()?
                .unwrap_or(Rational::ZERO);
            if tolerance < Rational::ZERO {
                return Err(error("tolerance须非负"));
            }
            let result = if tolerance == Rational::ZERO {
                q
            } else {
                simplest(&q - &tolerance, &q + &tolerance, ctx)?
            };
            Expr::number(Number::Rational(result).normalize())
        }
        "Chop" => {
            let tolerance = args
                .options
                .get("Tolerance")
                .map(|e| {
                    number(e)?
                        .to_f64()
                        .ok_or_else(|| error("tolerance需要机器范围实数"))
                })
                .transpose()?
                .unwrap_or(1e-10);
            if tolerance < 0.0 {
                return Err(error("tolerance须非负"));
            }
            chop(args.values[0], tolerance, ctx)?
        }
        "Decimal" => {
            let text = string(args.values[0])?;
            if text.len() > 20000 {
                return Err(error("十进制输入超过长度限制"));
            }
            let precision = args
                .options
                .get("WorkingPrecision")
                .map(|e| match e.as_number() {
                    Some(Number::Integer(n)) => u32::try_from(n)
                        .map(f64::from)
                        .map_err(|_| error("precision须为正整数")),
                    _ => Err(error("precision须为整数")),
                })
                .transpose()?
                .unwrap_or(50.0);
            if precision.fract() != 0.0 || !(1.0..=4931.0).contains(&precision) {
                return Err(error("precision须为1..4931十进制位数"));
            }
            let bits = (precision * std::f64::consts::LOG2_10).ceil() as usize;
            let (negative, text) = if let Some(t) = text.strip_prefix('-') {
                (true, t)
            } else {
                (false, text.strip_prefix('+').unwrap_or(text))
            };
            let (mantissa, exponent) = if let Some(i) = text.find(['e', 'E']) {
                (
                    &text[..i],
                    text[i + 1..]
                        .parse::<i32>()
                        .map_err(|_| error("无效十进制指数"))?,
                )
            } else {
                (text, 0)
            };
            let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
            if whole.len() + fraction.len() == 0
                || !whole
                    .bytes()
                    .chain(fraction.bytes())
                    .all(|c| c.is_ascii_digit())
            {
                return Err(error("需要纯十进制文字，不能包含代码或非有限值"));
            }
            let exponent = exponent
                .checked_sub(i32::try_from(fraction.len()).map_err(|_| error("输入过长"))?)
                .ok_or_else(|| error("十进制指数超出范围"))?;
            if exponent.unsigned_abs() > 20000 {
                return Err(error("十进制指数超过资源界限"));
            }
            let mut integer = format!("{whole}{fraction}")
                .parse::<Integer>()
                .map_err(|_| error("无效十进制文字"))?;
            if negative {
                integer = -integer;
            }
            let mut scale = Integer::ONE;
            for _ in 0..exponent.unsigned_abs() {
                ctx.tick()?;
                scale *= 10;
            }
            let rational = if exponent >= 0 {
                Rational::from(integer * scale)
            } else {
                Rational::from_parts(integer, scale.into_parts().1)
            };
            let value: om_num::BigFloat = rational.to_float(bits).value();
            Expr::number(Number::Real(om_num::Real::Big(value)))
        }
        "Rescale" => {
            let supplied = args.options.get("From").copied();
            let from = if let Some(from) = supplied {
                from.clone()
            } else {
                let (lo, hi) = extremes(&[args.values[0]], ctx)?;
                list([lo, hi])
            };
            let to = args
                .options
                .get("To")
                .map(|e| (*e).clone())
                .unwrap_or_else(|| list([Expr::int(0), Expr::int(1)]));
            if !(from.is_head(B::LIST) || from.is_head(B::SPAN))
                || from.args().len() != 2
                || !(to.is_head(B::LIST) || to.is_head(B::SPAN))
                || to.args().len() != 2
            {
                return Err(error("from/to需要两个端点"));
            }
            let (a, b, c, d) = (
                rational(&from.args()[0])?,
                rational(&from.args()[1])?,
                rational(&to.args()[0])?,
                rational(&to.args()[1])?,
            );
            if a == b {
                return Err(error("原区间宽度不能为0"));
            }
            let scale = Number::Rational((d - c) / (b - a.clone()));
            let offset = Number::Rational(a);
            let target = number(&to.args()[0])?;
            let transform = |value: &Expr| -> Result<Expr, EvalError> {
                ctx.tick()?;
                Ok(Expr::number(
                    number(value)?.add(&offset.neg()).mul(&scale).add(&target),
                ))
            };
            if args.values[0].is_head(B::LIST) {
                list(
                    args.values[0]
                        .args()
                        .iter()
                        .map(transform)
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                transform(args.values[0])?
            }
        }
        "Clip" => {
            let bounds = args
                .options
                .get("Bounds")
                .map(|e| vector(e, ctx))
                .transpose()?
                .unwrap_or(vec![
                    Number::Integer((-1).into()),
                    Number::Integer(1.into()),
                ]);
            if bounds.len() != 2 {
                return Err(error("bounds需要两个端点"));
            }
            let (lo, hi) = (
                crate::scalar::rational(&bounds[0]).ok_or_else(|| error("无效下界"))?,
                crate::scalar::rational(&bounds[1]).ok_or_else(|| error("无效上界"))?,
            );
            if lo > hi {
                return Err(error("bounds必须有序"));
            }
            let q = rational(args.values[0])?;
            if q < lo {
                Expr::number(bounds[0].clone())
            } else if q > hi {
                Expr::number(bounds[1].clone())
            } else {
                args.values[0].clone()
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
