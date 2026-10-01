//! Independent later axes remain parameters; unresolved equations stay explicit.
use super::convert::{self, Poly};
use crate::{
    Level, Solution, SolutionSet, SolveError, SolveOptions, Step, StepKind, StepSink, Verification,
    univariate::poly_uni,
};
use om_core::{BUILTIN as B, Expr};
use om_num::ctx::Interrupt;
fn independent(basis: &[Poly], subset: &[usize], ctx: &Interrupt) -> Result<bool, SolveError> {
    for p in basis {
        ctx.tick()?;
        if let Some((m, _)) = p.terms.first()
            && m.exps
                .iter()
                .enumerate()
                .all(|(i, e)| *e == 0 || subset.contains(&i))
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn maximum(
    basis: &[Poly],
    n: usize,
    size: usize,
    ctx: &Interrupt,
) -> Result<Vec<usize>, SolveError> {
    if size == 0 {
        return Ok(vec![]);
    }
    let mut indices = (n - size..n).collect::<Vec<_>>();
    loop {
        ctx.tick()?;
        if independent(basis, &indices, ctx)? {
            return Ok(indices);
        }
        let mut position = size;
        while position > 0
            && indices[position - 1]
                == if position == 1 {
                    0
                } else {
                    indices[position - 2] + 1
                }
        {
            position -= 1
        }
        if position == 0 {
            return Err(SolveError::Unsupported(
                "independent set certificate unavailable".into(),
            ));
        }
        indices[position - 1] -= 1;
        for (i, value) in indices.iter_mut().enumerate().skip(position) {
            *value = n - size + i
        }
    }
}
pub(super) struct Solved {
    pub roots: Vec<Solution>,
    pub assumptions: Vec<Expr>,
}
pub(super) fn solve(
    basis: &[Poly],
    axes: &[Expr],
    requested: usize,
    dimension: usize,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Solved>, SolveError> {
    let mut assumptions = vec![];
    let free = maximum(basis, axes.len(), dimension, ctx)?;
    let expressions = basis
        .iter()
        .map(|p| convert::render(p, axes, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    let mut roots = vec![Solution {
        rules: vec![],
        condition: None,
        constants: vec![],
        multiplicity: 1,
        verification: Verification::ByConstruction,
        numeric: None,
    }];
    for axis in (0..axes.len()).rev() {
        if free.contains(&axis) || axis >= requested {
            continue;
        }
        let mut next = vec![];
        for root in roots {
            ctx.tick()?;
            let mut selected = None;
            for expression in &expressions {
                let e = expression.replace_all(&root.rules);
                let Some(view) = om_simplify::convert::to_rational_function_with(
                    &e,
                    std::slice::from_ref(&axes[axis]),
                    ctx,
                )?
                else {
                    continue;
                };
                if view.gens.first() != Some(&axes[axis]) {
                    continue;
                }
                let mut dependent = false;
                for g in &view.gens[1..] {
                    dependent |= convert::depends(g, std::slice::from_ref(&axes[axis]), ctx)?
                }
                if dependent {
                    continue;
                }
                let degree = view
                    .num
                    .terms
                    .iter()
                    .map(|(m, _)| m.exps[0])
                    .max()
                    .unwrap_or(0);
                if degree == 0 || degree > 2 {
                    continue;
                }
                if expressions.len() > 1 && convert::depends(&e, &axes[..axis], ctx)? {
                    continue;
                }
                selected = Some(e);
                break;
            }
            if let Some(e) = selected {
                let mut inner = opts.clone();
                inner.domain = crate::Domain::Complexes;
                let result = poly_uni(&e, &axes[axis], &inner, ctx, sink)?;
                let SolutionSet::Finite(mut values) = result.set else {
                    return Ok(None);
                };
                for guard in result.assumptions {
                    if !assumptions.contains(&guard) {
                        assumptions.push(guard);
                    }
                }
                for value in &mut values {
                    value.rules.extend(root.rules.iter().cloned());
                    let mut conditions = value
                        .condition
                        .iter()
                        .chain(root.condition.iter())
                        .cloned()
                        .collect::<Vec<_>>();
                    value.condition = match conditions.len() {
                        0 => None,
                        1 => conditions.pop(),
                        _ => Some(Expr::call(B::AND, conditions)),
                    };
                    value.multiplicity = 1;
                    sink.record(|| {
                        Step::new(
                            StepKind::BackSubstitute {
                                var: axes[axis].clone(),
                                value: value.rules[0].1.clone(),
                            },
                            vec![e.clone()],
                            vec![value.rules[0].1.clone()],
                            Level::Major,
                        )
                    });
                }
                next.extend(values)
            } else {
                next.push(root)
            }
        }
        roots = next;
    }
    for root in &mut roots {
        let rules = root.rules.clone();
        for (_, v) in &mut root.rules {
            *v = v.replace_all(&rules)
        }
        root.rules.sort_by_key(|(v, _)| {
            axes.iter()
                .position(|a| a == v)
                .expect("invariant: system variable is an axis")
        });
        let mut conditions = root.condition.iter().cloned().collect::<Vec<_>>();
        for e in &expressions {
            let residual = e.replace_all(&root.rules);
            match om_simplify::zero::is_zero_with(&residual, ctx)? {
                om_simplify::zero::Tri::Zero => {}
                _ => conditions.push(Expr::call(B::EQUAL, [residual, Expr::int(0)])),
            }
        }
        root.condition = match conditions.len() {
            0 => None,
            1 => conditions.pop(),
            _ => Some(Expr::call(B::AND, conditions)),
        };
    }
    Ok(Some(Solved { roots, assumptions }))
}
