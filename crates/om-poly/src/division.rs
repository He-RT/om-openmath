//! Integral-domain pseudo division and exact coefficient/polynomial division.
use crate::{EuclideanRing, Ring, UPoly};
use om_num::ctx::{Abort, Interrupt};

impl<R: Ring> UPoly<R> {
    /// Pseudo division: scale*self = quotient*g + remainder, degree(remainder)<degree(g).
    /// Compatible integral-domain coefficients are required; zero divisors return None.
    pub fn pseudo_divrem(
        &self,
        g: &Self,
        ctx: &Interrupt,
    ) -> Result<Option<(R, Self, Self)>, Abort> {
        ctx.tick()?;
        let Some(dg) = g.degree() else {
            return Ok(None);
        };
        let mut r = Self::new(self.coeffs.clone());
        let Some(df) = r.degree() else {
            return Ok(Some((R::one(), Self::zero(), r)));
        };
        if df < dg {
            return Ok(Some((R::one(), Self::zero(), r)));
        }
        let delta = df - dg + 1;
        let lc = &g.coeffs[dg];
        let mut q = vec![R::zero(); delta];
        let mut iterations = 0;
        while let Some(dr) = r.degree().filter(|d| *d >= dg) {
            ctx.tick()?;
            let shift = dr - dg;
            let leading = r.coeffs[dr].clone();
            for c in &mut q {
                ctx.tick()?;
                *c = c.mul(lc);
            }
            q[shift] = q[shift].add(&leading);
            for c in &mut r.coeffs {
                ctx.tick()?;
                *c = c.mul(lc);
            }
            for i in 0..=dg {
                ctx.tick()?;
                r.coeffs[i + shift] = r.coeffs[i + shift].sub(&leading.mul(&g.coeffs[i]));
            }
            trim(&mut r, ctx)?;
            iterations += 1;
        }
        // Degree gaps can skip iterations. The plan's exponent is delta, not k.
        let missing = coefficient_power(lc, delta - iterations, ctx)?;
        let q = Self::new(q).scale(&missing, ctx)?;
        let r = r.scale(&missing, ctx)?;
        let scale = coefficient_power(lc, delta, ctx)?;
        Ok(Some((scale, q, r)))
    }
    /// lc(g)^(degree(self)-degree(g)+1) times the field remainder.
    /// Below the divisor degree, return self with unit scale.
    pub fn prem(&self, g: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        Ok(self.pseudo_divrem(g, ctx)?.map(|(_, _, r)| r))
    }
}
impl<R: EuclideanRing> UPoly<R> {
    /// Divide exactly in the coefficient ring. None rejects nondivisibility/zero divisor.
    pub fn exact_div(&self, g: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        let Some(dg) = g.degree() else {
            return Ok(None);
        };
        let mut r = Self::new(self.coeffs.clone());
        let Some(df) = r.degree() else {
            return Ok(Some(Self::zero()));
        };
        if df < dg {
            return Ok(None);
        }
        let mut q = vec![R::zero(); df - dg + 1];
        while let Some(dr) = r.degree().filter(|d| *d >= dg) {
            ctx.tick()?;
            let Some(c) = r.coeffs[dr].exact_div(&g.coeffs[dg]) else {
                return Ok(None);
            };
            let shift = dr - dg;
            q[shift] = q[shift].add(&c);
            for i in 0..=dg {
                ctx.tick()?;
                r.coeffs[i + shift] = r.coeffs[i + shift].sub(&c.mul(&g.coeffs[i]));
            }
            trim(&mut r, ctx)?;
        }
        Ok(r.is_zero().then(|| Self::new(q)))
    }
    /// Divide every coefficient exactly by a scalar in the same ring context.
    pub fn divide_scalar(&self, d: &R, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        // A known coefficient supplies the field context for a generic constant divisor.
        let d = if let Some(c) = self.lc() {
            d.add(&c.sub(c))
        } else {
            d.clone()
        };
        if d.is_zero() {
            return Ok(None);
        }
        let mut out = Vec::with_capacity(self.coeffs.len());
        for c in &self.coeffs {
            ctx.tick()?;
            if c.is_zero() {
                out.push(R::zero());
                continue;
            }
            let Some(c) = c.exact_div(&d) else {
                return Ok(None);
            };
            out.push(c);
        }
        Ok(Some(Self::new(out)))
    }
}

fn trim<R: Ring>(p: &mut UPoly<R>, ctx: &Interrupt) -> Result<(), Abort> {
    while p.coeffs.last().is_some_and(Ring::is_zero) {
        ctx.tick()?;
        p.coeffs.pop();
    }
    Ok(())
}
pub(crate) fn coefficient_power<R: Ring>(a: &R, mut n: usize, ctx: &Interrupt) -> Result<R, Abort> {
    ctx.tick()?;
    let mut value = R::one();
    let mut base = a.clone();
    while n > 0 {
        if n & 1 == 1 {
            ctx.tick()?;
            value = value.mul(&base);
        }
        n >>= 1;
        if n > 0 {
            ctx.tick()?;
            base = base.mul(&base);
        }
    }
    Ok(value)
}
