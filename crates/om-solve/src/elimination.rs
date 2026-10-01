//! Exact lex projection keeps retained-variable restrictions explicit.
use crate::{Domain, NoSteps, SolveError, normalize};
use om_core::{BUILTIN as B, Expr, canonical_cmp};
use om_num::ctx::Interrupt;
use om_poly::MonoOrder;
use om_simplify::convert::from_mpoly_with;
/// Eliminate polynomial axes through a lex Groebner basis with those axes first.
/// Unsupported pole projections or logical inputs return Unsupported; Abort propagates.
pub fn eliminate(eqs: &Expr, elim: &[Expr], ctx: &Interrupt) -> Result<Expr, SolveError> {
    let requested = normalize::normalize(
        &Expr::sym(B::TRUE),
        Some(elim),
        Domain::Complexes,
        ctx,
        &mut NoSteps,
    )?
    .vars;
    let mut remaining = eqs
        .free_symbols()
        .into_iter()
        .filter(|s| !om_core::builtins::names().contains(&s.name()))
        .map(Expr::sym)
        .filter(|e| !requested.contains(e))
        .collect::<Vec<_>>();
    remaining.sort_by(canonical_cmp);
    let axes = requested
        .iter()
        .chain(&remaining)
        .cloned()
        .collect::<Vec<_>>();
    let prepared = normalize::normalize(eqs, Some(&axes), Domain::Complexes, ctx, &mut NoSteps)?;
    if prepared.unsupported || prepared.branches.len() > 1 {
        return Err(SolveError::Unsupported(
            "Eliminate requires one polynomial conjunction".into(),
        ));
    }
    let Some(branch) = prepared.branches.first() else {
        return Ok(Expr::sym(B::FALSE));
    };
    if !branch.inequalities.is_empty()
        || branch.domains.iter().any(|(_, d)| *d != Domain::Complexes)
    {
        return Err(SolveError::Unsupported(
            "Eliminate requires complex polynomial equalities".into(),
        ));
    }
    let mut restrictions = branch.conditions.clone();
    for exclusion in &branch.exclusions {
        if requested.iter().any(|v| !exclusion.value.free_of(v)) {
            return Err(SolveError::Unsupported(
                "eliminated-variable pole projection is unresolved".into(),
            ));
        }
        restrictions.push(Expr::call(
            B::UNEQUAL,
            [exclusion.value.clone(), Expr::int(0)],
        ));
    }
    let mut inputs = vec![];
    for e in &branch.equations {
        let Some(view) = om_simplify::convert::to_rational_function_with(e, &axes, ctx)? else {
            return Err(SolveError::Unsupported(
                "Eliminate conversion unavailable".into(),
            ));
        };
        if view.gens != axes || !view.den.is_one() {
            return Err(SolveError::Unsupported(
                "Eliminate requires rational polynomial coefficients".into(),
            ));
        }
        inputs.push(view.num);
    }
    let basis = om_poly::groebner(&inputs, MonoOrder::Lex, ctx)?
        .ok_or_else(|| SolveError::Unsupported("Eliminate basis unavailable".into()))?;
    if basis
        .iter()
        .any(|p| p.terms.len() == 1 && p.terms[0].0.deg == 0 && !p.is_zero())
    {
        return Ok(Expr::sym(B::FALSE));
    }
    for p in basis {
        ctx.tick()?;
        if p.terms
            .iter()
            .any(|(m, _)| m.exps[..requested.len()].iter().any(|e| *e != 0))
        {
            continue;
        }
        let value = from_mpoly_with(&p, &axes, ctx)?.ok_or_else(|| {
            SolveError::Unsupported("Eliminate relation reconstruction unavailable".into())
        })?;
        let Some((leading, c)) = p.terms.first() else {
            continue;
        };
        let lhs = om_core::mul(
            std::iter::once(Expr::number(om_num::Number::Rational(c.clone()))).chain(
                axes.iter()
                    .zip(&leading.exps)
                    .filter(|(_, e)| **e != 0)
                    .map(|(v, e)| om_core::pow(v.clone(), Expr::int(i64::from(*e)))),
            ),
        );
        let rhs = om_core::sub(lhs.clone(), value);
        let rhs = om_simplify::algebra::expand_with(&rhs, ctx)?.unwrap_or(rhs);
        restrictions.push(Expr::call(B::EQUAL, [lhs, rhs]));
    }
    Ok(match restrictions.len() {
        0 => Expr::sym(B::TRUE),
        1 => restrictions.remove(0),
        _ => Expr::call(B::AND, restrictions),
    })
}
