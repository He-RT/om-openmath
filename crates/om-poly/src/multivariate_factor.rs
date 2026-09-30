//! Multivariate integer factorization by bounded Kronecker reconstruction.
mod kronecker;
mod subsets;
use crate::{
    FactorStatus, MPoly, Monomial, UPoly,
    gcd::{ground_content, ground_primitive},
};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
use std::cmp::Ordering;
type Sparse = MPoly<Integer>;

/// Exact multivariate factorization, with irreducibility search status.
#[derive(Clone, Debug, PartialEq)]
pub struct MultivariateFactorization {
    /// Signed ground content, chosen for a positive caller-order leading coefficient.
    pub content: Integer,
    /// Primitive positive-leading factors in the original variable/order context.
    pub factors: Vec<(Sparse, u32)>,
    /// Complete certification or a correctly reconstructing bounded partial result.
    pub status: FactorStatus,
}
impl MPoly<Integer> {
    /// Factor via integer Kronecker images and certified inverse subset trial divisions.
    /// Zero returns None; constants return signed content without nonconstant factors.
    /// At 4096 subset trials or encoded degree above 65536, return an exact partial
    /// factorization with PossiblyReducible status. All work charges the interrupt budget.
    pub fn factor_z(&self, ctx: &Interrupt) -> Result<Option<MultivariateFactorization>, Abort> {
        factor(self, 4096, ctx)
    }
}
fn factor(
    f: &Sparse,
    limit: usize,
    ctx: &Interrupt,
) -> Result<Option<MultivariateFactorization>, Abort> {
    ctx.tick()?;
    if f.is_zero() {
        return Ok(None);
    }
    let mut content = ground_content(f, ctx)?;
    if f.terms[0].1 < Integer::ZERO {
        content = -content;
    }
    let f = ground_primitive(f, ctx)?;
    let mut out = vec![];
    let mut minima = vec![u32::MAX; f.nvars];
    for (m, _) in &f.terms {
        for (minimum, e) in minima.iter_mut().zip(&m.exps) {
            ctx.tick()?;
            *minimum = (*minimum).min(*e);
        }
    }
    for (i, m) in minima.iter().enumerate() {
        ctx.tick()?;
        if *m > 0 {
            let mut exps = vec![0; f.nvars];
            exps[i] = 1;
            let factor = Sparse::new(
                f.nvars,
                vec![(
                    Monomial::new(exps).expect("invariant: a variable has total degree one"),
                    Integer::ONE,
                )],
                f.order,
                ctx,
            )?;
            out.push((factor, *m));
        }
    }
    let mut terms = Vec::with_capacity(f.terms.len());
    for (m, c) in &f.terms {
        ctx.tick()?;
        let mut exps = vec![];
        for (e, minimum) in m.exps.iter().zip(&minima) {
            ctx.tick()?;
            exps.push(e - minimum);
        }
        terms.push((
            Monomial::new(exps).expect("invariant: removing common powers reduces degree"),
            c.clone(),
        ));
    }
    let remainder = Sparse::new(f.nvars, terms, f.order, ctx)?;
    if remainder.is_one() {
        sort(&mut out, ctx)?;
        return Ok(Some(MultivariateFactorization {
            content,
            factors: out,
            status: FactorStatus::Complete,
        }));
    }
    let Some((mapping, image)) = kronecker::encode(&remainder, ctx)? else {
        out.push((remainder, 1));
        sort(&mut out, ctx)?;
        return Ok(Some(MultivariateFactorization {
            content,
            factors: out,
            status: FactorStatus::PossiblyReducible,
        }));
    };
    let image = image
        .factor_z(ctx)?
        .expect("invariant: injective Kronecker image is nonzero");
    let mut pieces = vec![];
    for (factor, m) in image.factors {
        for _ in 0..m {
            ctx.tick()?;
            pieces.push(factor.clone());
        }
    }
    let (found, complete) = subsets::run(remainder, pieces, &mapping, limit, ctx)?;
    for factor in found {
        ctx.tick()?;
        let mut existing = None;
        for (i, (old, _)) in out.iter().enumerate() {
            ctx.tick()?;
            if old == &factor {
                existing = Some(i);
                break;
            }
        }
        if let Some(i) = existing {
            out[i].1 = out[i]
                .1
                .checked_add(1)
                .expect("invariant: total factor multiplicity fits u32");
        } else {
            out.push((factor, 1));
        }
    }
    sort(&mut out, ctx)?;
    let status = if complete && image.status == FactorStatus::Complete {
        FactorStatus::Complete
    } else {
        FactorStatus::PossiblyReducible
    };
    Ok(Some(MultivariateFactorization {
        content,
        factors: out,
        status,
    }))
}
fn sort(factors: &mut [(Sparse, u32)], ctx: &Interrupt) -> Result<(), Abort> {
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
fn compare(a: &Sparse, b: &Sparse, ctx: &Interrupt) -> Result<Ordering, Abort> {
    let degree = |f: &Sparse| -> Result<u32, Abort> {
        let mut degree = 0;
        for (m, _) in &f.terms {
            ctx.tick()?;
            degree = degree.max(m.deg);
        }
        Ok(degree)
    };
    let order = degree(a)?.cmp(&degree(b)?);
    if order != Ordering::Equal {
        return Ok(order);
    }
    for ((ma, ca), (mb, cb)) in a.terms.iter().zip(&b.terms) {
        ctx.tick()?;
        let order = ma.cmp(mb, a.order).reverse().then_with(|| ca.cmp(cb));
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(a.terms.len().cmp(&b.terms.len()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subset_cap_returns_the_original_primitive_remainder() {
        let ctx = Interrupt::default();
        let f = Sparse::new(
            2,
            vec![
                (Monomial::new([2, 0]).unwrap(), 1.into()),
                (Monomial::new([0, 2]).unwrap(), (-1).into()),
            ],
            crate::MonoOrder::Lex,
            &ctx,
        )
        .unwrap();
        let result = factor(&f, 0, &ctx).unwrap().unwrap();
        assert_eq!(result.status, FactorStatus::PossiblyReducible);
        assert_eq!(result.factors, vec![(f, 1)]);
    }
}
