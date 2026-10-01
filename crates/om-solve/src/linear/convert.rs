//! Every affine row shares one coefficient-generator context before elimination.
use crate::{Level, SolveError, Step, StepKind, StepSink};
use om_core::{Expr, canonical_cmp};
use om_num::{
    Integer, Number, Rational,
    ctx::{Abort, Interrupt},
    gcd,
};
use om_poly::{MPoly, Monomial};
use om_simplify::convert::{from_mpoly_with, to_rational_function_with};
pub(crate) struct Matrix {
    pub a: Vec<Vec<MPoly<Integer>>>,
    pub b: Vec<MPoly<Integer>>,
    pub parameters: Vec<Expr>,
}
fn depends(e: &Expr, vars: &[Expr], ctx: &Interrupt) -> Result<bool, Abort> {
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if vars.contains(e) {
            return Ok(true);
        }
        if let om_core::ExprKind::Normal(n) = e.kind() {
            stack.push(&n.head);
            stack.extend(&n.args)
        }
    }
    Ok(false)
}
pub(crate) fn build(
    equations: &[Expr],
    vars: &[Expr],
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Matrix>, SolveError> {
    let mut parameters = vec![];
    for e in equations {
        let Some(view) = to_rational_function_with(e, vars, ctx)? else {
            return Ok(None);
        };
        if !view.gens.starts_with(vars) {
            return Ok(None);
        }
        for g in &view.gens[vars.len()..] {
            if depends(g, vars, ctx)? {
                return Ok(None);
            }
            if !parameters.contains(g) {
                parameters.push(g.clone())
            }
        }
    }
    parameters.sort_by(canonical_cmp);
    let gens = vars.iter().chain(&parameters).cloned().collect::<Vec<_>>();
    let mut a = vec![];
    let mut b = vec![];
    for e in equations {
        let Some(view) = to_rational_function_with(e, &gens, ctx)? else {
            return Ok(None);
        };
        if view.gens != gens
            || view
                .den
                .terms
                .iter()
                .any(|(m, _)| m.exps[..vars.len()].iter().any(|e| *e != 0))
        {
            return Ok(None);
        }
        let mut scale = Integer::ONE;
        for (_, q) in &view.num.terms {
            ctx.tick()?;
            let d = Integer::from(q.denominator().clone());
            scale = (&scale / gcd(&scale, &d)) * d
        }
        let mut groups = vec![vec![]; vars.len() + 1];
        for (m, q) in &view.num.terms {
            ctx.tick()?;
            let degree = m.exps[..vars.len()]
                .iter()
                .map(|e| u64::from(*e))
                .sum::<u64>();
            if degree > 1 {
                return Ok(None);
            }
            let column = if degree == 0 {
                vars.len()
            } else {
                m.exps[..vars.len()]
                    .iter()
                    .position(|e| *e == 1)
                    .expect("invariant: total affine degree one")
            };
            groups[column].push((
                Monomial::new(m.exps[vars.len()..].iter().copied())
                    .expect("invariant: coefficient degree does not exceed original"),
                q.numerator() * (&scale / Integer::from(q.denominator().clone())),
            ));
        }
        if scale != Integer::ONE {
            sink.record(|| {
                Step::new(
                    StepKind::ClearDenominators {
                        factor: Expr::number(Number::Integer(scale.clone())),
                    },
                    vec![e.clone()],
                    vec![om_core::mul([
                        Expr::number(Number::Integer(scale)),
                        e.clone(),
                    ])],
                    Level::Minor,
                )
            });
        }
        let mut row = vec![];
        for terms in groups {
            row.push(MPoly::new(parameters.len(), terms, view.num.order, ctx)?)
        }
        let constant = row
            .pop()
            .expect("invariant: augmented affine row contains rhs");
        a.push(row);
        b.push(constant.neg(ctx)?);
    }
    Ok(Some(Matrix { a, b, parameters }))
}
pub(super) fn render(
    p: &MPoly<Integer>,
    parameters: &[Expr],
    ctx: &Interrupt,
) -> Result<Expr, Abort> {
    if p.nvars == 0 {
        ctx.tick()?;
        return Ok(Expr::number(Number::Integer(
            p.terms.iter().fold(Integer::ZERO, |s, (_, c)| s + c),
        )));
    }
    let mut terms = vec![];
    for (m, c) in &p.terms {
        ctx.tick()?;
        terms.push((m.clone(), Rational::from(c.clone())))
    }
    let q = MPoly::new(p.nvars, terms, p.order, ctx)?;
    Ok(from_mpoly_with(&q, parameters, ctx)?
        .expect("invariant: validated coefficient context reconstructs"))
}
pub(super) fn fraction(
    f: &om_poly::ExactFraction<MPoly<Integer>>,
    parameters: &[Expr],
    ctx: &Interrupt,
) -> Result<Expr, SolveError> {
    let value = om_core::div(
        render(&f.num, parameters, ctx)?,
        render(&f.den, parameters, ctx)?,
    );
    om_simplify::algebra::cancel_with(&value, &[], ctx)?
        .ok_or_else(|| SolveError::Unsupported("linear fraction cancellation failed".into()))
}
