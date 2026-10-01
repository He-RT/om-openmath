//! Uncanceled Q rational-function arithmetic, checking every sparse product and exponent.
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{MPoly, MonoOrder, Monomial};
pub(super) type Poly = MPoly<Rational>;
#[derive(Clone)]
pub(super) struct Fraction {
    pub num: Poly,
    pub den: Poly,
}
pub(super) fn constant(q: Rational, n: usize, ctx: &Interrupt) -> Result<Fraction, Abort> {
    let unit = Monomial::new(vec![0; n]).expect("invariant: constant degree is zero");
    Ok(Fraction {
        num: Poly::new(n, vec![(unit.clone(), q)], MonoOrder::Lex, ctx)?,
        den: Poly::new(n, vec![(unit, Rational::ONE)], MonoOrder::Lex, ctx)?,
    })
}
pub(super) fn axis(
    i: usize,
    e: &Integer,
    n: usize,
    ctx: &Interrupt,
) -> Result<Option<Fraction>, Abort> {
    let Some(exponent) = u32::try_from(&e.clone().max(-e)).ok() else {
        return Ok(None);
    };
    let mut powers = vec![0; n];
    powers[i] = exponent;
    let Some(m) = Monomial::new(powers) else {
        return Ok(None);
    };
    let p = Poly::new(n, vec![(m, Rational::ONE)], MonoOrder::Lex, ctx)?;
    let one = constant(Rational::ONE, n, ctx)?.num;
    Ok(Some(if e < &Integer::ZERO {
        Fraction { num: one, den: p }
    } else {
        Fraction { num: p, den: one }
    }))
}
impl Fraction {
    pub fn combine(&self, b: &Self, add: bool, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        let Some(den) = self.den.mul(&b.den, ctx)? else {
            return Ok(None);
        };
        let num = if add {
            let (Some(a), Some(b)) = (self.num.mul(&b.den, ctx)?, b.num.mul(&self.den, ctx)?)
            else {
                return Ok(None);
            };
            a.add(&b, ctx)?
        } else {
            let Some(p) = self.num.mul(&b.num, ctx)? else {
                return Ok(None);
            };
            p
        };
        Ok(Some(Self { num, den }))
    }
    pub fn power(&self, e: &Integer, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        let Some(mut exponent) = u32::try_from(&e.clone().max(-e)).ok() else {
            return Ok(None);
        };
        let mut base = if e < &Integer::ZERO {
            if self.num.is_zero() {
                return Ok(None);
            }
            Self {
                num: self.den.clone(),
                den: self.num.clone(),
            }
        } else {
            self.clone()
        };
        let mut out = constant(Rational::ONE, self.num.nvars, ctx)?;
        while exponent > 0 {
            ctx.tick()?;
            if exponent & 1 != 0 {
                let Some(next) = out.combine(&base, false, ctx)? else {
                    return Ok(None);
                };
                out = next;
            }
            exponent >>= 1;
            if exponent > 0 {
                let Some(next) = base.combine(&base, false, ctx)? else {
                    return Ok(None);
                };
                base = next;
            }
        }
        Ok(Some(out))
    }
}
