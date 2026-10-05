//! Checked adapters keep algebraic failures symbolic and Abort observable.
#[path = "algebra_apart.rs"]
mod apart;
#[path = "algebra_arithmetic.rs"]
mod arithmetic;
#[path = "algebra_diff.rs"]
mod diff;
pub(crate) fn differentiate(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
    diff::apply(&[e.clone(), x.clone()], ctx)?
        .ok_or_else(|| EvalError::Other("无法按给定变量求导".into()))
}
pub(crate) fn partial_fractions(
    e: &Expr,
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    apart::apply(&[e.clone(), x.clone()], ctx)
}
#[path = "algebra_poly.rs"]
mod poly;
#[path = "algebra_root.rs"]
mod root;
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, Interrupt, MsgLevel};
use om_simplify::{
    algebra as a,
    simplify::{SimplifyOptions, complexity, simplify_with},
};
pub(crate) fn dispatch(
    ev: &mut Evaluator,
    name: &str,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    ctx.tick()?;
    let result = match name {
        "Expand" => a::expand_with(&args[0], ctx)?,
        "Factor" => a::factor_with(&args[0], &[], ctx)?,
        "Together" => a::together_with(&args[0], &[], ctx)?,
        "Cancel" => a::cancel_with(&args[0], &[], ctx)?,
        "Simplify" | "FullSimplify" => simplification(args, name == "FullSimplify", ctx)?,
        "Apart" => apart::apply(args, ctx)?,
        "D" => diff::apply(args, ctx)?,
        "RootReduce" => Some(root::transform(&args[0], false, ctx)?),
        "ToRadicals" => Some(root::transform(&args[0], true, ctx)?),
        "PolynomialGCD"
        | "PolynomialLCM"
        | "PolynomialQuotient"
        | "PolynomialRemainder"
        | "Resultant"
        | "Discriminant" => arithmetic::apply(name, args, ctx)?,
        _ => poly::query(ev, name, args, ctx)?,
    };
    if result.is_none() {
        ev.message(
            name,
            "poly",
            "Arguments or options are outside the supported exact algebraic method.".into(),
            MsgLevel::Warning,
        );
    }
    Ok(result)
}
fn simplification(args: &[Expr], full: bool, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let mut options = SimplifyOptions::default();
    let mut pending = args.get(1).into_iter().collect::<Vec<_>>();
    while let Some(e) = pending.pop() {
        ctx.tick()?;
        if e.as_symbol() == Some(B::TRUE) {
            continue;
        }
        if e.is_head(B::AND) {
            pending.extend(e.args());
            continue;
        }
        if e.is_head(B::RULE)
            && e.args().len() == 2
            && e.args()[0]
                .as_symbol()
                .is_some_and(|s| s.name() == "Assumptions")
        {
            pending.push(&e.args()[1]);
            continue;
        }
        if e.is_head(B::GREATER)
            && e.args().len() == 2
            && e.args()[1].is_zero()
            && let Some(s) = e.args()[0].as_symbol()
        {
            options.positive.push(s);
            continue;
        }
        return Ok(None);
    }
    let mut value = simplify_with(&args[0], &options, ctx)?;
    if full {
        for radicals in [false, true] {
            let next = root::transform(&value, radicals, ctx)?;
            let next = simplify_with(&next, &options, ctx)?;
            if complexity(&next) < complexity(&value) {
                value = next;
            }
        }
    }
    Ok(Some(value))
}
