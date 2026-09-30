//! Characteristic-zero Yun decomposition and denominator clearing.
mod finite;
use crate::{Ring, UPoly};
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
    gcd,
};

/// Square-free decomposition: input = content * product(factor^multiplicity).
/// Factors are grouped in increasing multiplicity and are pairwise coprime.
#[derive(Clone, Debug, PartialEq)]
pub struct SquareFree<R: Ring> {
    /// Signed content in characteristic zero, original leading scalar in a prime field.
    pub content: R,
    /// Nonconstant square-free primitive (Z/Q) or monic (Fp) factors and positive multiplicities.
    pub factors: Vec<(UPoly<R>, u32)>,
}
impl UPoly<Integer> {
    /// Yun decomposition with positive-leading primitive factors; zero returns None.
    /// Nonzero constants return their signed content and an empty factor list.
    pub fn square_free(&self, ctx: &Interrupt) -> Result<Option<SquareFree<Integer>>, Abort> {
        ctx.tick()?;
        if self.is_zero() {
            return Ok(None);
        }
        let (content, f) = self.content_pp(ctx)?;
        let mut factors = vec![];
        if f.degree() == Some(0) {
            return Ok(Some(SquareFree { content, factors }));
        }
        let derivative = f.derivative(ctx)?;
        let g = f.gcdheu(&derivative, ctx)?;
        let mut c = quotient(&f, &g, ctx)?;
        let mut d = quotient(&derivative, &g, ctx)?.sub(&c.derivative(ctx)?, ctx)?;
        let mut multiplicity = 1_u32;
        while c.degree().is_some_and(|d| d > 0) {
            ctx.tick()?;
            let a = c.gcdheu(&d, ctx)?;
            if a.degree().is_some_and(|d| d > 0) {
                factors.push((a.clone(), multiplicity));
            }
            c = quotient(&c, &a, ctx)?;
            d = quotient(&d, &a, ctx)?.sub(&c.derivative(ctx)?, ctx)?;
            multiplicity = multiplicity
                .checked_add(1)
                .expect("invariant: Yun multiplicity fits u32");
        }
        Ok(Some(SquareFree { content, factors }))
    }
}
impl UPoly<Rational> {
    /// Yun decomposition over Q after clearing denominators.
    /// Factors have primitive integer coefficients and a positive leading coefficient.
    /// The rational signed content reconstructs the original input; zero returns None.
    pub fn square_free(&self, ctx: &Interrupt) -> Result<Option<SquareFree<Rational>>, Abort> {
        ctx.tick()?;
        if self.is_zero() {
            return Ok(None);
        }
        let mut denominator = Integer::ONE;
        for c in &self.coeffs {
            ctx.tick()?;
            let d = Integer::from(c.denominator().clone());
            denominator = &denominator / gcd(&denominator, &d) * d;
        }
        let mut coefficients = Vec::with_capacity(self.coeffs.len());
        for c in &self.coeffs {
            ctx.tick()?;
            coefficients
                .push(c.numerator() * (&denominator / Integer::from(c.denominator().clone())));
        }
        let sf = UPoly::new(coefficients)
            .square_free(ctx)?
            .expect("invariant: clearing denominators preserves nonzero input");
        let content = Rational::from(sf.content) / Rational::from(denominator);
        let mut factors = Vec::with_capacity(sf.factors.len());
        for (f, m) in sf.factors {
            ctx.tick()?;
            let mut coefficients = Vec::with_capacity(f.coeffs.len());
            for c in f.coeffs {
                ctx.tick()?;
                coefficients.push(Rational::from(c));
            }
            factors.push((Self::new(coefficients), m));
        }
        Ok(Some(SquareFree { content, factors }))
    }
}
fn quotient(
    f: &UPoly<Integer>,
    g: &UPoly<Integer>,
    ctx: &Interrupt,
) -> Result<UPoly<Integer>, Abort> {
    Ok(f.exact_div(g, ctx)?
        .expect("invariant: Yun divides by a computed polynomial GCD"))
}
