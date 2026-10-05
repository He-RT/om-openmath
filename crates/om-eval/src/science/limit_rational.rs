//! Exact rational limits from polynomial vanishing orders and leading coefficients.
use super::*;
use om_core::{Expr, Interrupt};
fn dense(
    p: &om_poly::MPoly<Rational>,
    ctx: &Interrupt,
) -> Result<Option<Vec<Rational>>, EvalError> {
    let degree = p
        .terms
        .iter()
        .map(|(m, _)| m.exps[0] as usize)
        .max()
        .unwrap_or(0);
    if degree > 256 {
        return Ok(None);
    }
    let mut c = vec![Rational::ZERO; degree + 1];
    for (m, q) in &p.terms {
        ctx.tick()?;
        c[m.exps[0] as usize] = q.clone();
    }
    trim(&mut c);
    Ok(Some(c))
}
fn trim(p: &mut Vec<Rational>) {
    while p.len() > 1 && p.last() == Some(&Rational::ZERO) {
        p.pop();
    }
}
fn shift(p: &[Rational], a: &Rational, ctx: &Interrupt) -> Result<Vec<Rational>, EvalError> {
    let mut out = vec![Rational::ZERO];
    for c in p.iter().rev() {
        ctx.tick()?;
        let mut next = vec![Rational::ZERO; out.len() + 1];
        for (i, c) in out.iter().enumerate() {
            ctx.tick()?;
            next[i] += c * a;
            next[i + 1] += c;
        }
        next[0] += c;
        out = next;
        trim(&mut out);
    }
    Ok(out)
}
pub(super) fn apply(
    e: &Expr,
    x: &Expr,
    point: &Expr,
    side: i32,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let Some(v) = om_simplify::convert::to_rational_function_with(e, std::slice::from_ref(x), ctx)?
    else {
        return Ok(None);
    };
    if v.gens != [x.clone()] {
        return Ok(None);
    }
    let (Some(n), Some(d)) = (dense(&v.num, ctx)?, dense(&v.den, ctx)?) else {
        return Ok(None);
    };
    if d.iter().all(|q| *q == Rational::ZERO) {
        return Err(error("极限的分母恒等为零"));
    }
    if n.iter().all(|q| *q == Rational::ZERO) {
        return Ok(Some(Expr::int(0)));
    }
    if point.is_head(B::DIRECTED_INFINITY) && point.args().len() == 1 {
        let direction = if point.args()[0] == Expr::int(1) {
            1
        } else if point.args()[0] == Expr::int(-1) {
            -1
        } else {
            return Ok(None);
        };
        if n.len() < d.len() {
            return Ok(Some(Expr::int(0)));
        }
        let ratio = &n[n.len() - 1] / &d[d.len() - 1];
        if n.len() == d.len() {
            return Ok(Some(Expr::number(Number::Rational(ratio).normalize())));
        }
        let sign = if ratio < Rational::ZERO { -1 } else { 1 }
            * if direction < 0 && (n.len() - d.len()) % 2 == 1 {
                -1
            } else {
                1
            };
        return Ok(Some(Expr::call(B::DIRECTED_INFINITY, [Expr::int(sign)])));
    }
    let Some(point) = point
        .as_number()
        .filter(|n| n.is_exact())
        .and_then(crate::scalar::rational)
    else {
        return Ok(None);
    };
    let n = shift(&n, &point, ctx)?;
    let d = shift(&d, &point, ctx)?;
    let i = n
        .iter()
        .position(|q| *q != Rational::ZERO)
        .ok_or_else(|| error("极限分子阶数不确定"))?;
    let j = d
        .iter()
        .position(|q| *q != Rational::ZERO)
        .ok_or_else(|| error("极限分母阶数不确定"))?;
    if i > j {
        return Ok(Some(Expr::int(0)));
    }
    let ratio = &n[i] / &d[j];
    if i == j {
        return Ok(Some(Expr::number(Number::Rational(ratio).normalize())));
    }
    let sign = if ratio < Rational::ZERO { -1 } else { 1 }
        * if side < 0 && (j - i) % 2 == 1 { -1 } else { 1 };
    Ok(Some(Expr::call(B::DIRECTED_INFINITY, [Expr::int(sign)])))
}
