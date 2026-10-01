//! Krull dimension from the variable supports of a certified leading ideal.
use crate::{MPoly, MonoOrder, Monomial, groebner::compatible, normal_form, s_polynomial};
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
type Q = MPoly<Rational>;
/// Dimension of an ideal's affine variety, keeping the empty variety explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdealDimension {
    /// Unit ideal; there are no solutions over an algebraic closure.
    Empty,
    /// Nonnegative Krull dimension.
    Dimension(usize),
}
/// Compute dimension from a validated Groebner basis and its ambient variable count.
/// Empty basis is the zero ideal of dimension nvars; a unit basis returns Empty.
/// None rejects incompatible contexts/orders or failure of Buchberger's criterion.
pub fn ideal_dimension(
    basis: &[Q],
    nvars: usize,
    ctx: &Interrupt,
) -> Result<Option<IdealDimension>, Abort> {
    ctx.tick()?;
    let Some(basis) = validated(basis, nvars, ctx)? else {
        return Ok(None);
    };
    if basis.iter().any(|g| g.terms[0].0.deg == 0) {
        return Ok(Some(IdealDimension::Empty));
    }
    for size in (0..=nvars).rev() {
        ctx.tick()?;
        let mut subset: Vec<_> = (0..size).collect();
        loop {
            ctx.tick()?;
            if independent(&basis, &subset, ctx)? {
                return Ok(Some(IdealDimension::Dimension(size)));
            }
            let mut position = size;
            while position > 0 && subset[position - 1] == nvars - size + position - 1 {
                ctx.tick()?;
                position -= 1;
            }
            if position == 0 {
                break;
            }
            subset[position - 1] += 1;
            for i in position..size {
                ctx.tick()?;
                subset[i] = subset[i - 1] + 1;
            }
        }
    }
    unreachable!("invariant: empty subset is independent for a nonunit monomial ideal")
}
/// Whether the validated quotient algebra is finite-dimensional over Q.
/// Includes unit ideals (zero quotient); None rejects incompatible or non-Groebner input.
pub fn is_zero_dimensional(
    basis: &[Q],
    nvars: usize,
    ctx: &Interrupt,
) -> Result<Option<bool>, Abort> {
    ctx.tick()?;
    let Some(basis) = validated(basis, nvars, ctx)? else {
        return Ok(None);
    };
    if basis.iter().any(|g| g.terms[0].0.deg == 0) {
        return Ok(Some(true));
    }
    for variable in 0..nvars {
        ctx.tick()?;
        let mut bounded = false;
        for g in &basis {
            ctx.tick()?;
            let m = &g.terms[0].0;
            if m.exps[variable] > 0 && pure_power(m, variable, ctx)? {
                bounded = true;
                break;
            }
        }
        if !bounded {
            return Ok(Some(false));
        }
    }
    Ok(Some(true))
}
fn pure_power(m: &Monomial, variable: usize, ctx: &Interrupt) -> Result<bool, Abort> {
    for (j, e) in m.exps.iter().enumerate() {
        ctx.tick()?;
        if j != variable && *e != 0 {
            return Ok(false);
        }
    }
    Ok(true)
}
fn independent(basis: &[Q], subset: &[usize], ctx: &Interrupt) -> Result<bool, Abort> {
    for g in basis {
        ctx.tick()?;
        let m = &g.terms[0].0;
        let mut contained = true;
        for (j, e) in m.exps.iter().enumerate() {
            ctx.tick()?;
            if *e > 0 && subset.binary_search(&j).is_err() {
                contained = false;
                break;
            }
        }
        if contained {
            return Ok(false);
        }
    }
    Ok(true)
}
fn validated(basis: &[Q], nvars: usize, ctx: &Interrupt) -> Result<Option<Vec<Q>>, Abort> {
    let order = basis
        .iter()
        .find(|g| g.nvars > 0)
        .or_else(|| basis.first())
        .map_or(MonoOrder::Lex, |g| g.order);
    for g in basis {
        ctx.tick()?;
        if g.nvars != 0 && (g.nvars != nvars || g.order != order) {
            return Ok(None);
        }
    }
    let mut input = basis.to_vec();
    input.push(Q::zero_in(nvars, order));
    let Some(all) = compatible(&input, order, ctx)? else {
        return Ok(None);
    };
    let basis: Vec<_> = all.into_iter().filter(|g| !g.is_zero()).collect();
    for i in 0..basis.len() {
        for j in 0..i {
            ctx.tick()?;
            let Some(s) = s_polynomial(&basis[i], &basis[j], ctx)? else {
                return Ok(None);
            };
            if normal_form(&s, &basis, ctx)?.is_none_or(|r| !r.is_zero()) {
                return Ok(None);
            }
        }
    }
    Ok(Some(basis))
}
