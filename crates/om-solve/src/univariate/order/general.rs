//! Exact root identities break projection ties; disjoint balls certify strict order.
use super::{Candidate, Key};
use crate::SolveError;
use om_core::{BUILTIN as B, Expr};
use om_num::{Ball, BigFloat, CBall, Integer, Rational, ctx::Interrupt};
use om_poly::{Algebraic, UPoly, algebraic_root, algebraic_roots};
use std::{cell::RefCell, cmp::Ordering};
pub(super) struct ComplexKey {
    value: Algebraic,
    conjugate: Algebraic,
    re: RefCell<Option<Algebraic>>,
    im: RefCell<Option<Algebraic>>,
}
pub(super) fn assign(
    p: &UPoly<Integer>,
    found: &mut [Candidate],
    ctx: &Interrupt,
) -> Result<(), SolveError> {
    let n = p.degree().expect("invariant: nonconstant factor");
    if found.iter().map(|r| r.multiplicity as usize).sum::<usize>() != n {
        return Err(SolveError::Unsupported(
            "radical root count mismatch".into(),
        ));
    }
    let roots = algebraic_roots(p, ctx)?
        .ok_or_else(|| SolveError::Unsupported("radical root certification failed".into()))?;
    let real_count = roots
        .iter()
        .take_while(|r| !matches!(r, Algebraic::Complex(_)))
        .count();
    let mut used = vec![false; n];
    let mut certified = roots
        .iter()
        .map(|r| enclosure_of(r, 128, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    let function = Expr::call(
        B::FUNCTION,
        [super::polynomial(
            p,
            &Expr::call(B::SLOT, [Expr::int(1)]),
            ctx,
        )?],
    );
    let mut proofs = vec![];
    for candidate in found {
        let mut bits = 128;
        let index = loop {
            ctx.tick()?;
            if candidate.value.is_head(B::ROOT)
                && candidate.value.args().len() == 2
                && candidate.value.args()[0] == function
                && let Some(om_num::Number::Integer(k)) = candidate.value.args()[1].as_number()
                && let Ok(k) = usize::try_from(k)
                && (1..=n).contains(&k)
            {
                break k - 1;
            }
            let z = om_simplify::numeval::enclose(&candidate.value, bits, ctx)?
                .ok_or_else(|| SolveError::Unsupported("radical enclosure failed".into()))?;
            let mut matching = vec![];
            for (i, b) in certified.iter().enumerate() {
                ctx.tick()?;
                if overlaps(&z, b) {
                    matching.push(i);
                }
            }
            if matching.len() == 1 {
                break matching[0];
            }
            if bits == 16_352 {
                return Err(SolveError::Unsupported(
                    "ambiguous radical root association".into(),
                ));
            }
            bits = (bits * 2).min(16_352);
            certified = roots
                .iter()
                .map(|r| enclosure_of(r, bits, ctx))
                .collect::<Result<Vec<_>, _>>()?;
        };
        if used[index] {
            return Err(SolveError::Unsupported(
                "duplicate radical root association".into(),
            ));
        }
        used[index] = true;
        let value = roots[index].clone();
        proofs.push((candidate.value.clone(), value.clone()));
        if index < real_count {
            om_simplify::numeval::remember_real(&candidate.value, &value);
        }
        candidate.key = Some(if index < real_count {
            Key {
                nonreal: false,
                re: value,
                im: Algebraic::Rational(Rational::ZERO),
                general: None,
            }
        } else {
            let pair = if (index - real_count).is_multiple_of(2) {
                index + 1
            } else {
                index - 1
            };
            Key {
                nonreal: true,
                re: Algebraic::Rational(Rational::ZERO),
                im: Algebraic::Rational(Rational::ZERO),
                general: Some(ComplexKey {
                    value,
                    conjugate: roots[pair].clone(),
                    re: RefCell::new(None),
                    im: RefCell::new(None),
                }),
            }
        });
    }
    for (e, value) in proofs {
        super::remember(&e, &value);
    }
    Ok(())
}
fn enclosure_of(value: &Algebraic, bits: u32, ctx: &Interrupt) -> Result<CBall, SolveError> {
    ctx.tick()?;
    if let Algebraic::Complex(c) = value
        && c.disk.re.prec >= bits
        && c.disk.im.prec >= bits
    {
        return Ok(c.disk.clone());
    }
    value
        .enclosure(bits, ctx)?
        .ok_or_else(|| SolveError::Unsupported("certified root enclosure failed".into()))
}
pub(super) fn bounds(b: &Ball) -> Option<(Rational, Rational)> {
    fn dyadic(v: &BigFloat) -> Option<Rational> {
        let r = v.repr();
        if !r.is_finite() || r.exponent().unsigned_abs() > 262144 {
            return None;
        }
        let q = Rational::from(r.significand().clone());
        Some(if r.exponent() >= 0 {
            q * Rational::from(Integer::ONE << r.exponent() as usize)
        } else {
            q / Rational::from(Integer::ONE << r.exponent().unsigned_abs())
        })
    }
    let (m, r) = (dyadic(&b.mid)?, dyadic(&b.rad)?);
    Some((&m - &r, m + r))
}
pub(super) fn key(value: Algebraic, ctx: &Interrupt) -> Result<Key, SolveError> {
    if let Algebraic::Complex(c) = &value {
        let real_count = om_poly::isolate(&c.minpoly, ctx)?
            .ok_or_else(|| SolveError::Unsupported("coordinate real count unavailable".into()))?
            .len();
        let offset = c.index - 1 - real_count;
        let pair = if offset.is_multiple_of(2) {
            c.index + 1
        } else {
            c.index - 1
        };
        let conjugate = algebraic_root(&c.minpoly, pair, ctx)?
            .ok_or_else(|| SolveError::Unsupported("coordinate conjugate unavailable".into()))?;
        Ok(Key {
            nonreal: true,
            re: Algebraic::Rational(Rational::ZERO),
            im: Algebraic::Rational(Rational::ZERO),
            general: Some(ComplexKey {
                value,
                conjugate,
                re: RefCell::new(None),
                im: RefCell::new(None),
            }),
        })
    } else {
        Ok(Key {
            nonreal: false,
            re: value,
            im: Algebraic::Rational(Rational::ZERO),
            general: None,
        })
    }
}
fn overlaps(a: &CBall, b: &CBall) -> bool {
    [&a.re, &a.im]
        .into_iter()
        .zip([&b.re, &b.im])
        .all(|(a, b)| match (bounds(a), bounds(b)) {
            (Some(a), Some(b)) => a.0 <= b.1 && b.0 <= a.1,
            _ => false,
        })
}
fn enclosure(
    k: &Key,
    imaginary: bool,
    ctx: &Interrupt,
) -> Result<(Rational, Rational), SolveError> {
    let b = if let Some(g) = &k.general {
        let z = enclosure_of(&g.value, 128, ctx)?;
        if imaginary { z.im } else { z.re }
    } else {
        let a = if imaginary { &k.im } else { &k.re };
        a.enclosure(128, ctx)?
            .ok_or_else(|| SolveError::Unsupported("real projection enclosure failed".into()))?
            .re
    };
    bounds(&b).ok_or_else(|| SolveError::Unsupported("nonfinite projection enclosure".into()))
}
pub(super) fn compare(
    a: &Key,
    b: &Key,
    imaginary: bool,
    ctx: &Interrupt,
) -> Result<Ordering, SolveError> {
    ctx.tick()?;
    if !imaginary
        && let (Some(a), Some(b)) = (&a.general, &b.general)
        && (a.value.equals(&b.value, ctx)? == Some(true)
            || a.value.equals(&b.conjugate, ctx)? == Some(true))
    {
        return Ok(Ordering::Equal);
    }
    if a.general.is_none() && b.general.is_none() {
        let (a, b) = if imaginary {
            (&a.im, &b.im)
        } else {
            (&a.re, &b.re)
        };
        return a
            .cmp_real(b, ctx)?
            .ok_or_else(|| SolveError::Unsupported("exact projection comparison failed".into()));
    }
    let (av, bv) = (enclosure(a, imaginary, ctx)?, enclosure(b, imaginary, ctx)?);
    if av.1 < bv.0 {
        return Ok(Ordering::Less);
    }
    if bv.1 < av.0 {
        return Ok(Ordering::Greater);
    }
    let (a, b) = (
        projection(a, imaginary, ctx)?,
        projection(b, imaginary, ctx)?,
    );
    a.cmp_real(&b, ctx)?
        .ok_or_else(|| SolveError::Unsupported("radical projection comparison failed".into()))
}
fn projection(k: &Key, imaginary: bool, ctx: &Interrupt) -> Result<Algebraic, SolveError> {
    let Some(g) = &k.general else {
        return Ok(if imaginary {
            k.im.clone()
        } else {
            k.re.clone()
        });
    };
    let cache = if imaginary { &g.im } else { &g.re };
    if let Some(value) = cache.borrow().as_ref() {
        return Ok(value.clone());
    }
    let value = if imaginary {
        let i = algebraic_root(
            &UPoly::new(vec![Integer::ONE, Integer::ZERO, Integer::ONE]),
            2,
            ctx,
        )?
        .ok_or_else(|| SolveError::Unsupported("imaginary unit certification failed".into()))?;
        let Some(difference) = g.value.sub(&g.conjugate, ctx)? else {
            return Err(SolveError::Unsupported(
                "imaginary projection failed".into(),
            ));
        };
        let Some(twice_i) = i.mul(&Algebraic::Rational(Rational::from(2)), ctx)? else {
            return Err(SolveError::Unsupported(
                "imaginary projection scale failed".into(),
            ));
        };
        difference.div(&twice_i, ctx)?
    } else {
        let negative = g
            .value
            .neg(ctx)?
            .ok_or_else(|| SolveError::Unsupported("projection negation failed".into()))?;
        if negative.equals(&g.conjugate, ctx)? == Some(true) {
            Some(Algebraic::Rational(Rational::ZERO))
        } else {
            match g.value.add(&g.conjugate, ctx)? {
                Some(sum) => {
                    sum.mul(&Algebraic::Rational(Rational::ONE / Rational::from(2)), ctx)?
                }
                None => None,
            }
        }
    }
    .ok_or_else(|| {
        SolveError::Unsupported("exact radical projection exceeds algebraic limits".into())
    })?;
    *cache.borrow_mut() = Some(value.clone());
    Ok(value)
}
