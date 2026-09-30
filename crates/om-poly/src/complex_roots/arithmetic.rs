//! Ball evaluation and directed Euclidean norms at exact dyadic iteration centers.
use crate::UPoly;
use dashu::float::{FBig, round::mode::Up};
use om_num::{
    Ball, BigFloat, CBall, Integer, Rational,
    ctx::{Abort, Interrupt},
};
pub(super) type Upper = FBig<Up, 2>;

pub(super) fn rational(value: &BigFloat) -> Rational {
    let representation = value.repr();
    let exponent = representation.exponent();
    let numerator = representation.significand().clone();
    if exponent >= 0 {
        Rational::from(numerator << (exponent as usize))
    } else {
        Rational::from(numerator) / Rational::from(Integer::ONE << exponent.unsigned_abs())
    }
}
pub(super) fn point(z: &CBall, bits: u32) -> CBall {
    CBall {
        re: Ball {
            mid: z.re.mid.clone().with_precision(bits as usize).value(),
            rad: BigFloat::ZERO,
            prec: bits,
        },
        im: Ball {
            mid: z.im.mid.clone().with_precision(bits as usize).value(),
            rad: BigFloat::ZERO,
            prec: bits,
        },
    }
}
pub(super) fn finite(z: &CBall) -> bool {
    [&z.re, &z.im]
        .iter()
        .all(|b| b.mid.repr().is_finite() && b.rad.repr().is_finite())
}
pub(super) fn coefficients(
    f: &UPoly<Integer>,
    bits: u32,
    ctx: &Interrupt,
) -> Result<Vec<CBall>, Abort> {
    let lc = Rational::from(
        f.lc()
            .expect("invariant: nonzero complex root input")
            .clone(),
    );
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for c in &f.coeffs {
        ctx.tick()?;
        coefficients.push(CBall::exact(
            &(Rational::from(c.clone()) / &lc),
            &Rational::ZERO,
            bits,
        ));
    }
    Ok(coefficients)
}
pub(super) fn evaluate(
    coefficients: &[CBall],
    z: &CBall,
    ctx: &Interrupt,
) -> Result<(CBall, CBall), Abort> {
    let mut value = coefficients
        .last()
        .expect("invariant: nonempty polynomial coefficients")
        .clone();
    let mut derivative = CBall::exact(&Rational::ZERO, &Rational::ZERO, z.re.prec);
    for c in coefficients[..coefficients.len() - 1].iter().rev() {
        ctx.tick()?;
        derivative = derivative.mul(z).add(&value);
        value = value.mul(z).add(c);
    }
    Ok((value, derivative))
}
pub(super) fn upper_norm(z: &CBall) -> Option<Upper> {
    if !finite(z) {
        return None;
    }
    let upper = |b: &Ball| {
        let mid = if b.mid < BigFloat::ZERO {
            -&b.mid
        } else {
            b.mid.clone()
        };
        mid.with_rounding::<Up>() + b.rad.clone().with_rounding::<Up>()
    };
    let (a, b) = (upper(&z.re), upper(&z.im));
    Some((&a * &a + &b * &b).sqrt())
}
pub(super) fn upper_integer(n: usize, bits: u32) -> Upper {
    Upper::from(Integer::from(n))
        .with_precision(bits as usize)
        .value()
}
