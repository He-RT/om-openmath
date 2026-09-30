//! Zassenhaus reconstruction of integer factors, from PLAN §8.2d.
mod modular;
mod recombine;
use crate::UPoly;
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
    isqrt,
};
use std::cmp::Ordering;
type Poly = UPoly<Integer>;
const SUBSET_LIMIT: usize = 65_536;

/// Whether all returned nonconstant factors were proved irreducible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactorStatus {
    /// All modular subset searches completed, certifying irreducibility over Z.
    Complete,
    /// A search reached the subset limit; some returned factors may still be reducible.
    PossiblyReducible,
}
/// Integer factorization: input = content * product(factor^multiplicity).
#[derive(Clone, Debug, PartialEq)]
pub struct IntegerFactorization {
    /// Signed integer content.
    pub content: Integer,
    /// Primitive positive-leading nonconstant factors with their positive multiplicities.
    pub factors: Vec<(Poly, u32)>,
    /// Completeness of the irreducibility search; reconstruction is always exact.
    pub status: FactorStatus,
}
impl UPoly<Integer> {
    /// Factor over Z using Yun, odd-prime DDF/EDF, quadratic Hensel and bounded Zassenhaus.
    /// Zero returns None; nonzero constants return signed content and no factors.
    /// All factors reconstruct exactly, including a PossiblyReducible remainder at the cap.
    pub fn factor_z(&self, ctx: &Interrupt) -> Result<Option<IntegerFactorization>, Abort> {
        factor(self, SUBSET_LIMIT, ctx)
    }
    /// The plan's Mignotte coefficient bound (isqrt(n+1)+1)*2^n*maxnorm(f)*abs(lc(f)).
    /// Zero has bound zero; integer powers and norm traversal are exact and portable.
    pub fn mignotte_bound(&self, ctx: &Interrupt) -> Result<Integer, Abort> {
        ctx.tick()?;
        let Some(n) = self.degree() else {
            return Ok(Integer::ZERO);
        };
        let mut norm = Integer::ZERO;
        for c in &self.coeffs {
            ctx.tick()?;
            norm = norm.max(c.clone().max(-c));
        }
        let lc = self.lc().expect("invariant: nonzero Mignotte input");
        Ok(
            (isqrt(&Integer::from(n + 1)) + 1_u8)
                * (Integer::ONE << n)
                * norm
                * lc.clone().max(-lc),
        )
    }
}
fn factor(
    input: &Poly,
    limit: usize,
    ctx: &Interrupt,
) -> Result<Option<IntegerFactorization>, Abort> {
    ctx.tick()?;
    if input.is_zero() {
        return Ok(None);
    }
    let (content, f) = input.content_pp(ctx)?;
    let mut factors = vec![];
    let mut status = FactorStatus::Complete;
    let mut valuation = 0;
    for c in &f.coeffs {
        ctx.tick()?;
        if !c.is_zero() {
            break;
        }
        valuation += 1;
    }
    if valuation > 0 {
        factors.push((
            Poly::new(vec![Integer::ZERO, Integer::ONE]),
            u32::try_from(valuation).expect("invariant: factor multiplicity fits u32"),
        ));
    }
    let f = Poly::new(f.coeffs[valuation..].to_vec());
    let sf = f
        .square_free(ctx)?
        .expect("invariant: removing a monomial leaves a nonzero polynomial");
    for (f, m) in sf.factors {
        ctx.tick()?;
        let (polys, complete) = zassenhaus(&f, limit, ctx)?;
        if !complete {
            status = FactorStatus::PossiblyReducible;
        }
        for f in polys {
            ctx.tick()?;
            factors.push((f, m));
        }
    }
    sort(&mut factors, ctx)?;
    Ok(Some(IntegerFactorization {
        content,
        factors,
        status,
    }))
}
fn zassenhaus(f: &Poly, limit: usize, ctx: &Interrupt) -> Result<(Vec<Poly>, bool), Abort> {
    ctx.tick()?;
    if f.degree().is_none_or(|n| n <= 1) {
        return Ok((vec![f.clone()], true));
    }
    let (p, factors) = modular::select(f, ctx)?;
    if factors.len() == 1 {
        return Ok((vec![f.clone()], true));
    }
    let bound = f.mignotte_bound(ctx)?;
    let mut modulus = Integer::from(p);
    let mut exponent = 1_u32;
    while modulus <= &bound * 2 {
        ctx.tick()?;
        modulus *= p;
        exponent = exponent
            .checked_add(1)
            .expect("invariant: lifting exponent fits u32");
    }
    let lifted = crate::hensel_lift(p, f, &factors, exponent, ctx)?
        .expect("invariant: selected good prime supplies complete coprime factors");
    recombine::run(f.clone(), lifted, &modulus, limit, ctx)
}
fn sort(factors: &mut [(Poly, u32)], ctx: &Interrupt) -> Result<(), Abort> {
    for i in 1..factors.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            if compare(&factors[j - 1].0, &factors[j].0, ctx)? != Ordering::Greater {
                break;
            }
            factors.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(())
}
fn compare(a: &Poly, b: &Poly, ctx: &Interrupt) -> Result<Ordering, Abort> {
    let degree = a.degree().cmp(&b.degree());
    if degree != Ordering::Equal {
        return Ok(degree);
    }
    for (a, b) in a.coeffs.iter().zip(&b.coeffs) {
        ctx.tick()?;
        let order = a.cmp(b);
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(Ordering::Equal)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hard_subset_cap_retains_an_exact_unfactored_remainder() {
        let ctx = Interrupt::default();
        let f = Poly::new(vec![
            Integer::from(4),
            Integer::ZERO,
            Integer::ZERO,
            Integer::ZERO,
            Integer::ONE,
        ]);
        let result = factor(&f, 0, &ctx).unwrap().unwrap();
        assert_eq!(result.status, FactorStatus::PossiblyReducible);
        assert_eq!(result.content, Integer::ONE);
        assert_eq!(result.factors, vec![(f, 1)]);
    }
    #[test]
    fn symmetric_residues_keep_the_positive_half_endpoint() {
        let ctx = Interrupt::default();
        let f = Poly::new(vec![
            Integer::from(5),
            Integer::from(-5),
            Integer::from(-6),
            Integer::from(16),
        ]);
        assert_eq!(
            recombine::symmetric(&f, &10.into(), &ctx).unwrap(),
            Poly::new(vec![5.into(), 5.into(), 4.into(), (-4).into()])
        );
    }
}
