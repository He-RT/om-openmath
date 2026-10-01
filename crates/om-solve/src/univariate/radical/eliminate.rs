//! Reverse radical resultants share one sparse integral coefficient context.
use super::super::{extract, reductions};
use super::collect;
use crate::{Level, SolveError, Step, StepKind, StepSink};
use om_core::{Expr, add, canonical_cmp, pow, sub};
use om_num::{Integer, Rational, ctx::Interrupt, gcd};
use om_poly::{MPoly, Monomial, UPoly};
use om_simplify::convert::{from_mpoly_with, to_rational_function_with};

fn integers(p: &MPoly<Rational>, ctx: &Interrupt) -> Result<MPoly<Integer>, SolveError> {
    let mut scale = Integer::ONE;
    for (_, c) in &p.terms {
        ctx.tick()?;
        let d = Integer::from(c.denominator().clone());
        scale = (&scale / gcd(&scale, &d)) * d;
    }
    let mut terms = vec![];
    for (m, c) in &p.terms {
        ctx.tick()?;
        terms.push((
            m.clone(),
            c.numerator() * (&scale / Integer::from(c.denominator().clone())),
        ));
    }
    Ok(MPoly::new(p.nvars, terms, p.order, ctx)?)
}
fn dense(
    p: &MPoly<Integer>,
    axis: usize,
    ctx: &Interrupt,
) -> Result<UPoly<MPoly<Integer>>, SolveError> {
    let degree = p.terms.iter().map(|(m, _)| m.exps[axis]).max().unwrap_or(0) as usize;
    if degree > 4096 {
        return Err(SolveError::Unsupported(
            "dense radical resultant degree exceeds 4096".into(),
        ));
    }
    let mut terms = vec![vec![]; degree + 1];
    for (m, c) in &p.terms {
        ctx.tick()?;
        let mut exps = m.exps.clone();
        exps[axis] = 0;
        terms[m.exps[axis] as usize].push((
            Monomial::new(exps).expect("invariant: removing an exponent cannot overflow"),
            c.clone(),
        ));
    }
    let mut coefficients = vec![];
    for terms in terms {
        ctx.tick()?;
        coefficients.push(MPoly::new(p.nvars, terms, p.order, ctx)?);
    }
    Ok(UPoly::new(coefficients))
}
fn render(p: &MPoly<Integer>, gens: &[Expr], ctx: &Interrupt) -> Result<Expr, SolveError> {
    let mut terms = vec![];
    for (m, c) in &p.terms {
        ctx.tick()?;
        terms.push((m.clone(), Rational::from(c.clone())));
    }
    let q = MPoly::new(p.nvars, terms, p.order, ctx)?;
    from_mpoly_with(&q, gens, ctx)?
        .ok_or_else(|| SolveError::Unsupported("radical resultant reconstruction failed".into()))
}
pub(super) fn general(
    e: &Expr,
    x: &Expr,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Expr>, SolveError> {
    let groups = collect::discover(e, x, true, ctx)?;
    if groups.is_empty() {
        return Ok(None);
    }
    let mut axes = vec![];
    for group in &groups {
        ctx.tick()?;
        let y = reductions::fresh(
            &add(std::iter::once(e.clone()).chain(axes.iter().cloned())),
            x,
            ctx,
        )?;
        sink.record(|| {
            Step::new(
                StepKind::Substitute {
                    new_var: y.clone(),
                    def: group.expression(),
                },
                vec![group.expression()],
                vec![y.clone()],
                Level::Major,
            )
        });
        axes.push(y);
    }
    let mut expressions = vec![collect::rewrite(e, &groups, &axes, ctx)?];
    for (i, group) in groups.iter().enumerate() {
        let base = collect::rewrite(&group.base, &groups[..i], &axes[..i], ctx)?;
        expressions.push(sub(
            pow(axes[i].clone(), Expr::int(i64::from(group.degree))),
            base,
        ));
    }
    let requested = std::iter::once(x.clone())
        .chain(axes.iter().cloned())
        .collect::<Vec<_>>();
    let mut extra = vec![];
    for e in &expressions {
        let Some(view) = to_rational_function_with(e, &requested, ctx)? else {
            return Ok(None);
        };
        for g in view.gens {
            ctx.tick()?;
            if requested.contains(&g) || extra.contains(&g) {
                continue;
            }
            for axis in &requested {
                if extract::depends(&g, axis, ctx)? {
                    return Ok(None);
                }
            }
            extra.push(g);
        }
    }
    extra.sort_by(canonical_cmp);
    let gens = requested.into_iter().chain(extra).collect::<Vec<_>>();
    let mut polys = vec![];
    for e in &expressions {
        let Some(view) = to_rational_function_with(e, &gens, ctx)? else {
            return Ok(None);
        };
        if view.gens != gens {
            return Ok(None);
        }
        polys.push(integers(&view.num, ctx)?);
    }
    let mut p = polys.remove(0);
    for i in (0..groups.len()).rev() {
        ctx.tick()?;
        let before = if sink.enabled() {
            vec![render(&p, &gens, ctx)?, render(&polys[i], &gens, ctx)?]
        } else {
            vec![]
        };
        let result = dense(&p, i + 1, ctx)?.resultant(&dense(&polys[i], i + 1, ctx)?, ctx)?;
        let expr = render(&result, &gens, ctx)?;
        sink.record(|| {
            Step::new(
                StepKind::Resultant {
                    var: axes[i].clone(),
                    result: expr.clone(),
                },
                before,
                vec![expr],
                Level::Major,
            )
        });
        p = result;
    }
    let p = render(&p, &gens, ctx)?;
    Ok(extract::coefficients(&p, x, ctx)?.map(|_| p))
}
