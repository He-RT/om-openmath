//! Q coefficient denominators are cleared before certified Z gcd/factor operations.
use crate::convert::{PolyView, from_mpoly_with};
use om_core::{Expr, div, mul, pow};
use om_num::{
    Integer, Number, Rational,
    ctx::{Abort, Interrupt},
    gcd,
};
use om_poly::{FactorStatus, MPoly};
type Q = MPoly<Rational>;
type Z = MPoly<Integer>;
pub(super) fn integer(p: &Q, ctx: &Interrupt) -> Result<(Z, Integer), Abort> {
    let mut scale = Integer::ONE;
    for (_, c) in &p.terms {
        ctx.tick()?;
        let d = Integer::from(c.denominator().clone());
        scale = (&scale / gcd(&scale, &d)) * d;
    }
    let mut terms = vec![];
    for (m, c) in &p.terms {
        ctx.tick()?;
        terms.push((
            m.clone(),
            c.numerator() * (&scale / Integer::from(c.denominator().clone())),
        ));
    }
    Ok((Z::new(p.nvars, terms, p.order, ctx)?, scale))
}
fn rational(p: &Z, scale: &Rational, ctx: &Interrupt) -> Result<Q, Abort> {
    let mut terms = vec![];
    for (m, c) in &p.terms {
        ctx.tick()?;
        terms.push((m.clone(), Rational::from(c.clone()) * scale));
    }
    Q::new(p.nvars, terms, p.order, ctx)
}
pub(super) fn cancel(view: PolyView, ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    let (n, ns) = integer(&view.num, ctx)?;
    let (d, ds) = integer(&view.den, ctx)?;
    if d.is_zero() {
        return Ok(None);
    }
    let g = n.subresultant_gcd(&d, ctx)?;
    let (Some(n), Some(d)) = (n.exact_div(&g, ctx)?, d.exact_div(&g, ctx)?) else {
        return Ok(None);
    };
    let lc = Rational::from(d.terms[0].1.clone());
    let scale = Rational::from(ds) / Rational::from(ns) / &lc;
    let n = rational(&n, &scale, ctx)?;
    let d = rational(&d, &(Rational::ONE / lc), ctx)?;
    let (Some(num), Some(den)) = (
        from_mpoly_with(&n, &view.gens, ctx)?,
        from_mpoly_with(&d, &view.gens, ctx)?,
    ) else {
        return Ok(None);
    };
    Ok(Some(div(num, den)))
}
pub(super) fn factor(view: PolyView, ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    let (Some(n), Some(d)) = (
        factor_poly(&view.num, &view.gens, ctx)?,
        factor_poly(&view.den, &view.gens, ctx)?,
    ) else {
        return Ok(None);
    };
    Ok(Some(div(n, d)))
}
fn factor_poly(p: &Q, gens: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    ctx.tick()?;
    if p.is_zero() {
        return Ok(Some(Expr::int(0)));
    }
    let (z, den) = integer(p, ctx)?;
    let Some(f) = z.factor_z(ctx)? else {
        return Ok(None);
    };
    if f.status != FactorStatus::Complete {
        return Ok(None);
    }
    let mut factors = f.factors;
    for i in 1..factors.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            let degree = |p: &Z| p.terms.iter().map(|(m, _)| m.deg).max().unwrap_or(0);
            if degree(&factors[j - 1].0) <= degree(&factors[j].0) {
                break;
            }
            factors.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut terms = vec![Expr::number(Number::Rational(
        Rational::from(f.content) / Rational::from(den),
    ))];
    for (f, multiplicity) in factors {
        ctx.tick()?;
        let q = rational(&f, &Rational::ONE, ctx)?;
        let Some(f) = from_mpoly_with(&q, gens, ctx)? else {
            return Ok(None);
        };
        terms.push(pow(f, Expr::integer(Integer::from(multiplicity))));
    }
    Ok(Some(mul(terms)))
}
