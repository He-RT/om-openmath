//! Aberth-Ehrlich proposals followed by independent ball certificates, PLAN §8.2f.
mod arithmetic;
mod iteration;
use crate::UPoly;
use arithmetic::{coefficients, point, upper_norm};
use iteration::{initialize, step};
use om_num::{
    BigFloat, CBall, Integer, Rational,
    ctx::{Abort, Interrupt},
};
use std::cmp::Ordering;
const MAX_WORK_BITS: u32 = 16_384;

/// A closed certified complex disk with an exact dyadic center and outward radius.
#[derive(Clone, Debug, PartialEq)]
pub struct RootDisk {
    /// Real part of the dyadic center.
    pub re: BigFloat,
    /// Imaginary part of the dyadic center.
    pub im: BigFloat,
    /// Nonnegative outward radius; all published values are finite.
    pub radius: BigFloat,
}
impl RootDisk {
    /// Whether an exact rational complex point lies in this closed disk.
    pub fn contains(&self, re: &Rational, im: &Rational, ctx: &Interrupt) -> Result<bool, Abort> {
        ctx.tick()?;
        if !self.valid() {
            return Ok(false);
        }
        let a = arithmetic::rational(&self.re) - re;
        let b = arithmetic::rational(&self.im) - im;
        let r = arithmetic::rational(&self.radius);
        Ok(&a * &a + &b * &b <= &r * &r)
    }
    /// Prove that two closed disks are disjoint, using exact dyadic squared distances.
    pub fn disjoint(&self, other: &Self, ctx: &Interrupt) -> Result<bool, Abort> {
        ctx.tick()?;
        if !self.valid() || !other.valid() {
            return Ok(false);
        }
        let a = arithmetic::rational(&self.re) - arithmetic::rational(&other.re);
        let b = arithmetic::rational(&self.im) - arithmetic::rational(&other.im);
        let r = arithmetic::rational(&self.radius) + arithmetic::rational(&other.radius);
        Ok(&a * &a + &b * &b > &r * &r)
    }
    /// Rectangular enclosure of a valid disk, retaining its exact center and outward radius.
    pub fn to_cball(&self) -> CBall {
        let bits = self
            .re
            .precision()
            .max(self.im.precision())
            .max(self.radius.precision())
            .max(32);
        let prec = u32::try_from(bits).expect("invariant: certified root precision fits u32");
        CBall {
            re: om_num::Ball {
                mid: self.re.clone(),
                rad: self.radius.clone(),
                prec,
            },
            im: om_num::Ball {
                mid: self.im.clone(),
                rad: self.radius.clone(),
                prec,
            },
        }
    }
    fn valid(&self) -> bool {
        self.re.repr().is_finite()
            && self.im.repr().is_finite()
            && self.radius.repr().is_finite()
            && self.radius >= BigFloat::ZERO
    }
}
/// Compute all roots of a nonzero square-free integer polynomial with disjoint disk proofs.
/// Returned disks have radius <=2^-bits. Zero/repeated inputs, unsupported precision
/// (0 or >16352 bits), or exhausted numerical certification return None; constants return [].
/// Iteration uses 200 steps per precision with deterministic seeding and doubles guarded
/// precision on stagnation. No approximate centers are returned without certification.
pub fn complex_roots(
    f: &UPoly<Integer>,
    bits: u32,
    ctx: &Interrupt,
) -> Result<Option<Vec<RootDisk>>, Abort> {
    ctx.tick()?;
    if bits == 0 || bits > MAX_WORK_BITS - 32 || f.is_zero() {
        return Ok(None);
    }
    let f = f.primitive_part(ctx)?;
    let n = f.degree().expect("invariant: nonzero root input");
    if n == 0 {
        return Ok(Some(vec![]));
    }
    if !f.gcdheu(&f.derivative(ctx)?, ctx)?.is_one() {
        return Ok(None);
    }
    let radius = crate::isolation::cauchy_power_of_two(&f, ctx)?;
    let target = BigFloat::from_parts(Integer::ONE, -(bits as isize));
    let convergence = BigFloat::from_parts(Integer::ONE, -((bits.max(8) - 4) as isize));
    let mut work = bits.max(32) + 32;
    let mut centers = initialize(n, &radius, work, 0, ctx)?;
    let mut reseed = 0_u32;
    loop {
        ctx.tick()?;
        let coefficients = coefficients(&f, work, ctx)?;
        for z in &mut centers {
            ctx.tick()?;
            *z = point(z, work);
        }
        let mut best: Option<BigFloat> = None;
        let mut stagnant = 0;
        for round in 0..200 {
            ctx.tick()?;
            let Some((next, correction)) = step(&centers, &coefficients, work, ctx)? else {
                // Exact center collisions cannot be repaired by merely adding precision.
                reseed += 1;
                centers = initialize(n, &radius, work, reseed, ctx)?;
                break;
            };
            centers = next;
            if correction < convergence || round % 8 == 7 {
                if let Some(disks) = certify(&centers, &coefficients, &target, ctx)? {
                    return Ok(Some(disks));
                }
                if correction < convergence {
                    break;
                }
            }
            if best.as_ref().is_none_or(|old| correction < *old) {
                best = Some(correction);
                stagnant = 0;
            } else {
                stagnant += 1;
            }
            if stagnant >= 16 {
                break;
            }
        }
        if let Some(disks) = certify(&centers, &coefficients, &target, ctx)? {
            return Ok(Some(disks));
        }
        if work == MAX_WORK_BITS {
            return Ok(None);
        }
        work = (work * 2).min(MAX_WORK_BITS);
    }
}
fn certify(
    centers: &[CBall],
    coefficients: &[CBall],
    target: &BigFloat,
    ctx: &Interrupt,
) -> Result<Option<Vec<RootDisk>>, Abort> {
    let n = centers.len();
    let mut disks = Vec::with_capacity(n);
    for center in centers {
        ctx.tick()?;
        let (value, derivative) = arithmetic::evaluate(coefficients, center, ctx)?;
        let quotient = value.div(&derivative);
        let Some(norm) = upper_norm(&quotient) else {
            return Ok(None);
        };
        let radius = (norm * arithmetic::upper_integer(n, center.re.prec))
            .with_rounding::<dashu::float::round::mode::HalfEven>();
        if !radius.repr().is_finite() || &radius > target {
            return Ok(None);
        }
        let disk = RootDisk {
            re: center.re.mid.clone(),
            im: center.im.mid.clone(),
            radius,
        };
        for other in &disks {
            ctx.tick()?;
            if !disk.disjoint(other, ctx)? {
                return Ok(None);
            }
        }
        disks.push(disk);
    }
    sort(&mut disks, ctx)?;
    Ok(Some(disks))
}
fn sort(disks: &mut [RootDisk], ctx: &Interrupt) -> Result<(), Abort> {
    for i in 1..disks.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            let a = &disks[j - 1];
            let b = &disks[j];
            let order =
                a.re.partial_cmp(&b.re)
                    .expect("invariant: root centers are finite")
                    .then_with(|| {
                        a.im.partial_cmp(&b.im)
                            .expect("invariant: root centers are finite")
                    });
            if order != Ordering::Greater {
                break;
            }
            disks.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(())
}
