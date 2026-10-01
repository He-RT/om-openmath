//! Exact arithmetic bridge; ball intersections choose a branch, never establish equality.
use crate::convert::{exact, normalize, to_rational_function_with};
use om_core::{BUILTIN as B, Expr, pow};
use om_num::{
    Ball, BigFloat, CBall, Integer, Number, Rational,
    ctx::{Abort, Interrupt},
    gcd,
};
use om_poly::{Algebraic, UPoly, algebraic_root, real_alg};

/// Convert exact Q/radical/Root arithmetic to a certified algebraic value.
/// Invalid Root syntax, inexact atoms, unsupported kernels and degree >64 return None.
pub fn to_algebraic(e: &Expr, ctx: &Interrupt) -> Result<Option<Algebraic>, Abort> {
    ctx.tick()?;
    evaluate(&normalize(e, ctx)?, ctx)
}

pub(crate) fn evaluate(e: &Expr, ctx: &Interrupt) -> Result<Option<Algebraic>, Abort> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Build(&'a Expr),
    }
    let mut stack = vec![Frame::Enter(e)];
    let mut values = vec![];
    while let Some(frame) = stack.pop() {
        ctx.tick()?;
        match frame {
            Frame::Enter(e) => {
                if let Some(q) = exact(e) {
                    values.push(Algebraic::Rational(q));
                } else if let Some(Number::Complex(z)) = e.as_number() {
                    let (Some(re), Some(im)) = (
                        exact(&Expr::number(z.re.clone())),
                        exact(&Expr::number(z.im.clone())),
                    ) else {
                        return Ok(None);
                    };
                    // A Gaussian rational is a root of (x-re)^2+im^2.
                    let p = integer_polynomial(
                        &[
                            &re * &re + &im * &im,
                            -Rational::from(2) * re,
                            Rational::ONE,
                        ],
                        ctx,
                    )?;
                    let Some(value) = select(&p, e, ctx)? else {
                        return Ok(None);
                    };
                    values.push(value);
                } else if e.is_head(B::ROOT) {
                    let Some(value) = root_value(e, ctx)? else {
                        return Ok(None);
                    };
                    values.push(value);
                } else if e.is_head(B::PLUS) || e.is_head(B::TIMES) {
                    stack.push(Frame::Build(e));
                    stack.extend(e.args().iter().rev().map(Frame::Enter));
                } else if e.is_head(B::POWER)
                    && e.args().len() == 2
                    && exact(&e.args()[1]).is_some()
                {
                    stack.push(Frame::Build(e));
                    stack.push(Frame::Enter(&e.args()[0]));
                } else {
                    return Ok(None);
                }
            }
            Frame::Build(e) => {
                if e.is_head(B::POWER) {
                    let base = values.pop().expect("invariant: visited power base");
                    let exponent =
                        exact(&e.args()[1]).expect("invariant: checked rational exponent");
                    let Some(value) = power(e, base, &exponent, ctx)? else {
                        return Ok(None);
                    };
                    values.push(value);
                } else {
                    let args = values.split_off(values.len() - e.args().len());
                    let mut value =
                        Algebraic::Rational(Rational::from(i64::from(e.is_head(B::TIMES))));
                    for a in args {
                        ctx.tick()?;
                        let next = if e.is_head(B::PLUS) {
                            value.add(&a, ctx)?
                        } else {
                            value.mul(&a, ctx)?
                        };
                        let Some(next) = next else {
                            return Ok(None);
                        };
                        value = next;
                    }
                    values.push(value);
                }
            }
        }
    }
    Ok(values.pop())
}
fn power(
    e: &Expr,
    base: Algebraic,
    q: &Rational,
    ctx: &Interrupt,
) -> Result<Option<Algebraic>, Abort> {
    if q.denominator() == &1_u8.into() {
        return base.pow_int(q.numerator(), ctx);
    }
    if base.is_zero() {
        return Ok((q > &Rational::ZERO).then(|| Algebraic::Rational(Rational::ZERO)));
    }
    let Ok(den) = usize::try_from(q.denominator()) else {
        return Ok(None);
    };
    let p = base.minimal_polynomial(ctx)?;
    let degree = p
        .degree()
        .expect("invariant: algebraic minpoly nonconstant");
    if den > 64 || degree * den > 64 {
        return Ok(None);
    }
    let mut coeffs = vec![Integer::ZERO; degree * den + 1];
    for (i, c) in p.coeffs.into_iter().enumerate() {
        ctx.tick()?;
        coeffs[i * den] = c;
    }
    let root = pow(
        e.args()[0].clone(),
        Expr::number(Number::Rational(
            Rational::ONE / Rational::from(Integer::from(q.denominator().clone())),
        )),
    );
    let Some(value) = select(&UPoly::new(coeffs), &root, ctx)? else {
        return Ok(None);
    };
    value.pow_int(q.numerator(), ctx)
}
pub(crate) fn root_value(e: &Expr, ctx: &Interrupt) -> Result<Option<Algebraic>, Abort> {
    ctx.tick()?;
    if !e.is_head(B::ROOT) || e.args().len() != 2 {
        return Ok(None);
    }
    let f = &e.args()[0];
    let Some(index) = exact(&e.args()[1]) else {
        return Ok(None);
    };
    if !f.is_head(B::FUNCTION) || f.args().len() != 1 || index.denominator() != &1_u8.into() {
        return Ok(None);
    }
    let Ok(index) = usize::try_from(index.numerator()) else {
        return Ok(None);
    };
    let slot = Expr::call(B::SLOT, [Expr::int(1)]);
    let Some(view) = to_rational_function_with(&f.args()[0], std::slice::from_ref(&slot), ctx)?
    else {
        return Ok(None);
    };
    if view.gens != [slot] || view.den.terms.len() != 1 || view.den.terms[0].0.deg != 0 {
        return Ok(None);
    }
    let degree = view.num.terms.first().map_or(0, |(m, _)| m.deg as usize);
    if degree == 0 || degree > 64 {
        return Ok(None);
    }
    let mut coeffs = vec![Rational::ZERO; degree + 1];
    for (m, c) in view.num.terms {
        ctx.tick()?;
        coeffs[m.exps[0] as usize] = c;
    }
    algebraic_root(&integer_polynomial(&coeffs, ctx)?, index, ctx)
}
fn integer_polynomial(coeffs: &[Rational], ctx: &Interrupt) -> Result<UPoly<Integer>, Abort> {
    let mut den = Integer::ONE;
    for c in coeffs {
        ctx.tick()?;
        let d = Integer::from(c.denominator().clone());
        den = (&den / gcd(&den, &d)) * d;
    }
    let mut out = vec![];
    for c in coeffs {
        ctx.tick()?;
        out.push(c.numerator() * (&den / Integer::from(c.denominator().clone())));
    }
    Ok(UPoly::new(out))
}
fn dyadic(x: &BigFloat) -> Option<Rational> {
    let r = x.repr();
    if !r.is_finite() || r.exponent().unsigned_abs() > 262144 {
        return None;
    }
    let q = Rational::from(r.significand().clone());
    Some(if r.exponent() >= 0 {
        q * Rational::from(Integer::ONE << r.exponent() as usize)
    } else {
        q / Rational::from(Integer::ONE << r.exponent().unsigned_abs())
    })
}
fn endpoints(b: &Ball) -> Option<(Rational, Rational)> {
    let m = dyadic(&b.mid)?;
    let r = dyadic(&b.rad)?;
    Some((&m - &r, m + r))
}
fn overlaps(a: &CBall, b: &CBall) -> bool {
    [&a.re, &a.im]
        .iter()
        .zip([&b.re, &b.im])
        .all(|(a, b)| match (endpoints(a), endpoints(b)) {
            (Some(a), Some(b)) => a.0 <= b.1 && b.0 <= a.1,
            _ => false,
        })
}
fn select(p: &UPoly<Integer>, e: &Expr, ctx: &Interrupt) -> Result<Option<Algebraic>, Abort> {
    let mut candidates = None;
    for bits in [64, 256, 1024, 4096, 16352] {
        ctx.tick()?;
        let Some(z) = crate::numeval::enclose(e, bits, ctx)? else {
            return Ok(None);
        };
        if z.im.mid == BigFloat::ZERO && z.im.rad == BigFloat::ZERO {
            let Some(iv) = endpoints(&z.re) else {
                return Ok(None);
            };
            if let Some(value) = real_alg(p, iv, ctx)? {
                return Ok(Some(value));
            }
        } else {
            if candidates.is_none() {
                let mut roots = vec![];
                for index in 1..=p.degree().unwrap_or(0) {
                    ctx.tick()?;
                    let Some(a) = algebraic_root(p, index, ctx)? else {
                        return Ok(None);
                    };
                    roots.push(a);
                }
                candidates = Some(roots);
            }
            let mut found = None;
            for a in candidates
                .as_ref()
                .expect("invariant: initialized root candidates")
            {
                ctx.tick()?;
                let Some(b) = a.enclosure(bits, ctx)? else {
                    return Ok(None);
                };
                if overlaps(&z, &b) {
                    if found.is_some() {
                        found = None;
                        break;
                    }
                    found = Some(a.clone());
                }
            }
            if found.is_some() {
                return Ok(found);
            }
        }
    }
    Ok(None)
}
