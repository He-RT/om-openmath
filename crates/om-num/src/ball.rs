//! Certified intervals represented by a finite center and an outward-rounded radius.

use crate::{BigFloat, Integer, Rational};
use dashu::{
    base::BitTest,
    float::{
        FBig,
        round::mode::{Down, HalfEven, Up},
    },
};

type Lower = FBig<Down, 2>;
type Upper = FBig<Up, 2>;

/// Real ball [mid-rad, mid+rad], with rad>=0 and prec>0.
#[derive(Clone, Debug)]
pub struct Ball {
    /// Finite binary midpoint.
    pub mid: BigFloat,
    /// Nonnegative outward-rounded radius; +inf denotes the whole line.
    pub rad: BigFloat,
    /// Midpoint precision in bits.
    pub prec: u32,
}

/// Rectangular complex ball.
#[derive(Clone, Debug)]
pub struct CBall {
    /// Real component interval.
    pub re: Ball,
    /// Imaginary component interval.
    pub im: Ball,
}

impl Ball {
    /// Enclose an exact rational at the requested positive bit precision.
    pub fn exact(q: &Rational, prec: u32) -> Ball {
        assert!(prec > 0, "ball precision must be positive");
        let work = work_precision(prec);
        Self::from_bounds(
            q.to_float::<Down, 2>(work).value(),
            q.to_float::<Up, 2>(work).value(),
            prec,
        )
    }
    /// The whole real line.
    pub fn whole(prec: u32) -> Ball {
        assert!(prec > 0, "ball precision must be positive");
        Self {
            mid: BigFloat::ZERO.with_precision(prec as usize).value(),
            rad: BigFloat::INFINITY,
            prec,
        }
    }
    /// Enclose all pairwise sums.
    pub fn add(&self, o: &Ball) -> Ball {
        self.binary(o, Operation::Add)
    }
    /// Enclose all pairwise differences.
    pub fn sub(&self, o: &Ball) -> Ball {
        self.binary(o, Operation::Sub)
    }
    /// Enclose all pairwise products.
    pub fn mul(&self, o: &Ball) -> Ball {
        self.binary(o, Operation::Mul)
    }
    /// Enclose all quotients; a zero-containing denominator yields the whole line.
    pub fn div(&self, o: &Ball) -> Ball {
        self.binary(o, Operation::Div)
    }
    /// Enclose square roots; negative-containing inputs yield the whole line.
    pub fn sqrt(&self) -> Ball {
        self.validate();
        if self.rad.repr().is_infinite() {
            return Self::whole(self.prec);
        }
        let (lo, hi) = self.bounds(work_precision(self.prec));
        if lo < Lower::ZERO {
            return Self::whole(self.prec);
        }
        Self::from_bounds(lo.sqrt(), hi.sqrt(), self.prec)
    }
    /// Whether zero lies in the ball.
    pub fn contains_zero(&self) -> bool {
        !self.excludes_zero()
    }
    /// Whether the ball proves nonzero.
    pub fn excludes_zero(&self) -> bool {
        self.validate();
        let magnitude = if self.mid < BigFloat::ZERO {
            -&self.mid
        } else {
            self.mid.clone()
        };
        magnitude > self.rad
    }
    /// Lossy binary64 midpoint approximation.
    pub fn to_f64(&self) -> f64 {
        self.mid.to_f64().value()
    }

    fn validate(&self) {
        assert!(self.prec > 0, "ball precision must be positive");
        assert!(self.mid.repr().is_finite(), "ball midpoint must be finite");
        assert!(
            self.rad >= BigFloat::ZERO,
            "ball radius must be nonnegative"
        );
    }

    fn bounds(&self, work: usize) -> (Lower, Upper) {
        let rad = self
            .rad
            .clone()
            .with_rounding::<Up>()
            .with_precision(work)
            .value();
        let lo = self
            .mid
            .clone()
            .with_rounding::<Down>()
            .with_precision(work)
            .value()
            - rad.clone().with_rounding::<Down>();
        let hi = self
            .mid
            .clone()
            .with_rounding::<Up>()
            .with_precision(work)
            .value()
            + rad;
        (lo, hi)
    }

    fn from_bounds(lo: Lower, hi: Upper, prec: u32) -> Ball {
        if lo.repr().is_infinite() || hi.repr().is_infinite() {
            return Self::whole(prec);
        }
        assert!(lo <= hi, "lower endpoint must not exceed upper endpoint");
        let mid = ((lo.clone().with_rounding::<HalfEven>()
            + hi.clone().with_rounding::<HalfEven>())
            / 2_u8)
            .with_precision(prec as usize)
            .value();
        // The stored midpoint is exact as a dyadic value; charge its recentering error.
        let center = mid
            .clone()
            .with_rounding::<Up>()
            .with_precision(work_precision(prec))
            .value();
        let radius = (&center - lo.with_rounding::<Up>())
            .max(hi - center)
            .with_precision(prec.min(64) as usize)
            .value();
        Self {
            mid,
            rad: radius.with_rounding::<HalfEven>(),
            prec,
        }
    }

    fn binary(&self, o: &Ball, operation: Operation) -> Ball {
        self.validate();
        o.validate();
        let prec = self.prec.min(o.prec);
        if self.rad.repr().is_infinite() || o.rad.repr().is_infinite() {
            return Self::whole(prec);
        }
        let work = work_precision(prec);
        let ((alo, ahi), (blo, bhi)) = (self.bounds(work), o.bounds(work));
        match operation {
            Operation::Add => Self::from_bounds(alo + blo, ahi + bhi, prec),
            Operation::Sub => Self::from_bounds(
                alo - bhi.with_rounding::<Down>(),
                ahi - blo.with_rounding::<Up>(),
                prec,
            ),
            Operation::Mul | Operation::Div => {
                if matches!(operation, Operation::Div) && blo <= Lower::ZERO && bhi >= Upper::ZERO {
                    return Self::whole(prec);
                }
                let left = [alo, ahi.with_rounding::<Down>()];
                let right = [blo, bhi.with_rounding::<Down>()];
                let mut low: Option<Lower> = None;
                let mut high: Option<Upper> = None;
                for a in &left {
                    for b in &right {
                        let down = if matches!(operation, Operation::Mul) {
                            a * b
                        } else {
                            a / b
                        };
                        let (a, b) = (
                            a.clone().with_rounding::<Up>(),
                            b.clone().with_rounding::<Up>(),
                        );
                        let up = if matches!(operation, Operation::Mul) {
                            a * b
                        } else {
                            a / b
                        };
                        low = Some(low.map_or(down.clone(), |old| old.min(down)));
                        high = Some(high.map_or(up.clone(), |old| old.max(up)));
                    }
                }
                Self::from_bounds(
                    low.expect("invariant: four corners"),
                    high.expect("invariant: four corners"),
                    prec,
                )
            }
        }
    }

    fn square(&self) -> Ball {
        self.validate();
        if self.rad.repr().is_infinite() {
            return Self::whole(self.prec);
        }
        let (lo, hi) = self.bounds(work_precision(self.prec));
        let hi_down = hi.clone().with_rounding::<Down>();
        let low = if lo <= Lower::ZERO && hi >= Upper::ZERO {
            Lower::ZERO
                .with_precision(work_precision(self.prec))
                .value()
        } else {
            (&lo * &lo).min(&hi_down * &hi_down)
        };
        let lo_up = lo.with_rounding::<Up>();
        Self::from_bounds(low, (&lo_up * &lo_up).max(&hi * &hi), self.prec)
    }
}

#[derive(Clone, Copy)]
enum Operation {
    Add,
    Sub,
    Mul,
    Div,
}

fn work_precision(prec: u32) -> usize {
    (prec as usize).saturating_add(8)
}

impl CBall {
    /// Enclose an exact complex value.
    pub fn exact(re: &Rational, im: &Rational, prec: u32) -> CBall {
        Self {
            re: Ball::exact(re, prec),
            im: Ball::exact(im, prec),
        }
    }
    /// Add complex balls.
    pub fn add(&self, o: &CBall) -> CBall {
        Self {
            re: self.re.add(&o.re),
            im: self.im.add(&o.im),
        }
    }
    /// Subtract complex balls.
    pub fn sub(&self, o: &CBall) -> CBall {
        Self {
            re: self.re.sub(&o.re),
            im: self.im.sub(&o.im),
        }
    }
    /// Multiply complex balls.
    pub fn mul(&self, o: &CBall) -> CBall {
        Self {
            re: self.re.mul(&o.re).sub(&self.im.mul(&o.im)),
            im: self.re.mul(&o.im).add(&self.im.mul(&o.re)),
        }
    }
    /// Divide complex balls; zero-containing squared magnitude yields whole components.
    pub fn div(&self, o: &CBall) -> CBall {
        let norm = o.re.square().add(&o.im.square());
        Self {
            re: self.re.mul(&o.re).add(&self.im.mul(&o.im)).div(&norm),
            im: self.im.mul(&o.re).sub(&self.re.mul(&o.im)).div(&norm),
        }
    }
    /// Signed integer power with interval error propagation.
    /// Zero exponent of a zero-containing ball yields whole components.
    pub fn pow_int(&self, e: &Integer) -> CBall {
        let prec = self.re.prec.min(self.im.prec);
        if e.is_zero() && self.re.contains_zero() && self.im.contains_zero() {
            return Self {
                re: Ball::whole(prec),
                im: Ball::whole(prec),
            };
        }
        let one = Self::exact(&Rational::ONE, &Rational::ZERO, prec);
        let mut base = if e < &Integer::ZERO {
            one.div(self)
        } else {
            self.clone()
        };
        let exponent = e.clone().into_parts().1;
        let mut result = one;
        for bit in 0..exponent.bit_len() {
            if exponent.bit(bit) {
                result = result.mul(&base);
            }
            if bit + 1 < exponent.bit_len() {
                base = base.mul(&base);
            }
        }
        result
    }
}

#[cfg(test)]
#[path = "ball_tests.rs"]
mod tests;

#[path = "ball_complex_elementary.rs"]
mod complex_elementary;
#[path = "ball_elementary.rs"]
mod elementary;
#[cfg(test)]
#[path = "ball_elementary_tests.rs"]
mod elementary_tests;
