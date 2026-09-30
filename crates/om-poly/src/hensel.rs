//! Quadratic Hensel lifting from PLAN §8.2d, with explicit product and Bézout certificates.
mod arithmetic;
mod lift;
use crate::UPoly;
use arithmetic::{modulo, monic_divrem};
pub use lift::hensel_lift;
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
type Poly = UPoly<Integer>;

/// Two complementary integer factors and their modular Bézout coefficients.
#[derive(Clone, Debug, PartialEq)]
pub struct HenselPair {
    /// Factor retaining the target's exact leading coefficient.
    pub g: Poly,
    /// Monic complementary factor.
    pub h: Poly,
    /// Coefficient multiplying g in s*g+t*h=1 modulo the current modulus.
    pub s: Poly,
    /// Coefficient multiplying h in the modular Bézout identity.
    pub t: Poly,
}
/// Lift f=g*h and s*g+t*h=1 from modulus m to m² using the quadratic update.
/// None rejects m<2, zero/degree-incompatible factors, nonmonic h, a changed leading
/// coefficient or a broken product/Bézout input congruence.
/// Coefficients are reduced to [0,m²); g retains the target's exact leading coefficient.
pub fn hensel_step(
    m: &Integer,
    f: &Poly,
    g: &Poly,
    h: &Poly,
    s: &Poly,
    t: &Poly,
    ctx: &Interrupt,
) -> Result<Option<HenselPair>, Abort> {
    ctx.tick()?;
    if m < &Integer::from(2)
        || f.is_zero()
        || g.is_zero()
        || h.lc() != Some(&Integer::ONE)
        || g.lc() != f.lc()
    {
        return Ok(None);
    }
    if g.degree().expect("invariant: nonzero g") + h.degree().expect("invariant: nonzero h")
        != f.degree().expect("invariant: nonzero target")
    {
        return Ok(None);
    }
    let e = f.sub(&g.mul(h, ctx)?, ctx)?;
    let bezout = s
        .mul(g, ctx)?
        .add(&t.mul(h, ctx)?, ctx)?
        .sub(&Poly::one(), ctx)?;
    if !modulo(&e, m, ctx)?.is_zero() || !modulo(&bezout, m, ctx)?.is_zero() {
        return Ok(None);
    }
    let squared = m * m;
    let e = modulo(&e, &squared, ctx)?;
    let se = modulo(&s.mul(&e, ctx)?, &squared, ctx)?;
    let (q, r) = monic_divrem(&se, h, ctx)?;
    let new_g = g.add(&t.mul(&e, ctx)?, ctx)?.add(&q.mul(g, ctx)?, ctx)?;
    let new_h = h.add(&r, ctx)?;
    let mut new_g = modulo(&new_g, &squared, ctx)?;
    let new_h = modulo(&new_h, &squared, ctx)?;
    // The residue of lc(f) can be smaller; retain its exact value for recursive lifting.
    let degree = g.degree().expect("invariant: nonzero g");
    assert!(
        new_g.degree().is_none_or(|d| d <= degree),
        "Hensel correction cannot increase g degree"
    );
    new_g.coeffs.resize(degree + 1, Integer::ZERO);
    new_g.coeffs[degree] = f.coeffs[f.degree().expect("invariant: nonzero f")].clone();
    let b = s
        .mul(&new_g, ctx)?
        .add(&t.mul(&new_h, ctx)?, ctx)?
        .sub(&Poly::one(), ctx)?;
    let b = modulo(&b, &squared, ctx)?;
    let sb = modulo(&s.mul(&b, ctx)?, &squared, ctx)?;
    let (c, d) = monic_divrem(&sb, &new_h, ctx)?;
    let new_s = s.sub(&d, ctx)?;
    let new_t = t
        .sub(&t.mul(&b, ctx)?, ctx)?
        .sub(&c.mul(&new_g, ctx)?, ctx)?;
    Ok(Some(HenselPair {
        g: new_g,
        h: new_h,
        s: modulo(&new_s, &squared, ctx)?,
        t: modulo(&new_t, &squared, ctx)?,
    }))
}
