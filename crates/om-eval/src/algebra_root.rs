//! Certified algebraic identity chooses Root indices and equivalent radical branches.
use crate::EvalError;
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, add, mul, pow};
use om_num::{Integer, Number};
use om_poly::{Algebraic, UPoly, algebraic_roots};
use om_simplify::root_reduce::to_algebraic;
fn polynomial(p: &UPoly<Integer>, x: &Expr) -> Expr {
    add(p.coeffs.iter().enumerate().map(|(i, c)| {
        mul([
            Expr::integer(c.clone()),
            pow(x.clone(), Expr::integer(Integer::from(i))),
        ])
    }))
}
fn root_expr(a: &Algebraic, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    if let Algebraic::Rational(q) = a {
        return Ok(Some(Expr::number(Number::Rational(q.clone()))));
    }
    let p = a.minimal_polynomial(ctx)?;
    let index = if let Algebraic::Complex(a) = a {
        a.index
    } else {
        let Some(roots) = algebraic_roots(&p, ctx)? else {
            return Ok(None);
        };
        let mut index = None;
        for (i, b) in roots.iter().enumerate() {
            ctx.tick()?;
            if a.equals(b, ctx)? == Some(true) {
                index = Some(i + 1);
                break;
            }
        }
        let Some(index) = index else { return Ok(None) };
        index
    };
    let slot = Expr::call(B::SLOT, [Expr::int(1)]);
    Ok(Some(Expr::call(
        B::ROOT,
        [
            Expr::call(B::FUNCTION, [polynomial(&p, &slot)]),
            Expr::integer(Integer::from(index)),
        ],
    )))
}
fn radicals(e: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let Some(a) = to_algebraic(e, ctx)? else {
        return Ok(None);
    };
    if let Algebraic::Rational(q) = a {
        return Ok(Some(Expr::number(Number::Rational(q))));
    }
    let p = a.minimal_polynomial(ctx)?;
    let Some(numbered) = root_expr(&a, ctx)? else {
        return Ok(None);
    };
    let Some(Number::Integer(index)) = numbered.args().get(1).and_then(Expr::as_number) else {
        return Ok(None);
    };
    let Ok(index) = usize::try_from(index) else {
        return Ok(None);
    };
    let x = Expr::symbol("$om_radical_axis");
    let result = om_solve::univariate::poly_uni(
        &polynomial(&p, &x),
        &x,
        &om_solve::SolveOptions {
            cubics: true,
            quartics: true,
            record_steps: false,
            ..Default::default()
        },
        ctx,
        &mut om_solve::NoSteps,
    );
    let result = match result {
        Ok(r) => r,
        Err(om_solve::SolveError::Abort(e)) => return Err(e.into()),
        Err(_) => return Ok(None),
    };
    let om_solve::SolutionSet::Finite(roots) = result.set else {
        return Ok(None);
    };
    // poly_uni already associates every constructed value uniquely with this
    // square-free minimal polynomial's certified, ordered root batch. Rebuilding
    // a Ferrari expression's algebraic field here repeats expensive resultants.
    let Some(r) = index.checked_sub(1).and_then(|i| roots.get(i)) else {
        return Ok(None);
    };
    if roots.len() != p.degree().unwrap_or(0)
        || roots
            .iter()
            .any(|r| r.multiplicity != 1 || r.condition.is_some())
    {
        return Ok(None);
    }
    Ok(r.rules
        .first()
        .map(|(_, v)| v.clone())
        .filter(|v| !v.is_head(B::ROOT)))
}
pub(super) fn transform(e: &Expr, as_radicals: bool, ctx: &Interrupt) -> Result<Expr, EvalError> {
    enum Frame<'a> {
        Visit(&'a Expr),
        Build(&'a Expr),
    }
    let mut stack = vec![Frame::Visit(e)];
    let mut values = vec![];
    while let Some(f) = stack.pop() {
        ctx.tick()?;
        match f {
            Frame::Visit(e) => {
                let replacement = if as_radicals {
                    if e.is_head(B::ROOT) {
                        radicals(e, ctx)?
                    } else {
                        None
                    }
                } else if e.free_symbols().is_empty() {
                    if let Some(a) = to_algebraic(e, ctx)? {
                        root_expr(&a, ctx)?
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(r) = replacement {
                    values.push(r);
                    continue;
                }
                if let ExprKind::Normal(n) = e.kind() {
                    // An unsupported Root remains a single semantic object.
                    if e.is_head(B::ROOT) {
                        values.push(e.clone());
                        continue;
                    }
                    stack.push(Frame::Build(e));
                    stack.extend(e.args().iter().rev().map(Frame::Visit));
                    stack.push(Frame::Visit(&n.head));
                } else {
                    values.push(e.clone());
                }
            }
            Frame::Build(e) => {
                let args = values.split_off(values.len() - e.args().len());
                let head = values
                    .pop()
                    .expect("invariant: visited algebraic expression head");
                let result = if let Some(s) = head.as_symbol() {
                    om_core::func(s, args)
                } else {
                    Expr::normal(head, args)
                };
                values.push(result);
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: algebraic traversal produces one result"))
}
