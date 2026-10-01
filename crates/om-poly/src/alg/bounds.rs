//! Exact endpoint tests keep root identity independent of numerical midpoint guesses.
use super::{Algebraic, Poly};
use crate::{RootInterval, refine};
use dashu::float::round::mode::Up;
use om_num::{
    Ball, BigFloat, CBall, Integer, Rational,
    ctx::{Abort, Interrupt},
};
#[derive(Clone)]
pub(super) struct BoxBounds {
    pub re: (Rational, Rational),
    pub im: (Rational, Rational),
}
pub(super) fn dyadic(value: &BigFloat) -> Rational {
    let rep = value.repr();
    let q = Rational::from(rep.significand().clone());
    if rep.exponent() >= 0 {
        q * Rational::from(Integer::ONE << rep.exponent() as usize)
    } else {
        q / Rational::from(Integer::ONE << rep.exponent().unsigned_abs())
    }
}
pub(super) fn ball_bounds(b: &CBall) -> Option<BoxBounds> {
    if [&b.re, &b.im]
        .iter()
        .any(|b| !b.mid.repr().is_finite() || !b.rad.repr().is_finite())
    {
        return None;
    }
    let pair = |b: &Ball| {
        let m = dyadic(&b.mid);
        let r = dyadic(&b.rad);
        (&m - &r, m + r)
    };
    Some(BoxBounds {
        re: pair(&b.re),
        im: pair(&b.im),
    })
}
pub(super) fn overlap(a: &(Rational, Rational), b: &(Rational, Rational)) -> bool {
    a.0 <= b.1 && b.0 <= a.1
}
impl BoxBounds {
    pub fn intersects(&self, b: &Self) -> bool {
        overlap(&self.re, &b.re) && overlap(&self.im, &b.im)
    }
    pub fn conjugate(&self) -> Self {
        Self {
            re: self.re.clone(),
            im: (-&self.im.1, -&self.im.0),
        }
    }
}
pub(super) fn interval(iv: &(Rational, Rational)) -> RootInterval {
    RootInterval {
        lo: iv.0.clone(),
        hi: iv.1.clone(),
        exact: iv.0 == iv.1,
    }
}
pub(super) fn refine_iv(
    f: &Poly,
    iv: &(Rational, Rational),
    bits: u32,
    ctx: &Interrupt,
) -> Result<Option<(Rational, Rational)>, Abort> {
    Ok(refine(f, &interval(iv), bits, ctx)?.map(|iv| (iv.lo, iv.hi)))
}
pub(super) fn real_bounds(a: &Algebraic) -> Option<(Rational, Rational)> {
    match a {
        Algebraic::Rational(q) => Some((q.clone(), q.clone())),
        Algebraic::Real(a) => Some(a.iv.clone()),
        Algebraic::Complex(_) => None,
    }
}
pub(super) fn real_ball(iv: &(Rational, Rational), bits: u32) -> CBall {
    let center = (&iv.0 + &iv.1) / Rational::from(2);
    let r = (&iv.1 - &iv.0) / Rational::from(2);
    let mut re = Ball::exact(&center, bits);
    re.rad =
        (re.rad.with_rounding::<Up>() + r.to_float::<Up, 2>(bits as usize).value()).with_rounding();
    CBall {
        re,
        im: Ball::exact(&Rational::ZERO, bits),
    }
}
pub(super) fn product(a: &(Rational, Rational), b: &(Rational, Rational)) -> (Rational, Rational) {
    let values = [&a.0 * &b.0, &a.0 * &b.1, &a.1 * &b.0, &a.1 * &b.1];
    let lo = values
        .iter()
        .min()
        .expect("invariant: four interval corners")
        .clone();
    let hi = values
        .iter()
        .max()
        .expect("invariant: four interval corners")
        .clone();
    (lo, hi)
}
