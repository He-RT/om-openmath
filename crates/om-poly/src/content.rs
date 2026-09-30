//! Signed integer content and positive-leading primitive parts.
use crate::UPoly;
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
    gcd,
};

impl UPoly<Integer> {
    /// Coefficient gcd, signed so the primitive part has a positive leading coefficient.
    /// The zero polynomial has content zero.
    pub fn content(&self, ctx: &Interrupt) -> Result<Integer, Abort> {
        ctx.tick()?;
        let mut value = Integer::ZERO;
        for c in &self.coeffs {
            ctx.tick()?;
            value = gcd(&value, c);
        }
        Ok(if self.lc().is_some_and(|c| c < &Integer::ZERO) {
            -value
        } else {
            value
        })
    }
    /// Signed content and primitive part. Zero returns (0,0); constants return (c,1).
    pub fn content_pp(&self, ctx: &Interrupt) -> Result<(Integer, Self), Abort> {
        let content = self.content(ctx)?;
        if content.is_zero() {
            return Ok((content, Self::zero()));
        }
        let primitive = self
            .divide_scalar(&content, ctx)?
            .expect("invariant: the coefficient gcd divides every coefficient");
        Ok((content, primitive))
    }
    /// Remove signed content, making the nonzero leading coefficient positive.
    pub fn primitive_part(&self, ctx: &Interrupt) -> Result<Self, Abort> {
        Ok(self.content_pp(ctx)?.1)
    }
}
