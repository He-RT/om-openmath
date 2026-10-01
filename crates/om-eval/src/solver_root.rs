//! Root syntax and generic degree are checked before public algebraic certification.
use crate::{EvalError, Evaluator, pattern};
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_num::Number;
use om_simplify::{convert::to_rational_function_with, root_reduce::to_algebraic};
fn invalid(s: &str) -> EvalError {
    EvalError::Other(s.into())
}
pub(super) fn apply(
    ev: &mut Evaluator,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let f = ev.evaluate(&args[0], ctx)?;
    let index = ev.evaluate(&args[1], ctx)?;
    let Some(Number::Integer(k)) = index.as_number() else {
        return Err(invalid("Root index must be a positive integer"));
    };
    let k = usize::try_from(k)
        .ok()
        .filter(|k| *k > 0)
        .ok_or_else(|| invalid("Root index must be a positive integer"))?;
    if !f.is_head(B::FUNCTION) {
        return Err(invalid("Root requires a polynomial pure function"));
    }
    let slot = Expr::call(B::SLOT, [Expr::int(1)]);
    let body = match f.args() {
        [body] => body.clone(),
        [variable, body] => {
            let variable = if variable.is_head(B::LIST) && variable.args().len() == 1 {
                &variable.args()[0]
            } else {
                variable
            };
            let v = variable
                .as_symbol()
                .ok_or_else(|| invalid("Root function must have one parameter"))?;
            pattern::substitute(body, &[(v, slot.clone())].into_iter().collect())
        }
        _ => return Err(invalid("Root function must have one argument")),
    };
    let body = super::resolve::resolve(ev, &body, ctx)?;
    let view = to_rational_function_with(&body, std::slice::from_ref(&slot), ctx)?
        .ok_or_else(|| invalid("Root polynomial conversion unavailable"))?;
    if view.gens.first() != Some(&slot)
        || dependent(&view.gens[1..], ctx)?
        || view.den.terms.iter().any(|(m, _)| m.exps[0] != 0)
    {
        return Err(invalid("Root requires polynomial dependence on slot #1"));
    }
    let degree = view
        .num
        .terms
        .iter()
        .map(|(m, _)| m.exps[0] as usize)
        .max()
        .unwrap_or(0);
    if degree == 0 || degree > 4096 || k > degree {
        return Err(invalid("Root index is outside the polynomial degree"));
    }
    let expr = Expr::call(
        B::ROOT,
        [
            Expr::call(
                B::FUNCTION,
                [om_simplify::convert::canonicalize_with(&body, ctx)?],
            ),
            index,
        ],
    );
    if view.gens.len() == 1 {
        let value = to_algebraic(&expr, ctx)?.ok_or_else(|| {
            invalid("Root certification is unavailable for this polynomial or index")
        })?;
        if let om_poly::Algebraic::Rational(q) = value {
            return Ok(Some(Expr::number(Number::Rational(q))));
        }
    }
    Ok(Some(expr))
}

fn dependent(gens: &[Expr], ctx: &Interrupt) -> Result<bool, EvalError> {
    let mut stack = gens.iter().collect::<Vec<_>>();
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if e.is_head(B::SLOT) {
            return Ok(true);
        }
        if e.is_head(B::ROOT) {
            continue;
        }
        if let om_core::ExprKind::Normal(n) = e.kind() {
            stack.push(&n.head);
            stack.extend(&n.args);
        }
    }
    Ok(false)
}
