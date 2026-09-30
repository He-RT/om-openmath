//! GCDHEU evaluation bounds, symmetric radix reconstruction and exact trial certificates.
use super::sparse::{evaluate, interpolate};
use super::{Sparse, ground_content, ground_primitive, positive, scale, trivial};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
    gcd, isqrt,
};

pub(super) fn attempt(
    f: &Sparse,
    g: &Sparse,
    attempts: usize,
    ctx: &Interrupt,
) -> Result<Option<Sparse>, Abort> {
    ctx.tick()?;
    if let Some(h) = trivial(f, g, ctx)? {
        return Ok(Some(h));
    }
    let content = gcd(&ground_content(f, ctx)?, &ground_content(g, ctx)?);
    let (f, g) = (ground_primitive(f, ctx)?, ground_primitive(g, ctx)?);
    let (nf, ng) = (norm(&f, ctx)?, norm(&g, ctx)?);
    let bound: Integer = 2 * nf.clone().min(ng.clone()) + 29;
    let ratio = 2 * (nf / leading_magnitude(&f, ctx)?).min(ng / leading_magnitude(&g, ctx)?) + 2;
    let mut xi = bound.clone().min(99 * isqrt(&bound)).max(ratio);
    for _ in 0..attempts {
        ctx.tick()?;
        let (ff, gg) = (evaluate(&f, &xi, ctx)?, evaluate(&g, &xi, ctx)?);
        if !ff.is_zero() && !gg.is_zero() {
            let Some(h) = attempt(&ff, &gg, attempts, ctx)? else {
                return Ok(None);
            };
            if let Some(candidate) = interpolate(&h, &xi, ctx)?
                && let Some(answer) = certify(&candidate, &f, &g, &content, ctx)?
            {
                return Ok(Some(answer));
            }
            // Reconstruct either cofactor when the evaluated GCD has spurious factors.
            for (source, evaluated) in [(&f, &ff), (&g, &gg)] {
                ctx.tick()?;
                let cofactor = evaluated
                    .exact_div(&h, ctx)?
                    .expect("invariant: evaluated GCD divides its inputs");
                if let Some(cofactor) = interpolate(&cofactor, &xi, ctx)?
                    && let Some(candidate) = source.exact_div(&cofactor, ctx)?
                    && let Some(answer) = certify(&candidate, &f, &g, &content, ctx)?
                {
                    return Ok(Some(answer));
                }
            }
        }
        ctx.tick()?;
        xi = (&xi * isqrt(&isqrt(&xi)) * 73794) / 27011;
    }
    Ok(None)
}
fn certify(
    candidate: &Sparse,
    f: &Sparse,
    g: &Sparse,
    content: &Integer,
    ctx: &Interrupt,
) -> Result<Option<Sparse>, Abort> {
    ctx.tick()?;
    let candidate = ground_primitive(candidate, ctx)?;
    if !candidate.is_zero()
        && f.exact_div(&candidate, ctx)?.is_some()
        && g.exact_div(&candidate, ctx)?.is_some()
    {
        return Ok(Some(positive(scale(&candidate, content, ctx)?, ctx)?));
    }
    Ok(None)
}
fn norm(f: &Sparse, ctx: &Interrupt) -> Result<Integer, Abort> {
    let mut value = Integer::ZERO;
    for (_, c) in &f.terms {
        ctx.tick()?;
        value = value.max(c.clone().max(-c));
    }
    Ok(value)
}
fn leading_magnitude(f: &Sparse, ctx: &Interrupt) -> Result<Integer, Abort> {
    // The dense recursion prioritizes the last variable, independently of sparse display order.
    let mut leading = f
        .terms
        .first()
        .expect("invariant: primitive input is nonzero");
    for term in &f.terms {
        ctx.tick()?;
        if term
            .0
            .exps
            .iter()
            .rev()
            .cmp(leading.0.exps.iter().rev())
            .is_gt()
        {
            leading = term;
        }
    }
    Ok(leading.1.clone().max(-&leading.1))
}
