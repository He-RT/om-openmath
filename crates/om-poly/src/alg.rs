//! Certified real/complex algebraic numbers, independent of expression representation.
mod bounds;
mod comparison;
mod operations;
mod ordering;
mod resultant;
use crate::{FactorStatus, UPoly, complex_roots, isolate};
use bounds::{ball_bounds, overlap, real_ball, refine_iv};
use om_num::{
    CBall, Integer, Rational,
    ctx::{Abort, Interrupt},
};
type Poly = UPoly<Integer>;
pub(super) const DEGREE_LIMIT: usize = 64;
pub(super) const MAX_BITS: u32 = 16_352;

/// A real root of a primitive irreducible positive-leading integer polynomial.
/// Construct through real_alg/algebraic_root; iv is a certified unique-root interval.
#[derive(Clone, Debug)]
pub struct RealAlg {
    /// Primitive irreducible positive-leading minimal polynomial of degree >=2.
    pub minpoly: UPoly<Integer>,
    /// Exact ordered endpoints enclosing only this simple root.
    pub iv: (Rational, Rational),
}
/// A nonreal root, isolated by a certified rectangular enclosure.
/// Construct through algebraic_root; index is one-based within its minimal polynomial.
#[derive(Clone, Debug)]
pub struct ComplexAlg {
    /// Primitive irreducible positive-leading minimal polynomial.
    pub minpoly: UPoly<Integer>,
    /// Ball enclosure containing exactly this root of minpoly.
    pub disk: CBall,
    /// Root number within minpoly, following PLAN §8.2g.
    pub index: usize,
}
/// Exact algebraic values; degree-one minimal polynomials demote to rational numbers.
#[derive(Clone, Debug)]
pub enum Algebraic {
    /// Exact rational value.
    Rational(Rational),
    /// Certified real irrational value.
    Real(RealAlg),
    /// Certified nonreal value.
    Complex(ComplexAlg),
}
impl Algebraic {
    /// Return the primitive positive-leading minimal polynomial, including rational values.
    pub fn minimal_polynomial(&self, ctx: &Interrupt) -> Result<UPoly<Integer>, Abort> {
        ctx.tick()?;
        Ok(match self {
            Self::Rational(q) => Poly::new(vec![
                -q.numerator().clone(),
                Integer::from(q.denominator().clone()),
            ]),
            Self::Real(a) => a.minpoly.clone(),
            Self::Complex(a) => a.minpoly.clone(),
        })
    }
    /// Exact zero test for values created through the certified constructors.
    pub fn is_zero(&self) -> bool {
        matches!(self,Self::Rational(q) if q==&Rational::ZERO)
    }
    /// Certified rectangular enclosure at the requested precision; unsupported bits yield None.
    pub fn enclosure(&self, bits: u32, ctx: &Interrupt) -> Result<Option<CBall>, Abort> {
        ctx.tick()?;
        if bits == 0 || bits > MAX_BITS {
            return Ok(None);
        }
        let Some(a) = self.refined(bits, ctx)? else {
            return Ok(None);
        };
        Ok(Some(a.current_ball(bits)))
    }
    pub(super) fn current_ball(&self, bits: u32) -> CBall {
        match self {
            Self::Rational(q) => CBall::exact(q, &Rational::ZERO, bits),
            Self::Real(a) => real_ball(&a.iv, bits),
            Self::Complex(a) => a.disk.clone(),
        }
    }
    pub(super) fn refined(&self, bits: u32, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        if bits == 0 {
            return Ok(None);
        }
        match self {
            Self::Rational(_) => Ok(Some(self.clone())),
            Self::Real(a) => Ok(refine_iv(&a.minpoly, &a.iv, bits, ctx)?.map(|iv| {
                Self::Real(RealAlg {
                    minpoly: a.minpoly.clone(),
                    iv,
                })
            })),
            Self::Complex(a) => {
                if bits > MAX_BITS {
                    return Ok(None);
                }
                let Some(old) = ball_bounds(&a.disk) else {
                    return Ok(None);
                };
                let mut work = bits;
                loop {
                    ctx.tick()?;
                    let Some(disks) = complex_roots(&a.minpoly, work, ctx)? else {
                        return Ok(None);
                    };
                    let balls: Vec<_> = disks.iter().map(|d| d.to_cball()).collect();
                    let bounds: Vec<_> = balls
                        .iter()
                        .map(|b| ball_bounds(b).expect("invariant: finite certified root balls"))
                        .collect();
                    let mut found = None;
                    let mut ambiguous = false;
                    for (i, b) in bounds.iter().enumerate() {
                        ctx.tick()?;
                        if old.intersects(b) {
                            if found.is_some() {
                                ambiguous = true;
                                break;
                            }
                            found = Some(i);
                        }
                    }
                    if let Some(i) = found
                        && !ambiguous
                    {
                        // Circle disjointness alone does not prove that the enclosing
                        // rectangle excludes diagonally neighboring roots.
                        for (j, other) in bounds.iter().enumerate() {
                            ctx.tick()?;
                            if i != j && bounds[i].intersects(other) {
                                ambiguous = true;
                                break;
                            }
                        }
                        if !ambiguous {
                            return Ok(Some(Self::Complex(ComplexAlg {
                                minpoly: a.minpoly.clone(),
                                disk: balls[i].clone(),
                                index: a.index,
                            })));
                        }
                    }
                    let Some(next) = next_precision(work, true) else {
                        return Ok(None);
                    };
                    work = next;
                }
            }
        }
    }
}
pub(super) fn next_precision(bits: u32, complex: bool) -> Option<u32> {
    if complex {
        (bits < MAX_BITS).then(|| (bits * 2).min(MAX_BITS))
    } else {
        bits.checked_mul(2)
    }
}
/// Select the unique real root in iv and reduce its defining polynomial over Q.
/// Invalid intervals, constants, degree >64 or incomplete factorization return None.
pub fn real_alg(
    f: &UPoly<Integer>,
    iv: (Rational, Rational),
    ctx: &Interrupt,
) -> Result<Option<Algebraic>, Abort> {
    ctx.tick()?;
    let Some(factors) = factors(f, ctx)? else {
        return Ok(None);
    };
    let mut squarefree = Poly::one();
    for p in &factors {
        ctx.tick()?;
        squarefree = squarefree.mul(p, ctx)?;
    }
    let Some(iv) = refine_iv(&squarefree, &iv, 32, ctx)? else {
        return Ok(None);
    };
    operations::select_real(&factors, iv, ctx)
}
/// Select a one-based distinct root, with reals first and conjugate pairs negative-first.
/// Input multiplicities are ignored; irreducible factors and root identity are certified.
/// Invalid index, zero/constant input, degree >64 or exhausted certification return None.
pub fn algebraic_root(
    f: &UPoly<Integer>,
    index: usize,
    ctx: &Interrupt,
) -> Result<Option<Algebraic>, Abort> {
    ctx.tick()?;
    if index == 0 {
        return Ok(None);
    }
    Ok(algebraic_roots(f, ctx)?.and_then(|roots| roots.into_iter().nth(index - 1)))
}
/// Certify all distinct roots in the same one-based order used by algebraic_root.
/// Shares one factorization/isolation/sort; multiplicities are ignored.
/// Zero/constant input, degree >64 or exhausted certification returns None.
pub fn algebraic_roots(
    f: &UPoly<Integer>,
    ctx: &Interrupt,
) -> Result<Option<Vec<Algebraic>>, Abort> {
    ctx.tick()?;
    let Some(factors) = factors(f, ctx)? else {
        return Ok(None);
    };
    let mut all = vec![];
    for p in factors {
        ctx.tick()?;
        let Some(roots) = ordering::roots(&p, ctx)? else {
            return Ok(None);
        };
        all.extend(roots);
    }
    let Some(all) = ordering::sort(all, ctx)? else {
        return Ok(None);
    };
    Ok(Some(all))
}
pub(super) fn factors(f: &Poly, ctx: &Interrupt) -> Result<Option<Vec<Poly>>, Abort> {
    ctx.tick()?;
    if !f.degree().is_some_and(|n| n > 0 && n <= DEGREE_LIMIT) {
        return Ok(None);
    }
    let Some(factorization) = f.factor_z(ctx)? else {
        return Ok(None);
    };
    if factorization.status != FactorStatus::Complete {
        return Ok(None);
    }
    Ok(Some(
        factorization.factors.into_iter().map(|(p, _)| p).collect(),
    ))
}
pub(super) fn real_roots(f: &Poly, ctx: &Interrupt) -> Result<Option<Vec<Algebraic>>, Abort> {
    ctx.tick()?;
    if f.degree() == Some(1) {
        return Ok(Some(vec![Algebraic::Rational(
            -Rational::from(f.coeffs[0].clone()) / Rational::from(f.coeffs[1].clone()),
        )]));
    }
    let Some(roots) = isolate(f, ctx)? else {
        return Ok(None);
    };
    Ok(Some(
        roots
            .into_iter()
            .map(|r| {
                Algebraic::Real(RealAlg {
                    minpoly: f.clone(),
                    iv: (r.lo, r.hi),
                })
            })
            .collect(),
    ))
}
pub(super) fn real_intersects(a: &Algebraic, b: &(Rational, Rational)) -> bool {
    bounds::real_bounds(a).is_some_and(|iv| overlap(&iv, b))
}
