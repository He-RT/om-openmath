//! Odd-prime distinct/equal-degree factorization, from PLAN §8.2d.
use crate::{FpElem, UPoly, division::coefficient_power};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
    rng::SplitMix64,
};
use std::cmp::Ordering;
type Poly = UPoly<FpElem>;

impl UPoly<FpElem> {
    /// Group monic square-free factors by irreducible degree over a bound odd prime field.
    /// None rejects zero/repeated inputs or a missing/unsupported field context.
    /// Nonzero constants return an empty list; a nonmonic input is normalized first.
    pub fn ddf(&self, ctx: &Interrupt) -> Result<Option<Vec<(Self, usize)>>, Abort> {
        let Some((f, p)) = prepare(self, ctx)? else {
            return Ok(None);
        };
        Ok(Some(distinct(f, p, ctx)?))
    }
    /// Split a square-free group into monic irreducibles of the prescribed positive degree.
    /// Invalid degree groups return None. Caller-seeded SplitMix64 makes retries reproducible.
    /// Nonzero constants return an empty list. All retries consume the supplied budget.
    pub fn edf(
        &self,
        degree: usize,
        seed: u64,
        ctx: &Interrupt,
    ) -> Result<Option<Vec<Self>>, Abort> {
        ctx.tick()?;
        if degree == 0 {
            return Ok(None);
        }
        let Some((f, p)) = prepare(self, ctx)? else {
            return Ok(None);
        };
        if f.degree() == Some(0) {
            return Ok(Some(vec![]));
        }
        if !f
            .degree()
            .expect("invariant: prepared polynomial is nonzero")
            .is_multiple_of(degree)
        {
            return Ok(None);
        }
        let groups = distinct(f.clone(), p, ctx)?;
        if groups.len() != 1 || groups[0].1 != degree {
            return Ok(None);
        }
        let exponent: Integer =
            (coefficient_power(&Integer::from(p), degree, ctx)? - 1_u8) >> 1_usize;
        let mut rng = SplitMix64::new(seed);
        let mut pending = vec![f];
        let mut out = vec![];
        while let Some(f) = pending.pop() {
            ctx.tick()?;
            let n = f
                .degree()
                .expect("invariant: pending EDF factor is nonconstant");
            if n == degree {
                out.push(f);
                continue;
            }
            loop {
                ctx.tick()?;
                let mut coefficients = Vec::with_capacity(n);
                for _ in 0..n {
                    ctx.tick()?;
                    coefficients.push(FpElem {
                        v: rng.next_range(0, p),
                        p,
                    });
                }
                let a = Self::new(coefficients);
                let mut g = gcd(&a, &f, ctx)?;
                if !proper(&g, n) {
                    let powered = a
                        .pow_mod(&exponent, &f, ctx)?
                        .expect("invariant: bound prime-field modular power exists");
                    g = gcd(&powered.sub(&Self::one(), ctx)?, &f, ctx)?;
                }
                if proper(&g, n) {
                    let quotient = f
                        .exact_div(&g, ctx)?
                        .expect("invariant: trial GCD divides EDF input");
                    pending.push(quotient);
                    pending.push(g);
                    break;
                }
            }
        }
        sort(&mut out, ctx)?;
        Ok(Some(out))
    }
}
fn prepare(f: &Poly, ctx: &Interrupt) -> Result<Option<(Poly, u64)>, Abort> {
    ctx.tick()?;
    if f.is_zero() {
        return Ok(None);
    }
    let mut p = 0;
    for c in &f.coeffs {
        ctx.tick()?;
        if c.p != 0 {
            assert!(
                p == 0 || p == c.p,
                "factorization coefficient fields must match"
            );
            p = c.p;
        }
    }
    if p < 3 || p.is_multiple_of(2) {
        return Ok(None);
    }
    let f = f
        .monic(ctx)?
        .expect("invariant: bound prime-field input has a leading inverse");
    if !gcd(&f, &f.derivative(ctx)?, ctx)?.is_one() {
        return Ok(None);
    }
    Ok(Some((f, p)))
}
fn distinct(mut f: Poly, p: u64, ctx: &Interrupt) -> Result<Vec<(Poly, usize)>, Abort> {
    let x = Poly::new(vec![FpElem { v: 0, p }, FpElem { v: 1, p }]);
    let mut h = x.clone();
    let mut degree = 1;
    let mut out = vec![];
    let exponent = Integer::from(p);
    while f.degree().is_some_and(|d| degree <= d / 2) {
        ctx.tick()?;
        h = h
            .pow_mod(&exponent, &f, ctx)?
            .expect("invariant: bound prime-field modular power exists");
        let g = gcd(&f, &h.sub(&x, ctx)?, ctx)?;
        if !g.is_one() {
            f = f
                .exact_div(&g, ctx)?
                .expect("invariant: DDF GCD divides its input");
            out.push((g, degree));
            if f.degree() == Some(0) {
                break;
            }
            h = h
                .divrem(&f, ctx)?
                .expect("invariant: DDF residual has a leading inverse")
                .1;
        }
        degree += 1;
    }
    if let Some(d) = f.degree().filter(|d| *d > 0) {
        out.push((f, d));
    }
    Ok(out)
}
fn gcd(f: &Poly, g: &Poly, ctx: &Interrupt) -> Result<Poly, Abort> {
    Ok(f.monic_gcd(g, ctx)?
        .expect("invariant: bound prime-field GCD exists"))
}
fn proper(f: &Poly, degree: usize) -> bool {
    f.degree().is_some_and(|d| d > 0 && d < degree)
}
fn sort(factors: &mut [Poly], ctx: &Interrupt) -> Result<(), Abort> {
    for i in 1..factors.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            if compare(&factors[j - 1], &factors[j], ctx)? != Ordering::Greater {
                break;
            }
            factors.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(())
}
fn compare(f: &Poly, g: &Poly, ctx: &Interrupt) -> Result<Ordering, Abort> {
    for (a, b) in f.coeffs.iter().zip(&g.coeffs) {
        ctx.tick()?;
        let order = a.v.cmp(&b.v);
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(f.coeffs.len().cmp(&g.coeffs.len()))
}
