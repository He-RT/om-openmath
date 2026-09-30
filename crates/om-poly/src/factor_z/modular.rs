//! Fixed increasing odd-prime schedule and five-prime factor-count selection.
use super::Poly;
use crate::{FpElem, UPoly};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

pub(super) fn select(f: &Poly, ctx: &Interrupt) -> Result<(u64, Vec<UPoly<FpElem>>), Abort> {
    let mut prime = 3_u64;
    let mut good = 0;
    let mut best: Option<(u64, Vec<UPoly<FpElem>>)> = None;
    while good < 5 {
        ctx.tick()?;
        if FpElem::new(0, prime).is_some() {
            let ff = convert(f, prime, ctx)?;
            if ff.degree() == f.degree()
                && ff
                    .monic_gcd(&ff.derivative(ctx)?, ctx)?
                    .expect("invariant: bound prime-field GCD exists")
                    .is_one()
            {
                let mut factors = vec![];
                for (group, d) in ff
                    .ddf(ctx)?
                    .expect("invariant: good prime gives square-free input")
                {
                    ctx.tick()?;
                    factors.extend(
                        group
                            .edf(d, 0x0A5E_ED00_0000_0001, ctx)?
                            .expect("invariant: DDF gives a valid EDF degree group"),
                    );
                }
                if best
                    .as_ref()
                    .is_none_or(|(_, old)| factors.len() < old.len())
                {
                    best = Some((prime, factors));
                }
                good += 1;
            }
        }
        prime = prime
            .checked_add(2)
            .expect("invariant: odd-prime search remains within u64");
    }
    Ok(best.expect("invariant: five good primes include a best factorization"))
}
fn convert(f: &Poly, p: u64, ctx: &Interrupt) -> Result<UPoly<FpElem>, Abort> {
    let modulus = Integer::from(p);
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for c in &f.coeffs {
        ctx.tick()?;
        let mut r = c % &modulus;
        if r < Integer::ZERO {
            r += &modulus;
        }
        coefficients.push(FpElem {
            v: u64::try_from(r).expect("invariant: residue is smaller than u64 prime"),
            p,
        });
    }
    Ok(UPoly::new(coefficients))
}
