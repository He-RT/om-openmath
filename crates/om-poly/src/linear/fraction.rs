//! Reduced fractions are formed only through exact domain gcd/division certificates.
use super::ExactDomain;
use om_num::ctx::{Abort, Interrupt};
/// Normalized exact fraction over an integral domain, with a positive-leading denominator.
#[derive(Clone, Debug, PartialEq)]
pub struct ExactFraction<D: ExactDomain> {
    /// Numerator, coprime to den; a zero fraction has a unit denominator.
    pub num: D,
    /// Nonzero positive-leading denominator.
    pub den: D,
}
impl<D: ExactDomain> ExactFraction<D> {
    /// Cancel an exact numerator/denominator pair; None rejects zero or invalid denominators.
    pub fn new(num: D, den: D, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        if !num.compatible(&den) {
            return Ok(None);
        }
        let (Some(num), Some(den)) = (num.canonical(ctx)?, den.canonical(ctx)?) else {
            return Ok(None);
        };
        if den.is_zero() {
            return Ok(None);
        }
        let gcd = num.gcd(&den, ctx)?;
        let (Some(mut num), Some(mut den)) = (
            num.exact_quotient(&gcd, ctx)?,
            den.exact_quotient(&gcd, ctx)?,
        ) else {
            return Ok(None);
        };
        if den.negative_leading() {
            num = num.neg();
            den = den.neg();
        }
        Ok(Some(Self { num, den }))
    }
    /// Exact canceled sum, or None for incompatible contexts/degree overflow.
    pub fn add(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        self.combine(other, false, ctx)
    }
    /// Exact canceled difference, or None for incompatible contexts/degree overflow.
    pub fn sub(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        self.combine(other, true, ctx)
    }
    fn combine(
        &self,
        other: &Self,
        subtract: bool,
        ctx: &Interrupt,
    ) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        if !self.compatible(other) {
            return Ok(None);
        }
        let (Some(a), Some(b), Some(den)) = (
            self.num.checked_mul(&other.den, ctx)?,
            other.num.checked_mul(&self.den, ctx)?,
            self.den.checked_mul(&other.den, ctx)?,
        ) else {
            return Ok(None);
        };
        Self::new(if subtract { a.sub(&b) } else { a.add(&b) }, den, ctx)
    }
    /// Exact canceled product, or None for incompatible contexts/degree overflow.
    pub fn mul(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        if !self.compatible(other) {
            return Ok(None);
        }
        let (Some(num), Some(den)) = (
            self.num.checked_mul(&other.num, ctx)?,
            self.den.checked_mul(&other.den, ctx)?,
        ) else {
            return Ok(None);
        };
        Self::new(num, den, ctx)
    }
    /// Exact canceled quotient; zero divisors or incompatible/overflow contexts yield None.
    pub fn div(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        if !self.compatible(other) {
            return Ok(None);
        }
        let (Some(num), Some(den)) = (
            self.num.checked_mul(&other.den, ctx)?,
            self.den.checked_mul(&other.num, ctx)?,
        ) else {
            return Ok(None);
        };
        Self::new(num, den, ctx)
    }
    fn compatible(&self, other: &Self) -> bool {
        self.num.compatible(&other.num)
            && self.den.compatible(&other.den)
            && self.num.compatible(&self.den)
            && other.num.compatible(&other.den)
            && self.num.compatible(&other.den)
    }
}
