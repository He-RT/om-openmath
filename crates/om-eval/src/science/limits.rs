//! Real-side limits use exact rational orders and analyticity/L'Hopital certificates, never nearby samples.
use super::calculus_source as source;
use super::*;
use om_core::{Symbol, add, div, mul, pow};
fn inf(sign: i64) -> Expr {
    Expr::call(B::DIRECTED_INFINITY, [Expr::int(sign)])
}
fn infinity(e: &Expr) -> Option<i32> {
    if e.is_head(B::DIRECTED_INFINITY) && e.args().len() == 1 {
        if e.args()[0] == Expr::int(1) {
            Some(1)
        } else if e.args()[0] == Expr::int(-1) {
            Some(-1)
        } else {
            None
        }
    } else {
        None
    }
}
fn zero(ev: &mut Evaluator, e: &Expr, ctx: &Interrupt) -> Result<Option<bool>, EvalError> {
    let e = ev.evaluate(
        &Expr::call(Symbol::intern("FullSimplify"), [e.clone()]),
        ctx,
    )?;
    if e.is_zero() {
        return Ok(Some(true));
    }
    if let Some(n) = e.as_number() {
        return Ok(Some(n.is_zero()));
    }
    Ok(source::sign(&e, ctx)?.map(|s| s == 0))
}
fn fraction(e: &Expr, ctx: &Interrupt, depth: usize) -> Result<(Expr, Expr), EvalError> {
    ctx.tick()?;
    if depth > 64 {
        return Err(error("极限通分超过64层界限"));
    }
    if e.is_head(B::TIMES) {
        let (mut n, mut d) = (vec![], vec![]);
        for e in e.args() {
            let (a, b) = fraction(e, ctx, depth + 1)?;
            n.push(a);
            d.push(b);
        }
        return Ok((mul(n), mul(d)));
    }
    if e.is_head(B::PLUS) {
        let (mut n, mut d) = (Expr::int(0), Expr::int(1));
        for e in e.args() {
            let (a, b) = fraction(e, ctx, depth + 1)?;
            n = add([mul([n, b.clone()]), mul([a, d.clone()])]);
            d = mul([d, b]);
        }
        return Ok((n, d));
    }
    if e.is_head(B::POWER)
        && e.args().len() == 2
        && let Some(Number::Integer(n)) = e.args()[1].as_number()
        && let Ok(n) = i32::try_from(n)
        && (-64..=64).contains(&n)
    {
        let (a, b) = fraction(&e.args()[0], ctx, depth + 1)?;
        let power = Expr::int(i64::from(n.unsigned_abs()));
        return Ok(if n < 0 {
            (pow(b, power.clone()), pow(a, power))
        } else {
            (pow(a, power.clone()), pow(b, power))
        });
    }
    Ok((e.clone(), Expr::int(1)))
}
fn leading(
    ev: &mut Evaluator,
    e: &Expr,
    x: &Expr,
    p: &Expr,
    ctx: &Interrupt,
) -> Result<Option<(usize, Expr)>, EvalError> {
    if !source::regular(ev, e, x, p, ctx)? {
        return Ok(None);
    }
    let mut d = e.clone();
    for n in 0..=64 {
        ctx.tick()?;
        let value = source::at(ev, &d, x, p, ctx)?;
        if !source::finite(&value, ctx)? {
            return Ok(None);
        }
        match zero(ev, &value, ctx)? {
            Some(false) => return Ok(Some((n, value))),
            Some(true) => {}
            None => return Ok(None),
        }
        if n < 64 {
            d = crate::algebra::differentiate(&d, x, ctx)?;
        }
    }
    Ok(None)
}
fn quotient(
    ev: &mut Evaluator,
    e: &Expr,
    x: &Expr,
    p: &Expr,
    side: i32,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let (n, d) = fraction(e, ctx, 0)?;
    if d == Expr::int(1) {
        return Ok(None);
    }
    if !source::regular(ev, &n, x, p, ctx)? || !source::regular(ev, &d, x, p, ctx)? {
        return Ok(None);
    }
    let Some((j, b)) = leading(ev, &d, x, p, ctx)? else {
        return Ok(None);
    };
    if n.is_zero() {
        return Ok(Some(Expr::int(0)));
    }
    let Some((i, a)) = leading(ev, &n, x, p, ctx)? else {
        return Ok(None);
    };
    if i > j {
        return Ok(Some(Expr::int(0)));
    }
    if i == j {
        return Ok(Some(ev.evaluate(&div(a, b), ctx)?));
    }
    let ratio = ev.evaluate(&div(a, b), ctx)?;
    let Some(sign) = source::sign(&ratio, ctx)? else {
        return Ok(None);
    };
    if sign == 0 {
        return Ok(None);
    }
    Ok(Some(inf(i64::from(
        sign * if side < 0 && (j - i) % 2 == 1 { -1 } else { 1 },
    ))))
}
fn real_argument(e: &Expr, x: &Expr, ctx: &Interrupt, depth: usize) -> Result<bool, EvalError> {
    ctx.tick()?;
    if depth > 32 {
        return Ok(false);
    }
    if e.free_of(x) {
        return Ok(
            om_simplify::numeval::enclose(e, 128, ctx)?.is_some_and(|v| {
                v.im.mid == om_num::BigFloat::ZERO && v.im.rad == om_num::BigFloat::ZERO
            }),
        );
    }
    if om_simplify::convert::to_rational_function_with(e, std::slice::from_ref(x), ctx)?
        .is_some_and(|v| v.gens == [x.clone()] && !v.den.is_zero())
    {
        return Ok(true);
    }
    if matches!(
        e.head_symbol().map(|s| s.name()),
        Some("Exp" | "Sin" | "Cos" | "Sinh" | "Cosh" | "Tanh")
    ) && e.args().len() == 1
    {
        return real_argument(&e.args()[0], x, ctx, depth + 1);
    }
    Ok(false)
}
fn bounded(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<bool, EvalError> {
    ctx.tick()?;
    if e.free_of(x) {
        return super::calculus_source::finite(e, ctx);
    }
    if matches!(
        e.head_symbol().map(|s| s.name()),
        Some("Sin" | "Cos" | "Tanh" | "Erf" | "Erfc")
    ) && e.args().len() == 1
    {
        return real_argument(&e.args()[0], x, ctx, 0);
    }
    if e.is_head(B::POWER)
        && e.args().len() == 2
        && let Some(Number::Integer(n)) = e.args()[1].as_number()
        && u32::try_from(n).is_ok()
    {
        return bounded(&e.args()[0], x, ctx);
    }
    Ok(false)
}
fn invert_infinity(
    ev: &mut Evaluator,
    e: &Expr,
    x: &Expr,
    p: &Expr,
    ctx: &Interrupt,
    depth: usize,
) -> Result<Option<Expr>, EvalError> {
    let Some(direction) = infinity(p) else {
        return Ok(None);
    };
    let mut id = 0;
    let used = e.free_symbols();
    let variable = loop {
        ctx.tick()?;
        let s = Symbol::intern(&format!("$om$limit${id}"));
        id += 1;
        if !used.contains(&s) && ev.own(s).is_none() && ev.defs.down_values(s).is_empty() {
            break s;
        }
    };
    let y = Expr::sym(variable);
    let transformed = super::symbolic_integration::substitute(
        e,
        x,
        &Expr::call(B::POWER, [y.clone(), Expr::int(-1)]),
        ctx,
    )?;
    let previous = ev.scopes.len();
    ev.scopes.push([(variable, None)].into_iter().collect());
    let result = side(
        ev,
        &transformed,
        &y,
        &Expr::int(0),
        direction,
        ctx,
        depth + 1,
    );
    ev.scopes.truncate(previous);
    result
}
fn side(
    ev: &mut Evaluator,
    e: &Expr,
    x: &Expr,
    p: &Expr,
    direction: i32,
    ctx: &Interrupt,
    depth: usize,
) -> Result<Option<Expr>, EvalError> {
    ctx.tick()?;
    if depth > 32 {
        return Ok(None);
    }
    if e.is_head(B::SQRT) && e.args().len() == 1 {
        return side(
            ev,
            &Expr::call(B::POWER, [e.args()[0].clone(), Expr::rational(1, 2)]),
            x,
            p,
            direction,
            ctx,
            depth + 1,
        );
    }
    if e.free_of(x) {
        let value = ev.evaluate(e, ctx)?;
        return Ok((source::finite(&value, ctx)? || infinity(&value).is_some()).then_some(value));
    }
    if let Some(value) = super::limit_rational::apply(e, x, p, direction, ctx)? {
        return Ok(Some(value));
    }
    let point_inf = infinity(p);
    if point_inf.is_none() && source::regular(ev, e, x, p, ctx)? {
        return Ok(Some(source::at(ev, e, x, p, ctx)?));
    }
    if point_inf.is_none()
        && let Some(value) = quotient(ev, e, x, p, direction, ctx)?
    {
        return Ok(Some(value));
    }
    let (head, arg) =
        if e.is_head(B::POWER) && e.args().len() == 2 && e.args()[0] == Expr::sym(B::E) {
            ("Exp", Some(&e.args()[1]))
        } else {
            (
                e.head_symbol().map_or("", |s| s.name()),
                e.args().first().filter(|_| e.args().len() == 1),
            )
        };
    if let Some(arg) = arg {
        let Some(value) = side(ev, arg, x, p, direction, ctx, depth + 1)? else {
            return Ok(None);
        };
        if let Some(sign) = infinity(&value) {
            return Ok(match head {
                "Exp" => Some(if sign < 0 { Expr::int(0) } else { inf(1) }),
                "Log" if sign > 0 => Some(inf(1)),
                "ArcTan" => Some(mul([Expr::rational(i64::from(sign), 2), Expr::sym(B::PI)])),
                "Tanh" | "Erf" => Some(Expr::int(i64::from(sign))),
                "Erfc" => Some(Expr::int(if sign > 0 { 0 } else { 2 })),
                "Sinh" => Some(inf(i64::from(sign))),
                "Cosh" => Some(inf(1)),
                "CubeRoot" => Some(inf(i64::from(sign))),
                _ => None,
            });
        }
        if head == "Log" && value.is_zero() {
            if let Some((order, coefficient)) = leading(ev, arg, x, p, ctx)?
                && let Some(sign) = source::sign(&coefficient, ctx)?
            {
                let sign = sign
                    * if direction < 0 && order % 2 == 1 {
                        -1
                    } else {
                        1
                    };
                if sign > 0 {
                    return Ok(Some(inf(-1)));
                }
            }
            return Ok(None);
        }
        if head == "Log" && source::sign(&value, ctx)? == Some(1) {
            return Ok(Some(ev.evaluate(&Expr::call(B::LOG, [value]), ctx)?));
        }
        if head == "CubeRoot" && source::sign(&value, ctx)?.is_some() {
            return Ok(Some(ev.evaluate(
                &Expr::call(Symbol::intern("CubeRoot"), [value]),
                ctx,
            )?));
        }
        if matches!(
            head,
            "Exp" | "Sin" | "Cos" | "Sinh" | "Cosh" | "Tanh" | "Erf" | "Erfc"
        ) && source::finite(&value, ctx)?
        {
            return Ok(Some(
                ev.evaluate(&Expr::call(Symbol::intern(head), [value]), ctx)?,
            ));
        }
    }
    if e.is_head(B::POWER)
        && e.args().len() == 2
        && !e.args()[1].free_of(x)
        && let Some(base) = side(ev, &e.args()[0], x, p, direction, ctx, depth + 1)?
        && source::sign(&base, ctx)? == Some(1)
        && real_argument(&e.args()[0], x, ctx, 0)?
    {
        let transformed = Expr::call(
            B::EXP,
            [Expr::call(
                B::TIMES,
                [
                    e.args()[1].clone(),
                    Expr::call(B::LOG, [e.args()[0].clone()]),
                ],
            )],
        );
        return side(ev, &transformed, x, p, direction, ctx, depth + 1);
    }
    if e.is_head(B::POWER)
        && e.args().len() == 2
        && let Some(q) = e.args()[1].as_number().and_then(crate::scalar::rational)
        && let Some(value) = side(ev, &e.args()[0], x, p, direction, ctx, depth + 1)?
    {
        if let Some(s) = infinity(&value) {
            if q > Rational::ZERO && (s > 0 || q.denominator() == &1u32.into()) {
                let odd = q.numerator() % 2u32 == 1;
                return Ok(Some(inf(if s < 0 && odd { -1 } else { 1 })));
            }
            if q < Rational::ZERO && (s > 0 || q.denominator() == &1u32.into()) {
                return Ok(Some(Expr::int(0)));
            }
        }
        if value.is_zero()
            && point_inf.is_none()
            && let Some((order, c)) = leading(ev, &e.args()[0], x, p, ctx)?
            && let Some(sign) = source::sign(&c, ctx)?
        {
            let sign = sign
                * if direction < 0 && order % 2 == 1 {
                    -1
                } else {
                    1
                };
            if sign > 0 || q.denominator() == &1u32.into() {
                if q > Rational::ZERO {
                    return Ok(Some(Expr::int(0)));
                }
                if q < Rational::ZERO {
                    return Ok(Some(inf(if sign < 0 && q.numerator() % 2u32 != 0 {
                        -1
                    } else {
                        1
                    })));
                }
            }
        }
    }
    if e.is_head(B::PLUS) || e.is_head(B::TIMES) {
        let mut values = vec![];
        let mut failed = false;
        for term in e.args() {
            if let Some(value) = side(ev, term, x, p, direction, ctx, depth + 1)? {
                values.push(value);
            } else if e.is_head(B::TIMES) && bounded(term, x, ctx)? {
                failed = true;
            } else {
                return Ok(None);
            }
        }
        if e.is_head(B::TIMES) && failed && values.iter().any(Expr::is_zero) {
            let mut finite = true;
            for e in &values {
                ctx.tick()?;
                finite &= source::finite(e, ctx)?;
            }
            if finite {
                return Ok(Some(Expr::int(0)));
            }
        }
        if failed {
            return Ok(None);
        }
        let value = if e.is_head(B::PLUS) {
            add(values)
        } else {
            mul(values)
        };
        if source::finite(&value, ctx)? || infinity(&value).is_some() {
            return Ok(Some(value));
        }
        if point_inf.is_some() {
            return invert_infinity(ev, e, x, p, ctx, depth);
        }
        return Ok(None);
    }
    Ok(None)
}
pub(super) fn dispatch(
    ev: &Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if name != "Limit" {
        return Ok(None);
    }
    let mut fork = ev.fork_readonly();
    let axis = args.values[1];
    let (variable, point) = if axis.is_head(B::RULE) && axis.args().len() == 2 {
        if args.options.contains_key("At") {
            return Err(error("极限点不能重复提供"));
        }
        (
            source::axis(&axis.args()[0])?,
            fork.evaluate(&axis.args()[1], ctx)?,
        )
    } else {
        (
            source::axis(axis)?,
            fork.evaluate(
                args.options
                    .get("At")
                    .ok_or_else(|| error("极限必须显式提供at点"))?,
                ctx,
            )?,
        )
    };
    let x = Expr::sym(variable);
    let raw = fork.prepare_numeric(args.values[0], &[(variable, None)], ctx)?;
    source::exact_source(&raw, ctx)?;
    source::exact_source(&point, ctx)?;
    if infinity(&point).is_none() && source::sign(&point, ctx)?.is_none() {
        return Err(error("极限点需要闭合有限精确实数或±inf"));
    }
    let direction = args
        .options
        .get("Direction")
        .map(|e| fork.evaluate(e, ctx))
        .transpose()?;
    let direction = direction
        .as_ref()
        .map(|e| string(e))
        .transpose()?
        .unwrap_or("both");
    if !matches!(direction, "both" | "left" | "right") {
        return Err(error("极限direction仅支持both/left/right"));
    }
    fork.scopes.push([(variable, None)].into_iter().collect());
    if infinity(&point).is_some() {
        if direction != "both" {
            return Err(error("无限点由at:±inf指定方向，不接受有限点的左右参数"));
        }
        return Ok(Some(
            side(&mut fork, &raw, &x, &point, 1, ctx, 0)?
                .ok_or_else(|| error("当前方法不能证明此无限极限，保留原式"))?,
        ));
    }
    let left = if direction != "right" {
        side(&mut fork, &raw, &x, &point, -1, ctx, 0)?
    } else {
        None
    };
    let right = if direction != "left" {
        side(&mut fork, &raw, &x, &point, 1, ctx, 0)?
    } else {
        None
    };
    let out = match direction {
        "left" => left,
        "right" => right,
        _ => {
            if let (Some(a), Some(b)) = (left, right) {
                let equal = a == b
                    || infinity(&a).is_none()
                        && infinity(&b).is_none()
                        && zero(&mut fork, &om_core::sub(a.clone(), b), ctx)? == Some(true);
                equal.then_some(a)
            } else {
                None
            }
        }
    };
    Ok(Some(out.ok_or_else(|| {
        error("左右极限不同、振荡或超出可证明的方法范围；保留原式")
    })?))
}
