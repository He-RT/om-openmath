//! Cohen/Collins fraction-free resultants in Z or Z[parameters], from PLAN §8.2e.
mod domain;
use crate::{MPoly, UPoly, division::coefficient_power};
use domain::ResultantDomain;
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

macro_rules! resultant_methods {
    ($coefficient:ty) => {
        impl UPoly<$coefficient> {
            /// Resultant in the dense variable, over compatible integer/parameter coefficients.
            /// Any zero input returns zero; two nonzero constants have resultant one.
            pub fn resultant(&self, other: &Self, ctx: &Interrupt) -> Result<$coefficient, Abort> {
                compute(self, other, ctx)
            }
            /// Discriminant: (-1)^(n(n-1)/2) resultant(f,f') / lc(f).
            /// Zero/degree-zero inputs return zero; linear inputs return one.
            pub fn discriminant(&self, ctx: &Interrupt) -> Result<$coefficient, Abort> {
                discriminant(self, ctx)
            }
        }
    };
}
resultant_methods!(Integer);
resultant_methods!(MPoly<Integer>);

fn compute<R: ResultantDomain>(f: &UPoly<R>, g: &UPoly<R>, ctx: &Interrupt) -> Result<R, Abort> {
    ctx.tick()?;
    let (Some(df), Some(dg)) = (f.degree(), g.degree()) else {
        return Ok(R::zero());
    };
    let (fc, mut a) = R::content_primitive(f, ctx)?;
    let (gc, mut b) = R::content_primitive(g, ctx)?;
    let scale = coefficient_power(&fc, dg, ctx)?.mul(&coefficient_power(&gc, df, ctx)?);
    let mut negative = false;
    if a.degree() < b.degree() {
        negative = df % 2 == 1 && dg % 2 == 1;
        std::mem::swap(&mut a, &mut b);
    }
    let (mut previous_lc, mut h) = (R::one(), R::one());
    while b.degree().is_some_and(|d| d > 0) {
        ctx.tick()?;
        let da = a.degree().expect("invariant: nonzero resultant dividend");
        let db = b.degree().expect("invariant: nonzero resultant divisor");
        let delta = da - db;
        if da % 2 == 1 && db % 2 == 1 {
            negative = !negative;
        }
        let remainder = a
            .prem(&b, ctx)?
            .expect("invariant: nonzero resultant divisor");
        if remainder.is_zero() {
            return Ok(R::zero());
        }
        let divisor = previous_lc.mul(&coefficient_power(&h, delta, ctx)?);
        let next = divide_coefficients(&remainder, &divisor, ctx)?;
        previous_lc = b
            .lc()
            .expect("invariant: nonzero resultant divisor")
            .clone();
        if delta > 0 {
            h = exact(
                &coefficient_power(&previous_lc, delta, ctx)?,
                &coefficient_power(&h, delta - 1, ctx)?,
                ctx,
            )?;
        }
        a = b;
        b = next;
    }
    let da = a
        .degree()
        .expect("invariant: nonzero final resultant dividend");
    let numerator = coefficient_power(
        b.lc()
            .expect("invariant: nonzero constant resultant remainder"),
        da,
        ctx,
    )?;
    // da=0 denotes two constants; their empty Sylvester determinant is one.
    let tail = if da == 0 {
        R::one()
    } else {
        exact(&numerator, &coefficient_power(&h, da - 1, ctx)?, ctx)?
    };
    ctx.tick()?;
    let result = scale.mul(&tail);
    Ok(if negative { result.neg() } else { result })
}
fn discriminant<R: ResultantDomain>(f: &UPoly<R>, ctx: &Interrupt) -> Result<R, Abort> {
    ctx.tick()?;
    let Some(n) = f.degree().filter(|n| *n > 0) else {
        return Ok(R::zero());
    };
    let lc = f.lc().expect("invariant: nonzero discriminant input");
    let res = compute(f, &f.derivative(ctx)?, ctx)?;
    let disc = exact(&res, lc, ctx)?;
    // n(n-1)/2 is odd precisely for n mod 4 in {2,3}, avoiding size overflow.
    Ok(if n % 4 >= 2 { disc.neg() } else { disc })
}
fn exact<R: ResultantDomain>(a: &R, b: &R, ctx: &Interrupt) -> Result<R, Abort> {
    Ok(a.exact_quotient(b, ctx)?
        .expect("invariant: Cohen resultant coefficient division is exact"))
}
fn divide_coefficients<R: ResultantDomain>(
    f: &UPoly<R>,
    c: &R,
    ctx: &Interrupt,
) -> Result<UPoly<R>, Abort> {
    ctx.tick()?;
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for a in &f.coeffs {
        ctx.tick()?;
        coefficients.push(exact(a, c, ctx)?);
    }
    Ok(UPoly::new(coefficients))
}
