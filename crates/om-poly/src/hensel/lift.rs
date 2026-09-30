//! Balanced factor-tree lifting; repeated squaring overshoots then reduces to exactly p^l.
use super::{
    Poly,
    arithmetic::{integer, inverse, modulo, to_field},
    hensel_step,
};
use crate::{FpElem, Ring, UPoly, division::coefficient_power};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

/// Lift a complete pairwise-coprime monic factorization modulo p to modulus p^exponent.
/// Return monic integer residue polynomials in the original factor order. Their product
/// times lc(f) equals f modulo the final modulus. None rejects zero input, exponent zero,
/// a nonprime/degree-dropping p or an incomplete/mismatched/nonmonic/noncoprime factor list.
pub fn hensel_lift(
    p: u64,
    f: &Poly,
    factors: &[UPoly<FpElem>],
    exponent: u32,
    ctx: &Interrupt,
) -> Result<Option<Vec<Poly>>, Abort> {
    ctx.tick()?;
    if exponent == 0 || f.is_zero() || FpElem::new(0, p).is_none() {
        return Ok(None);
    }
    let ff = to_field(f, p, ctx)?;
    if ff.degree() != f.degree() {
        return Ok(None);
    }
    let lc = *ff.lc().expect("invariant: a good prime preserves degree");
    let mut product = UPoly::one();
    let mut normalized = Vec::with_capacity(factors.len());
    for factor in factors {
        ctx.tick()?;
        let mut coefficients = Vec::with_capacity(factor.coeffs.len());
        for c in &factor.coeffs {
            ctx.tick()?;
            if c.p != 0 && c.p != p {
                return Ok(None);
            }
            coefficients.push(c.add(&FpElem { v: 0, p }));
        }
        let factor = UPoly::new(coefficients);
        if factor.degree().is_none_or(|d| d == 0) || factor.lc().is_none_or(|c| c.v != 1) {
            return Ok(None);
        }
        if !product
            .monic_gcd(&factor, ctx)?
            .expect("invariant: bound prime-field GCD exists")
            .is_one()
        {
            return Ok(None);
        }
        product = product.mul(&factor, ctx)?;
        normalized.push(factor);
    }
    if product.scale(&lc, ctx)? != ff {
        return Ok(None);
    }
    let modulus = coefficient_power(&Integer::from(p), exponent as usize, ctx)?;
    Ok(Some(lift_node(p, f, &normalized, exponent, &modulus, ctx)?))
}
fn lift_node(
    p: u64,
    f: &Poly,
    factors: &[UPoly<FpElem>],
    exponent: u32,
    modulus: &Integer,
    ctx: &Interrupt,
) -> Result<Vec<Poly>, Abort> {
    ctx.tick()?;
    if factors.is_empty() {
        return Ok(vec![]);
    }
    let lc = f.lc().expect("invariant: lifted target is nonzero");
    if factors.len() == 1 {
        let inverse = inverse(lc, modulus, ctx)?
            .expect("invariant: good-prime leading coefficient is invertible modulo p^l");
        return Ok(vec![modulo(&f.scale(&inverse, ctx)?, modulus, ctx)?]);
    }
    let mid = factors.len() / 2;
    let product = |factors: &[UPoly<FpElem>]| -> Result<UPoly<FpElem>, Abort> {
        let mut out = UPoly::one();
        for f in factors {
            ctx.tick()?;
            out = out.mul(f, ctx)?;
        }
        Ok(out)
    };
    let left = product(&factors[..mid])?;
    let right = product(&factors[mid..])?;
    let field_lc = *to_field(f, p, ctx)?
        .lc()
        .expect("invariant: good prime preserves lifted target degree");
    let left = left.scale(&field_lc, ctx)?;
    let (gcd, s, t) = left
        .extended_gcd(&right, ctx)?
        .expect("invariant: bound prime-field Bézout computation exists");
    assert!(gcd.is_one(), "Hensel factor tree requires coprime branches");
    let mut g = integer(&left, ctx)?;
    let mut h = integer(&right, ctx)?;
    let mut s = integer(&s, ctx)?;
    let mut t = integer(&t, ctx)?;
    let dg = g.degree().expect("invariant: nonempty left branch");
    g.coeffs[dg] = lc.clone();
    let mut m = Integer::from(p);
    let mut achieved = 1_u64;
    while achieved < u64::from(exponent) {
        ctx.tick()?;
        let pair = hensel_step(&m, f, &g, &h, &s, &t, ctx)?
            .expect("invariant: factor tree supplies Hensel input certificates");
        g = pair.g;
        h = pair.h;
        s = pair.s;
        t = pair.t;
        m = &m * &m;
        achieved *= 2;
    }
    g = modulo(&g, modulus, ctx)?;
    g.coeffs[dg] = lc.clone();
    h = modulo(&h, modulus, ctx)?;
    let mut out = lift_node(p, &g, &factors[..mid], exponent, modulus, ctx)?;
    out.extend(lift_node(p, &h, &factors[mid..], exponent, modulus, ctx)?);
    Ok(out)
}
