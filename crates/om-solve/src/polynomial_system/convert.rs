//! Lex axes retain requested order; independent coefficient expressions follow them.
use crate::{Level, SolveError, Step, StepKind, StepSink};
use om_core::{Expr, ExprKind, canonical_cmp};
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
    gcd,
};
use om_poly::{FactorStatus, MPoly, Monomial, UPoly};
use om_simplify::convert::{from_mpoly_with, to_rational_function_with};
pub(super) type Poly = MPoly<Rational>;
type Converted = (Vec<Poly>, Vec<Expr>);
type Factors = Vec<(UPoly<Rational>, u32)>;
pub(super) fn depends(e: &Expr, vars: &[Expr], ctx: &Interrupt) -> Result<bool, Abort> {
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if vars.contains(e) {
            return Ok(true);
        }
        if let ExprKind::Normal(n) = e.kind() {
            stack.push(&n.head);
            stack.extend(&n.args)
        }
    }
    Ok(false)
}
pub(super) fn build(
    equations: &[Expr],
    vars: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Converted>, SolveError> {
    let mut extra = vec![];
    for e in equations {
        let Some(v) = to_rational_function_with(e, vars, ctx)? else {
            return Ok(None);
        };
        for g in &v.gens[vars.len()..] {
            if depends(g, vars, ctx)? {
                return Ok(None);
            }
            if !extra.contains(g) {
                extra.push(g.clone())
            }
        }
    }
    extra.sort_by(canonical_cmp);
    let axes = vars.iter().chain(&extra).cloned().collect::<Vec<_>>();
    let mut polys = vec![];
    for e in equations {
        let Some(v) = to_rational_function_with(e, &axes, ctx)? else {
            return Ok(None);
        };
        if v.gens != axes
            || v.den
                .terms
                .iter()
                .any(|(m, _)| m.exps[..vars.len()].iter().any(|e| *e != 0))
        {
            return Ok(None);
        }
        polys.push(v.num)
    }
    Ok(Some((polys, axes)))
}
pub(super) fn render(p: &Poly, axes: &[Expr], ctx: &Interrupt) -> Result<Expr, SolveError> {
    from_mpoly_with(p, axes, ctx)?
        .ok_or_else(|| SolveError::Unsupported("polynomial system context reconstruction".into()))
}
pub(super) fn basis(
    inputs: &[Poly],
    axes: &[Expr],
    order: om_poly::MonoOrder,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Poly>>, SolveError> {
    let Some(result) = om_poly::groebner(inputs, order, ctx)? else {
        return Ok(None);
    };
    record_basis(inputs, &result, axes, order, ctx, sink)?;
    Ok(Some(result))
}
pub(super) fn record_basis(
    before: &[Poly],
    after: &[Poly],
    axes: &[Expr],
    order: om_poly::MonoOrder,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<(), SolveError> {
    if sink.enabled() {
        let before = before
            .iter()
            .map(|p| render(p, axes, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let after = after
            .iter()
            .map(|p| render(p, axes, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        sink.record(|| {
            Step::new(
                StepKind::Groebner {
                    order,
                    basis: after.clone(),
                },
                before,
                after,
                Level::Major,
            )
        });
    }
    Ok(())
}
pub(super) fn univariate(
    p: &Poly,
    axis: usize,
    ctx: &Interrupt,
) -> Result<Option<UPoly<Rational>>, SolveError> {
    let degree = p.terms.iter().map(|(m, _)| m.exps[axis]).max().unwrap_or(0) as usize;
    if degree > 4096 {
        return Ok(None);
    }
    let mut c = vec![Rational::ZERO; degree + 1];
    for (m, q) in &p.terms {
        ctx.tick()?;
        if m.exps.iter().enumerate().any(|(i, e)| i != axis && *e != 0) {
            return Ok(None);
        }
        c[m.exps[axis] as usize] += q;
    }
    Ok(Some(UPoly::new(c)))
}
pub(super) fn embed(
    p: &UPoly<Rational>,
    axis: usize,
    nvars: usize,
    ctx: &Interrupt,
) -> Result<Poly, SolveError> {
    let mut terms = vec![];
    for (i, c) in p.coeffs.iter().enumerate() {
        ctx.tick()?;
        if c == &Rational::ZERO {
            continue;
        }
        let mut exps = vec![0; nvars];
        exps[axis] = u32::try_from(i).expect("invariant: bounded dense univariate degree");
        terms.push((
            Monomial::new(exps).expect("invariant: bounded pure-power degree"),
            c.clone(),
        ))
    }
    Ok(Poly::new(nvars, terms, om_poly::MonoOrder::Lex, ctx)?)
}
pub(super) fn factors(p: &UPoly<Rational>, ctx: &Interrupt) -> Result<Option<Factors>, SolveError> {
    let mut scale = Integer::ONE;
    for q in &p.coeffs {
        ctx.tick()?;
        let d = Integer::from(q.denominator().clone());
        scale = (&scale / gcd(&scale, &d)) * d
    }
    let mut c = vec![];
    for q in &p.coeffs {
        ctx.tick()?;
        c.push(q.numerator() * (&scale / Integer::from(q.denominator().clone())))
    }
    let Some(f) = UPoly::new(c).factor_z(ctx)? else {
        return Ok(None);
    };
    if f.status != FactorStatus::Complete {
        return Ok(None);
    }
    Ok(Some(
        f.factors
            .into_iter()
            .map(|(p, m)| {
                (
                    UPoly::new(p.coeffs.into_iter().map(Rational::from).collect()),
                    m,
                )
            })
            .collect(),
    ))
}
