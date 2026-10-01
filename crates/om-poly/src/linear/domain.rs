//! Exact integral-domain operations for numeric and parameter Bareiss coefficients.
use crate::{EuclideanRing, MPoly, Monomial, Ring};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
    gcd,
};
/// Integral-domain adapter for certified fraction-free elimination and cancellation.
/// Implementations must preserve exact division/gcd and validate runtime ring contexts.
pub trait ExactDomain: Ring {
    /// Whether runtime coefficient contexts are compatible (constant factories may be unbound).
    fn compatible(&self, other: &Self) -> bool;
    /// Validate and normalize a coefficient; None rejects malformed representations.
    fn canonical(&self, ctx: &Interrupt) -> Result<Option<Self>, Abort>;
    /// Exact checked product; None rejects representational degree overflow.
    fn checked_mul(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort>;
    /// Exact quotient, rejecting zero divisors/nondivisibility.
    fn exact_quotient(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort>;
    /// Positive-leading gcd used to cancel fractions.
    fn gcd(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort>;
    /// Whether the leading scalar coefficient is negative.
    fn negative_leading(&self) -> bool;
    /// Whether this nonzero value contains unresolved parameters.
    fn parameter(&self) -> bool;
    /// Primitive positive-leading condition equivalent to this value being nonzero.
    fn assumption(&self, ctx: &Interrupt) -> Result<Self, Abort>;
}
impl ExactDomain for Integer {
    fn compatible(&self, _: &Self) -> bool {
        true
    }
    fn canonical(&self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        Ok(Some(self.clone()))
    }
    fn checked_mul(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        Ok(Some(self * other))
    }
    fn exact_quotient(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        Ok(EuclideanRing::exact_div(self, other))
    }
    fn gcd(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        ctx.tick()?;
        Ok(gcd(self, other))
    }
    fn negative_leading(&self) -> bool {
        self < &Integer::ZERO
    }
    fn parameter(&self) -> bool {
        false
    }
    fn assumption(&self, ctx: &Interrupt) -> Result<Self, Abort> {
        ctx.tick()?;
        Ok(self.clone().max(-self))
    }
}
impl ExactDomain for MPoly<Integer> {
    fn compatible(&self, other: &Self) -> bool {
        self.nvars == 0
            || other.nvars == 0
            || (self.nvars == other.nvars && self.order == other.order)
    }
    fn canonical(&self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        for (m, _) in &self.terms {
            ctx.tick()?;
            if m.exps.len() != self.nvars
                || Monomial::new(m.exps.iter().copied()).as_ref() != Some(m)
            {
                return Ok(None);
            }
        }
        Ok(Some(Self::new(
            self.nvars,
            self.terms.clone(),
            self.order,
            ctx,
        )?))
    }
    fn checked_mul(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        self.mul(other, ctx)
    }
    fn exact_quotient(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        self.exact_div(other, ctx)
    }
    fn gcd(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        self.subresultant_gcd(other, ctx)
    }
    fn negative_leading(&self) -> bool {
        self.terms.first().is_some_and(|(_, c)| c < &Integer::ZERO)
    }
    fn parameter(&self) -> bool {
        self.terms.iter().any(|(m, _)| m.deg > 0)
    }
    fn assumption(&self, ctx: &Interrupt) -> Result<Self, Abort> {
        ctx.tick()?;
        let mut content = Integer::ZERO;
        for (_, c) in &self.terms {
            ctx.tick()?;
            content = gcd(&content, c);
        }
        if content.is_zero() {
            return Ok(self.clone());
        }
        if self.negative_leading() {
            content = -content;
        }
        let mut terms = vec![];
        for (m, c) in &self.terms {
            ctx.tick()?;
            terms.push((m.clone(), c / &content));
        }
        Self::new(self.nvars, terms, self.order, ctx)
    }
}
