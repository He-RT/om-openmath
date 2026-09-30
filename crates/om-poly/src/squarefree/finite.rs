//! Square-free prime-field factors with characteristic-p residual descent.
use crate::{Field, FpElem, Ring, SquareFree, UPoly};
use om_num::ctx::{Abort, Interrupt};

impl UPoly<FpElem> {
    /// Monic square-free factors grouped by multiplicity, with the original leading scalar.
    /// Zero or a nonconstant polynomial without a bound prime modulus returns None.
    pub fn square_free(&self, ctx: &Interrupt) -> Result<Option<SquareFree<FpElem>>, Abort> {
        ctx.tick()?;
        let Some(lc) = self.lc() else {
            return Ok(None);
        };
        let mut modulus = 0;
        for c in &self.coeffs {
            ctx.tick()?;
            if c.p != 0 {
                assert!(
                    modulus == 0 || modulus == c.p,
                    "square-free coefficient fields must match"
                );
                modulus = c.p;
            }
        }
        if self.degree() == Some(0) {
            return Ok(Some(SquareFree {
                content: *lc,
                factors: vec![],
            }));
        }
        if modulus == 0 {
            return Ok(None);
        }
        let zero = FpElem { v: 0, p: modulus };
        let content = lc.add(&zero);
        let inverse = content
            .inv()
            .expect("invariant: a nonzero prime-field element is invertible");
        let f = self.scale(&inverse, ctx)?;
        let mut pending = vec![(f, 1_u32)];
        let mut factors = vec![];
        while let Some((f, multiple)) = pending.pop() {
            ctx.tick()?;
            let derivative = f.derivative(ctx)?;
            let mut c = f
                .monic_gcd(&derivative, ctx)?
                .expect("invariant: bound prime-field GCD exists");
            let mut w = quotient(&f, &c, ctx)?;
            let mut multiplicity = 1_u32;
            while w.degree().is_some_and(|d| d > 0) {
                ctx.tick()?;
                let y = w
                    .monic_gcd(&c, ctx)?
                    .expect("invariant: bound prime-field GCD exists");
                let z = quotient(&w, &y, ctx)?;
                if z.degree().is_some_and(|d| d > 0) {
                    factors.push((
                        z,
                        multiplicity
                            .checked_mul(multiple)
                            .expect("invariant: factor multiplicity fits u32"),
                    ));
                }
                w = y;
                c = quotient(&c, &w, ctx)?;
                multiplicity = multiplicity
                    .checked_add(1)
                    .expect("invariant: square-free multiplicity fits u32");
            }
            if c.degree().is_some_and(|d| d > 0) {
                let multiple = multiple
                    .checked_mul(
                        u32::try_from(modulus)
                            .expect("invariant: characteristic is bounded by residual degree"),
                    )
                    .expect("invariant: Frobenius multiplicity fits u32");
                pending.push((pth_root(&c, modulus, ctx)?, multiple));
            }
        }
        // Insertion sort charges comparisons; Frobenius factors can precede earlier entries.
        for i in 1..factors.len() {
            let mut j = i;
            while j > 0 {
                ctx.tick()?;
                if factors[j - 1].1 <= factors[j].1 {
                    break;
                }
                factors.swap(j - 1, j);
                j -= 1;
            }
        }
        Ok(Some(SquareFree { content, factors }))
    }
}
fn quotient(f: &UPoly<FpElem>, g: &UPoly<FpElem>, ctx: &Interrupt) -> Result<UPoly<FpElem>, Abort> {
    Ok(f.exact_div(g, ctx)?
        .expect("invariant: finite-field square-free division is exact"))
}
fn pth_root(f: &UPoly<FpElem>, p: u64, ctx: &Interrupt) -> Result<UPoly<FpElem>, Abort> {
    ctx.tick()?;
    let p = usize::try_from(p)
        .expect("invariant: characteristic is bounded by nonconstant residual degree");
    let mut coefficients = Vec::with_capacity(f.coeffs.len() / p + 1);
    for (i, c) in f.coeffs.iter().enumerate() {
        ctx.tick()?;
        if i.is_multiple_of(p) {
            coefficients.push(*c);
        } else {
            assert!(
                c.is_zero(),
                "Frobenius residual must have only p-divisible exponents"
            );
        }
    }
    Ok(UPoly::new(coefficients))
}
