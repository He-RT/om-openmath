//! Exact rational simplification, arithmetic expansion and certified integer factorization.
mod expansion;
mod rational;
use crate::convert::{from_mpoly_with, to_rational_function_with};
use om_core::{Expr, canonicalize, div};
use om_num::ctx::{Abort, Interrupt};
/// Put an expression over a common polynomial denominator, without GCD cancellation.
pub fn together(e: &Expr) -> Expr {
    convenience(e, |ctx| together_with(e, &[], ctx))
}
/// Cancel common polynomial factors over Q using normalized expression generators.
pub fn cancel(e: &Expr) -> Expr {
    convenience(e, |ctx| cancel_with(e, &[], ctx))
}
/// Distribute arithmetic multiplication and positive integer powers; preserve other kernels.
pub fn expand(e: &Expr) -> Expr {
    convenience(e, |ctx| expand_with(e, ctx))
}
/// Factor rational numerator/denominator over Q, restoring contents and multiplicities.
pub fn factor(e: &Expr) -> Expr {
    convenience(e, |ctx| factor_with(e, &[], ctx))
}
fn convenience(e: &Expr, f: impl FnOnce(&Interrupt) -> Result<Option<Expr>, Abort>) -> Expr {
    f(&Interrupt::default())
        .ok()
        .flatten()
        .unwrap_or_else(|| canonicalize(e))
}
/// Interruptible common-denominator construction with requested generator priority.
pub fn together_with(e: &Expr, vars: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    ctx.tick()?;
    let Some(view) = to_rational_function_with(e, vars, ctx)? else {
        return Ok(None);
    };
    let (Some(n), Some(d)) = (
        from_mpoly_with(&view.num, &view.gens, ctx)?,
        from_mpoly_with(&view.den, &view.gens, ctx)?,
    ) else {
        return Ok(None);
    };
    Ok(Some(div(n, d)))
}
/// Interruptible certified polynomial GCD cancellation; None reports representational failure.
pub fn cancel_with(e: &Expr, vars: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    ctx.tick()?;
    let Some(view) = to_rational_function_with(e, vars, ctx)? else {
        return Ok(None);
    };
    rational::cancel(view, ctx)
}
/// Interruptible full arithmetic expansion; unsupported exponent sizes return None.
pub fn expand_with(e: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    ctx.tick()?;
    expansion::expand(e, ctx)
}
/// Interruptible complete Q factorization; None includes incomplete bounded factorization.
pub fn factor_with(e: &Expr, vars: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    ctx.tick()?;
    let Some(view) = to_rational_function_with(e, vars, ctx)? else {
        return Ok(None);
    };
    rational::factor(view, ctx)
}
