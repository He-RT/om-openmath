//! Polynomial extraction with expression coefficients; input precision is not converted into fake exact data.
use super::*;
use om_core::{add, mul, pow};
const MAX_DEGREE: usize = 256;
pub(super) fn product(
    a: &[Expr],
    b: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Vec<Expr>>, EvalError> {
    if a.len() + b.len() > MAX_DEGREE + 2 {
        return Ok(None);
    }
    let mut out = vec![Expr::int(0); a.len() + b.len() - 1];
    for (i, a) in a.iter().enumerate() {
        for (j, b) in b.iter().enumerate() {
            ctx.tick()?;
            out[i + j] = add([out[i + j].clone(), mul([a.clone(), b.clone()])]);
        }
    }
    Ok(Some(out))
}
pub(super) fn coefficients(
    e: &Expr,
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Option<Vec<Expr>>, EvalError> {
    fn walk(
        e: &Expr,
        x: &Expr,
        ctx: &Interrupt,
        depth: usize,
    ) -> Result<Option<Vec<Expr>>, EvalError> {
        ctx.tick()?;
        if depth > 64 {
            return Ok(None);
        }
        if e.free_of(x) {
            return Ok(Some(vec![e.clone()]));
        }
        if e == x {
            return Ok(Some(vec![Expr::int(0), Expr::int(1)]));
        }
        let out = if e.is_head(B::PLUS) {
            let mut sum = vec![Expr::int(0)];
            for term in e.args() {
                let Some(p) = walk(term, x, ctx, depth + 1)? else {
                    return Ok(None);
                };
                sum.resize(sum.len().max(p.len()), Expr::int(0));
                for (i, c) in p.into_iter().enumerate() {
                    ctx.tick()?;
                    sum[i] = add([sum[i].clone(), c]);
                }
            }
            sum
        } else if e.is_head(B::TIMES) {
            let mut out = vec![Expr::int(1)];
            for f in e.args() {
                let Some(p) = walk(f, x, ctx, depth + 1)? else {
                    return Ok(None);
                };
                let Some(p) = product(&out, &p, ctx)? else {
                    return Ok(None);
                };
                out = p;
            }
            out
        } else if e.is_head(B::POWER) && e.args().len() == 2 {
            let Some(Number::Integer(n)) = e.args()[1].as_number() else {
                return Ok(None);
            };
            let Ok(mut n) = usize::try_from(n) else {
                return Ok(None);
            };
            if n > MAX_DEGREE {
                return Ok(None);
            }
            let Some(mut base) = walk(&e.args()[0], x, ctx, depth + 1)? else {
                return Ok(None);
            };
            let mut out = vec![Expr::int(1)];
            while n > 0 {
                ctx.tick()?;
                if n % 2 == 1 {
                    let Some(p) = product(&out, &base, ctx)? else {
                        return Ok(None);
                    };
                    out = p;
                }
                n /= 2;
                if n > 0 {
                    let Some(p) = product(&base, &base, ctx)? else {
                        return Ok(None);
                    };
                    base = p;
                }
            }
            out
        } else {
            return Ok(None);
        };
        let mut out = out;
        while out.len() > 1 && out.last().is_some_and(Expr::is_zero) {
            out.pop();
        }
        Ok(Some(out))
    }
    walk(e, x, ctx, 0)
}
pub(super) fn primitive(coefficients: &[Expr], x: &Expr) -> Expr {
    add(coefficients.iter().enumerate().map(|(i, c)| {
        om_core::div(
            mul([c.clone(), pow(x.clone(), Expr::int((i + 1) as i64))]),
            Expr::int((i + 1) as i64),
        )
    }))
}
pub(super) fn affine(
    e: &Expr,
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Option<(Expr, Expr)>, EvalError> {
    let Some(c) = coefficients(e, x, ctx)? else {
        return Ok(None);
    };
    if c.len() > 2 {
        return Ok(None);
    }
    Ok(Some((
        c.get(1).cloned().unwrap_or_else(|| Expr::int(0)),
        c[0].clone(),
    )))
}
