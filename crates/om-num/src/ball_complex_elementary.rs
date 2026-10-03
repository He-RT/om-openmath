//! Principal complex elementary functions with rectangular enclosures.
use super::elementary::{integer, negate, point, repack};
use super::*;

impl CBall {
    /// Enclose the complex exponential.
    pub fn exp(&self) -> CBall {
        let scale = self.re.exp();
        Self {
            re: scale.mul(&self.im.cos()),
            im: scale.mul(&self.im.sin()),
        }
    }
    /// Enclose principal logarithms, including negative-axis branch-cut uncertainty.
    pub fn ln(&self) -> CBall {
        let prec = self.re.prec.min(self.im.prec);
        if unbounded(self) || (self.re.contains_zero() && self.im.contains_zero()) {
            return whole(prec);
        }
        Self {
            re: self.log_magnitude(),
            im: argument(self, prec),
        }
    }
    /// Enclose ln(|z|), without computing an argument or choosing a logarithm branch.
    /// Unbounded rectangles and rectangles containing the origin return the whole line.
    pub fn log_magnitude(&self) -> Ball {
        let prec = self.re.prec.min(self.im.prec);
        if unbounded(self) || (self.re.contains_zero() && self.im.contains_zero()) {
            return Ball::whole(prec);
        }
        self.re
            .square()
            .add(&self.im.square())
            .ln()
            .div(&integer(2, prec))
    }
    /// Enclose principal square roots, with nonnegative real part.
    pub fn sqrt(&self) -> CBall {
        let prec = self.re.prec.min(self.im.prec);
        if unbounded(self) {
            return whole(prec);
        }
        let work = prec.saturating_add(32);
        let x = repack(&self.re, work);
        let y = repack(&self.im, work);
        let r = sqrt_nonnegative(&x.square().add(&y.square()));
        let u = sqrt_nonnegative(&r.add(&x).div(&integer(2, work)));
        let vmag = sqrt_nonnegative(&r.sub(&x).div(&integer(2, work)));
        let (xlo, xhi) = x.bounds(work_precision(work));
        let (ylo, yhi) = y.bounds(work_precision(work));
        let (u, v) = if xlo >= Lower::ZERO && u.excludes_zero() {
            let v = y.div(&u.mul(&integer(2, work)));
            (u, v)
        } else {
            // Stable real component on the left half-plane; subtraction r+x cancels there.
            let u = if xhi < Upper::ZERO && vmag.excludes_zero() {
                abs_ball(&y).div(&vmag.mul(&integer(2, work)))
            } else {
                u
            };
            let v = if ylo >= Lower::ZERO {
                vmag
            } else if yhi < Upper::ZERO {
                negate(&vmag)
            } else {
                let (_, hi) = vmag.bounds(work_precision(work));
                Ball::from_bounds(-hi.clone().with_rounding::<Down>(), hi, work)
            };
            (u, v)
        };
        Self {
            re: repack(&u, prec),
            im: repack(&v, prec),
        }
    }
    /// Enclose the complex sine.
    pub fn sin(&self) -> CBall {
        let (cosh, sinh) = hyperbolic(&self.im);
        Self {
            re: self.re.sin().mul(&cosh),
            im: self.re.cos().mul(&sinh),
        }
    }
    /// Enclose the complex cosine.
    pub fn cos(&self) -> CBall {
        let (cosh, sinh) = hyperbolic(&self.im);
        Self {
            re: self.re.cos().mul(&cosh),
            im: negate(&self.re.sin().mul(&sinh)),
        }
    }
}

fn whole(prec: u32) -> CBall {
    CBall {
        re: Ball::whole(prec),
        im: Ball::whole(prec),
    }
}
fn unbounded(z: &CBall) -> bool {
    z.re.validate();
    z.im.validate();
    z.re.rad.repr().is_infinite() || z.im.rad.repr().is_infinite()
}
fn hyperbolic(x: &Ball) -> (Ball, Ball) {
    let (a, b) = (x.exp(), negate(x).exp());
    (
        a.add(&b).div(&integer(2, x.prec)),
        a.sub(&b).div(&integer(2, x.prec)),
    )
}

fn sqrt_nonnegative(b: &Ball) -> Ball {
    if b.rad.repr().is_infinite() {
        return Ball::whole(b.prec);
    }
    // Norm and r±x are provably nonnegative, even when dependency/rounding widens the enclosure.
    let (lo, hi) = b.bounds(work_precision(b.prec));
    Ball::from_bounds(
        lo.max(Lower::ZERO).sqrt(),
        hi.max(Upper::ZERO).sqrt(),
        b.prec,
    )
}

fn abs_ball(b: &Ball) -> Ball {
    let (lo, hi) = b.bounds(work_precision(b.prec));
    let low = if lo <= Lower::ZERO && hi >= Upper::ZERO {
        Lower::ZERO.with_precision(work_precision(b.prec)).value()
    } else {
        lo.clone().max(-hi.clone().with_rounding::<Down>())
    };
    let high = (-lo.with_rounding::<Up>()).max(hi);
    Ball::from_bounds(low, high, b.prec)
}

fn argument(z: &CBall, prec: u32) -> Ball {
    let work = prec.saturating_add(32);
    let ((xlo, xhi), (ylo, yhi)) = (
        z.re.bounds(work_precision(work)),
        z.im.bounds(work_precision(work)),
    );
    let pi = Ball::pi(work);
    if (xlo < Lower::ZERO && ylo < Lower::ZERO && yhi >= Upper::ZERO)
        || (xlo <= Lower::ZERO && xhi >= Upper::ZERO && ylo <= Lower::ZERO && yhi >= Upper::ZERO)
    {
        let (_, hi) = pi.bounds(work_precision(work));
        return Ball::from_bounds(-hi.clone().with_rounding::<Down>(), hi, prec);
    }
    let mut low: Option<Lower> = None;
    let mut high: Option<Upper> = None;
    for x in [
        xlo.with_rounding::<HalfEven>(),
        xhi.with_rounding::<HalfEven>(),
    ] {
        for y in [
            ylo.clone().with_rounding::<HalfEven>(),
            yhi.clone().with_rounding::<HalfEven>(),
        ] {
            let arg = atan2_point(&y, &x, work);
            let (lo, hi) = arg.bounds(work_precision(work));
            low = Some(low.map_or(lo.clone(), |old| old.min(lo)));
            high = Some(high.map_or(hi.clone(), |old| old.max(hi)));
        }
    }
    Ball::from_bounds(
        low.expect("invariant: four argument corners"),
        high.expect("invariant: four argument corners"),
        prec,
    )
}

fn atan2_point(y: &BigFloat, x: &BigFloat, prec: u32) -> Ball {
    if x == &BigFloat::ZERO {
        let half = Ball::pi(prec).div(&integer(2, prec));
        return if y < &BigFloat::ZERO {
            negate(&half)
        } else {
            half
        };
    }
    let angle = point(y.clone(), prec).div(&point(x.clone(), prec)).atan();
    if x > &BigFloat::ZERO {
        angle
    } else if y < &BigFloat::ZERO {
        angle.sub(&Ball::pi(prec))
    } else {
        angle.add(&Ball::pi(prec))
    }
}
