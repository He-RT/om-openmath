//! Certified elementary real functions.
use super::*;
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

impl Ball {
    /// Enclose exp over the input interval.
    pub fn exp(&self) -> Ball {
        self.validate();
        if self.rad.repr().is_infinite() {
            return Self::whole(self.prec);
        }
        let (lo, hi) = self.bounds(work_precision(self.prec));
        // Dashu's exp and ln certify their directed rounding through Ziv iterations.
        Self::from_bounds(lo.exp(), hi.exp(), self.prec)
    }
    /// Enclose ln; inputs containing nonpositive values yield the whole line.
    pub fn ln(&self) -> Ball {
        self.validate();
        if self.rad.repr().is_infinite() {
            return Self::whole(self.prec);
        }
        let (lo, hi) = self.bounds(work_precision(self.prec));
        if lo <= Lower::ZERO {
            return Self::whole(self.prec);
        }
        Self::from_bounds(lo.ln(), hi.ln(), self.prec)
    }
    /// Enclose sin, including extrema inside the input interval.
    pub fn sin(&self) -> Ball {
        trig(self, false)
    }
    /// Enclose cos, including extrema inside the input interval.
    pub fn cos(&self) -> Ball {
        trig(self, true)
    }
    /// Enclose the real principal arctangent.
    pub fn atan(&self) -> Ball {
        self.validate();
        if self.rad.repr().is_infinite() {
            let half_pi = Self::pi(self.prec).div(&integer(2, self.prec));
            let (_, hi) = half_pi.bounds(work_precision(self.prec));
            return Self::from_bounds(-hi.clone().with_rounding::<Down>(), hi, self.prec);
        }
        let prec = self.prec.saturating_add(32);
        let (lo, hi) = self.bounds(work_precision(prec));
        let lower = atan_point(lo.with_rounding::<HalfEven>(), prec)
            .bounds(work_precision(prec))
            .0;
        let upper = atan_point(hi.with_rounding::<HalfEven>(), prec)
            .bounds(work_precision(prec))
            .1;
        Self::from_bounds(lower, upper, self.prec)
    }
    /// Certified Machin-formula pi, cached by bit precision.
    pub fn pi(prec: u32) -> Ball {
        assert!(prec > 0, "ball precision must be positive");
        static CACHE: OnceLock<Mutex<BTreeMap<u32, Ball>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
        if let Some(pi) = cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&prec)
            .cloned()
        {
            return pi;
        }
        let work = prec.saturating_add(32);
        let a = atan_series(&Self::exact(
            &Rational::from_parts(1.into(), 5_u8.into()),
            work,
        ));
        let b = atan_series(&Self::exact(
            &Rational::from_parts(1.into(), 239_u16.into()),
            work,
        ));
        let pi = repack(
            &a.mul(&integer(16, work)).sub(&b.mul(&integer(4, work))),
            prec,
        );
        cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(prec, pi.clone());
        pi
    }
}

pub(super) fn integer(n: i64, prec: u32) -> Ball {
    Ball::exact(&n.into(), prec)
}
pub(super) fn point(x: BigFloat, prec: u32) -> Ball {
    Ball {
        mid: x,
        rad: BigFloat::ZERO.with_precision(prec as usize).value(),
        prec,
    }
}
pub(super) fn negate(b: &Ball) -> Ball {
    Ball {
        mid: -&b.mid,
        rad: b.rad.clone(),
        prec: b.prec,
    }
}
pub(super) fn repack(b: &Ball, prec: u32) -> Ball {
    if b.rad.repr().is_infinite() {
        return Ball::whole(prec);
    }
    let (lo, hi) = b.bounds(work_precision(b.prec.max(prec)));
    Ball::from_bounds(lo, hi, prec)
}
fn magnitude(b: &Ball) -> Upper {
    let (lo, hi) = b.bounds(work_precision(b.prec));
    (-lo.with_rounding::<Up>()).max(hi)
}
fn widen(b: &Ball, error: Upper) -> Ball {
    let rad = (b.rad.clone().with_rounding::<Up>() + error)
        .with_precision(b.prec.min(64) as usize)
        .value();
    Ball {
        mid: b.mid.clone(),
        rad: rad.with_rounding::<HalfEven>(),
        prec: b.prec,
    }
}

fn atan_point(x: BigFloat, prec: u32) -> Ball {
    let negative = x < BigFloat::ZERO;
    let abs = if negative { -x } else { x };
    let one = integer(1, prec);
    let abs_ball = point(abs.clone(), prec);
    let result = if abs > BigFloat::ONE {
        Ball::pi(prec)
            .div(&integer(2, prec))
            .sub(&atan_reduced(&one.div(&abs_ball)))
    } else {
        atan_reduced(&abs_ball)
    };
    if negative { negate(&result) } else { result }
}

fn atan_reduced(x: &Ball) -> Ball {
    let mut reduced = x.clone();
    let one = integer(1, x.prec);
    // Two half-angle reductions map [-1,1] strictly inside [-1/4,1/4].
    for _ in 0..2 {
        reduced = reduced.div(&one.add(&one.add(&reduced.square()).sqrt()));
    }
    atan_series(&reduced).mul(&integer(4, x.prec))
}

fn atan_series(x: &Ball) -> Ball {
    debug_assert!(
        magnitude(x)
            < Upper::from(1)
                .with_precision(work_precision(x.prec))
                .value()
                / 4_u8
    );
    let epsilon = Upper::from_parts(Integer::ONE, -(x.prec as isize) - 8);
    let square = x.square();
    let mut power = x.clone();
    let mut sum = x.clone();
    for n in 1_u64.. {
        power = negate(&power.mul(&square));
        let term = power.div(&Ball::exact(&Integer::from(2 * n + 1).into(), x.prec));
        let tail = magnitude(&term);
        // Alternating decreasing terms: remainder after the previous term <= |next term|.
        if tail <= epsilon {
            return widen(&sum, tail);
        }
        sum = sum.add(&term);
    }
    unreachable!("invariant: reduced atan series converges")
}

fn trig(input: &Ball, cosine: bool) -> Ball {
    input.validate();
    let unit = || Ball {
        mid: BigFloat::ZERO.with_precision(input.prec as usize).value(),
        rad: BigFloat::ONE.with_precision(input.prec as usize).value(),
        prec: input.prec,
    };
    if input.rad.repr().is_infinite() || input.rad >= BigFloat::ONE {
        return unit();
    }
    if input.mid == BigFloat::ZERO {
        let value = if cosine {
            integer(1, input.prec)
        } else {
            integer(0, input.prec)
        };
        return clamp_unit(&widen(&value, input.rad.clone().with_rounding::<Up>()));
    }
    // Pi uncertainty is amplified by the reduction multiple: add the argument's magnitude bits.
    let magnitude_bits = input
        .mid
        .repr()
        .exponent()
        .saturating_add(input.mid.repr().significand().bit_len() as isize)
        .max(0);
    let Some(work) = u32::try_from(magnitude_bits)
        .ok()
        .and_then(|m| input.prec.checked_add(m))
        .and_then(|p| p.checked_add(32))
    else {
        return unit();
    };
    let period = Ball::pi(work).mul(&integer(2, work));
    let multiple = (&input.mid / &period.mid).to_int().value();
    let phase =
        point(input.mid.clone(), work).sub(&period.mul(&Ball::exact(&multiple.into(), work)));
    if magnitude(&phase) > Upper::from(4) {
        return unit();
    }
    let phase = repack(&phase, input.prec.saturating_add(32));
    let value = trig_series(&phase, cosine);
    let widened = widen(&value, input.rad.clone().with_rounding::<Up>());
    repack(&clamp_unit(&widened), input.prec)
}

fn trig_series(x: &Ball, cosine: bool) -> Ball {
    let square = x.square();
    let epsilon = Upper::from_parts(Integer::ONE, -(x.prec as isize) - 8);
    let mut term = if cosine {
        integer(1, x.prec)
    } else {
        x.clone()
    };
    let mut sum = term.clone();
    for n in 1_u64.. {
        let degree = if cosine { 2 * n } else { 2 * n + 1 };
        let denominator = Integer::from(degree) * Integer::from(degree - 1);
        term = negate(
            &term
                .mul(&square)
                .div(&Ball::exact(&denominator.into(), x.prec)),
        );
        let tail = magnitude(&term);
        // With |x|<=4, the alternating terms decrease from degree 8 onward.
        if n >= 4 && tail <= epsilon {
            return widen(&sum, tail);
        }
        sum = sum.add(&term);
    }
    unreachable!("invariant: reduced sine/cosine series converges")
}

fn clamp_unit(b: &Ball) -> Ball {
    let (lo, hi) = b.bounds(work_precision(b.prec));
    Ball::from_bounds(lo.max(Lower::NEG_ONE), hi.min(Upper::ONE), b.prec)
}
