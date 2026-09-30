//! Exact Descartes/VCA real-root isolation from PLAN §8.2f.
mod refinement;
mod transform;
use crate::UPoly;
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};
pub use refinement::refine;
use transform::{bisect_polynomial, sign_at, unit_variations};
type Poly = UPoly<Integer>;

/// A certified single real root, or an exact rational singleton.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootInterval {
    /// Lower endpoint; strict below the root unless exact.
    pub lo: Rational,
    /// Upper endpoint; strict above the root unless exact.
    pub hi: Rational,
    /// Exact roots have equal endpoints; otherwise both endpoints are nonroots.
    pub exact: bool,
}
/// Isolate every real root of a nonzero square-free integer polynomial.
/// Zero/repeated-root inputs return None; constants return an empty list.
/// Nonexact intervals have nonroot endpoints and all output intervals are strictly
/// separated in increasing order. Midpoint roots and zero are emitted once as exact points.
pub fn isolate(f: &Poly, ctx: &Interrupt) -> Result<Option<Vec<RootInterval>>, Abort> {
    let Some(original) = prepare(f, ctx)? else {
        return Ok(None);
    };
    let mut out = vec![];
    let f = if original.coeffs[0].is_zero() {
        out.push(point(Rational::ZERO));
        Poly::new(original.coeffs[1..].to_vec())
    } else {
        original.clone()
    };
    if f.degree() == Some(0) {
        return Ok(Some(out));
    }
    let bound = cauchy_power_of_two(&f, ctx)?;
    for negative in [false, true] {
        ctx.tick()?;
        let scale = if negative { -&bound } else { bound.clone() };
        let g = transform::scale_variable(&f, &scale, ctx)?.primitive_part(ctx)?;
        let mut stack = vec![(g, Rational::ZERO, Rational::ONE)];
        while let Some((p, a, b)) = stack.pop() {
            ctx.tick()?;
            let variations = unit_variations(&p, ctx)?;
            if variations == 0 {
                continue;
            }
            let interval = map_interval(&a, &b, &bound, negative);
            if variations == 1
                && sign_at(&original, &interval.lo, ctx)? != 0
                && sign_at(&original, &interval.hi, ctx)? != 0
            {
                out.push(interval);
                continue;
            }
            let mid = (&a + &b) / Rational::from(2);
            let (left, mut right) = bisect_polynomial(&p, ctx)?;
            if right.coeffs[0].is_zero() {
                out.push(point(map_point(&mid, &bound, negative)));
                right = Poly::new(right.coeffs[1..].to_vec());
            }
            stack.push((right, mid.clone(), b));
            stack.push((left, a, mid));
        }
    }
    sort(&mut out, ctx)?;
    // Adjacent open VCA intervals can share a nonroot boundary. Narrow until their
    // closed endpoints are separated as required by algebraic-number comparisons.
    for i in 1..out.len() {
        while out[i - 1].hi >= out[i].lo {
            ctx.tick()?;
            assert!(
                !out[i - 1].exact || !out[i].exact,
                "VCA cannot emit a root twice"
            );
            if !out[i - 1].exact {
                out[i - 1] = refinement::bisect_known(&original, &out[i - 1], ctx)?;
            }
            if !out[i].exact {
                out[i] = refinement::bisect_known(&original, &out[i], ctx)?;
            }
        }
    }
    Ok(Some(out))
}
fn prepare(f: &Poly, ctx: &Interrupt) -> Result<Option<Poly>, Abort> {
    ctx.tick()?;
    if f.is_zero() {
        return Ok(None);
    }
    let f = f.primitive_part(ctx)?;
    if f.degree().is_some_and(|n| n > 0) && !f.gcdheu(&f.derivative(ctx)?, ctx)?.is_one() {
        return Ok(None);
    }
    Ok(Some(f))
}
pub(crate) fn cauchy_power_of_two(f: &Poly, ctx: &Interrupt) -> Result<Integer, Abort> {
    ctx.tick()?;
    let n = f.degree().expect("invariant: nonzero bound input");
    let lc = f.coeffs[n].clone().max(-&f.coeffs[n]);
    let mut norm = Integer::ZERO;
    for c in &f.coeffs[..n] {
        ctx.tick()?;
        norm = norm.max(c.clone().max(-c));
    }
    let mut bound = Integer::ONE;
    while (&bound - 1) * &lc <= norm {
        ctx.tick()?;
        bound <<= 1;
    }
    Ok(bound)
}
fn map_point(a: &Rational, bound: &Integer, negative: bool) -> Rational {
    let a = a * Rational::from(bound.clone());
    if negative { -a } else { a }
}
fn map_interval(a: &Rational, b: &Rational, bound: &Integer, negative: bool) -> RootInterval {
    let (a, b) = (map_point(a, bound, negative), map_point(b, bound, negative));
    let (lo, hi) = if negative { (b, a) } else { (a, b) };
    RootInterval {
        lo,
        hi,
        exact: false,
    }
}
fn point(value: Rational) -> RootInterval {
    RootInterval {
        lo: value.clone(),
        hi: value,
        exact: true,
    }
}
fn sort(intervals: &mut Vec<RootInterval>, ctx: &Interrupt) -> Result<(), Abort> {
    let len = intervals.len();
    let mut width = 1;
    while width < len {
        let mut sorted = Vec::with_capacity(len);
        for start in (0..len).step_by(2 * width) {
            let mid = (start + width).min(len);
            let end = (start + 2 * width).min(len);
            let (mut a, mut b) = (start, mid);
            while a < mid || b < end {
                ctx.tick()?;
                if a < mid && (b == end || intervals[a].lo < intervals[b].lo) {
                    sorted.push(intervals[a].clone());
                    a += 1;
                } else {
                    sorted.push(intervals[b].clone());
                    b += 1;
                }
            }
        }
        *intervals = sorted;
        width *= 2;
    }
    Ok(())
}
