//! Coprime denominator powers are inverted exactly, then expanded in their factors.
use super::poly;
use crate::EvalError;
use om_core::{Expr, Interrupt, add, div, mul, pow};
use om_num::{Integer, Rational, gcd};
use om_poly::{FactorStatus, UPoly};

fn expression(p: &UPoly<Rational>, x: &Expr) -> Expr {
    add(p.coeffs.iter().enumerate().map(|(i, c)| {
        mul([
            Expr::number(om_num::Number::Rational(c.clone())),
            pow(x.clone(), Expr::integer(Integer::from(i))),
        ])
    }))
}
fn dense(p: &om_poly::MPoly<Rational>) -> Option<UPoly<Rational>> {
    let degree = p
        .terms
        .iter()
        .map(|(m, _)| m.exps[0] as usize)
        .max()
        .unwrap_or(0);
    if degree > 4096 {
        return None;
    }
    let mut c = vec![Rational::ZERO; degree + 1];
    for (m, q) in &p.terms {
        c[m.exps[0] as usize] = q.clone();
    }
    Some(UPoly::new(c))
}
pub(super) fn apply(args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let vars = if let Some(v) = args.get(1) {
        let Some(v) = poly::vars(v).filter(|v| v.len() == 1) else {
            return Ok(None);
        };
        v
    } else {
        let mut symbols = args[0].free_symbols().into_iter().collect::<Vec<_>>();
        symbols.sort_by_key(|s| s.name().to_owned());
        if symbols.len() != 1 {
            return Ok(args[0]
                .as_number()
                .filter(|n| n.is_exact())
                .map(|_| args[0].clone()));
        }
        vec![Expr::sym(symbols[0])]
    };
    // The general polynomial query permits independent coefficient kernels;
    // Apart's narrower Q contract does not.
    let Some(view) = om_simplify::convert::to_rational_function_with(&args[0], &vars, ctx)? else {
        return Ok(None);
    };
    if view.gens != vars {
        return Ok(None);
    }
    let (Some(n), Some(d)) = (dense(&view.num), dense(&view.den)) else {
        return Ok(None);
    };
    let Some(lc) = d.lc() else { return Ok(None) };
    let n = n.scale(&(Rational::ONE / lc), ctx)?;
    let Some(d) = d.monic(ctx)? else {
        return Ok(None);
    };
    let Some((quotient, n)) = n.divrem(&d, ctx)? else {
        return Ok(None);
    };
    let mut terms = vec![expression(&quotient, &vars[0])];
    if n.is_zero() {
        return Ok(Some(add(terms)));
    }
    let mut scale = Integer::ONE;
    for q in &d.coeffs {
        ctx.tick()?;
        let den = Integer::from(q.denominator().clone());
        scale = (&scale / gcd(&scale, &den)) * den;
    }
    let z = UPoly::new(
        d.coeffs
            .iter()
            .map(|q| q.numerator() * (&scale / Integer::from(q.denominator().clone())))
            .collect(),
    );
    let Some(f) = z.factor_z(ctx)? else {
        return Ok(None);
    };
    if f.status != FactorStatus::Complete {
        return Ok(None);
    }
    for (f, m) in f.factors {
        ctx.tick()?;
        let f = UPoly::new(f.coeffs.into_iter().map(Rational::from).collect());
        let base = expression(&f, &vars[0]);
        let Some(leading) = f.lc().cloned() else {
            return Ok(None);
        };
        let Some(f) = f.monic(ctx)? else {
            return Ok(None);
        };
        let mut power = UPoly::one();
        for _ in 0..m {
            ctx.tick()?;
            power = power.mul(&f, ctx)?;
        }
        let Some((other, remainder)) = d.divrem(&power, ctx)? else {
            return Ok(None);
        };
        if !remainder.is_zero() {
            return Ok(None);
        }
        let Some((g, inverse, _)) = other.extended_gcd(&power, ctx)? else {
            return Ok(None);
        };
        if !g.is_one() {
            return Ok(None);
        }
        let Some((_, mut numerator)) = n.mul(&inverse, ctx)?.divrem(&power, ctx)? else {
            return Ok(None);
        };
        for k in (1..=m).rev() {
            ctx.tick()?;
            let Some((q, r)) = numerator.divrem(&f, ctx)? else {
                return Ok(None);
            };
            if !r.is_zero() {
                let scale = Rational::from(leading.numerator().pow(k as usize));
                terms.push(div(
                    expression(&r.scale(&scale, ctx)?, &vars[0]),
                    pow(base.clone(), Expr::integer(Integer::from(k))),
                ));
            }
            numerator = q;
        }
        if !numerator.is_zero() {
            return Ok(None);
        }
    }
    Ok(Some(add(terms)))
}
