//! Integer polynomial GCDs with exact certificates and recursive coefficient rings.
mod heuristic;
mod prs;
mod sparse;

use crate::{MPoly, MonoOrder, Monomial, UPoly};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
    gcd,
};
use sparse::{join, split};

pub(super) type Sparse = MPoly<Integer>;
pub(super) type Dense = UPoly<Sparse>;

impl UPoly<Integer> {
    /// Positive-leading integer GCD using six heuristic attempts, then Collins PRS.
    pub fn gcdheu(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        let (f, g) = (from_univariate(self, ctx)?, from_univariate(other, ctx)?);
        to_univariate(&f.gcdheu(&g, ctx)?, ctx)
    }
    /// Positive-leading integer GCD using the subresultant remainder sequence.
    pub fn subresultant_gcd(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        let (f, g) = (from_univariate(self, ctx)?, from_univariate(other, ctx)?);
        to_univariate(&f.subresultant_gcd(&g, ctx)?, ctx)
    }
}
impl MPoly<Integer> {
    /// Integer GCD, recursively evaluating the last variable and certifying trial divisions.
    /// Zero inputs retain the nonzero input's content; gcd(0,0)=0.
    /// Canonical compatible contexts and representable intermediate degrees are required.
    pub fn gcdheu(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        let (f, g) = compatible(self, other, ctx)?;
        with_attempts(&f, &g, 6, ctx)
    }
    /// Recursive Collins PRS GCD in Z[x1,...,xn-1][xn], with positive leading coefficient.
    /// Canonical compatible contexts and representable intermediate degrees are required.
    pub fn subresultant_gcd(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        let (f, g) = compatible(self, other, ctx)?;
        prs::compute(&f, &g, ctx)
    }
}
impl UPoly<MPoly<Integer>> {
    /// GCDHEU with the dense variable following all sparse coefficient variables.
    pub fn gcdheu(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        let (f, g) = from_parameters(self, other, ctx)?;
        split(&f.gcdheu(&g, ctx)?, ctx)
    }
    /// Subresultant GCD with recursive polynomial coefficient contents.
    pub fn subresultant_gcd(&self, other: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        let (f, g) = from_parameters(self, other, ctx)?;
        split(&f.subresultant_gcd(&g, ctx)?, ctx)
    }
}

fn with_attempts(
    f: &Sparse,
    g: &Sparse,
    attempts: usize,
    ctx: &Interrupt,
) -> Result<Sparse, Abort> {
    if let Some(h) = heuristic::attempt(f, g, attempts, ctx)? {
        return Ok(h);
    }
    prs::compute(f, g, ctx)
}

pub(super) fn compatible(
    f: &Sparse,
    g: &Sparse,
    ctx: &Interrupt,
) -> Result<(Sparse, Sparse), Abort> {
    ctx.tick()?;
    let (nvars, order) = if f.nvars == 0 {
        (g.nvars, g.order)
    } else {
        (f.nvars, f.order)
    };
    if f.nvars != 0 && g.nvars != 0 {
        assert_eq!(f.nvars, g.nvars, "GCD variable contexts must match");
        assert_eq!(f.order, g.order, "GCD monomial orders must match");
    }
    let zero = Sparse::zero_in(nvars, order);
    Ok((f.add(&zero, ctx)?, g.add(&zero, ctx)?))
}
pub(super) fn positive(f: Sparse, ctx: &Interrupt) -> Result<Sparse, Abort> {
    ctx.tick()?;
    if f.terms.first().is_some_and(|(_, c)| c < &Integer::ZERO) {
        f.neg(ctx)
    } else {
        Ok(f)
    }
}
pub(super) fn constant(
    nvars: usize,
    order: MonoOrder,
    c: Integer,
    ctx: &Interrupt,
) -> Result<Sparse, Abort> {
    ctx.tick()?;
    let monomial = Monomial::new(std::iter::repeat_n(0, nvars))
        .expect("invariant: a constant monomial has zero degree");
    Sparse::new(nvars, vec![(monomial, c)], order, ctx)
}
pub(super) fn scale(f: &Sparse, c: &Integer, ctx: &Interrupt) -> Result<Sparse, Abort> {
    let mut terms = Vec::with_capacity(f.terms.len());
    for (m, a) in &f.terms {
        ctx.tick()?;
        terms.push((m.clone(), a * c));
    }
    Sparse::new(f.nvars, terms, f.order, ctx)
}
pub(super) fn ground_content(f: &Sparse, ctx: &Interrupt) -> Result<Integer, Abort> {
    ctx.tick()?;
    let mut c = Integer::ZERO;
    for (_, a) in &f.terms {
        ctx.tick()?;
        c = gcd(&c, a);
    }
    Ok(c)
}
pub(super) fn ground_primitive(f: &Sparse, ctx: &Interrupt) -> Result<Sparse, Abort> {
    let c = ground_content(f, ctx)?;
    if c.is_zero() {
        return Ok(f.clone());
    }
    let mut terms = Vec::with_capacity(f.terms.len());
    for (m, a) in &f.terms {
        ctx.tick()?;
        terms.push((m.clone(), a / &c));
    }
    positive(Sparse::new(f.nvars, terms, f.order, ctx)?, ctx)
}
pub(super) fn trivial(f: &Sparse, g: &Sparse, ctx: &Interrupt) -> Result<Option<Sparse>, Abort> {
    ctx.tick()?;
    if f.is_zero() {
        return Ok(Some(positive(g.clone(), ctx)?));
    }
    if g.is_zero() {
        return Ok(Some(positive(f.clone(), ctx)?));
    }
    if is_constant(f, ctx)? || is_constant(g, ctx)? {
        let c = gcd(&ground_content(f, ctx)?, &ground_content(g, ctx)?);
        return Ok(Some(constant(f.nvars, f.order, c, ctx)?));
    }
    Ok(None)
}
fn is_constant(f: &Sparse, ctx: &Interrupt) -> Result<bool, Abort> {
    for (m, _) in &f.terms {
        ctx.tick()?;
        if m.deg != 0 {
            return Ok(false);
        }
    }
    Ok(true)
}
fn from_univariate(f: &UPoly<Integer>, ctx: &Interrupt) -> Result<Sparse, Abort> {
    let mut terms = vec![];
    for (i, c) in f.coeffs.iter().enumerate() {
        ctx.tick()?;
        if !c.is_zero() {
            let exponent = u32::try_from(i).expect("invariant: polynomial degree fits u32");
            terms.push((
                Monomial::new([exponent]).expect("invariant: univariate degree fits u32"),
                c.clone(),
            ));
        }
    }
    Sparse::new(1, terms, MonoOrder::Lex, ctx)
}
fn to_univariate(f: &Sparse, ctx: &Interrupt) -> Result<UPoly<Integer>, Abort> {
    let dense = split(f, ctx)?;
    let mut coefficients = Vec::with_capacity(dense.coeffs.len());
    for c in dense.coeffs {
        ctx.tick()?;
        coefficients.push(c.terms.first().map_or(Integer::ZERO, |(_, c)| c.clone()));
    }
    Ok(UPoly::new(coefficients))
}
fn from_parameters(f: &Dense, g: &Dense, ctx: &Interrupt) -> Result<(Sparse, Sparse), Abort> {
    ctx.tick()?;
    let mut context = None;
    for c in f.coeffs.iter().chain(&g.coeffs) {
        ctx.tick()?;
        if c.nvars != 0 {
            if let Some((nvars, order)) = context {
                assert_eq!(
                    (c.nvars, c.order),
                    (nvars, order),
                    "parameter coefficient contexts must match"
                );
            } else {
                context = Some((c.nvars, c.order));
            }
        }
    }
    let (nvars, order) = context.unwrap_or((0, MonoOrder::Lex));
    Ok((
        join(f, nvars + 1, order, ctx)?,
        join(g, nvars + 1, order, ctx)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_heuristic_attempts_execute_the_prs_fallback() {
        let ctx = Interrupt::default();
        let f = from_univariate(&UPoly::new(vec![(-1).into(), 0.into(), 1.into()]), &ctx).unwrap();
        let g = from_univariate(&UPoly::new(vec![2.into(), (-3).into(), 1.into()]), &ctx).unwrap();
        let want = from_univariate(&UPoly::new(vec![(-1).into(), 1.into()]), &ctx).unwrap();
        assert_eq!(heuristic::attempt(&f, &g, 0, &ctx).unwrap(), None);
        assert_eq!(
            heuristic::attempt(&f, &g, 6, &ctx).unwrap(),
            Some(want.clone())
        );
        assert_eq!(with_attempts(&f, &g, 0, &ctx).unwrap(), want);
    }
    #[test]
    fn symmetric_radix_digits_include_positive_half_and_negative_values() {
        let ctx = Interrupt::default();
        let f = constant(0, MonoOrder::Lex, (-115).into(), &ctx).unwrap();
        let decoded = sparse::interpolate(&f, &10.into(), &ctx).unwrap().unwrap();
        assert_eq!(
            to_univariate(&decoded, &ctx).unwrap(),
            UPoly::new(vec![5.into(), (-2).into(), (-1).into()])
        );
        assert_eq!(sparse::evaluate(&decoded, &10.into(), &ctx).unwrap(), f);
    }
}
