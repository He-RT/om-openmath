//! Reduce-lite preserves raw restrictions before creating an exact rational chart.
mod chart;
mod model;
use crate::{
    Bound, Domain, Interval, NoSteps, SolutionSet, SolveError, SolveOptions, SolveOutcome,
    StepRecorder, StepSink, normalize,
};
use om_core::{BUILTIN as B, Expr, ExprKind, Message, MsgLevel};
use om_num::ctx::Interrupt;

pub(crate) fn has(e: &Expr, ctx: &Interrupt) -> Result<bool, SolveError> {
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if matches!(
            e.head_symbol(),
            Some(B::LESS | B::LESS_EQUAL | B::GREATER | B::GREATER_EQUAL | B::INEQUALITY)
        ) {
            return Ok(true);
        }
        if let ExprKind::Normal(n) = e.kind() {
            stack.push(&n.head);
            stack.extend(&n.args);
        }
    }
    Ok(false)
}
/// Reduce equations to Boolean relations, or one-variable exact rational inequalities to intervals.
/// Unsupported charts return Unevaluated; invalid inputs and injected interruption propagate.
pub fn reduce(
    expr: &Expr,
    vars: &[Expr],
    domain: Domain,
    ctx: &Interrupt,
) -> Result<SolveOutcome, SolveError> {
    let opts = SolveOptions {
        domain,
        ..SolveOptions::default()
    };
    reduce_with_options(expr, vars, &opts, ctx)
}
/// Reduction with construction options and computation-time recording control.
pub fn reduce_with_options(
    expr: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
) -> Result<SolveOutcome, SolveError> {
    if !has(expr, ctx)? {
        let mut result = crate::solve(expr, vars, opts, ctx)?;
        result.set = equations(result.set, vars, ctx)?;
        for m in &mut result.messages {
            m.symbol = "Reduce".into();
        }
        return Ok(result);
    }
    reduce_with(expr, vars, opts, ctx, "Reduce")
}
fn equations(set: SolutionSet, vars: &[Expr], ctx: &Interrupt) -> Result<SolutionSet, SolveError> {
    let mut intervals = vec![];
    let cond = match set {
        SolutionSet::All => Expr::sym(B::TRUE),
        SolutionSet::Finite(roots) => {
            let mut branches = vec![];
            for root in roots {
                ctx.tick()?;
                if vars.len() == 1
                    && root.rules.len() == 1
                    && root.constants.is_empty()
                    && root.condition.is_none()
                    && matches!(
                        crate::univariate::order::algebraic(&root.rules[0].1, ctx)?,
                        Some(om_poly::Algebraic::Rational(_) | om_poly::Algebraic::Real(_))
                    )
                {
                    let value = root.rules[0].1.clone();
                    intervals.push(Interval {
                        lo: Bound::Closed(value.clone()),
                        hi: Bound::Closed(value),
                    });
                }
                let mut conditions = root
                    .rules
                    .into_iter()
                    .map(|(v, a)| Expr::call(B::EQUAL, [v, a]))
                    .collect::<Vec<_>>();
                conditions.extend(root.condition);
                conditions.extend(root.constants.into_iter().map(|(c, d)| {
                    Expr::call(
                        B::ELEMENT,
                        [
                            c,
                            Expr::sym(match d {
                                Domain::Integers => B::INTEGERS,
                                Domain::Rationals => B::RATIONALS,
                                Domain::Reals => B::REALS,
                                Domain::Complexes => B::COMPLEXES,
                            }),
                        ],
                    )
                }));
                let branch = match conditions.len() {
                    0 => Expr::sym(B::TRUE),
                    1 => conditions.remove(0),
                    _ => Expr::call(B::AND, conditions),
                };
                if branch.as_symbol() == Some(B::TRUE) {
                    return Ok(SolutionSet::Region {
                        cond: branch,
                        intervals: vec![],
                    });
                }
                if !branches.contains(&branch) {
                    branches.push(branch);
                }
            }
            match branches.len() {
                0 => Expr::sym(B::FALSE),
                1 => branches.remove(0),
                _ => Expr::call(B::OR, branches),
            }
        }
        _ => return Ok(set),
    };
    Ok(SolutionSet::Region { cond, intervals })
}
pub(crate) fn reduce_with(
    expr: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    symbol: &str,
) -> Result<SolveOutcome, SolveError> {
    if opts.record_steps {
        let mut sink = StepRecorder::new();
        let (set, messages) = run(expr, vars, opts, ctx, symbol, &mut sink)?;
        Ok(SolveOutcome {
            set,
            messages,
            steps: Some(sink.finish()),
        })
    } else {
        let (set, messages) = run(expr, vars, opts, ctx, symbol, &mut NoSteps)?;
        Ok(SolveOutcome {
            set,
            messages,
            steps: None,
        })
    }
}
fn run(
    expr: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    symbol: &str,
    sink: &mut impl StepSink,
) -> Result<(SolutionSet, Vec<Message>), SolveError> {
    let mut messages = vec![];
    let result = (|| {
        let prepared = normalize::normalize(expr, Some(vars), opts.domain, ctx, sink)?;
        messages = prepared.messages;
        if prepared.unsupported
            || prepared.vars.len() != 1
            || matches!(opts.domain, Domain::Integers | Domain::Rationals)
        {
            return Err(SolveError::Unsupported(
                "only one-variable real rational inequalities are supported".into(),
            ));
        }
        let x = &prepared.vars[0];
        if complex_alternatives(&prepared.branches, x, opts, ctx)? {
            return Ok(if symbol == "Solve" {
                SolutionSet::All
            } else {
                SolutionSet::Region {
                    cond: Expr::sym(B::TRUE),
                    intervals: vec![],
                }
            });
        }
        let mut branches = vec![];
        for branch in prepared.branches {
            if matches!(branch.domains[0].1, Domain::Integers | Domain::Rationals) {
                return Err(SolveError::Unsupported(
                    "discrete inequality domains are unsupported".into(),
                ));
            }
            let mut constraints = vec![];
            let mut allowed = true;
            for cond in &branch.conditions {
                match crate::univariate::transcendental::check::allows(cond, ctx)? {
                    Some(true) => {}
                    Some(false) => allowed = false,
                    None => {
                        return Err(SolveError::Unsupported(
                            "parameter inequality conditions are unsupported".into(),
                        ));
                    }
                }
            }
            if !allowed {
                continue;
            }
            for e in &branch.original {
                constraints.push(model::constraint(e, B::EQUAL, x, ctx)?);
            }
            for e in &branch.exclusions {
                constraints.push(model::constraint(&e.value, B::UNEQUAL, x, ctx)?);
            }
            for e in &branch.inequalities {
                let residual = om_core::sub(e.args()[0].clone(), e.args()[1].clone());
                constraints.push(model::constraint(
                    &residual,
                    e.head_symbol()
                        .expect("invariant: normalized inequality has a relation head"),
                    x,
                    ctx,
                )?);
            }
            branches.push(constraints);
        }
        let points = model::critical(&branches, x, ctx)?;
        let intervals = chart::intervals(&branches, &points, ctx, sink)?;
        let cond = chart::boolean(&intervals, x);
        Ok(SolutionSet::Region { cond, intervals })
    })();
    for message in &mut messages {
        message.symbol = symbol.into();
    }
    match result {
        Err(SolveError::Unsupported(reason)) => {
            let msg = Message {
                symbol: symbol.into(),
                tag: "nsmet".into(),
                text: reason,
                level: MsgLevel::Warning,
            };
            sink.record(|| {
                crate::Step::new(
                    crate::StepKind::Note { msg: msg.clone() },
                    vec![expr.clone()],
                    vec![],
                    crate::Level::Minor,
                )
            });
            messages.push(msg);
            Ok((SolutionSet::Unevaluated, messages))
        }
        result => Ok((result?, messages)),
    }
}

// Examine every equation alternative before declining, since All absorbs the union.
fn complex_alternatives(
    branches: &[normalize::NormalizedBranch],
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
) -> Result<bool, SolveError> {
    let mut unsupported = false;
    let inner = SolveOptions {
        record_steps: false,
        ..opts.clone()
    };
    for branch in branches {
        if !branch.inequalities.is_empty() || branch.domains[0].1 != Domain::Complexes {
            continue;
        }
        let result = crate::solve(
            &crate::dispatch::input(branch),
            std::slice::from_ref(x),
            &inner,
            ctx,
        )?;
        match result.set {
            SolutionSet::All => return Ok(true),
            SolutionSet::Finite(roots) => {
                for root in roots {
                    ctx.tick()?;
                    if root.rules.len() != 1
                        || root.condition.is_some()
                        || !root.constants.is_empty()
                        || !matches!(
                            crate::univariate::order::algebraic(&root.rules[0].1, ctx)?,
                            Some(om_poly::Algebraic::Rational(_) | om_poly::Algebraic::Real(_))
                        )
                    {
                        unsupported = true;
                    }
                }
            }
            _ => unsupported = true,
        }
    }
    if unsupported {
        return Err(SolveError::Unsupported(
            "a logical alternative cannot be represented by a complete real rational chart".into(),
        ));
    }
    Ok(false)
}
