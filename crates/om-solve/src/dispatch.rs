//! The public P0-P3 pipeline retains raw sources through candidate generation.
mod order;
use crate::{
    MaxExtra, NoSteps, Solution, SolutionSet, SolveError, SolveOptions, SolveOutcome, StepRecorder,
    StepSink, normalize,
    univariate::{PolynomialRoots, poly_uni, radical_candidates, transcendental_candidates},
    verification::{self, Origin},
};
use om_core::{BUILTIN as B, Expr, ExprKind, Message};
use om_num::ctx::Interrupt;

/// Solve explicit ordered variables, preserving raw input restrictions and derivation data.
/// Unsupported methods return Unevaluated; interruption and malformed inputs propagate.
pub fn solve(
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
) -> Result<SolveOutcome, SolveError> {
    if crate::inequality::has(eqs, ctx)? {
        return crate::inequality::reduce_with(eqs, vars, opts, ctx, "Solve");
    }
    if opts.record_steps {
        let mut recorder = StepRecorder::new();
        let (set, messages) = run(eqs, vars, opts, ctx, &mut recorder)?;
        Ok(SolveOutcome {
            set,
            steps: Some(recorder.finish()),
            messages,
        })
    } else {
        let (set, messages) = run(eqs, vars, opts, ctx, &mut NoSteps)?;
        Ok(SolveOutcome {
            set,
            steps: None,
            messages,
        })
    }
}
pub(crate) fn input(branch: &normalize::NormalizedBranch) -> Expr {
    Expr::call(
        B::LIST,
        branch
            .original
            .iter()
            .map(|e| Expr::call(B::EQUAL, [e.clone(), Expr::int(0)]))
            .chain(
                branch
                    .exclusions
                    .iter()
                    .map(|e| Expr::call(B::UNEQUAL, [e.value.clone(), Expr::int(0)])),
            )
            .chain(branch.conditions.iter().cloned())
            .chain(branch.domains.iter().map(|(v, d)| {
                Expr::call(
                    B::ELEMENT,
                    [
                        v.clone(),
                        Expr::sym(match d {
                            crate::Domain::Complexes => B::COMPLEXES,
                            crate::Domain::Reals => B::REALS,
                            crate::Domain::Integers => B::INTEGERS,
                            crate::Domain::Rationals => B::RATIONALS,
                        }),
                    ],
                )
            })),
    )
}
fn depends(e: &Expr, vars: &[Expr], ctx: &Interrupt) -> Result<bool, SolveError> {
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
fn polynomial(
    branch: &normalize::NormalizedBranch,
    vars: &[Expr],
    ctx: &Interrupt,
) -> Result<bool, SolveError> {
    for e in &branch.equations {
        let Some(view) = om_simplify::convert::to_rational_function_with(e, vars, ctx)? else {
            return Ok(false);
        };
        if !view.gens.starts_with(vars) {
            return Ok(false);
        }
        for g in &view.gens[vars.len()..] {
            if depends(g, vars, ctx)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn candidates(
    branch: &normalize::NormalizedBranch,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<(PolynomialRoots, Origin), SolveError> {
    let mut inner = opts.clone();
    inner.max_extra_conditions = MaxExtra::Zero;
    if branch.equations.is_empty() {
        return Ok((
            PolynomialRoots {
                set: SolutionSet::All,
                assumptions: vec![],
                messages: vec![],
            },
            Origin::Constructed,
        ));
    }
    let poly = polynomial(branch, vars, ctx)?;
    if vars.len() == 1 && branch.equations.len() == 1 {
        inner.domain = branch.domains[0].1;
        if poly {
            return Ok((
                poly_uni(&branch.equations[0], &vars[0], &inner, ctx, sink)?,
                Origin::Constructed,
            ));
        }
        let probe = radical_candidates(&branch.equations[0], &vars[0], &inner, ctx, &mut NoSteps);
        match probe {
            Ok(result) if !matches!(result.set, SolutionSet::Unevaluated) => {
                let result = if sink.enabled() {
                    radical_candidates(&branch.equations[0], &vars[0], &inner, ctx, sink)?
                } else {
                    result
                };
                return Ok((result, Origin::BranchSensitive));
            }
            Err(e @ SolveError::Abort(_)) | Err(e @ SolveError::Invalid(_)) => return Err(e),
            _ => {}
        }
        if matches!(
            inner.domain,
            crate::Domain::Integers | crate::Domain::Rationals
        ) {
            inner.domain = crate::Domain::Reals
        }
        return Ok((
            transcendental_candidates(&branch.equations[0], &vars[0], &inner, ctx, sink)?,
            Origin::BranchSensitive,
        ));
    }
    let source = input(branch);
    if poly {
        if crate::linear::convert::build(&branch.equations, vars, ctx, &mut NoSteps)?.is_some() {
            return Ok((
                crate::linear_system(&source, vars, &inner, ctx, sink)?,
                Origin::Constructed,
            ));
        }
        return Ok((
            crate::poly_system(&source, vars, &inner, ctx, sink)?,
            Origin::Constructed,
        ));
    }
    Ok((
        crate::substitution_system(&source, vars, &inner, ctx, sink)?,
        Origin::BranchSensitive,
    ))
}
fn same(a: &Solution, b: &Solution, ctx: &Interrupt) -> Result<bool, SolveError> {
    if a.constants != b.constants || a.condition != b.condition || a.rules.len() != b.rules.len() {
        return Ok(false);
    }
    for ((av, a), (bv, b)) in a.rules.iter().zip(&b.rules) {
        if av != bv
            || (a != b
                && crate::univariate::verification_zero(&om_core::sub(a.clone(), b.clone()), ctx)?
                    != om_simplify::zero::Tri::Zero)
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn run(
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<(SolutionSet, Vec<Message>), SolveError> {
    let prepared = match normalize::normalize(eqs, Some(vars), opts.domain, ctx, sink) {
        Err(SolveError::Unsupported(reason)) => {
            let mut messages = vec![];
            verification::note(eqs, "nsmet", &reason, &mut messages, sink);
            return Ok((SolutionSet::Unevaluated, messages));
        }
        result => result?,
    };
    let mut messages = prepared.messages;
    if prepared.unsupported {
        return Ok((SolutionSet::Unevaluated, messages));
    }
    let mut union = vec![];
    let mut all = false;
    let mut unavailable = false;
    for (i, branch) in prepared.branches.iter().enumerate() {
        ctx.tick()?;
        if !branch.inequalities.is_empty() {
            verification::note(
                eqs,
                "ineq",
                "Inequalities require the reduction path.",
                &mut messages,
                sink,
            );
            unavailable = true;
            continue;
        }
        if prepared.branches.len() > 1 && sink.enabled() {
            sink.enter(&format!("solve_branch_{}", i + 1))
        }
        let candidate = candidates(branch, &prepared.vars, opts, ctx, sink)
            .and_then(|(r, origin)| verification::apply(branch, r, origin, opts, ctx, sink));
        if prepared.branches.len() > 1 && sink.enabled() {
            sink.exit()
        }
        let result = match candidate {
            Err(SolveError::Unsupported(reason)) => {
                verification::note(eqs, "nsmet", &reason, &mut messages, sink);
                unavailable = true;
                continue;
            }
            result => result?,
        };
        messages.extend(result.messages);
        match result.set {
            SolutionSet::All => all = true,
            SolutionSet::Finite(roots) => {
                for root in roots {
                    let mut duplicate = None;
                    for (j, old) in union.iter().enumerate() {
                        if same(old, &root, ctx)? {
                            duplicate = Some(j);
                            break;
                        }
                    }
                    if let Some(j) = duplicate {
                        union[j].multiplicity = union[j].multiplicity.max(root.multiplicity)
                    } else {
                        union.push(root)
                    }
                }
            }
            _ => unavailable = true,
        }
    }
    let set = if all {
        SolutionSet::All
    } else if unavailable {
        verification::note(
            eqs,
            "nsmet",
            "This system has an incomplete solution branch.",
            &mut messages,
            sink,
        );
        SolutionSet::Unevaluated
    } else {
        match order::sort(&mut union, &prepared.vars, ctx) {
            Err(SolveError::Unsupported(reason)) => {
                verification::note(eqs, "nsmet", &reason, &mut messages, sink);
                return Ok((SolutionSet::Unevaluated, messages));
            }
            result => result?,
        }
        SolutionSet::Finite(union)
    };
    Ok((set, messages))
}
