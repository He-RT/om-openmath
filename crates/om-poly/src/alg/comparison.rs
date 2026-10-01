//! Exact root ranks and separating intervals establish equality, signs and real order.
use super::{
    Algebraic,
    bounds::{overlap, real_bounds},
    real_roots,
};
use om_num::{
    BitTest, Rational,
    ctx::{Abort, Interrupt},
};
use std::cmp::Ordering;
impl Algebraic {
    /// Exact real sign, or None for a nonreal value or failed certification.
    pub fn real_sign(&self, ctx: &Interrupt) -> Result<Option<i8>, Abort> {
        ctx.tick()?;
        if let Self::Rational(q) = self {
            return Ok(Some(if q > &Rational::ZERO {
                1
            } else if q < &Rational::ZERO {
                -1
            } else {
                0
            }));
        }
        if matches!(self, Self::Complex(_)) {
            return Ok(None);
        }
        let mut a = self.clone();
        let mut bits = relative_precision(self);
        loop {
            ctx.tick()?;
            let Some(next) = a.refined(bits, ctx)? else {
                return Ok(None);
            };
            a = next;
            let iv = real_bounds(&a).expect("invariant: real sign input");
            if iv.0 > Rational::ZERO {
                return Ok(Some(1));
            }
            if iv.1 < Rational::ZERO {
                return Ok(Some(-1));
            }
            let Some(next) = super::next_precision(bits, false) else {
                return Ok(None);
            };
            bits = next;
        }
    }
    /// Exact ordering for real values. Equal minimal polynomials compare exact root ranks.
    pub fn cmp_real(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Ordering>, Abort> {
        ctx.tick()?;
        if real_bounds(self).is_none() || real_bounds(other).is_none() {
            return Ok(None);
        }
        if let (Self::Rational(a), Self::Rational(b)) = (self, other) {
            return Ok(Some(a.cmp(b)));
        }
        if let (Self::Real(a), Self::Real(b)) = (self, other)
            && a.minpoly == b.minpoly
        {
            let (Some(a), Some(b)) = (rank(self, ctx)?, rank(other, ctx)?) else {
                return Ok(None);
            };
            return Ok(Some(a.cmp(&b)));
        }
        let (mut a, mut b) = (self.clone(), other.clone());
        let mut bits = relative_precision(self).max(relative_precision(other));
        loop {
            ctx.tick()?;
            let av = real_bounds(&a).expect("invariant: real comparison input");
            let bv = real_bounds(&b).expect("invariant: real comparison input");
            if av.1 < bv.0 {
                return Ok(Some(Ordering::Less));
            }
            if bv.1 < av.0 {
                return Ok(Some(Ordering::Greater));
            }
            let (Some(an), Some(bn)) = (a.refined(bits, ctx)?, b.refined(bits, ctx)?) else {
                return Ok(None);
            };
            a = an;
            b = bn;
            let Some(next) = super::next_precision(bits, false) else {
                return Ok(None);
            };
            bits = next;
        }
    }
    /// Exact algebraic equality for certified values, or None if real comparison aborts certification.
    pub fn equals(&self, other: &Self, ctx: &Interrupt) -> Result<Option<bool>, Abort> {
        ctx.tick()?;
        match (self, other) {
            (Self::Complex(a), Self::Complex(b)) => {
                Ok(Some(a.minpoly == b.minpoly && a.index == b.index))
            }
            (Self::Complex(_), _) | (_, Self::Complex(_)) => Ok(Some(false)),
            _ => Ok(self.cmp_real(other, ctx)?.map(|c| c == Ordering::Equal)),
        }
    }
}
fn relative_precision(value: &Algebraic) -> u32 {
    let Algebraic::Real(a) = value else {
        return 32;
    };
    let width = &a.iv.1 - &a.iv.0;
    let bits = width
        .denominator()
        .bit_len()
        .saturating_sub(width.numerator().bit_len())
        .saturating_add(2);
    u32::try_from(bits).unwrap_or(u32::MAX).max(32)
}
fn rank(value: &Algebraic, ctx: &Interrupt) -> Result<Option<usize>, Abort> {
    let Algebraic::Real(a) = value else {
        return Ok(None);
    };
    let Some(mut roots) = real_roots(&a.minpoly, ctx)? else {
        return Ok(None);
    };
    let mut current = value.clone();
    let mut bits = 32;
    loop {
        ctx.tick()?;
        let iv = real_bounds(&current).expect("invariant: real rank input");
        let mut found = None;
        let mut ambiguous = false;
        for (i, root) in roots.iter().enumerate() {
            ctx.tick()?;
            let rv = real_bounds(root).expect("invariant: real root list");
            if overlap(&iv, &rv) {
                if found.is_some() {
                    ambiguous = true;
                    break;
                }
                found = Some(i);
            }
        }
        if !ambiguous {
            return Ok(found);
        }
        let Some(next) = current.refined(bits, ctx)? else {
            return Ok(None);
        };
        current = next;
        for root in &mut roots {
            ctx.tick()?;
            let Some(next) = root.refined(bits, ctx)? else {
                return Ok(None);
            };
            *root = next;
        }
        let Some(next) = super::next_precision(bits, false) else {
            return Ok(None);
        };
        bits = next;
    }
}
