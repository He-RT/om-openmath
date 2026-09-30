//! Simultaneous Aberth-Ehrlich updates; their midpoints are proposals, not enclosures.
use super::arithmetic::{evaluate, finite, point, upper_norm};
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
    let one = CBall::exact(&Rational::ONE, &Rational::ZERO, bits);
    let zero = CBall::exact(&Rational::ZERO, &Rational::ZERO, bits);
    let mut next = Vec::with_capacity(centers.len());
    let mut max = BigFloat::ZERO;
    for (j, z) in centers.iter().enumerate() {
        ctx.tick()?;
        let (value, derivative) = evaluate(coefficients, z, ctx)?;
        let newton = value.div(&derivative);
        if !finite(&newton) {
            return Ok(None);
        }
        let mut interactions = zero.clone();
        for (k, other) in centers.iter().enumerate() {
            ctx.tick()?;
            if j != k {
                let inverse = one.div(&z.sub(other));
                if !finite(&inverse) {
                    return Ok(None);
                }
                interactions = interactions.add(&inverse);
            }
        }
        let correction = newton.div(&one.sub(&newton.mul(&interactions)));
        let Some(norm) = upper_norm(&correction) else {
            return Ok(None);
        };
        max = max.max(norm.with_rounding::<dashu::float::round::mode::HalfEven>());
        let center = z.sub(&correction);
        if !finite(&center) {
            return Ok(None);
        }
        next.push(point(&center, bits));
    }
    Ok(Some((next, max)))
}
