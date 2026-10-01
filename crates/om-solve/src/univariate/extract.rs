//! Split sparse arithmetic into polynomial coefficients with independent parameter kernels.
use crate::SolveError;
use om_core::{BUILTIN as B, Expr, ExprKind, func};
use om_num::{Integer, Number, Rational, ctx::Interrupt, gcd};
use om_poly::{MPoly, Monomial};
use om_simplify::{
    algebra::cancel_with,
    convert::{from_mpoly_with, to_rational_function_with},
};
use std::collections::BTreeMap;
pub(super) struct Coefficients {
    pub values: Vec<Expr>,
    pub denominator: Expr,
    pub content: Expr,
}
pub(super) fn coefficients(
    e: &Expr,
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Option<Coefficients>, SolveError> {
    let e = special(e, ctx)?;
    let Some(view) = to_rational_function_with(&e, std::slice::from_ref(x), ctx)? else {
        return Ok(None);
    };
    if view.gens.first() != Some(x) {
        return Ok(None);
    }
    for g in &view.gens[1..] {
        if depends(g, x, ctx)? {
            return Ok(None);
        }
    }
    for (m, _) in &view.den.terms {
        ctx.tick()?;
        if m.exps[0] != 0 {
            return Ok(None);
        }
    }
    let degree = view
        .num
        .terms
        .iter()
        .map(|(m, _)| m.exps[0] as usize)
        .max()
        .unwrap_or(0);
    if degree > 4096 {
        return Err(SolveError::Unsupported(
            "dense univariate degree exceeds 4096".into(),
        ));
    }
    let mut groups: BTreeMap<usize, Vec<(Monomial, Rational)>> = BTreeMap::new();
    for (m, c) in &view.num.terms {
        ctx.tick()?;
        groups.entry(m.exps[0] as usize).or_default().push((
            Monomial::new(m.exps[1..].iter().copied())
                .expect("invariant: coefficient degree cannot exceed its original term"),
            c.clone(),
        ));
    }
    let den = MPoly::new(
        view.gens.len() - 1,
        view.den
            .terms
            .iter()
            .map(|(m, c)| {
                (
                    Monomial::new(m.exps[1..].iter().copied())
                        .expect("invariant: independent denominator degree fits u32"),
                    c.clone(),
                )
            })
            .collect(),
        view.den.order,
        ctx,
    )?;
    let denominator = from_mpoly_with(&den, &view.gens[1..], ctx)?
        .ok_or_else(|| SolveError::Unsupported("invalid coefficient denominator".into()))?;
    let mut scale = Integer::ONE;
    for (_, c) in &view.num.terms {
        ctx.tick()?;
        let d = Integer::from(c.denominator().clone());
        scale = (&scale / gcd(&scale, &d)) * d;
    }
    let mut content = MPoly::<Integer>::zero_in(view.gens.len() - 1, view.num.order);
    let mut values = vec![];
    for i in 0..=degree {
        ctx.tick()?;
        let p = MPoly::new(
            view.gens.len() - 1,
            groups.remove(&i).unwrap_or_default(),
            view.num.order,
            ctx,
        )?;
        if p.nvars > 0 && content.terms.first().is_none_or(|(m, _)| m.deg != 0) {
            let mut terms = vec![];
            for (m, c) in &p.terms {
                ctx.tick()?;
                terms.push((
                    m.clone(),
                    c.numerator() * (&scale / Integer::from(c.denominator().clone())),
                ));
            }
            content = content.subresultant_gcd(&MPoly::new(p.nvars, terms, p.order, ctx)?, ctx)?;
        }
        let n = from_mpoly_with(&p, &view.gens[1..], ctx)?
            .ok_or_else(|| SolveError::Unsupported("invalid polynomial coefficient".into()))?;
        let q = cancel_with(&om_core::div(n, denominator.clone()), &[], ctx)?
            .ok_or_else(|| SolveError::Unsupported("coefficient cancellation incomplete".into()))?;
        values.push(q);
    }
    while values.last().is_some_and(Expr::is_zero) {
        values.pop();
    }
    let content = if content.terms.first().is_some_and(|(m, _)| m.deg > 0) {
        let p = MPoly::new(
            content.nvars,
            content
                .terms
                .into_iter()
                .map(|(m, c)| (m, Rational::from(c)))
                .collect(),
            content.order,
            ctx,
        )?;
        from_mpoly_with(&p, &view.gens[1..], ctx)?
            .ok_or_else(|| SolveError::Unsupported("invalid coefficient content".into()))?
    } else {
        Expr::int(1)
    };
    Ok(Some(Coefficients {
        values,
        denominator,
        content,
    }))
}
pub(super) fn exact(e: &Expr) -> Option<Rational> {
    match e.as_number() {
        Some(Number::Integer(n)) => Some(n.clone().into()),
        Some(Number::Rational(q)) => Some(q.clone()),
        _ => None,
    }
}
pub(super) fn depends(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<bool, om_num::ctx::Abort> {
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if e == x {
            return Ok(true);
        }
        if let ExprKind::Normal(n) = e.kind() {
            stack.push(&n.head);
        }
        stack.extend(e.args().iter());
    }
    Ok(false)
}
pub(super) fn special(e: &Expr, ctx: &Interrupt) -> Result<Expr, om_num::ctx::Abort> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut stack = vec![Frame::Enter(e)];
    let mut values = vec![];
    while let Some(frame) = stack.pop() {
        ctx.tick()?;
        match frame {
            Frame::Enter(e) => {
                if let ExprKind::Normal(n) = e.kind() {
                    stack.push(Frame::Build(n));
                    stack.extend(n.args.iter().rev().map(Frame::Enter));
                    stack.push(Frame::Enter(&n.head));
                } else {
                    values.push(om_simplify::convert::canonicalize_with(e, ctx)?);
                }
            }
            Frame::Build(n) => {
                let args = values.split_off(values.len() - n.args.len());
                let head = values.pop().expect("invariant: visited expression head");
                let e = if let Some(s) = head.as_symbol() {
                    func(s, args)
                } else {
                    Expr::normal(head, args)
                };
                // Arithmetic heads are already canonical; unknown elementary kernels remain.
                let next = if e.is_head(B::ROOT) {
                    e.clone()
                } else {
                    om_simplify::special::eval(&e).unwrap_or_else(|| e.clone())
                };
                values.push(next);
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: one special-normalized root"))
}
