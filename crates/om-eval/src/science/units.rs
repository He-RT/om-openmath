//! Scalar real quantities use exact unit scales and explicit dimensions; no hidden approximate conversion.
use super::unit_parse::{Unit, parse};
use super::*;
use om_core::Symbol;
use om_num::Integer;
fn is_quantity(e: &Expr) -> bool {
    e.head_symbol().is_some_and(|h| h.name() == "Quantity")
}
fn unpack(e: &Expr, ctx: &Interrupt) -> Result<(Number, Unit, String), EvalError> {
    if !is_quantity(e) || e.args().len() != 2 {
        return Err(error("需要有效的Quantity对象"));
    }
    let value = number(&e.args()[0])?;
    rational(&e.args()[0])?;
    let text = string(&e.args()[1])?.trim();
    let unit = parse(text, ctx)?;
    Ok((value, unit, text.to_owned()))
}
fn quantity(value: Number, text: &str) -> Expr {
    Expr::call(
        Symbol::intern("Quantity"),
        [Expr::number(value), Expr::string(text)],
    )
}
fn converted(e: &Expr, text: &str, ctx: &Interrupt) -> Result<Expr, EvalError> {
    let (value, from, _) = unpack(e, ctx)?;
    let target = parse(text, ctx)?;
    if from.dims != target.dims {
        return Err(error("单位换算的量纲不一致"));
    }
    let value = value.mul(&Number::Rational(from.scale / target.scale));
    rational(&Expr::number(value.clone()))?;
    Ok(quantity(value, text.trim()))
}
pub(super) fn dispatch(
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    Ok(Some(match name {
        "Quantity" => {
            if args.values.get(1).is_some() && args.options.contains_key("Unit") {
                return Err(error("单位不能同时使用位置和命名写法"));
            }
            let text = string(
                args.values
                    .get(1)
                    .copied()
                    .or_else(|| args.options.get("Unit").copied())
                    .ok_or_else(|| error("必须提供单位"))?,
            )?
            .trim();
            parse(text, ctx)?;
            if is_quantity(args.values[0]) {
                converted(args.values[0], text, ctx)?
            } else {
                let value = number(args.values[0])?;
                rational(args.values[0])?;
                quantity(value, text)
            }
        }
        "UnitConvert" => converted(args.values[0], string(args.values[1])?, ctx)?,
        "QuantityMagnitude" => Expr::number(unpack(args.values[0], ctx)?.0),
        "QuantityUnit" => Expr::string(&unpack(args.values[0], ctx)?.2),
        _ => return Ok(None),
    }))
}
pub(super) fn arithmetic(
    head: Symbol,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if !args.iter().any(is_quantity)
        || !matches!(
            head.name(),
            "Plus" | "Times" | "Power" | "Subtract" | "Divide" | "Minus"
        )
    {
        return Ok(None);
    }
    ctx.tick()?;
    if head.name() == "Minus" {
        let (n, _, text) = unpack(&args[0], ctx)?;
        return Ok(Some(quantity(n.neg(), &text)));
    }
    if head.name() == "Subtract" {
        let mut terms = args.to_vec();
        terms[1] = if is_quantity(&terms[1]) {
            let (n, _, text) = unpack(&terms[1], ctx)?;
            quantity(n.neg(), &text)
        } else {
            om_core::neg(terms[1].clone())
        };
        return arithmetic(B::PLUS, &terms, ctx);
    }
    if head.name() == "Divide" {
        let divisor = arithmetic(B::POWER, &[args[1].clone(), Expr::int(-1)], ctx)?
            .unwrap_or_else(|| om_core::pow(args[1].clone(), Expr::int(-1)));
        return arithmetic(B::TIMES, &[args[0].clone(), divisor], ctx);
    }
    if head == B::POWER {
        if args.len() != 2 || !is_quantity(&args[0]) {
            return Err(error("单位量不能作为指数"));
        }
        let Some(Number::Integer(n)) = args[1].as_number() else {
            return Err(error("单位量首版只支持整数乘方"));
        };
        let exponent = i32::try_from(n).map_err(|_| error("单位乘方指数超出范围"))?;
        let (n, unit, _) = unpack(&args[0], ctx)?;
        let power = unit.power(exponent, ctx)?;
        let value = n
            .pow_int(&Integer::from(exponent))
            .map_err(|e| error(&e.to_string()))?
            .mul(&Number::Rational(power.scale.clone()));
        rational(&Expr::number(value.clone()))?;
        return Ok(Some(quantity(value, &power.base_text())));
    }
    if head == B::PLUS {
        let first = args
            .iter()
            .find(|e| is_quantity(e))
            .ok_or_else(|| error("缺少单位量"))?;
        let (_, target, text) = unpack(first, ctx)?;
        let mut total = Number::Integer(Integer::ZERO);
        for e in args {
            ctx.tick()?;
            let value = if is_quantity(e) {
                let (value, unit, _) = unpack(e, ctx)?;
                if unit.dims != target.dims {
                    return Err(error("加减的量纲不一致"));
                }
                value.mul(&Number::Rational(unit.scale / &target.scale))
            } else {
                let value = number(e)?;
                if target.dims != [0; 7] && !value.is_zero() {
                    return Err(error("有量纲量不能与非零裸数相加"));
                }
                if target.dims == [0; 7] {
                    value.mul(&Number::Rational(Rational::ONE / &target.scale))
                } else {
                    value
                }
            };
            total = total.add(&value);
        }
        rational(&Expr::number(total.clone()))?;
        return Ok(Some(quantity(total, &text)));
    }
    let mut dims = Unit {
        dims: [0; 7],
        scale: Rational::ONE,
    };
    let mut value = Number::Integer(Integer::ONE);
    for e in args {
        ctx.tick()?;
        if is_quantity(e) {
            let (number, unit, _) = unpack(e, ctx)?;
            value = value.mul(&number);
            dims = dims.combine(&unit, false)?;
        } else {
            value = value.mul(&number(e)?);
        }
    }
    value = value.mul(&Number::Rational(dims.scale.clone()));
    rational(&Expr::number(value.clone()))?;
    Ok(Some(quantity(value, &dims.base_text())))
}
