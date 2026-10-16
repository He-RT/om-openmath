//! Simultaneous Aberth-Ehrlich updates; their midpoints are proposals, not enclosures.
use super::arithmetic::{point, upper_norm};
use om_num::{
    Ball, BigFloat, CBall, Integer, Rational,
    ctx::{Abort, Interrupt},
};

pub(super) fn initialize(
    n: usize,
    radius: &Integer,
    bits: u32,
    reseed: u32,
    ctx: &Interrupt,
) -> Result<Vec<CBall>, Abort> {
    ctx.tick()?;
    let pi = Ball::pi(bits);
    let radius = Ball::exact(&Rational::from(radius.clone()), bits);
    let phase = Rational::from(2) / Rational::from(5) + Rational::from(reseed) / Rational::from(7);
    let mut centers = Vec::with_capacity(n);
    for j in 0..n {
        ctx.tick()?;
        let angle = pi
            .mul(&Ball::exact(
                &(Rational::from(2 * j) / Rational::from(n)),
                bits,
            ))
            .add(&Ball::exact(&phase, bits));
        let center = CBall {
            re: radius.mul(&angle.cos()),
            im: radius.mul(&angle.sin()),
        };
        centers.push(point(&center, bits));
    }
    Ok(centers)
}
pub(super) fn step(
    centers: &[CBall],
    coefficients: &[CBall],
    bits: u32,
    ctx: &Interrupt,
) -> Result<Option<(Vec<CBall>, BigFloat)>, Abort> {
    let one = Proposal {
        re: BigFloat::ONE.with_precision(bits as usize).value(),
        im: BigFloat::ZERO,
    };
    let zero = Proposal {
        re: BigFloat::ZERO,
        im: BigFloat::ZERO,
    };
    let coefficients: Vec<_> = coefficients.iter().map(Proposal::from).collect();
    let centers: Vec<_> = centers.iter().map(Proposal::from).collect();
    let mut next = Vec::with_capacity(centers.len());
    let mut max = BigFloat::ZERO;
    for (j, z) in centers.iter().enumerate() {
        ctx.tick()?;
        let mut value = coefficients
            .last()
            .expect("invariant: nonempty polynomial coefficients")
            .clone();
        let mut derivative = zero.clone();
        for c in coefficients[..coefficients.len() - 1].iter().rev() {
            ctx.tick()?;
            derivative = derivative.mul(z).add(&value);
            value = value.mul(z).add(c);
        }
        let Some(newton) = value.div(&derivative) else {
            return Ok(None);
        };
        let mut interactions = zero.clone();
        for (k, other) in centers.iter().enumerate() {
            ctx.tick()?;
            if j != k {
                let Some(inverse) = one.div(&z.sub(other)) else {
                    return Ok(None);
                };
                interactions = interactions.add(&inverse);
            }
        }
        let Some(correction) = newton.div(&one.sub(&newton.mul(&interactions))) else {
            return Ok(None);
        };
        let Some(norm) = upper_norm(&correction.ball(bits)) else {
            return Ok(None);
        };
        max = max.max(norm.with_rounding::<dashu::float::round::mode::HalfEven>());
        let center = z.sub(&correction);
        if !center.finite() {
            return Ok(None);
        }
        next.push(center.ball(bits));
    }
    Ok(Some((next, max)))
}

/// Iteration only proposes centers; the separate directed-ball certificate still proves every
/// returned disk and its radius. Carrying interval radii through these discarded proposals
/// multiplies arbitrary-precision work without strengthening that final certificate.
#[derive(Clone)]
struct Proposal {
    re: BigFloat,
    im: BigFloat,
}
impl From<&CBall> for Proposal {
    fn from(z: &CBall) -> Self {
        Self {
            re: z.re.mid.clone(),
            im: z.im.mid.clone(),
        }
    }
}
impl Proposal {
    fn add(&self, other: &Self) -> Self {
        Self {
            re: &self.re + &other.re,
            im: &self.im + &other.im,
        }
    }
    fn sub(&self, other: &Self) -> Self {
        Self {
            re: &self.re - &other.re,
            im: &self.im - &other.im,
        }
    }
    fn mul(&self, other: &Self) -> Self {
        Self {
            re: &self.re * &other.re - &self.im * &other.im,
            im: &self.re * &other.im + &self.im * &other.re,
        }
    }
    fn div(&self, other: &Self) -> Option<Self> {
        let denominator = &other.re * &other.re + &other.im * &other.im;
        if denominator == BigFloat::ZERO || !denominator.repr().is_finite() {
            return None;
        }
        let value = Self {
            re: (&self.re * &other.re + &self.im * &other.im) / &denominator,
            im: (&self.im * &other.re - &self.re * &other.im) / denominator,
        };
        value.finite().then_some(value)
    }
    fn finite(&self) -> bool {
        self.re.repr().is_finite() && self.im.repr().is_finite()
    }
    fn ball(&self, bits: u32) -> CBall {
        CBall {
            re: Ball {
                mid: self.re.clone(),
                rad: BigFloat::ZERO,
                prec: bits,
            },
            im: Ball {
                mid: self.im.clone(),
                rad: BigFloat::ZERO,
                prec: bits,
            },
        }
    }
}
