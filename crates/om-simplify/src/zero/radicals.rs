//! Branch-safe quadratic denesting and exact generator relations.
use super::Tri;
use crate::convert::{exact, from_mpoly_with, to_rational_function_with};
use om_core::{BUILTIN as B, Expr, ExprKind, add, func, mul, pow, sqrt};
use om_num::{
    BitTest, Integer, Number, Rational,
    ctx::{Abort, Interrupt},
    exact_root, gcd,
};
use om_poly::{MPoly, Monomial, normal_form};

pub(crate) fn rewrite(
    e: &Expr,
    ctx: &Interrupt,
    mut transform: impl FnMut(&Expr) -> Result<Expr, Abort>,
) -> Result<Expr, Abort> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut stack = vec![Frame::Enter(e)];
    let mut values = vec![];
    while let Some(frame) = stack.pop() {
        ctx.tick()?;
        match frame {
            Frame::Enter(e) => {
                if let ExprKind::Normal(n) = e.kind() {
                    stack.push(Frame::Build(n));
                    stack.extend(n.args.iter().rev().map(Frame::Enter));
                    stack.push(Frame::Enter(&n.head));
                } else {
                    values.push(transform(e)?);
                }
            }
            Frame::Build(n) => {
                let args = values.split_off(values.len() - n.args.len());
                let head = values.pop().expect("invariant: visited expression head");
                let node = if let Some(s) = head.as_symbol() {
                    func(s, args)
                } else {
                    Expr::normal(head, args)
                };
                values.push(transform(&node)?);
            }
        }
    }
    Ok(values.pop().expect("invariant: rewritten root value"))
}
pub(crate) fn denest(e: &Expr, ctx: &Interrupt) -> Result<Expr, Abort> {
    rewrite(e, ctx, |e| Ok(quadratic(e).unwrap_or_else(|| e.clone())))
}
fn square_root(e: &Expr) -> Option<&Expr> {
    (e.is_head(B::POWER)
        && e.args().len() == 2
        && exact(&e.args()[1]) == Some(Rational::ONE / Rational::from(2)))
    .then(|| &e.args()[0])
}
fn quadratic(e: &Expr) -> Option<Expr> {
    let base = square_root(e)?;
    if !base.is_head(B::PLUS) || base.args().len() != 2 {
        return None;
    }
    let (a, term) = if let Some(a) = exact(&base.args()[0]) {
        (a, &base.args()[1])
    } else {
        (exact(&base.args()[1])?, &base.args()[0])
    };
    if a <= Rational::ZERO {
        return None;
    }
    let (b, c) = if let Some(c) = square_root(term) {
        (Rational::ONE, exact(c)?)
    } else {
        if !term.is_head(B::TIMES) || term.args().len() != 2 {
            return None;
        }
        (
            exact(&term.args()[0])?,
            exact(square_root(&term.args()[1])?)?,
        )
    };
    if c < Rational::ZERO {
        return None;
    }
    let d = &a * &a - &b * &b * c;
    if d < Rational::ZERO {
        return None;
    }
    let r = Rational::from(exact_root(d.numerator(), 2)?)
        / Rational::from(exact_root(&Integer::from(d.denominator().clone()), 2)?);
    let u = (&a + &r) / Rational::from(2);
    let v = (a - r) / Rational::from(2);
    Some(add([
        sqrt(Expr::number(om_num::Number::Rational(u))),
        mul([
            Expr::int(if b < Rational::ZERO { -1 } else { 1 }),
            sqrt(Expr::number(om_num::Number::Rational(v))),
        ]),
    ]))
}
pub(super) fn relations(e: &Expr, ctx: &Interrupt) -> Result<Option<Tri>, Abort> {
    let Some(view) = to_rational_function_with(e, &[], ctx)? else {
        return Ok(None);
    };
    if view.gens.is_empty() {
        return Ok(Some(if view.num.is_zero() {
            Tri::Zero
        } else {
            Tri::NonZero
        }));
    }
    let mut basis = vec![];
    for (i, g) in view.gens.iter().enumerate() {
        ctx.tick()?;
        // Numeric radical binomials need no factorization; Root uses its certified minpoly.
        let coeffs = if g.is_head(B::POWER) && g.args().len() == 2 {
            let (Some(a), Some(q)) = (exact(&g.args()[0]), exact(&g.args()[1])) else {
                return Ok(None);
            };
            if q.numerator() != &Integer::ONE {
                return Ok(None);
            }
            let Ok(den) = usize::try_from(q.denominator()) else {
                return Ok(None);
            };
            if den > 64 {
                return Ok(None);
            }
            let mut coeffs = vec![Rational::ZERO; den + 1];
            coeffs[0] = -a;
            coeffs[den] = Rational::ONE;
            coeffs
        } else {
            let Some(a) = crate::root_reduce::evaluate(g, ctx)? else {
                return Ok(None);
            };
            a.minimal_polynomial(ctx)?
                .coeffs
                .into_iter()
                .map(Rational::from)
                .collect()
        };
        let mut terms = vec![];
        for (degree, c) in coeffs.into_iter().enumerate() {
            ctx.tick()?;
            if c == Rational::ZERO {
                continue;
            }
            let mut exps = vec![0; view.gens.len()];
            exps[i] = degree as u32;
            terms.push((
                Monomial::new(exps).expect("invariant: single axis degree <=64"),
                c,
            ));
        }
        basis.push(MPoly::new(view.gens.len(), terms, view.num.order, ctx)?);
    }
    let Some(num) = normal_form(&view.num, &basis, ctx)? else {
        return Ok(None);
    };
    // Reconstruct the reduced numerator to expose cross-generator products, e.g.
    // (2^(1/3))^2*3^(1/3) = 12^(1/3), before expensive resultants.
    let Some(numerator) = from_mpoly_with(&num, &view.gens, ctx)? else {
        return Ok(None);
    };
    let numerator = rewrite(&numerator, ctx, |e| product_roots(e, ctx))?;
    let zero = if let Some(q) = exact(&numerator) {
        q == Rational::ZERO
    } else {
        let Some(value) = crate::root_reduce::evaluate(&numerator, ctx)? else {
            return Ok(None);
        };
        value.is_zero()
    };
    let Some(den) = normal_form(&view.den, &basis, ctx)? else {
        return Ok(None);
    };
    if den.is_zero() {
        return Ok(None);
    }
    let Some(den) = from_mpoly_with(&den, &view.gens, ctx)? else {
        return Ok(None);
    };
    let defined = if let Some(z) = crate::numeval::enclose(&den, 64, ctx)? {
        z.re.excludes_zero() || z.im.excludes_zero()
    } else {
        false
    };
    if !defined {
        let Some(value) = crate::root_reduce::evaluate(&den, ctx)? else {
            return Ok(None);
        };
        if value.is_zero() {
            return Ok(None);
        }
    }
    Ok(Some(if zero { Tri::Zero } else { Tri::NonZero }))
}

fn product_roots(e: &Expr, ctx: &Interrupt) -> Result<Expr, Abort> {
    if !e.is_head(B::TIMES) {
        return Ok(e.clone());
    }
    let mut roots = vec![];
    let mut rest = vec![];
    let mut den = Integer::ONE;
    for factor in e.args() {
        ctx.tick()?;
        if factor.is_head(B::POWER)
            && factor.args().len() == 2
            && let (Some(base), Some(q)) = (exact(&factor.args()[0]), exact(&factor.args()[1]))
            && base > Rational::ZERO
        {
            let d = Integer::from(q.denominator().clone());
            den = (&den / gcd(&den, &d)) * d;
            if den > Integer::from(64) {
                return Ok(e.clone());
            }
            roots.push((base, q));
        } else {
            rest.push(factor.clone());
        }
    }
    if roots.len() < 2 {
        return Ok(e.clone());
    }
    let mut radicand = Rational::ONE;
    for (base, q) in roots {
        ctx.tick()?;
        let n = q.numerator() * (&den / Integer::from(q.denominator().clone()));
        let Ok(size) = i64::try_from(&n) else {
            return Ok(e.clone());
        };
        if size.unsigned_abs() > 64
            || base.numerator().bit_len().max(base.denominator().bit_len())
                * size.unsigned_abs() as usize
                > 65536
        {
            return Ok(e.clone());
        }
        let Ok(value) = Number::Rational(base).pow_int(&n) else {
            return Ok(e.clone());
        };
        let Some(value) = exact(&Expr::number(value)) else {
            return Ok(e.clone());
        };
        radicand *= value;
        if radicand
            .numerator()
            .bit_len()
            .max(radicand.denominator().bit_len())
            > 65536
        {
            return Ok(e.clone());
        }
    }
    rest.push(pow(
        Expr::number(Number::Rational(radicand)),
        Expr::number(Number::Rational(Rational::ONE / Rational::from(den))),
    ));
    Ok(mul(rest))
}
