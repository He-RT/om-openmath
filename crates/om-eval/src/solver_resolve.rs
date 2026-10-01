//! Expand session definitions without canonicalizing arithmetic or source relations.
use crate::{Attributes as A, EvalError, Evaluator, pattern};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
enum Frame {
    Visit(Expr, u32),
    Build(usize, u32),
}
fn structural(ev: &Evaluator, h: om_core::Symbol) -> bool {
    matches!(
        h,
        B::PLUS
            | B::TIMES
            | B::POWER
            | B::SQRT
            | B::EXP
            | B::LIST
            | B::EQUAL
            | B::UNEQUAL
            | B::LESS
            | B::LESS_EQUAL
            | B::GREATER
            | B::GREATER_EQUAL
            | B::INEQUALITY
            | B::AND
            | B::OR
            | B::NOT
            | B::ELEMENT
            | B::CONDITIONAL_EXPRESSION
    ) || ev.attributes(h).contains(A::NUMERIC_FUNCTION)
        || matches!(
            h.name(),
            "Log"
                | "Abs"
                | "Sin"
                | "Cos"
                | "Tan"
                | "Sec"
                | "Csc"
                | "Cot"
                | "Sinh"
                | "Cosh"
                | "Tanh"
                | "Coth"
                | "Sech"
                | "Csch"
                | "ArcSin"
                | "ArcCos"
                | "ArcTan"
                | "ArcCot"
                | "ArcSec"
                | "ArcCsc"
                | "ArcSinh"
                | "ArcCosh"
                | "ArcTanh"
                | "ArcCoth"
                | "ArcSech"
                | "ArcCsch"
                | "ProductLog"
                | "CubeRoot"
                | "Re"
                | "Im"
                | "Conjugate"
                | "Arg"
        )
}
pub(super) fn resolve(
    ev: &mut Evaluator,
    source: &Expr,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    let mut work = vec![Frame::Visit(source.clone(), ev.depth + 1)];
    let mut values = vec![];
    while let Some(frame) = work.pop() {
        ctx.tick()?;
        match frame {
            Frame::Visit(e, depth) => {
                if depth > ev.settings.recursion_limit {
                    return Err(EvalError::Recursion(ev.settings.recursion_limit));
                }
                if let Some(s) = e.as_symbol() {
                    if let Some(value) = ev.own(s) {
                        work.push(Frame::Visit(value, depth + 1));
                    } else {
                        values.push(e);
                    }
                } else if e.is_head(B::ROOT) {
                    if e.args().len() != 2 {
                        return Err(EvalError::Other(
                            "Root expects two arguments in the solving source".into(),
                        ));
                    }
                    // A rejected Root cannot become a trusted coefficient in the outer solve.
                    let previous = ev.depth;
                    ev.depth = depth;
                    let result = super::root::apply(ev, e.args(), ctx);
                    ev.depth = previous;
                    values.push(result?.ok_or_else(|| {
                        EvalError::Other("Unsupported Root in the solving source".into())
                    })?);
                } else if let ExprKind::Normal(n) = e.kind() {
                    // Lexical function bodies and explicitly held syntax belong to their own scopes.
                    if matches!(
                        n.head.as_symbol(),
                        Some(B::FUNCTION | B::HOLD | B::HOLD_FORM | B::PATTERN)
                    ) {
                        values.push(e);
                        continue;
                    }
                    work.push(Frame::Build(n.args.len(), depth));
                    work.extend(
                        n.args
                            .iter()
                            .rev()
                            .cloned()
                            .map(|e| Frame::Visit(e, depth + 1)),
                    );
                    work.push(Frame::Visit(n.head.clone(), depth + 1));
                } else {
                    values.push(e);
                }
            }
            Frame::Build(count, depth) => {
                let args = values.split_off(values.len() - count);
                let head = values.pop().expect("invariant: resolved head");
                let e = Expr::normal(head.clone(), args.clone());
                let mut replacement = None;
                if head.is_head(B::FUNCTION) {
                    replacement = crate::structure::apply_function(ev, &head, &args);
                } else if head.as_symbol() == Some(B::CONDITIONAL_EXPRESSION) && args.len() == 2 {
                    match args[1].as_symbol() {
                        Some(B::TRUE) => replacement = Some(args[0].clone()),
                        Some(B::FALSE) => {
                            return Err(EvalError::Other(
                                "False conditional expression is undefined in the solving source"
                                    .into(),
                            ));
                        }
                        _ => {}
                    }
                } else if head.as_symbol().is_some_and(|h| h.name() == "Divide") && args.len() == 2
                {
                    replacement = Some(Expr::call(
                        B::TIMES,
                        [
                            args[0].clone(),
                            Expr::call(B::POWER, [args[1].clone(), Expr::int(-1)]),
                        ],
                    ));
                } else if head.as_symbol().is_some_and(|h| h.name() == "Subtract")
                    && args.len() == 2
                {
                    replacement = Some(Expr::call(
                        B::PLUS,
                        [
                            args[0].clone(),
                            Expr::call(B::TIMES, [Expr::int(-1), args[1].clone()]),
                        ],
                    ));
                } else if head.as_symbol().is_some_and(|h| h.name() == "Minus") && args.len() == 1 {
                    replacement = Some(Expr::call(B::TIMES, [Expr::int(-1), args[0].clone()]));
                } else if let Some(h) = head.as_symbol() {
                    let rules = ev.defs.down.get(&h).cloned().unwrap_or_default();
                    for rule in rules {
                        let (lhs, rhs) = if rule.delayed
                            && rule.rhs.is_head(B::CONDITION)
                            && rule.rhs.args().len() == 2
                        {
                            (
                                Expr::call(
                                    B::CONDITION,
                                    [rule.lhs.clone(), rule.rhs.args()[1].clone()],
                                ),
                                rule.rhs.args()[0].clone(),
                            )
                        } else {
                            (rule.lhs, rule.rhs)
                        };
                        if let Some(bindings) = pattern::match_rule(&lhs, &e, ev, ctx)? {
                            replacement = Some(pattern::substitute(&rhs, &bindings));
                            break;
                        }
                    }
                    if replacement.is_none() && ev.builtins.get(h).is_some() {
                        let closed_numeric = ev.attributes(h).contains(A::NUMERIC_FUNCTION)
                            && !matches!(h, B::PLUS | B::TIMES | B::POWER | B::SQRT | B::EXP)
                            && e.free_symbols().is_empty()
                            && defined(&e, ctx)?;
                        if !structural(ev, h) || closed_numeric {
                            replacement = Some(ev.evaluate(&e, ctx)?);
                        }
                    }
                }
                if let Some(next) = replacement.filter(|r| *r != e) {
                    work.push(Frame::Visit(next, depth + 1));
                } else {
                    values.push(e);
                }
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: held resolution produced a root"))
}

// A closed numeric coefficient may evaluate only after its original domain is known.
fn defined(e: &Expr, ctx: &Interrupt) -> Result<bool, EvalError> {
    let probe = Expr::call(B::EQUAL, [e.clone(), e.clone()]);
    let prepared = om_solve::normalize::normalize(
        &probe,
        Some(&[]),
        om_solve::Domain::Complexes,
        ctx,
        &mut om_solve::NoSteps,
    )
    .map_err(super::error)?;
    Ok(!prepared.unsupported && !prepared.branches.is_empty())
}
