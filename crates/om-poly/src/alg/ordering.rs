//! Real-first Root numbering uses certified membership and explicit conjugate pairs.
use super::{
    Algebraic, ComplexAlg, MAX_BITS, Poly,
    bounds::{BoxBounds, ball_bounds, overlap, real_bounds},
    real_roots,
};
use crate::complex_roots;
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
use std::cmp::Ordering;

pub(super) fn roots(f: &Poly, ctx: &Interrupt) -> Result<Option<Vec<Algebraic>>, Abort> {
    let Some(mut reals) = real_roots(f, ctx)? else {
        return Ok(None);
    };
    let n = f
        .degree()
        .expect("invariant: irreducible nonconstant polynomial");
    if reals.len() == n {
        return Ok(Some(reals));
    }
    let mut bits = 400;
    loop {
        ctx.tick()?;
        for real in &mut reals {
            ctx.tick()?;
            let Some(next) = real.refined(bits, ctx)? else {
                return Ok(None);
            };
            *real = next;
        }
        let Some(disks) = complex_roots(f, bits, ctx)? else {
            return Ok(None);
        };
        let boxes: Vec<_> = disks.iter().map(|d| d.to_cball()).collect();
        let bounds: Vec<_> = boxes
            .iter()
            .map(|b| ball_bounds(b).expect("invariant: finite certified disks"))
            .collect();
        let mut used = vec![false; n];
        let mut failed = false;
        for real in &reals {
            ctx.tick()?;
            let iv = real_bounds(real).expect("invariant: real isolation result");
            let mut found = None;
            for (i, b) in bounds.iter().enumerate() {
                ctx.tick()?;
                if overlap(&iv, &b.re) && b.im.0 <= Rational::ZERO && b.im.1 >= Rational::ZERO {
                    if found.is_some() {
                        failed = true;
                        break;
                    }
                    found = Some(i);
                }
            }
            if let Some(i) = found {
                if used[i] {
                    failed = true;
                }
                used[i] = true;
            } else {
                failed = true;
            }
        }
        // Rectangles, not just circles, must isolate root identities for later refinement.
        for i in 0..n {
            for j in 0..i {
                ctx.tick()?;
                if bounds[i].intersects(&bounds[j]) {
                    failed = true;
                }
            }
        }
        let mut pairs = vec![];
        for i in 0..n {
            ctx.tick()?;
            if used[i] {
                continue;
            }
            if bounds[i].im.1 < Rational::ZERO {
                let reflected = bounds[i].conjugate();
                let mut found = None;
                for j in 0..n {
                    ctx.tick()?;
                    if !used[j]
                        && bounds[j].im.0 > Rational::ZERO
                        && reflected.intersects(&bounds[j])
                    {
                        if found.is_some() {
                            failed = true;
                            break;
                        }
                        found = Some(j);
                    }
                }
                if let Some(j) = found {
                    let value = |k: usize| {
                        Algebraic::Complex(ComplexAlg {
                            minpoly: f.clone(),
                            disk: boxes[k].clone(),
                            index: 0,
                        })
                    };
                    pairs.push((value(i), value(j)));
                    used[i] = true;
                    used[j] = true;
                } else {
                    failed = true;
                }
            } else if bounds[i].im.0 <= Rational::ZERO {
                failed = true;
            }
        }
        if used.iter().any(|u| !*u) {
            failed = true;
        }
        if !failed {
            sort_pairs(&mut pairs, ctx)?;
            for (negative, positive) in pairs {
                reals.push(negative);
                reals.push(positive);
            }
            for (i, root) in reals.iter_mut().enumerate() {
                if let Algebraic::Complex(a) = root {
                    a.index = i + 1;
                }
            }
            return Ok(Some(reals));
        }
        if bits == MAX_BITS {
            return Ok(None);
        }
        bits = (bits * 2).min(MAX_BITS);
    }
}
pub(super) fn sort(all: Vec<Algebraic>, ctx: &Interrupt) -> Result<Option<Vec<Algebraic>>, Abort> {
    let mut reals = vec![];
    let mut complex = vec![];
    for a in all {
        ctx.tick()?;
        if matches!(a, Algebraic::Complex(_)) {
            complex.push(a);
        } else {
            reals.push(a);
        }
    }
    for i in 1..reals.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            let Some(order) = reals[j - 1].cmp_real(&reals[j], ctx)? else {
                return Ok(None);
            };
            if order != Ordering::Greater {
                break;
            }
            reals.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut pairs = vec![];
    let mut i = 0;
    while i < complex.len() {
        ctx.tick()?;
        if i + 1 == complex.len() {
            return Ok(None);
        }
        pairs.push((complex[i].clone(), complex[i + 1].clone()));
        i += 2;
    }
    sort_pairs(&mut pairs, ctx)?;
    for (negative, positive) in pairs {
        reals.push(negative);
        reals.push(positive);
    }
    Ok(Some(reals))
}
fn box_of(pair: &(Algebraic, Algebraic)) -> BoxBounds {
    let Algebraic::Complex(a) = &pair.0 else {
        unreachable!("invariant: conjugate pair is nonreal");
    };
    ball_bounds(&a.disk).expect("invariant: finite certified pair box")
}
fn sort_pairs(pairs: &mut [(Algebraic, Algebraic)], ctx: &Interrupt) -> Result<(), Abort> {
    // Overlap components at >=400 bits make tied real keys transitive, avoiding
    // a nontransitive comparator for three mutually overlapping projections.
    for i in 1..pairs.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            if box_of(&pairs[j - 1]).re.0 <= box_of(&pairs[j]).re.0 {
                break;
            }
            pairs.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut start = 0;
    while start < pairs.len() {
        ctx.tick()?;
        let mut high = box_of(&pairs[start]).re.1;
        let mut end = start + 1;
        while end < pairs.len() {
            ctx.tick()?;
            let b = box_of(&pairs[end]);
            if b.re.0 > high {
                break;
            }
            high = high.max(b.re.1);
            end += 1;
        }
        for i in start + 1..end {
            let mut j = i;
            while j > start {
                ctx.tick()?;
                let a = box_of(&pairs[j - 1]);
                let b = box_of(&pairs[j]);
                // Negative members give |Im| = -Im. Disjoint projections order
                // exactly; unresolved absolute-imaginary ties use dyadic centers.
                let am = -(&a.im.0 + &a.im.1);
                let bm = -(&b.im.0 + &b.im.1);
                if am <= bm {
                    break;
                }
                pairs.swap(j - 1, j);
                j -= 1;
            }
        }
        start = end;
    }
    Ok(())
}
