//! Exact polynomial operations use shared rings and recover denominator scales.
use super::poly;
use crate::EvalError;
use om_core::{Expr, Interrupt, add, div, mul, neg, pow};
use om_num::{Integer, Rational, gcd};
use om_poly::{MPoly, Monomial, UPoly};
use om_simplify::{
    algebra::cancel_with,
    convert::{PolyView, from_mpoly_with, to_rational_function_with},
};
type ParameterPolynomial = UPoly<MPoly<Integer>>;
pub(super) fn integer(
    p: &MPoly<Rational>,
    ctx: &Interrupt,
) -> Result<(MPoly<Integer>, Integer), EvalError> {
    let mut scale = Integer::ONE;
    for (_, q) in &p.terms {
        ctx.tick()?;
        let d = Integer::from(q.denominator().clone());
        scale = (&scale / gcd(&scale, &d)) * d;
    }
    let mut terms = vec![];
    for (m, q) in &p.terms {
        ctx.tick()?;
        terms.push((
            m.clone(),
            q.numerator() * (&scale / Integer::from(q.denominator().clone())),
        ));
    }
    Ok((MPoly::new(p.nvars, terms, p.order, ctx)?, scale))
}
fn expression(
    p: &MPoly<Integer>,
    gens: &[Expr],
    scale: &Rational,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let mut terms = vec![];
    for (m, c) in &p.terms {
        ctx.tick()?;
        terms.push((m.clone(), Rational::from(c.clone()) * scale));
    }
    let q = MPoly::new(p.nvars, terms, p.order, ctx)?;
    Ok(from_mpoly_with(&q, gens, ctx)?)
}
fn shared(
    es: &[Expr],
    priority: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Vec<PolyView>>, EvalError> {
    let mut gens = priority.to_vec();
    for e in es {
        let Some(view) = to_rational_function_with(e, priority, ctx)? else {
            return Ok(None);
        };
        for g in view.gens {
            ctx.tick()?;
            if g.as_symbol().is_none() {
                return Ok(None);
            }
            if !gens.contains(&g) {
                gens.push(g);
            }
        }
    }
    gens[priority.len()..].sort_by(om_core::canonical_cmp);
    let mut views = vec![];
    for e in es {
        let Some(view) = to_rational_function_with(e, &gens, ctx)? else {
            return Ok(None);
        };
        if view.gens != gens {
            return Ok(None);
        }
        views.push(view);
    }
    Ok(Some(views))
}
fn common(name: &str, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let Some(views) = shared(args, &[], ctx)? else {
        return Ok(None);
    };
    let mut polys = vec![];
    let mut scale = Integer::ONE;
    for v in &views {
        if v.den.terms.iter().any(|(m, _)| m.deg != 0) {
            return Ok(None);
        }
        let denominator = v
            .den
            .terms
            .first()
            .map(|(_, q)| q.clone())
            .unwrap_or(Rational::ZERO);
        if denominator == Rational::ZERO {
            return Ok(None);
        }
        let terms = v
            .num
            .terms
            .iter()
            .map(|(m, q)| (m.clone(), q / &denominator))
            .collect();
        let p = MPoly::new(v.num.nvars, terms, v.num.order, ctx)?;
        let (p, s) = integer(&p, ctx)?;
        scale = (&scale / gcd(&scale, &s)) * &s;
        polys.push((p, s));
    }
    let mut all = vec![];
    for (p, s) in polys {
        let factor = &scale / s;
        let terms = p.terms.into_iter().map(|(m, c)| (m, c * &factor)).collect();
        all.push(MPoly::new(p.nvars, terms, p.order, ctx)?);
    }
    let mut result = all.remove(0);
    for p in all {
        ctx.tick()?;
        let g = result.subresultant_gcd(&p, ctx)?;
        result = if name == "PolynomialGCD" {
            g
        } else if result.is_zero() || p.is_zero() {
            MPoly::zero_in(p.nvars, p.order)
        } else {
            let Some(q) = result.exact_div(&g, ctx)? else {
                return Ok(None);
            };
            let Some(l) = q.mul(&p, ctx)? else {
                return Ok(None);
            };
            l
        };
    }
    if result
        .terms
        .first()
        .is_some_and(|(_, c)| c < &Integer::ZERO)
    {
        result = result.neg(ctx)?;
    }
    let scale = Rational::ONE / Rational::from(scale);
    expression(&result, &views[0].gens, &scale, ctx)
}
fn reduced(e: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    Ok(cancel_with(e, &[], ctx)?)
}
fn divide(args: &[Expr], remainder: bool, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let Some(vars) = poly::vars(&args[2]).filter(|v| v.len() == 1) else {
        return Ok(None);
    };
    let (Some(f), Some(g)) = (
        poly::polynomial(&args[0], &vars, ctx)?,
        poly::polynomial(&args[1], &vars, ctx)?,
    ) else {
        return Ok(None);
    };
    let (Some(mut f), Some(g)) = (
        poly::coefficients(&f, 0, ctx)?,
        poly::coefficients(&g, 0, ctx)?,
    ) else {
        return Ok(None);
    };
    let Some(dg) = g.len().checked_sub(1) else {
        return Ok(None);
    };
    let mut q = vec![Expr::int(0); f.len().saturating_sub(dg).max(1)];
    while f.len() > dg {
        ctx.tick()?;
        let shift = f.len() - 1 - dg;
        let Some(c) = reduced(&div(f[f.len() - 1].clone(), g[dg].clone()), ctx)? else {
            return Ok(None);
        };
        q[shift] = c.clone();
        for (i, coefficient) in g.iter().enumerate() {
            let value = add([
                f[shift + i].clone(),
                neg(mul([c.clone(), coefficient.clone()])),
            ]);
            let Some(value) = reduced(&value, ctx)? else {
                return Ok(None);
            };
            f[shift + i] = value;
        }
        if !f.last().is_some_and(Expr::is_zero) {
            return Ok(None);
        }
        while f.last().is_some_and(Expr::is_zero) {
            f.pop();
        }
    }
    let values = if remainder { f } else { q };
    Ok(Some(add(values.into_iter().enumerate().map(|(i, c)| {
        mul([c, pow(vars[0].clone(), Expr::integer(Integer::from(i)))])
    }))))
}
fn dense(
    view: &PolyView,
    ctx: &Interrupt,
) -> Result<Option<(ParameterPolynomial, Expr)>, EvalError> {
    if view.gens.is_empty() || view.den.terms.iter().any(|(m, _)| m.exps[0] != 0) {
        return Ok(None);
    }
    let (p, scale) = integer(&view.num, ctx)?;
    let degree = p
        .terms
        .iter()
        .map(|(m, _)| m.exps[0] as usize)
        .max()
        .unwrap_or(0);
    if degree > 4096 {
        return Ok(None);
    }
    let mut groups = vec![vec![]; degree + 1];
    for (m, c) in p.terms {
        ctx.tick()?;
        groups[m.exps[0] as usize].push((
            Monomial::new(m.exps[1..].iter().copied()).expect("invariant: submonomial fits degree"),
            c,
        ));
    }
    let mut coefficients = vec![];
    for terms in groups {
        coefficients.push(MPoly::new(view.gens.len() - 1, terms, p.order, ctx)?);
    }
    let Some(den) = from_mpoly_with(&view.den, &view.gens, ctx)? else {
        return Ok(None);
    };
    Ok(Some((
        UPoly::new(coefficients),
        mul([Expr::integer(scale), den]),
    )))
}
fn eliminant(name: &str, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let axis = if name == "Discriminant" {
        &args[1]
    } else {
        &args[2]
    };
    let Some(vars) = poly::vars(axis).filter(|v| v.len() == 1 && v[0].as_symbol().is_some()) else {
        return Ok(None);
    };
    let inputs = if name == "Discriminant" {
        &args[..1]
    } else {
        &args[..2]
    };
    let Some(views) = shared(inputs, &vars, ctx)? else {
        return Ok(None);
    };
    let Some((f, fs)) = dense(&views[0], ctx)? else {
        return Ok(None);
    };
    let (result, scale) = if name == "Discriminant" {
        let n = f.degree().unwrap_or(0);
        (
            f.discriminant(ctx)?,
            pow(
                fs,
                Expr::integer(Integer::from(n.saturating_mul(2).saturating_sub(2))),
            ),
        )
    } else {
        let Some((g, gs)) = dense(&views[1], ctx)? else {
            return Ok(None);
        };
        let scale = mul([
            pow(fs, Expr::integer(Integer::from(g.degree().unwrap_or(0)))),
            pow(gs, Expr::integer(Integer::from(f.degree().unwrap_or(0)))),
        ]);
        (f.resultant(&g, ctx)?, scale)
    };
    let Some(e) = expression(&result, &views[0].gens[1..], &Rational::ONE, ctx)? else {
        return Ok(None);
    };
    reduced(&div(e, scale), ctx)
}
pub(super) fn apply(name: &str, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    match name {
        "PolynomialGCD" | "PolynomialLCM" => common(name, args, ctx),
        "PolynomialQuotient" | "PolynomialRemainder" => {
            divide(args, name == "PolynomialRemainder", ctx)
        }
        _ => eliminant(name, args, ctx),
    }
}
