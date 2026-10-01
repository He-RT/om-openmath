//! Pure Q roots are ordered by certified algebraic real/imaginary projections.
use super::Candidate;
use crate::SolveError;
use om_core::{Expr, canonical_cmp};
use om_num::{Integer, Rational, ctx::Interrupt};
use om_poly::{Algebraic, UPoly, isolate, real_alg};
use std::cell::RefCell;
use std::cmp::Ordering;
mod general;
pub(crate) struct Key {
    pub nonreal: bool,
    pub re: Algebraic,
    pub im: Algebraic,
    general: Option<general::ComplexKey>,
}
pub(crate) struct Coordinate {
    value: Expr,
    key: RefCell<Option<Key>>,
}
pub(crate) fn coordinate(value: &Expr) -> Coordinate {
    Coordinate {
        value: value.clone(),
        key: RefCell::new(None),
    }
}
/// Compare numeric coordinates using directed projections, then exact algebraic ties.
/// Parameter expressions retain the canonical symbolic ordering policy.
pub(crate) fn compare_coordinates(
    a_coordinate: &Coordinate,
    b_coordinate: &Coordinate,
    ctx: &Interrupt,
) -> Result<Ordering, SolveError> {
    ctx.tick()?;
    let a = &a_coordinate.value;
    let b = &b_coordinate.value;
    if a == b {
        return Ok(Ordering::Equal);
    }
    if !a.free_symbols().is_empty() || !b.free_symbols().is_empty() {
        return Ok(canonical_cmp(a, b));
    }
    if let (Some(a), Some(b)) = (
        om_simplify::numeval::enclose(a, 128, ctx)?,
        om_simplify::numeval::enclose(b, 128, ctx)?,
    ) {
        let classify = |b: &om_num::Ball| {
            if b.mid == om_num::BigFloat::ZERO && b.rad == om_num::BigFloat::ZERO {
                Some(false)
            } else if b.excludes_zero() {
                Some(true)
            } else {
                None
            }
        };
        if let (Some(ar), Some(br)) = (classify(&a.im), classify(&b.im)) {
            let real = ar.cmp(&br);
            if real != Ordering::Equal {
                return Ok(real);
            }
            for (aa, bb) in [(&a.re, &b.re), (&a.im, &b.im)] {
                let (Some(aa), Some(bb)) = (general::bounds(aa), general::bounds(bb)) else {
                    break;
                };
                if aa.1 < bb.0 {
                    return Ok(Ordering::Less);
                }
                if bb.1 < aa.0 {
                    return Ok(Ordering::Greater);
                }
                if aa.0 != aa.1 || bb.0 != bb.1 {
                    break;
                }
            }
        }
    }
    for c in [a_coordinate, b_coordinate] {
        if c.key.borrow().is_none() {
            let value =
                om_simplify::root_reduce::to_algebraic(&c.value, ctx)?.ok_or_else(|| {
                    SolveError::Unsupported("coordinate ordering certification unavailable".into())
                })?;
            *c.key.borrow_mut() = Some(general::key(value, ctx)?);
        }
    }
    let a_key = a_coordinate.key.borrow();
    let b_key = b_coordinate.key.borrow();
    let a = a_key
        .as_ref()
        .expect("invariant: coordinate key just certified");
    let b = b_key
        .as_ref()
        .expect("invariant: coordinate key just certified");
    let real = a.nonreal.cmp(&b.nonreal);
    if real != Ordering::Equal {
        return Ok(real);
    }
    let re = general::compare(a, b, false, ctx)?;
    if re != Ordering::Equal {
        return Ok(re);
    }
    general::compare(a, b, true, ctx)
}
pub(super) fn assign(
    p: &UPoly<Integer>,
    found: &mut [Candidate],
    ctx: &Interrupt,
) -> Result<(), SolveError> {
    if p.degree().is_some_and(|n| n > 2) {
        return general::assign(p, found, ctx);
    }
    let keys = keys(p, ctx)?;
    if found.len() != keys.len() {
        return Err(SolveError::Unsupported(
            "root ordering count mismatch".into(),
        ));
    }
    for (root, key) in found.iter_mut().zip(keys) {
        root.key = Some(key);
    }
    Ok(())
}
pub(super) fn keys(p: &UPoly<Integer>, ctx: &Interrupt) -> Result<Vec<Key>, SolveError> {
    if p.degree() == Some(1) {
        return Ok(vec![Key {
            nonreal: false,
            re: Algebraic::Rational(
                -Rational::from(p.coeffs[0].clone()) / Rational::from(p.coeffs[1].clone()),
            ),
            im: Algebraic::Rational(Rational::ZERO),
            general: None,
        }]);
    }
    let a = &p.coeffs[2];
    let b = &p.coeffs[1];
    let c = &p.coeffs[0];
    let d = b * b - Integer::from(4) * a * c;
    let real = d >= Integer::ZERO;
    let projection = if real {
        p.clone()
    } else {
        UPoly::new(vec![d, Integer::ZERO, Integer::from(4) * a * a])
    };
    let roots = isolate(&projection, ctx)?
        .ok_or_else(|| SolveError::Unsupported("root ordering isolation failed".into()))?;
    let mut keys = vec![];
    for root in roots {
        ctx.tick()?;
        let value = real_alg(&projection, (root.lo, root.hi), ctx)?
            .ok_or_else(|| SolveError::Unsupported("root ordering certification failed".into()))?;
        keys.push(if real {
            Key {
                nonreal: false,
                re: value,
                im: Algebraic::Rational(Rational::ZERO),
                general: None,
            }
        } else {
            Key {
                nonreal: true,
                re: Algebraic::Rational(
                    -Rational::from(b.clone()) / Rational::from(Integer::from(2) * a),
                ),
                im: value,
                general: None,
            }
        });
    }
    Ok(keys)
}
pub(super) fn sort(roots: &mut [Candidate], ctx: &Interrupt) -> Result<(), SolveError> {
    for i in 1..roots.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            if compare(&roots[j - 1], &roots[j], ctx)? != Ordering::Greater {
                break;
            }
            roots.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(())
}
fn compare(a: &Candidate, b: &Candidate, ctx: &Interrupt) -> Result<Ordering, SolveError> {
    if let (Some(a), Some(b)) = (&a.key, &b.key) {
        let real = a.nonreal.cmp(&b.nonreal);
        if real != Ordering::Equal {
            return Ok(real);
        }
        let re = general::compare(a, b, false, ctx)?;
        if re != Ordering::Equal {
            return Ok(re);
        }
        let im = general::compare(a, b, true, ctx)?;
        if im != Ordering::Equal {
            return Ok(im);
        }
    }
    Ok(canonical_cmp(&a.value, &b.value))
}
pub(super) fn polynomial(
    p: &UPoly<Integer>,
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Expr, om_num::ctx::Abort> {
    let mut terms = vec![];
    for (i, c) in p.coeffs.iter().enumerate() {
        ctx.tick()?;
        if !c.is_zero() {
            terms.push(om_core::mul([
                Expr::integer(c.clone()),
                om_core::pow(x.clone(), Expr::integer(Integer::from(i))),
            ]));
        }
    }
    Ok(om_core::add(terms))
}
