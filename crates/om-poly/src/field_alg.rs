//! Formal derivatives and monic normalization for the first field-GCD consumers.
use crate::{Field, Ring, UPoly};
use om_num::ctx::{Abort, Interrupt};

impl<R: Ring> UPoly<R> {
    /// Formal derivative in the coefficient ring's characteristic, without numerical division.
    pub fn derivative(&self, ctx: &Interrupt) -> Result<Self, Abort> {
        ctx.tick()?;
        let zero = context_zero(&self.coeffs, ctx)?;
        let mut out = Vec::with_capacity(self.coeffs.len().saturating_sub(1));
        for (i, c) in self.coeffs.iter().enumerate().skip(1) {
            ctx.tick()?;
            let mut exponent = i;
            let mut term = c.add(&zero);
            let mut value = zero.clone();
            // Doubling avoids casting the degree into a coefficient or a machine integer.
            while exponent > 0 {
                ctx.tick()?;
                if exponent & 1 != 0 {
                    value = value.add(&term);
                }
                exponent >>= 1;
                if exponent > 0 {
                    term = term.add(&term);
                }
            }
            out.push(value);
        }
        Ok(Self::new(out))
    }
}
impl<R: Field> UPoly<R> {
    /// Divide by the leading coefficient; zero returns zero, a failed inverse returns None.
    pub fn monic(&self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        let Some(lc) = self.lc() else {
            return Ok(Some(Self::zero()));
        };
        let zero = context_zero(&self.coeffs, ctx)?;
        let Some(inverse) = lc.add(&zero).inv() else {
            return Ok(None);
        };
        Ok(Some(self.scale(&inverse, ctx)?))
    }
    /// Monic Euclidean GCD over a compatible field, with gcd(0,0)=0.
    /// None means an unavailable field inverse.
    pub fn monic_gcd(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        let mut a = self.clone();
        let mut b = other.clone();
        while !b.is_zero() {
            ctx.tick()?;
            let Some(monic) = b.monic(ctx)? else {
                return Ok(None);
            };
            let Some((_, remainder)) = a.divrem(&monic, ctx)? else {
                return Ok(None);
            };
            a = monic;
            b = remainder;
        }
        a.monic(ctx)
    }
}
fn context_zero<R: Ring>(coefficients: &[R], ctx: &Interrupt) -> Result<R, Abort> {
    let mut zero = R::zero();
    for c in coefficients {
        ctx.tick()?;
        zero = zero.add(&c.sub(c));
    }
    Ok(zero)
}
