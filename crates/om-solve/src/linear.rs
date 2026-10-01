//! Affine solving preserves original restrictions and observes the shared Bareiss kernel.
pub(crate) mod convert;
mod trace;
use crate::{
    Domain, Level, MaxExtra, NoSteps, Solution, SolutionSet, SolveError, SolveOptions, Step,
    StepKind, StepSink, Verification, normalize, univariate::PolynomialRoots,
};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel, add, mul};
use om_num::ctx::Interrupt;
use om_poly::LinearResult;
use om_simplify::zero::{Tri, is_zero_with};

/// Solve a single affine conjunction using exact fraction-free elimination.
/// Free columns retain their original variable names. Generic pivot conditions and
/// original exclusions are retained, and enabled steps reflect the actual updates.
/// Disjunction orchestration and non-affine/domain-specific algorithms are separate.
pub fn linear_system(
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    if !opts.record_steps {
        let result = run(eqs, vars, opts, ctx, &mut NoSteps)?;
        return crate::domain::filter_input(eqs, vars, opts, result, ctx, &mut NoSteps);
    }
    let result = run(eqs, vars, opts, ctx, sink)?;
    crate::domain::filter_input(eqs, vars, opts, result, ctx, sink)
}
fn note(
    tag: &str,
    text: &str,
    before: &Expr,
    messages: &mut Vec<Message>,
    sink: &mut impl StepSink,
) {
    let msg = Message {
        symbol: "Solve".into(),
        tag: tag.into(),
        text: text.into(),
        level: MsgLevel::Warning,
    };
    sink.record(|| {
        Step::new(
            StepKind::Note { msg: msg.clone() },
            vec![before.clone()],
            vec![],
            Level::Minor,
        )
    });
    messages.push(msg);
}
fn conjunction(conditions: &[Expr]) -> Option<Expr> {
    match conditions.len() {
        0 => None,
        1 => Some(conditions[0].clone()),
        _ => Some(Expr::call(B::AND, conditions.iter().cloned())),
    }
}
fn assume(value: Expr, assumptions: &mut Vec<Expr>, sink: &mut impl StepSink) {
    let cond = Expr::call(B::UNEQUAL, [value.clone(), Expr::int(0)]);
    if !assumptions.contains(&cond) {
        sink.record(|| {
            Step::new(
                StepKind::GenericAssumption { cond: cond.clone() },
                vec![value],
                vec![],
                Level::Minor,
            )
        });
        assumptions.push(cond)
    }
}
fn run(
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    ctx.tick()?;
    let prepared = normalize::normalize(eqs, Some(vars), opts.domain, ctx, sink)?;
    let mut messages = prepared.messages;
    if prepared.unsupported || prepared.branches.len() > 1 {
        note(
            "nsmet",
            "This linear kernel requires a single affine conjunction.",
            eqs,
            &mut messages,
            sink,
        );
        return Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions: vec![],
            messages,
        });
    }
    let Some(branch) = prepared.branches.first() else {
        return Ok(PolynomialRoots {
            set: SolutionSet::Finite(vec![]),
            assumptions: vec![],
            messages,
        });
    };
    if !branch.inequalities.is_empty() {
        note(
            "nsmet",
            "Linear inequalities require the reduction path.",
            eqs,
            &mut messages,
            sink,
        );
        return Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions: vec![],
            messages,
        });
    }
    let vars = &prepared.vars;
    if !vars.is_empty() && branch.domains.iter().all(|(_, d)| *d == Domain::Integers) {
        return crate::domain::integer_linear(eqs, vars, branch, opts, ctx, sink, messages);
    }
    let Some(matrix) = convert::build(&branch.equations, vars, ctx, sink)? else {
        note(
            "nsmet",
            "The conjunction is not affine in all requested variables.",
            eqs,
            &mut messages,
            sink,
        );
        return Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions: vec![],
            messages,
        });
    };
    let mut augmented = vec![];
    for (a, b) in matrix.a.iter().zip(&matrix.b) {
        let mut row = a.clone();
        row.push(b.clone());
        augmented.push(row)
    }
    let current = if sink.enabled() {
        trace::matrix(&augmented, &matrix.parameters, ctx)?
    } else {
        vec![]
    };
    let mut trace = trace::Trace {
        current,
        parameters: &matrix.parameters,
        ctx,
        sink,
    };
    let result =
        om_poly::linear_solve_observed(&matrix.a, &matrix.b, vars.len(), ctx, &mut |op, m| {
            trace.observe(op, m)
        })?;
    let Some(result) = result else {
        note(
            "nsmet",
            "Exact linear elimination could not be completed.",
            eqs,
            &mut messages,
            trace.sink,
        );
        return Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions: vec![],
            messages,
        });
    };
    let sink = trace.sink;
    let mut assumptions = vec![];
    let linear = match result {
        LinearResult::Inconsistent => {
            return Ok(PolynomialRoots {
                set: SolutionSet::Finite(vec![]),
                assumptions,
                messages,
            });
        }
        LinearResult::GenericInconsistent {
            assumptions: guards,
        } => {
            for guard in guards {
                assume(
                    convert::render(&guard, &matrix.parameters, ctx)?,
                    &mut assumptions,
                    sink,
                )
            }
            return Ok(PolynomialRoots {
                set: SolutionSet::Finite(vec![]),
                assumptions,
                messages,
            });
        }
        LinearResult::Consistent(solution) => solution,
    };
    for guard in &linear.assumptions {
        assume(
            convert::render(guard, &matrix.parameters, ctx)?,
            &mut assumptions,
            sink,
        )
    }
    let mut rules = vec![];
    for (i, var) in vars.iter().enumerate() {
        ctx.tick()?;
        if linear.free_columns.contains(&i) {
            continue;
        }
        let mut terms = vec![convert::fraction(
            &linear.particular[i],
            &matrix.parameters,
            ctx,
        )?];
        for (vector, column) in linear.nullspace.iter().zip(&linear.free_columns) {
            terms.push(mul([
                convert::fraction(&vector[i], &matrix.parameters, ctx)?,
                vars[*column].clone(),
            ]))
        }
        let value = om_simplify::algebra::cancel_with(&add(terms), &[], ctx)?.ok_or_else(|| {
            SolveError::Unsupported("linear back substitution cancellation failed".into())
        })?;
        sink.record(|| {
            Step::new(
                StepKind::BackSubstitute {
                    var: var.clone(),
                    value: value.clone(),
                },
                vec![eqs.clone()],
                vec![Expr::call(B::RULE, [var.clone(), value.clone()])],
                Level::Major,
            )
        });
        rules.push((var.clone(), value));
    }
    if !linear.free_columns.is_empty() && !branch.equations.is_empty() {
        note(
            "svars",
            "Equations leave some requested variables free.",
            eqs,
            &mut messages,
            sink,
        )
    }
    let expose = match opts.max_extra_conditions {
        MaxExtra::Zero => false,
        MaxExtra::All => true,
        MaxExtra::Count(n) => assumptions.len() <= n as usize,
    };
    let mut conditions = branch
        .conditions
        .iter()
        .map(|e| e.replace_all(&rules))
        .collect::<Vec<_>>();
    if expose {
        conditions.extend(assumptions.iter().cloned())
    }
    for exclusion in &branch.exclusions {
        let value = exclusion.value.replace_all(&rules);
        let zero = is_zero_with(&value, ctx)?;
        if zero == Tri::Zero {
            return Ok(PolynomialRoots {
                set: SolutionSet::Finite(vec![]),
                assumptions,
                messages,
            });
        }
        if !value.free_symbols().is_empty() {
            let condition = Expr::call(B::UNEQUAL, [value, Expr::int(0)]);
            if !conditions.contains(&condition) {
                conditions.push(condition)
            }
        } else if matches!(zero, Tri::Unknown(_)) {
            let Some(z) = om_simplify::numeval::enclose(&value, 512, ctx)? else {
                return Ok(PolynomialRoots {
                    set: SolutionSet::Unevaluated,
                    assumptions,
                    messages,
                });
            };
            if !z.re.excludes_zero() && !z.im.excludes_zero() {
                return Ok(PolynomialRoots {
                    set: SolutionSet::Unevaluated,
                    assumptions,
                    messages,
                });
            }
        }
    }
    let mut exact = true;
    for original in &branch.original {
        let residual = original.replace_all(&rules);
        let outcome = is_zero_with(&residual, ctx)?;
        sink.record(|| {
            Step::new(
                StepKind::Verify {
                    candidate: rules.clone(),
                    outcome,
                    residual: Some(residual.clone()),
                },
                vec![original.clone()],
                vec![residual],
                Level::Minor,
            )
        });
        match outcome {
            Tri::Zero => {}
            Tri::Unknown(_) => exact = false,
            Tri::NonZero => {
                return Ok(PolynomialRoots {
                    set: SolutionSet::Unevaluated,
                    assumptions,
                    messages,
                });
            }
        }
    }
    let mut numeric = vec![];
    let mut all_numeric = true;
    for (var, value) in &rules {
        let ball = om_simplify::numeval::enclose(value, 256, ctx)?;
        let domain = branch
            .domains
            .iter()
            .find(|(v, _)| v == var)
            .map_or(opts.domain, |(_, d)| *d);
        if domain == Domain::Reals {
            if ball.as_ref().is_some_and(|z| z.im.excludes_zero()) {
                return Ok(PolynomialRoots {
                    set: SolutionSet::Finite(vec![]),
                    assumptions,
                    messages,
                });
            }
            if !ball.as_ref().is_some_and(|z| {
                z.im.mid == om_num::BigFloat::ZERO && z.im.rad == om_num::BigFloat::ZERO
            }) {
                conditions.push(Expr::call(B::ELEMENT, [value.clone(), Expr::sym(B::REALS)]))
            }
        }
        if let Some(z) = ball {
            let value = (z.re.to_f64(), z.im.to_f64());
            if value.0.is_finite() && value.1.is_finite() {
                numeric.push(value)
            } else {
                all_numeric = false
            }
        } else {
            all_numeric = false
        }
    }
    let set = if rules.is_empty() && conditions.is_empty() {
        SolutionSet::All
    } else {
        SolutionSet::Finite(vec![Solution {
            rules,
            condition: conjunction(&conditions),
            constants: vec![],
            multiplicity: 1,
            verification: if exact {
                Verification::Exact
            } else {
                Verification::ByConstruction
            },
            numeric: all_numeric.then_some(numeric),
        }])
    };
    Ok(PolynomialRoots {
        set,
        assumptions,
        messages,
    })
}
