//! Pure Q roots are ordered by certified algebraic real/imaginary projections.
use super::Candidate;
use crate::SolveError;
use om_core::{Expr, canonical_cmp};
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{Algebraic, UPoly, isolate, real_alg};
use std::cell::RefCell;
use std::{cmp::Ordering, rc::Rc};
mod general;
thread_local! {
    static CERTIFICATES: RefCell<Vec<(Expr,Algebraic)>> = const {RefCell::new(Vec::new())};
}
pub(super) fn remember(e: &Expr, value: &Algebraic) {
    CERTIFICATES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((_, a)) = cache.iter_mut().find(|(v, _)| v == e) {
            *a = value.clone();
        } else {
            if cache.len() == 256 {
                cache.remove(0);
            }
            cache.push((e.clone(), value.clone()));
        }
    });
}
pub(crate) fn certified(e: &Expr) -> Option<Algebraic> {
    CERTIFICATES.with(|cache| {
        cache
            .borrow()
            .iter()
            .rev()
            .find(|(v, _)| v == e)
            .map(|(_, a)| a.clone())
    })
}
pub(crate) fn algebraic(e: &Expr, ctx: &Interrupt) -> Result<Option<Algebraic>, Abort> {
    ctx.tick()?;
    if let Some(value) = certified(e) {
        Ok(Some(value))
    } else {
        // Primitive-element recovery is a small polynomial in radicals. Expanding
        // that private projection avoids resultants for terms that cancel exactly.
        let expanded = om_simplify::algebra::expand_with(e, ctx)?;
        let projection = expanded.as_ref().unwrap_or(e);
        let value = if let Some(value) = certified(projection) {
            Some(value)
        } else {
            let value = om_simplify::root_reduce::to_algebraic(projection, ctx)?;
            if let Some(value) = &value {
                remember(projection, value);
            }
            value
        };
        if let Some(value) = &value {
            remember(e, value);
            om_simplify::numeval::remember_real(e, value);
        }
        Ok(value)
    }
}
pub(crate) struct Key {
    pub nonreal: bool,
    pub re: Algebraic,
    pub im: Algebraic,
    general: Option<general::ComplexKey>,
}
type CachedComplexKey = (UPoly<Integer>, usize, Rc<Key>);
thread_local! {
    static COMPLEX_KEYS: RefCell<Vec<CachedComplexKey>> = const { RefCell::new(Vec::new()) };
}
fn coordinate_key(value: Algebraic, ctx: &Interrupt) -> Result<Rc<Key>, SolveError> {
    use om_num::BitTest;
    ctx.tick()?;
    let identity = if let Algebraic::Complex(c) = &value
        && c.minpoly.coeffs.iter().all(|n| n.bit_len() <= 4096)
    {
        Some((c.minpoly.clone(), c.index))
    } else {
        None
    };
    if let Some((p, index)) = &identity
        && let Some(key) = COMPLEX_KEYS.with(|keys| {
            keys.borrow()
                .iter()
                .find(|(q, i, _)| p == q && index == i)
                .map(|(_, _, key)| key.clone())
        })
    {
        return Ok(key);
    }
    let key = Rc::new(general::key(value, ctx)?);
    if let Some((p, index)) = identity {
        COMPLEX_KEYS.with(|keys| {
            let mut keys = keys.borrow_mut();
            if keys.len() == 32 {
                keys.remove(0);
            }
            keys.push((p, index, key.clone()));
        });
    }
    Ok(key)
}
pub(crate) struct Coordinate {
    value: Expr,
    key: RefCell<Option<Rc<Key>>>,
    enclosure: RefCell<Option<Option<om_num::CBall>>>,
}
pub(crate) fn coordinate(value: &Expr) -> Coordinate {
    Coordinate {
        value: value.clone(),
        key: RefCell::new(None),
        enclosure: RefCell::new(None),
    }
}
impl Coordinate {
    fn enclosure(&self, ctx: &Interrupt) -> Result<Option<om_num::CBall>, Abort> {
        ctx.tick()?;
        if let Some(value) = &*self.enclosure.borrow() {
            return Ok(value.clone());
        }
        let value = if let Some(certificate) = certified(&self.value) {
            certificate.enclosure(128, ctx)?
        } else {
            om_simplify::numeval::enclose(&self.value, 128, ctx)?
        };
        *self.enclosure.borrow_mut() = Some(value.clone());
        Ok(value)
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
    if let (Some(a), Some(b)) = (a_coordinate.enclosure(ctx)?, b_coordinate.enclosure(ctx)?) {
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
            let value = algebraic(&c.value, ctx)?.ok_or_else(|| {
                SolveError::Unsupported("coordinate ordering certification unavailable".into())
            })?;
            *c.key.borrow_mut() = Some(coordinate_key(value, ctx)?);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identical_complex_root_certificates_compare_both_components_without_reprojection() {
        let root = om_poly::algebraic_root(
            &UPoly::new(vec![Integer::ONE, Integer::ZERO, Integer::ONE]),
            2,
            &Interrupt::default(),
        )
        .unwrap()
        .unwrap();
        let a = general::key(root.clone(), &Interrupt::default()).unwrap();
        let b = general::key(root, &Interrupt::default()).unwrap();
        for imaginary in [false, true] {
            let ctx = Interrupt {
                steps_left: std::cell::Cell::new(2),
                ..Interrupt::default()
            };
            assert_eq!(
                general::compare(&a, &b, imaginary, &ctx).unwrap(),
                Ordering::Equal
            );
        }
    }
    #[test]
    fn polynomial_recovery_coordinates_use_a_bounded_exact_expansion_for_ties() {
        let value = om_core::canonicalize(
            &om_parse::parse_expr(
                "((-I/2*Sqrt[3]-5/2)^3/126)-((-I/2*Sqrt[3]-5/2)^2/21)-8*(-I/2*Sqrt[3]-5/2)/21+7/18",
                om_parse::Dialect::Wolfram,
            )
            .unwrap(),
        );
        let ctx = Interrupt {
            steps_left: std::cell::Cell::new(4096),
            ..Interrupt::default()
        };
        assert_eq!(
            compare_coordinates(&coordinate(&value), &coordinate(&Expr::int(1)), &ctx).unwrap(),
            Ordering::Equal
        );
        let numeric = om_simplify::numeval::approximate(
            &value,
            om_num::Precision::Bits(100),
            &Interrupt::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(numeric.to_complex_f64(), (1.0, 0.0));
        assert!(!matches!(numeric, om_num::Number::Complex(_)));
    }
}
