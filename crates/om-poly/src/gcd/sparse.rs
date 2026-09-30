//! Sparse exact division and last-variable evaluation/recursive representation.
use super::{Dense, Sparse, compatible};
use crate::{EuclideanRing, MPoly, MonoOrder, Monomial, UPoly};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

impl MPoly<Integer> {
    /// Divide exactly over the integer coefficient ring; reject zero/nondivisibility.
    /// Both canonical operands must have compatible variable and monomial-order contexts.
    pub fn exact_div(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        let (mut remainder, divisor) = compatible(self, other, ctx)?;
        let Some((leading, lc)) = divisor.terms.first() else {
            return Ok(None);
        };
        let (nvars, order) = (remainder.nvars, remainder.order);
        let mut terms = vec![];
        while let Some((m, c)) = remainder.terms.first() {
            ctx.tick()?;
            let mut exps = Vec::with_capacity(nvars);
            for (a, b) in m.exps.iter().zip(&leading.exps) {
                ctx.tick()?;
                let Some(e) = a.checked_sub(*b) else {
                    return Ok(None);
                };
                exps.push(e);
            }
            let Some(c) = c.exact_div(lc) else {
                return Ok(None);
            };
            let m =
                Monomial::new(exps).expect("invariant: quotient degree is bounded by the dividend");
            let term = Sparse::new(nvars, vec![(m.clone(), c.clone())], order, ctx)?;
            remainder = remainder.sub(&multiply(&divisor, &term, ctx)?, ctx)?;
            terms.push((m, c));
        }
        Ok(Some(Sparse::new(nvars, terms, order, ctx)?))
    }
}
pub(super) fn multiply(a: &Sparse, b: &Sparse, ctx: &Interrupt) -> Result<Sparse, Abort> {
    Ok(a.mul(b, ctx)?
        .expect("invariant: intermediate polynomial degree fits u32"))
}
pub(super) fn power(a: &Sparse, mut n: usize, ctx: &Interrupt) -> Result<Sparse, Abort> {
    ctx.tick()?;
    let mut result = Sparse::one();
    let mut base = a.clone();
    while n > 0 {
        ctx.tick()?;
        if n & 1 != 0 {
            result = multiply(&result, &base, ctx)?;
        }
        n >>= 1;
        if n > 0 {
            base = multiply(&base, &base, ctx)?;
        }
    }
    Ok(result)
}
pub(super) fn split(f: &Sparse, ctx: &Interrupt) -> Result<Dense, Abort> {
    ctx.tick()?;
    assert!(
        f.nvars > 0,
        "last-variable splitting requires at least one variable"
    );
    let mut buckets: Vec<Vec<(Monomial, Integer)>> = vec![];
    for (m, c) in &f.terms {
        ctx.tick()?;
        let exponent = m.exps[f.nvars - 1] as usize;
        while buckets.len() <= exponent {
            ctx.tick()?;
            buckets.push(vec![]);
        }
        let m = Monomial::new(m.exps[..f.nvars - 1].iter().copied())
            .expect("invariant: removing a variable reduces total degree");
        buckets[exponent].push((m, c.clone()));
    }
    let mut coefficients = Vec::with_capacity(buckets.len());
    for bucket in buckets {
        ctx.tick()?;
        coefficients.push(Sparse::new(f.nvars - 1, bucket, f.order, ctx)?);
    }
    Ok(UPoly::new(coefficients))
}
pub(super) fn join(
    f: &Dense,
    nvars: usize,
    order: MonoOrder,
    ctx: &Interrupt,
) -> Result<Sparse, Abort> {
    ctx.tick()?;
    let mut terms = vec![];
    for (i, c) in f.coeffs.iter().enumerate() {
        ctx.tick()?;
        assert!(
            c.nvars == 0 || (c.nvars == nvars - 1 && c.order == order),
            "recursive coefficient context must match"
        );
        for (m, a) in &c.terms {
            ctx.tick()?;
            let mut exps = m.exps.clone();
            if c.nvars == 0 {
                exps.resize(nvars - 1, 0);
            }
            exps.push(u32::try_from(i).expect("invariant: dense degree fits u32"));
            terms.push((
                Monomial::new(exps).expect("invariant: recursive total degree fits u32"),
                a.clone(),
            ));
        }
    }
    Sparse::new(nvars, terms, order, ctx)
}
pub(super) fn divide_coefficients(f: &Dense, c: &Sparse, ctx: &Interrupt) -> Result<Dense, Abort> {
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for a in &f.coeffs {
        ctx.tick()?;
        coefficients.push(
            a.exact_div(c, ctx)?
                .expect("invariant: PRS/content scalar division is exact"),
        );
    }
    Ok(UPoly::new(coefficients))
}
pub(super) fn evaluate(f: &Sparse, xi: &Integer, ctx: &Interrupt) -> Result<Sparse, Abort> {
    ctx.tick()?;
    let mut terms = Vec::with_capacity(f.terms.len());
    for (m, c) in &f.terms {
        ctx.tick()?;
        let d = m.exps[f.nvars - 1] as usize;
        let mut power = Integer::ONE;
        let mut base = xi.clone();
        let mut exponent = d;
        while exponent > 0 {
            ctx.tick()?;
            if exponent & 1 != 0 {
                power *= &base;
            }
            exponent >>= 1;
            if exponent > 0 {
                base = &base * &base;
            }
        }
        let m = Monomial::new(m.exps[..f.nvars - 1].iter().copied())
            .expect("invariant: evaluation reduces total degree");
        terms.push((m, c * power));
    }
    Sparse::new(f.nvars - 1, terms, f.order, ctx)
}
pub(super) fn interpolate(
    f: &Sparse,
    xi: &Integer,
    ctx: &Interrupt,
) -> Result<Option<Sparse>, Abort> {
    ctx.tick()?;
    let mut terms = vec![];
    for (m, c) in &f.terms {
        ctx.tick()?;
        let mut value = c.clone();
        let mut exponent = 0_u32;
        while !value.is_zero() {
            ctx.tick()?;
            let (_, mut digit) = value.divrem(xi);
            if &digit * 2 > *xi {
                digit -= xi;
            }
            value = (&value - &digit) / xi;
            if !digit.is_zero() {
                let mut exps = m.exps.clone();
                exps.push(exponent);
                let Some(m) = Monomial::new(exps) else {
                    return Ok(None);
                };
                terms.push((m, digit));
            }
            let Some(next) = exponent.checked_add(1) else {
                return Ok(None);
            };
            exponent = next;
        }
    }
    Ok(Some(Sparse::new(f.nvars + 1, terms, f.order, ctx)?))
}
