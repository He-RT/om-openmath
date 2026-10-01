//! Automatic axes use source symbols and names, excluding builtin and generated kernels.
use crate::{Domain, SolveError};
use om_core::{BUILTIN as B, Expr, ExprKind};
use om_num::ctx::{Abort, Interrupt};
use om_simplify::convert::canonicalize_with;
use std::collections::BTreeMap;
pub(super) fn infer(e: &Expr, ctx: &Interrupt) -> Result<Vec<Expr>, Abort> {
    let mut symbols = BTreeMap::new();
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        match e.kind() {
            ExprKind::Symbol(s) if !om_core::builtins::names().contains(&s.name()) => {
                symbols.insert(s.name(), Expr::sym(*s));
            }
            ExprKind::Normal(n) if !e.is_head(B::C) && !e.is_head(B::ROOT) => {
                if matches!(n.head.kind(), ExprKind::Normal(_)) {
                    stack.push(&n.head);
                }
                stack.extend(n.args.iter());
            }
            _ => {}
        }
    }
    Ok(symbols.into_values().collect())
}
pub(super) fn requested(vars: &[Expr], ctx: &Interrupt) -> Result<Vec<Expr>, SolveError> {
    let mut result = vec![];
    for v in vars {
        ctx.tick()?;
        let v = canonicalize_with(v, ctx)?;
        let valid = match v.kind() {
            ExprKind::Symbol(s) => !om_core::builtins::names().contains(&s.name()),
            ExprKind::Normal(_) => ![
                B::PLUS,
                B::TIMES,
                B::POWER,
                B::LIST,
                B::EQUAL,
                B::AND,
                B::OR,
            ]
            .iter()
            .any(|h| v.is_head(*h)),
            _ => false,
        };
        if !valid {
            return Err(SolveError::Invalid("invalid solving variable".into()));
        }
        if !result.contains(&v) {
            result.push(v);
        }
    }
    Ok(result)
}
pub(super) fn domain(e: &Expr) -> Option<Domain> {
    match e.as_symbol()? {
        B::COMPLEXES => Some(Domain::Complexes),
        B::REALS => Some(Domain::Reals),
        B::INTEGERS => Some(Domain::Integers),
        B::RATIONALS => Some(Domain::Rationals),
        _ => None,
    }
}
pub(super) fn meet(a: Domain, b: Domain) -> Domain {
    let rank = |d| match d {
        Domain::Complexes => 3,
        Domain::Reals => 2,
        Domain::Rationals => 1,
        Domain::Integers => 0,
    };
    if rank(a) < rank(b) { a } else { b }
}
