//! Active-variable radix encoding, bounded before allocating its dense image.
use super::Sparse;
use crate::{MonoOrder, Monomial, UPoly};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
const MAX_DEGREE: usize = 65_536;

pub(super) struct Mapping {
    nvars: usize,
    order: MonoOrder,
    base: usize,
    active: Vec<usize>,
}
pub(super) fn encode(
    f: &Sparse,
    ctx: &Interrupt,
) -> Result<Option<(Mapping, UPoly<Integer>)>, Abort> {
    let mut degrees = vec![0_u32; f.nvars];
    for (m, _) in &f.terms {
        for (degree, e) in degrees.iter_mut().zip(&m.exps) {
            ctx.tick()?;
            *degree = (*degree).max(*e);
        }
    }
    let mut active = vec![];
    let mut maximum = 0_u32;
    for (i, d) in degrees.iter().enumerate() {
        ctx.tick()?;
        maximum = maximum.max(*d);
        if *d > 0 {
            active.push(i);
        }
    }
    let Some(base) = (maximum as usize).checked_add(1) else {
        return Ok(None);
    };
    let mut weights = vec![];
    let mut weight = 1_usize;
    for i in 0..active.len() {
        ctx.tick()?;
        weights.push(weight);
        if i + 1 < active.len() {
            let Some(next) = weight.checked_mul(base).filter(|v| *v <= MAX_DEGREE) else {
                return Ok(None);
            };
            weight = next;
        }
    }
    let mut indexed = Vec::with_capacity(f.terms.len());
    let mut maximum = 0;
    for (m, c) in &f.terms {
        let mut index = 0_usize;
        for (i, w) in active.iter().zip(&weights) {
            ctx.tick()?;
            let Some(next) = (*w)
                .checked_mul(m.exps[*i] as usize)
                .and_then(|e| index.checked_add(e))
                .filter(|v| *v <= MAX_DEGREE)
            else {
                return Ok(None);
            };
            index = next;
        }
        maximum = maximum.max(index);
        indexed.push((index, c.clone()));
    }
    let mut coefficients = Vec::with_capacity(maximum + 1);
    for _ in 0..=maximum {
        ctx.tick()?;
        coefficients.push(Integer::ZERO);
    }
    for (i, c) in indexed {
        ctx.tick()?;
        coefficients[i] += c;
    }
    Ok(Some((
        Mapping {
            nvars: f.nvars,
            order: f.order,
            base,
            active,
        },
        UPoly::new(coefficients),
    )))
}
impl Mapping {
    pub(super) fn decode(
        &self,
        f: &UPoly<Integer>,
        ctx: &Interrupt,
    ) -> Result<Option<Sparse>, Abort> {
        let mut terms = vec![];
        for (i, c) in f.coeffs.iter().enumerate() {
            ctx.tick()?;
            if c.is_zero() {
                continue;
            }
            let mut index = i;
            let mut exps = vec![0; self.nvars];
            for variable in &self.active {
                ctx.tick()?;
                let Ok(e) = u32::try_from(index % self.base) else {
                    return Ok(None);
                };
                exps[*variable] = e;
                index /= self.base;
            }
            if index != 0 {
                return Ok(None);
            }
            let Some(m) = Monomial::new(exps) else {
                return Ok(None);
            };
            terms.push((m, c.clone()));
        }
        Ok(Some(Sparse::new(self.nvars, terms, self.order, ctx)?))
    }
}
