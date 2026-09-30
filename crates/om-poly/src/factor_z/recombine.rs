//! Lexicographic subset reconstruction with the plan's constant-term pruning and hard cap.
use super::Poly;
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

pub(super) fn run(
    mut f: Poly,
    mut lifted: Vec<Poly>,
    modulus: &Integer,
    limit: usize,
    ctx: &Interrupt,
) -> Result<(Vec<Poly>, bool), Abort> {
    let mut out = vec![];
    let mut size = 1;
    let mut tried = 0;
    while size <= lifted.len() / 2 {
        ctx.tick()?;
        let mut subset = (0..size).collect::<Vec<_>>();
        let mut found = false;
        loop {
            ctx.tick()?;
            if tried >= limit {
                out.push(f);
                return Ok((out, false));
            }
            tried += 1;
            let lc = f.lc().expect("invariant: nonzero Zassenhaus remainder");
            let mut tc = lc.clone();
            for i in &subset {
                ctx.tick()?;
                tc = symmetric_scalar(&(tc * constant(&lifted[*i])), modulus);
            }
            if !tc.is_zero() && (lc * constant(&f) % &tc).is_zero() {
                let mut product = Poly::one();
                for i in &subset {
                    ctx.tick()?;
                    product = symmetric(&product.mul(&lifted[*i], ctx)?, modulus, ctx)?;
                }
                let candidate =
                    symmetric(&product.scale(lc, ctx)?, modulus, ctx)?.primitive_part(ctx)?;
                if candidate.degree().is_some_and(|d| d > 0)
                    && candidate.degree() < f.degree()
                    && let Some(quotient) = f.exact_div(&candidate, ctx)?
                {
                    out.push(candidate);
                    f = quotient;
                    let mut remaining = Vec::with_capacity(lifted.len() - size);
                    for (i, factor) in lifted.into_iter().enumerate() {
                        ctx.tick()?;
                        if !subset.contains(&i) {
                            remaining.push(factor);
                        }
                    }
                    lifted = remaining;
                    found = true;
                    break;
                }
            }
            if !next(&mut subset, lifted.len(), ctx)? {
                break;
            }
        }
        if !found {
            size += 1;
        }
    }
    out.push(f);
    Ok((out, true))
}
fn constant(f: &Poly) -> Integer {
    f.coeffs.first().cloned().unwrap_or(Integer::ZERO)
}
fn symmetric_scalar(c: &Integer, modulus: &Integer) -> Integer {
    let mut r = c % modulus;
    if r < Integer::ZERO {
        r += modulus;
    }
    if &r * 2 > *modulus {
        r -= modulus;
    }
    r
}
pub(super) fn symmetric(f: &Poly, modulus: &Integer, ctx: &Interrupt) -> Result<Poly, Abort> {
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for c in &f.coeffs {
        ctx.tick()?;
        coefficients.push(symmetric_scalar(c, modulus));
    }
    Ok(Poly::new(coefficients))
}
fn next(subset: &mut [usize], len: usize, ctx: &Interrupt) -> Result<bool, Abort> {
    for i in (0..subset.len()).rev() {
        ctx.tick()?;
        if subset[i] < len - subset.len() + i {
            subset[i] += 1;
            for j in i + 1..subset.len() {
                ctx.tick()?;
                subset[j] = subset[j - 1] + 1;
            }
            return Ok(true);
        }
    }
    Ok(false)
}
