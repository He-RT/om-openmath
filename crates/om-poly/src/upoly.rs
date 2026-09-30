//! Dense univariate polynomials, with a tick at every coefficient operation.
use crate::{Field, Ring};
use om_num::ctx::{Abort, Interrupt};

/// Dense coefficients in increasing degree; canonical zero has no coefficients.
#[derive(Clone, Debug, PartialEq)]
pub struct UPoly<R: Ring> {
    /// Low-degree coefficients first, with no trailing zeros after construction.
    pub coeffs: Vec<R>,
}
impl<R: Ring> UPoly<R> {
    /// Trim trailing zeros and canonicalize interior zero factories.
    /// Nonzero runtime coefficient contexts must match.
    pub fn new(mut coeffs: Vec<R>) -> Self {
        while coeffs.last().is_some_and(Ring::is_zero) {
            coeffs.pop();
        }
        for c in &mut coeffs {
            if c.is_zero() {
                *c = R::zero();
            }
        }
        Self { coeffs }
    }
    /// The zero polynomial.
    pub fn zero() -> Self {
        Self { coeffs: vec![] }
    }
    /// The constant unit polynomial.
    pub fn one() -> Self {
        Self::new(vec![R::one()])
    }
    /// Degree, or None for zero; also tolerates raw trailing zeros.
    pub fn degree(&self) -> Option<usize> {
        self.coeffs.iter().rposition(|c| !c.is_zero())
    }
    /// Leading coefficient, or None for zero.
    pub fn lc(&self) -> Option<&R> {
        self.degree().map(|d| &self.coeffs[d])
    }
    /// Whether every coefficient is zero.
    pub fn is_zero(&self) -> bool {
        self.degree().is_none()
    }
    /// Whether the polynomial is the constant unit in its coefficient context.
    pub fn is_one(&self) -> bool {
        self.degree() == Some(0) && self.coeffs[0].sub(&R::one()).is_zero()
    }
    /// Add compatible polynomials.
    pub fn add(&self, o: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        self.combine(o, false, ctx)
    }
    /// Subtract compatible polynomials.
    pub fn sub(&self, o: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        self.combine(o, true, ctx)
    }
    fn combine(&self, o: &Self, subtract: bool, ctx: &Interrupt) -> Result<Self, Abort> {
        ctx.tick()?;
        let len = self.coeffs.len().max(o.coeffs.len());
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            ctx.tick()?;
            let a = self.coeffs.get(i).cloned().unwrap_or_else(R::zero);
            let b = o.coeffs.get(i).cloned().unwrap_or_else(R::zero);
            out.push(if subtract { a.sub(&b) } else { a.add(&b) });
        }
        Ok(Self::new(out))
    }
    /// Negate all coefficients.
    pub fn neg(&self, ctx: &Interrupt) -> Result<Self, Abort> {
        ctx.tick()?;
        let mut out = Vec::with_capacity(self.coeffs.len());
        for c in &self.coeffs {
            ctx.tick()?;
            out.push(c.neg());
        }
        Ok(Self::new(out))
    }
    /// Multiply by a compatible coefficient.
    pub fn scale(&self, a: &R, ctx: &Interrupt) -> Result<Self, Abort> {
        ctx.tick()?;
        let mut out = Vec::with_capacity(self.coeffs.len());
        for c in &self.coeffs {
            ctx.tick()?;
            out.push(c.mul(a));
        }
        Ok(Self::new(out))
    }
    /// Schoolbook multiplication with exact coefficient arithmetic.
    pub fn mul(&self, o: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        ctx.tick()?;
        let (Some(a), Some(b)) = (self.degree(), o.degree()) else {
            return Ok(Self::zero());
        };
        let mut out = vec![R::zero(); a + b + 1];
        for i in 0..=a {
            ctx.tick()?;
            if self.coeffs[i].is_zero() {
                continue;
            }
            for j in 0..=b {
                ctx.tick()?;
                out[i + j] = out[i + j].add(&self.coeffs[i].mul(&o.coeffs[j]));
            }
        }
        Ok(Self::new(out))
    }
}
impl<R: Field> UPoly<R> {
    /// Long division over a field. None means zero divisor or no leading inverse.
    pub fn divrem(&self, g: &Self, ctx: &Interrupt) -> Result<Option<(Self, Self)>, Abort> {
        ctx.tick()?;
        let Some(dg) = g.degree() else {
            return Ok(None);
        };
        let Some(inv) = g.coeffs[dg].inv() else {
            return Ok(None);
        };
        let mut r = Self::new(self.coeffs.clone());
        let Some(df) = r.degree() else {
            return Ok(Some((Self::zero(), r)));
        };
        if df < dg {
            return Ok(Some((Self::zero(), r)));
        }
        let mut q = vec![R::zero(); df - dg + 1];
        while let Some(dr) = r.degree().filter(|d| *d >= dg) {
            ctx.tick()?;
            let shift = dr - dg;
            let c = r.coeffs[dr].mul(&inv);
            q[shift] = q[shift].add(&c);
            for i in 0..=dg {
                ctx.tick()?;
                r.coeffs[i + shift] = r.coeffs[i + shift].sub(&c.mul(&g.coeffs[i]));
            }
            while r.coeffs.last().is_some_and(Ring::is_zero) {
                ctx.tick()?;
                r.coeffs.pop();
            }
        }
        Ok(Some((Self::new(q), Self::new(r.coeffs))))
    }
}
