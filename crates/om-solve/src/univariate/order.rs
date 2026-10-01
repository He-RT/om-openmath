//! Pure Q roots are ordered by certified algebraic real/imaginary projections.
use super::Candidate;
use crate::SolveError;
use om_core::{Expr, canonical_cmp};
use om_num::{Integer, Rational, ctx::Interrupt};
use om_poly::{Algebraic, UPoly, isolate, real_alg};
use std::cmp::Ordering;
pub(super) struct Key {
    pub nonreal: bool,
    pub re: Algebraic,
    pub im: Algebraic,
}
pub(super) fn keys(p: &UPoly<Integer>, ctx: &Interrupt) -> Result<Vec<Key>, SolveError> {
    if p.degree() == Some(1) {
        return Ok(vec![Key {
            nonreal: false,
            re: Algebraic::Rational(
                -Rational::from(p.coeffs[0].clone()) / Rational::from(p.coeffs[1].clone()),
            ),
            im: Algebraic::Rational(Rational::ZERO),
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
            }
        } else {
            Key {
                nonreal: true,
                re: Algebraic::Rational(
                    -Rational::from(b.clone()) / Rational::from(Integer::from(2) * a),
                ),
                im: value,
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
        let re =
            a.re.cmp_real(&b.re, ctx)?
                .ok_or_else(|| SolveError::Unsupported("real root comparison failed".into()))?;
        if re != Ordering::Equal {
            return Ok(re);
        }
        let im = a
            .im
            .cmp_real(&b.im, ctx)?
            .ok_or_else(|| SolveError::Unsupported("imaginary root comparison failed".into()))?;
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
