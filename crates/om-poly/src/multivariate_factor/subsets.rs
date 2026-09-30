//! Inverse candidate products are accepted only after exact sparse polynomial division.
use super::{Sparse, UPoly, ground_primitive, kronecker::Mapping};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

pub(super) fn run(
    mut f: Sparse,
    mut pieces: Vec<UPoly<Integer>>,
    mapping: &Mapping,
    limit: usize,
    ctx: &Interrupt,
) -> Result<(Vec<Sparse>, bool), Abort> {
    let mut out = vec![];
    let mut size = 1;
    let mut tried = 0;
    while size <= pieces.len() / 2 && !f.is_one() {
        let mut subset = (0..size).collect::<Vec<_>>();
        let mut found = false;
        loop {
            ctx.tick()?;
            if tried >= limit {
                out.push(f);
                return Ok((out, false));
            }
            tried += 1;
            let mut product = UPoly::one();
            for i in &subset {
                ctx.tick()?;
                product = product.mul(&pieces[*i], ctx)?;
            }
            if let Some(candidate) = mapping.decode(&product, ctx)? {
                let candidate = ground_primitive(&candidate, ctx)?;
                if !candidate.is_one()
                    && candidate != f
                    && !candidate.is_zero()
                    && let Some(quotient) = f.exact_div(&candidate, ctx)?
                {
                    out.push(candidate);
                    f = quotient;
                    let mut rest = Vec::with_capacity(pieces.len() - size);
                    for (i, factor) in pieces.into_iter().enumerate() {
                        ctx.tick()?;
                        if !subset.contains(&i) {
                            rest.push(factor);
                        }
                    }
                    pieces = rest;
                    found = true;
                    break;
                }
            }
            if !next(&mut subset, pieces.len(), ctx)? {
                break;
            }
        }
        if !found {
            size += 1;
        }
    }
    if !f.is_one() {
        out.push(f);
    }
    Ok((out, true))
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
