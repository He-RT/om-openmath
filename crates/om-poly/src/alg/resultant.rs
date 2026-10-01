//! Annihilators in x obtained by eliminating y from the plan's exact identities.
use super::Poly;
use crate::{MPoly, MonoOrder, Monomial, UPoly};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
pub(super) fn annihilator(
    p: &Poly,
    q: &Poly,
    multiply: bool,
    ctx: &Interrupt,
) -> Result<Poly, Abort> {
    let n = q
        .degree()
        .expect("invariant: nonconstant algebraic minpoly");
    let mut left = Vec::with_capacity(p.coeffs.len());
    for c in &p.coeffs {
        ctx.tick()?;
        left.push(sparse(vec![(0, c.clone())], ctx)?);
    }
    let mut terms = vec![vec![]; n + 1];
    for j in 0..=n {
        ctx.tick()?;
        if multiply {
            terms[n - j].push((j, q.coeffs[j].clone()));
        } else {
            let mut binomial = Integer::ONE;
            for (k, term) in terms.iter_mut().enumerate().take(j + 1) {
                ctx.tick()?;
                let c = &q.coeffs[j] * &binomial;
                term.push((j - k, if k % 2 == 0 { c } else { -c }));
                if k < j {
                    binomial = binomial * Integer::from(j - k) / Integer::from(k + 1);
                }
            }
        }
    }
    let mut right = Vec::with_capacity(terms.len());
    for term in terms {
        ctx.tick()?;
        right.push(sparse(term, ctx)?);
    }
    let res = UPoly::new(left).resultant(&UPoly::new(right), ctx)?;
    let degree = res
        .terms
        .iter()
        .map(|(m, _)| m.exps[0] as usize)
        .max()
        .unwrap_or(0);
    let mut out = vec![Integer::ZERO; degree + 1];
    for (m, c) in res.terms {
        ctx.tick()?;
        out[m.exps[0] as usize] = c;
    }
    Poly::new(out).primitive_part(ctx)
}
fn sparse(terms: Vec<(usize, Integer)>, ctx: &Interrupt) -> Result<MPoly<Integer>, Abort> {
    let mut out = Vec::with_capacity(terms.len());
    for (e, c) in terms {
        ctx.tick()?;
        out.push((
            Monomial::new([e as u32]).expect("invariant: capped algebraic degree fits u32"),
            c,
        ));
    }
    MPoly::new(1, out, MonoOrder::Lex, ctx)
}
