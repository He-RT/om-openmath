//! Monomial and exact coefficient operations used by integer and rational reductions.
use crate::{MPoly, Monomial, Ring};
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
    gcd as igcd,
};
pub(super) type Z = MPoly<Integer>;
pub(super) type Q = MPoly<Rational>;
pub(super) fn divides(a: &Monomial, b: &Monomial) -> bool {
    a.exps.iter().zip(&b.exps).all(|(x, y)| x <= y)
}
pub(super) fn quotient(a: &Monomial, b: &Monomial) -> Option<Monomial> {
    Monomial::new(
        a.exps
            .iter()
            .zip(&b.exps)
            .map(|(x, y)| x.checked_sub(*y))
            .collect::<Option<Vec<_>>>()?,
    )
}
pub(super) fn lcm(a: &Monomial, b: &Monomial) -> Option<Monomial> {
    Monomial::new(a.exps.iter().zip(&b.exps).map(|(x, y)| (*x).max(*y)))
}
pub(super) fn coprime(a: &Monomial, b: &Monomial) -> bool {
    a.exps.iter().zip(&b.exps).all(|(x, y)| *x == 0 || *y == 0)
}
pub(super) fn shift<R: Ring>(
    f: &MPoly<R>,
    m: &Monomial,
    c: &R,
    ctx: &Interrupt,
) -> Result<Option<MPoly<R>>, Abort> {
    let mut terms = Vec::with_capacity(f.terms.len());
    for (n, a) in &f.terms {
        ctx.tick()?;
        let Some(exps) = m
            .exps
            .iter()
            .zip(&n.exps)
            .map(|(x, y)| x.checked_add(*y))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };
        let Some(m) = Monomial::new(exps) else {
            return Ok(None);
        };
        terms.push((m, a.mul(c)));
    }
    Ok(Some(MPoly::new(f.nvars, terms, f.order, ctx)?))
}
pub(super) fn primitive(f: Z, ctx: &Interrupt) -> Result<Z, Abort> {
    ctx.tick()?;
    let mut content = Integer::ZERO;
    for (_, c) in &f.terms {
        ctx.tick()?;
        content = igcd(&content, c);
    }
    if content.is_zero() {
        return Ok(f);
    }
    if f.terms[0].1 < Integer::ZERO {
        content = -content;
    }
    let mut terms = Vec::with_capacity(f.terms.len());
    for (m, c) in f.terms {
        ctx.tick()?;
        terms.push((m, c / &content));
    }
    Z::new(f.nvars, terms, f.order, ctx)
}
pub(super) fn to_integer(f: &Q, ctx: &Interrupt) -> Result<Z, Abort> {
    let mut denominator = Integer::ONE;
    for (_, c) in &f.terms {
        ctx.tick()?;
        let d = Integer::from(c.denominator().clone());
        denominator = (&denominator / igcd(&denominator, &d)) * d;
    }
    let mut terms = vec![];
    for (m, c) in &f.terms {
        ctx.tick()?;
        terms.push((
            m.clone(),
            c.numerator() * (&denominator / Integer::from(c.denominator().clone())),
        ));
    }
    primitive(Z::new(f.nvars, terms, f.order, ctx)?, ctx)
}
pub(super) fn to_rational(f: &Z, ctx: &Interrupt) -> Result<Q, Abort> {
    let Some((_, lc)) = f.terms.first() else {
        return Ok(Q::zero_in(f.nvars, f.order));
    };
    let lc = Rational::from(lc.clone());
    let mut terms = vec![];
    for (m, c) in &f.terms {
        ctx.tick()?;
        terms.push((m.clone(), Rational::from(c.clone()) / &lc));
    }
    Q::new(f.nvars, terms, f.order, ctx)
}
pub(super) fn reduce(
    mut p: Z,
    basis: &[(&Z, u64)],
    mut sugar: u64,
    ctx: &Interrupt,
) -> Result<Option<(Z, u64)>, Abort> {
    loop {
        ctx.tick()?;
        let mut reducer = None;
        'term: for (m, c) in &p.terms {
            ctx.tick()?;
            for (i, (g, _)) in basis.iter().enumerate() {
                ctx.tick()?;
                if let Some((n, b)) = g.terms.first()
                    && let Some(shift) = quotient(m, n)
                {
                    reducer = Some((i, shift, c.clone(), b.clone()));
                    break 'term;
                }
            }
        }
        let Some((i, m, c, lc)) = reducer else {
            return Ok(Some((p, sugar)));
        };
        // Scaling the entire polynomial, including irreducible terms, keeps the
        // remainder proportional to the rational normal form after content removal.
        let unit = Monomial::new(vec![0; p.nvars]).expect("invariant: degree-zero monomial");
        let Some(scaled) = shift(&p, &unit, &lc, ctx)? else {
            return Ok(None);
        };
        let Some(subtrahend) = shift(basis[i].0, &m, &c, ctx)? else {
            return Ok(None);
        };
        p = primitive(scaled.sub(&subtrahend, ctx)?, ctx)?;
        // The caller provides reducer sugars separately for nonhomogeneous inputs.
        sugar = sugar.max(basis[i].1 + u64::from(m.deg));
    }
}
pub(super) fn z_spoly(f: &Z, g: &Z, ctx: &Interrupt) -> Result<Option<Z>, Abort> {
    let (Some((m, a)), Some((n, b))) = (f.terms.first(), g.terms.first()) else {
        return Ok(Some(Z::zero_in(f.nvars, f.order)));
    };
    let Some(l) = lcm(m, n) else {
        return Ok(None);
    };
    let (lm, ln) = (
        quotient(&l, m).expect("invariant: lcm divisible by first LM"),
        quotient(&l, n).expect("invariant: lcm divisible by second LM"),
    );
    let (Some(left), Some(right)) = (shift(f, &lm, b, ctx)?, shift(g, &ln, a, ctx)?) else {
        return Ok(None);
    };
    Ok(Some(primitive(left.sub(&right, ctx)?, ctx)?))
}
