//! Sparse coefficient grouping preserves requested axes and independent coefficients.
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, Interrupt, add, div, mul, pow};
use om_num::{Integer, Rational};
use om_poly::MPoly;
use om_simplify::convert::{PolyView, from_mpoly_with, to_rational_function_with};
use std::collections::BTreeMap;
pub(super) fn vars(e: &Expr) -> Option<Vec<Expr>> {
    let v = if e.is_head(B::LIST) {
        e.args().to_vec()
    } else {
        vec![e.clone()]
    };
    if v.len() > 16
        || v.iter().any(|v| {
            v.as_number().is_some()
                || matches!(v.kind(), om_core::ExprKind::String(_))
                || v.as_symbol()
                    .is_some_and(|s| matches!(s, B::TRUE | B::FALSE))
                || v == &Expr::int(0)
                || v.free_symbols().is_empty() && v.as_symbol().is_none()
        })
    {
        return None;
    }
    // Axis equality is structural; symbolic coefficients cannot collapse it.
    if v.iter().enumerate().any(|(i, x)| v[..i].contains(x)) {
        return None;
    }
    Some(v)
}
pub(super) fn polynomial(
    e: &Expr,
    vars: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<PolyView>, EvalError> {
    let Some(view) = to_rational_function_with(e, vars, ctx)? else {
        return Ok(None);
    };
    if !view.gens.starts_with(vars)
        || view.gens[vars.len()..]
            .iter()
            .any(|g| vars.iter().any(|v| !g.free_of(v)))
        || view
            .den
            .terms
            .iter()
            .any(|(m, _)| m.exps[..vars.len()].iter().any(|n| *n != 0))
        || view
            .num
            .terms
            .iter()
            .any(|(m, _)| m.exps[..vars.len()].iter().any(|n| *n > 4096))
    {
        return Ok(None);
    }
    Ok(Some(view))
}
pub(super) fn coefficients(
    view: &PolyView,
    axis: usize,
    ctx: &Interrupt,
) -> Result<Option<Vec<Expr>>, EvalError> {
    let degree = view
        .num
        .terms
        .iter()
        .map(|(m, _)| m.exps[axis] as usize)
        .max()
        .unwrap_or(0);
    if degree > 4096 {
        return Ok(None);
    }
    let mut result = vec![];
    for k in 0..=degree {
        ctx.tick()?;
        let Some(c) = coefficient(view, axis, k as u32, ctx)? else {
            return Ok(None);
        };
        result.push(c);
    }
    while result.last().is_some_and(Expr::is_zero) {
        result.pop();
    }
    Ok(Some(result))
}
fn coefficient(
    view: &PolyView,
    axis: usize,
    k: u32,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let mut terms = vec![];
    for (m, c) in &view.num.terms {
        ctx.tick()?;
        if m.exps[axis] == k {
            let mut m = m.clone();
            m.deg -= m.exps[axis];
            m.exps[axis] = 0;
            terms.push((m, c.clone()));
        }
    }
    let p = MPoly::new(view.gens.len(), terms, view.num.order, ctx)?;
    let (Some(n), Some(d)) = (
        from_mpoly_with(&p, &view.gens, ctx)?,
        from_mpoly_with(&view.den, &view.gens, ctx)?,
    ) else {
        return Ok(None);
    };
    Ok(Some(div(n, d)))
}
fn tensor(
    view: &PolyView,
    degrees: &[u32],
    key: &mut Vec<u32>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    ctx.tick()?;
    if key.len() == degrees.len() {
        let mut terms = vec![];
        for (m, c) in &view.num.terms {
            ctx.tick()?;
            if m.exps.starts_with(key) {
                let mut m = m.clone();
                for n in &mut m.exps[..key.len()] {
                    m.deg -= *n;
                    *n = 0;
                }
                terms.push((m, c.clone()));
            }
        }
        let p = MPoly::new(view.gens.len(), terms, view.num.order, ctx)?;
        let (Some(n), Some(d)) = (
            from_mpoly_with(&p, &view.gens, ctx)?,
            from_mpoly_with(&view.den, &view.gens, ctx)?,
        ) else {
            return Ok(None);
        };
        return Ok(Some(div(n, d)));
    }
    let mut rows = vec![];
    for i in 0..=degrees[key.len()] {
        key.push(i);
        let Some(v) = tensor(view, degrees, key, ctx)? else {
            return Ok(None);
        };
        key.pop();
        rows.push(v);
    }
    Ok(Some(Expr::call(B::LIST, rows)))
}
pub(super) fn query(
    ev: &mut Evaluator,
    name: &str,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if name == "Variables" {
        let Some(view) = to_rational_function_with(&args[0], &[], ctx)? else {
            return Ok(None);
        };
        return Ok(Some(Expr::call(B::LIST, view.gens)));
    }
    let Some(vars) = vars(&args[1]) else {
        return Ok((name == "PolynomialQ").then(|| Expr::sym(B::FALSE)));
    };
    let Some(view) = polynomial(&args[0], &vars, ctx)? else {
        return Ok((name == "PolynomialQ").then(|| Expr::sym(B::FALSE)));
    };
    if name == "PolynomialQ" {
        return Ok(Some(Expr::sym(B::TRUE)));
    }
    if vars.is_empty() {
        return Ok(None);
    }
    Ok(match name {
        "Coefficient" if vars.len() == 1 => {
            let n = if let Some(v) = args.get(2) {
                let Some(om_num::Number::Integer(n)) = v.as_number() else {
                    return Ok(None);
                };
                let Ok(n) = u32::try_from(n) else {
                    return Ok(None);
                };
                n
            } else {
                1
            };
            coefficient(&view, 0, n, ctx)?
        }
        "CoefficientList" => {
            if view.num.is_zero() {
                Some(Expr::call(B::LIST, []))
            } else {
                let degrees = (0..vars.len())
                    .map(|i| {
                        view.num
                            .terms
                            .iter()
                            .map(|(m, _)| m.exps[i])
                            .max()
                            .unwrap_or(0)
                    })
                    .collect::<Vec<_>>();
                let size = degrees
                    .iter()
                    .try_fold(1usize, |a, d| a.checked_mul(*d as usize + 1));
                if size.is_none_or(|s| s > 1_000_000) {
                    return Ok(None);
                }
                tensor(&view, &degrees, &mut vec![], ctx)?
            }
        }
        "Exponent" if vars.len() == 1 => Some(if view.num.is_zero() {
            Expr::call(B::DIRECTED_INFINITY, [Expr::int(-1)])
        } else {
            Expr::integer(Integer::from(
                view.num
                    .terms
                    .iter()
                    .map(|(m, _)| m.exps[0])
                    .max()
                    .unwrap_or(0),
            ))
        }),
        "Collect" => {
            let mut groups: BTreeMap<Vec<u32>, Vec<_>> = BTreeMap::new();
            for (m, c) in &view.num.terms {
                ctx.tick()?;
                let key = m.exps[..vars.len()].to_vec();
                let mut m = m.clone();
                for n in &mut m.exps[..vars.len()] {
                    m.deg -= *n;
                    *n = 0;
                }
                groups.entry(key).or_default().push((m, c.clone()));
            }
            let Some(den) = from_mpoly_with(&view.den, &view.gens, ctx)? else {
                return Ok(None);
            };
            let mut terms = vec![];
            for (key, group) in groups {
                let p = MPoly::<Rational>::new(view.gens.len(), group, view.num.order, ctx)?;
                let Some(c) = from_mpoly_with(&p, &view.gens, ctx)? else {
                    return Ok(None);
                };
                let mut c = div(c, den.clone());
                if let Some(h) = args.get(2) {
                    c = ev.evaluate(&Expr::normal(h.clone(), [c]), ctx)?;
                }
                terms.push(mul(std::iter::once(c).chain(
                    vars.iter()
                        .zip(key)
                        .map(|(v, n)| pow(v.clone(), Expr::integer(Integer::from(n)))),
                )));
            }
            Some(add(terms))
        }
        _ => None,
    })
}
