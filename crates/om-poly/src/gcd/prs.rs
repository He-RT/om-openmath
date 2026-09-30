//! Collins fraction-free recurrence from PLAN §8.2e; no primitive reduction inside PRS.
use super::sparse::{divide_coefficients, join, multiply, power, split};
use super::{Dense, Sparse, constant, positive, trivial};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

pub(super) fn compute(f: &Sparse, g: &Sparse, ctx: &Interrupt) -> Result<Sparse, Abort> {
    ctx.tick()?;
    if let Some(h) = trivial(f, g, ctx)? {
        return Ok(h);
    }
    let (fc, a) = content_primitive(&split(f, ctx)?, f.nvars - 1, f.order, ctx)?;
    let (gc, b) = content_primitive(&split(g, ctx)?, g.nvars - 1, g.order, ctx)?;
    let content = compute(&fc, &gc, ctx)?;
    let last = last_remainder(a, b, ctx)?;
    let (_, primitive) = content_primitive(&last, f.nvars - 1, f.order, ctx)?;
    let mut coefficients = Vec::with_capacity(primitive.coeffs.len());
    for c in &primitive.coeffs {
        ctx.tick()?;
        coefficients.push(multiply(c, &content, ctx)?);
    }
    positive(join(&Dense::new(coefficients), f.nvars, f.order, ctx)?, ctx)
}
fn content_primitive(
    f: &Dense,
    nvars: usize,
    order: crate::MonoOrder,
    ctx: &Interrupt,
) -> Result<(Sparse, Dense), Abort> {
    ctx.tick()?;
    let mut content = constant(nvars, order, Integer::ZERO, ctx)?;
    for c in &f.coeffs {
        ctx.tick()?;
        content = compute(&content, c, ctx)?;
        if content.is_one() {
            break;
        }
    }
    let primitive = divide_coefficients(f, &content, ctx)?;
    Ok((content, primitive))
}
fn last_remainder(mut a: Dense, mut b: Dense, ctx: &Interrupt) -> Result<Dense, Abort> {
    ctx.tick()?;
    if a.degree() < b.degree() {
        std::mem::swap(&mut a, &mut b);
    }
    let (mut previous_lc, mut h) = (Sparse::one(), Sparse::one());
    while b.degree().is_some_and(|d| d > 0) {
        ctx.tick()?;
        let delta = a.degree().expect("invariant: nonzero PRS dividend")
            - b.degree().expect("invariant: nonzero PRS divisor");
        let remainder = a.prem(&b, ctx)?.expect("invariant: nonzero PRS divisor");
        if remainder.is_zero() {
            return Ok(b);
        }
        let divisor = multiply(&previous_lc, &power(&h, delta, ctx)?, ctx)?;
        let next = divide_coefficients(&remainder, &divisor, ctx)?;
        previous_lc = b.lc().expect("invariant: nonzero PRS divisor").clone();
        // delta=0 is h_new=h; avoid a negative exponent in equal-degree first steps.
        if delta > 0 {
            h = power(&previous_lc, delta, ctx)?
                .exact_div(&power(&h, delta - 1, ctx)?, ctx)?
                .expect("invariant: Collins scaling division is exact");
        }
        a = b;
        b = next;
    }
    Ok(b)
}
