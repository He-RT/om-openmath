//! Integer residue arithmetic; composite lifting moduli are never used as finite fields.
use super::Poly;
use crate::{EuclideanRing, FpElem, UPoly};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

pub(super) fn modulo(f: &Poly, m: &Integer, ctx: &Interrupt) -> Result<Poly, Abort> {
    ctx.tick()?;
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for c in &f.coeffs {
        ctx.tick()?;
        let r = c % m;
        coefficients.push(if r < Integer::ZERO { r + m } else { r });
    }
    Ok(Poly::new(coefficients))
}
pub(super) fn integer(f: &UPoly<FpElem>, ctx: &Interrupt) -> Result<Poly, Abort> {
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for c in &f.coeffs {
        ctx.tick()?;
        coefficients.push(Integer::from(c.v));
    }
    Ok(Poly::new(coefficients))
}
pub(super) fn monic_divrem(f: &Poly, g: &Poly, ctx: &Interrupt) -> Result<(Poly, Poly), Abort> {
    assert_eq!(
        g.lc(),
        Some(&Integer::ONE),
        "integer Hensel division requires a monic divisor"
    );
    let (_, q, r) = f
        .pseudo_divrem(g, ctx)?
        .expect("invariant: Hensel divisor is nonzero");
    Ok((q, r))
}
pub(super) fn inverse(a: &Integer, m: &Integer, ctx: &Interrupt) -> Result<Option<Integer>, Abort> {
    ctx.tick()?;
    let (mut old_r, mut r) = (m.clone(), a.divrem(m).1);
    let (mut old_t, mut t) = (Integer::ZERO, Integer::ONE);
    while !r.is_zero() {
        ctx.tick()?;
        let q = &old_r / &r;
        let next_r = &old_r - &q * &r;
        let next_t = &old_t - q * &t;
        old_r = r;
        r = next_r;
        old_t = t;
        t = next_t;
    }
    Ok((old_r == Integer::ONE).then(|| old_t.divrem(m).1))
}
pub(super) fn to_field(f: &Poly, p: u64, ctx: &Interrupt) -> Result<UPoly<FpElem>, Abort> {
    let modulus = Integer::from(p);
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for c in &f.coeffs {
        ctx.tick()?;
        let v = u64::try_from(c.divrem(&modulus).1)
            .expect("invariant: residue is smaller than a u64 modulus");
        coefficients.push(FpElem { v, p });
    }
    Ok(UPoly::new(coefficients))
}
