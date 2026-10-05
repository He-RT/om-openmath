//! Shared exact coordinate scopes, substitutions and local analyticity checks; no numeric probing.
use super::*;
use om_core::Symbol;
use om_num::{BigFloat, Integer};
pub(super) fn axis(e: &Expr) -> Result<Symbol, EvalError> {
    e.as_symbol()
        .filter(|s| {
            !om_core::builtins::names().contains(&s.name())
                && om_core::catalog::by_runtime(s.name()).is_none()
        })
        .ok_or_else(|| error("微积分坐标必须是用户符号"))
}
pub(super) fn at(
    ev: &mut Evaluator,
    e: &Expr,
    x: &Expr,
    p: &Expr,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    ev.evaluate(&super::symbolic_integration::substitute(e, x, p, ctx)?, ctx)
}
pub(super) fn exact_source(e: &Expr, ctx: &Interrupt) -> Result<(), EvalError> {
    let mut pending = vec![e];
    while let Some(e) = pending.pop() {
        ctx.tick()?;
        if e.as_number().is_some_and(|n| !n.is_exact()) {
            return Err(error(
                "当前精确极限/Taylor要求精确数值输入，不把近似系数填成假精度",
            ));
        }
        pending.extend(e.args());
    }
    Ok(())
}
pub(super) fn finite(e: &Expr, ctx: &Interrupt) -> Result<bool, EvalError> {
    let mut pending = vec![e.clone()];
    while let Some(e) = pending.pop() {
        ctx.tick()?;
        if matches!(
            e.as_symbol(),
            Some(
                B::INDETERMINATE | B::INFINITY | B::COMPLEX_INFINITY | B::TRUE | B::FALSE | B::NULL
            )
        ) || matches!(
            e.head_symbol(),
            Some(B::LIST | B::RECORD | B::DATA_TABLE | B::SERIES_DATA)
        ) || e.is_head(B::DIRECTED_INFINITY)
            || e.head_symbol()
                .is_some_and(|h| matches!(h.name(), "Derivative" | "D"))
            || e.as_symbol().is_some_and(|s| s.name() == "Undefined")
        {
            return Ok(false);
        }
        pending.extend(e.args().iter().cloned());
        if e.head_symbol().is_none() && !e.args().is_empty() {
            pending.push(e.head());
        }
    }
    Ok(true)
}
pub(super) fn sign(e: &Expr, ctx: &Interrupt) -> Result<Option<i32>, EvalError> {
    if e.is_zero() {
        return Ok(Some(0));
    }
    if let Some(q) = e.as_number().and_then(crate::scalar::rational) {
        return Ok(Some(if q < Rational::ZERO { -1 } else { 1 }));
    }
    let Some(v) = om_simplify::numeval::enclose(e, 128, ctx)? else {
        return Ok(None);
    };
    if v.im.mid != BigFloat::ZERO || v.im.rad != BigFloat::ZERO {
        return Ok(None);
    }
    Ok(if v.re.mid > v.re.rad {
        Some(1)
    } else if -&v.re.mid > v.re.rad {
        Some(-1)
    } else if v.re.mid == BigFloat::ZERO && v.re.rad == BigFloat::ZERO {
        Some(0)
    } else {
        None
    })
}
fn nonzero(e: &Expr, ctx: &Interrupt) -> Result<bool, EvalError> {
    if let Some(n) = e.as_number() {
        return Ok(n.is_exact() && !n.is_zero());
    }
    let Some(v) = om_simplify::numeval::enclose(e, 128, ctx)? else {
        return Ok(false);
    };
    Ok(v.re.mid > v.re.rad || -v.re.mid > v.re.rad || v.im.mid > v.im.rad || -v.im.mid > v.im.rad)
}
/// Sufficient local analyticity on the supported real center; raw poles and branch points are retained.
pub(super) fn regular(
    ev: &mut Evaluator,
    e: &Expr,
    x: &Expr,
    p: &Expr,
    ctx: &Interrupt,
) -> Result<bool, EvalError> {
    let mut pending = vec![e];
    while let Some(e) = pending.pop() {
        ctx.tick()?;
        if matches!(e.head_symbol(), Some(B::LIST | B::RECORD | B::DATA_TABLE)) {
            return Ok(false);
        }
        if e.free_of(x) {
            if !finite(&at(ev, e, x, p, ctx)?, ctx)? {
                return Ok(false);
            }
            continue;
        }
        if e == x {
            continue;
        }
        let Some(h) = e.head_symbol() else {
            return Ok(false);
        };
        match h.name() {
            "Plus" | "Times" => {}
            "Power" if e.args().len() == 2 => {
                let base = at(ev, &e.args()[0], x, p, ctx)?;
                match e.args()[1].as_number() {
                    Some(Number::Integer(n)) if *n > Integer::ZERO => {}
                    Some(Number::Integer(_)) => {
                        if !nonzero(&base, ctx)? {
                            return Ok(false);
                        }
                    }
                    _ => {
                        if sign(&base, ctx)? != Some(1) {
                            return Ok(false);
                        }
                    }
                }
            }
            "Exp" | "Sin" | "Cos" | "Sinh" | "Cosh" | "Erf" | "Erfc" if e.args().len() == 1 => {}
            "Log" if e.args().len() == 1 => {
                if sign(&at(ev, &e.args()[0], x, p, ctx)?, ctx)? != Some(1) {
                    return Ok(false);
                }
            }
            "Sqrt" if e.args().len() == 1 => {
                if sign(&at(ev, &e.args()[0], x, p, ctx)?, ctx)? != Some(1) {
                    return Ok(false);
                }
            }
            "Tan" | "Sec" if e.args().len() == 1 => {
                let arg = at(ev, &e.args()[0], x, p, ctx)?;
                if !nonzero(&ev.evaluate(&Expr::call(B::COS, [arg]), ctx)?, ctx)? {
                    return Ok(false);
                }
            }
            "Cot" | "Csc" if e.args().len() == 1 => {
                let arg = at(ev, &e.args()[0], x, p, ctx)?;
                if !nonzero(&ev.evaluate(&Expr::call(B::SIN, [arg]), ctx)?, ctx)? {
                    return Ok(false);
                }
            }
            "Tanh" | "Sech" if e.args().len() == 1 => {}
            "Coth" | "Csch" if e.args().len() == 1 => {
                if sign(&at(ev, &e.args()[0], x, p, ctx)?, ctx)?.is_none_or(|s| s == 0) {
                    return Ok(false);
                }
            }
            "ArcTan" | "ArcSinh" if e.args().len() == 1 => {}
            "ArcSin" | "ArcCos" | "ArcTanh" if e.args().len() == 1 => {
                let a = at(ev, &e.args()[0], x, p, ctx)?;
                if sign(
                    &om_core::sub(Expr::int(1), om_core::pow(a, Expr::int(2))),
                    ctx,
                )? != Some(1)
                {
                    return Ok(false);
                }
            }
            "ArcCosh" if e.args().len() == 1 => {
                if sign(
                    &om_core::sub(at(ev, &e.args()[0], x, p, ctx)?, Expr::int(1)),
                    ctx,
                )? != Some(1)
                {
                    return Ok(false);
                }
            }
            "CubeRoot" | "ArcCsch" if e.args().len() == 1 => {
                if sign(&at(ev, &e.args()[0], x, p, ctx)?, ctx)?.is_none_or(|s| s == 0) {
                    return Ok(false);
                }
            }
            "ArcCoth" if e.args().len() == 1 => {
                if sign(
                    &om_core::sub(
                        om_core::pow(at(ev, &e.args()[0], x, p, ctx)?, Expr::int(2)),
                        Expr::int(1),
                    ),
                    ctx,
                )? != Some(1)
                {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }
        pending.extend(e.args());
    }
    finite(&at(ev, e, x, p, ctx)?, ctx)
}
