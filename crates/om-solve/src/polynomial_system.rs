//! Polynomial systems combine verified Groebner bases with complete algebraic shapes.
mod convert;
mod finite;
mod multiplicity;
mod positive;
use crate::{
    Domain, Level, NoSteps, SolutionSet, SolveError, SolveOptions, Step, StepKind, StepSink,
    Verification, normalize, univariate::PolynomialRoots,
};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel};
use om_num::ctx::Interrupt;
use om_poly::{IdealDimension, MonoOrder};
use om_simplify::zero::{Tri, is_zero_with};

/// Solve polynomial conjunctions using Groebner/FGLM and exact shape back substitution.
/// Finite component failure is atomic. Positive-dimensional unresolved relations
/// remain conditions on original free variables, with an explicit svars diagnostic.
pub fn poly_system(
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
fn message(
    tag: &str,
    text: &str,
    input: &Expr,
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
            vec![input.clone()],
            vec![],
            Level::Minor,
        )
    });
    messages.push(msg)
}
fn unsupported(
    input: &Expr,
    mut messages: Vec<Message>,
    sink: &mut impl StepSink,
) -> PolynomialRoots {
    message(
        "nsmet",
        "This polynomial system has unsupported kernels or no complete algebraic shape.",
        input,
        &mut messages,
        sink,
    );
    PolynomialRoots {
        set: SolutionSet::Unevaluated,
        assumptions: vec![],
        messages,
    }
}
fn condition(existing: Option<Expr>, next: Expr) -> Option<Expr> {
    Some(if let Some(c) = existing {
        if c == next {
            c
        } else {
            Expr::call(B::AND, [c, next])
        }
    } else {
        next
    })
}
fn run(
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    ctx.tick()?;
    if let Some(result) = crate::domain::affine(eqs, vars, opts, ctx, sink)? {
        return Ok(result);
    }
    let prepared = normalize::normalize(eqs, Some(vars), opts.domain, ctx, sink)?;
    let mut messages = prepared.messages;
    if prepared.unsupported || prepared.branches.len() > 1 {
        return Ok(unsupported(eqs, messages, sink));
    }
    let Some(branch) = prepared.branches.first() else {
        return Ok(PolynomialRoots {
            set: SolutionSet::Finite(vec![]),
            assumptions: vec![],
            messages,
        });
    };
    if !branch.inequalities.is_empty() {
        return Ok(unsupported(eqs, messages, sink));
    }
    let Some((polys, axes)) = convert::build(&branch.equations, vars, ctx)? else {
        return Ok(unsupported(eqs, messages, sink));
    };
    if polys.is_empty() {
        return crate::linear_system(eqs, vars, opts, ctx, sink);
    }
    let Some(grevlex) = convert::basis(&polys, &axes, MonoOrder::GrevLex, ctx, sink)? else {
        return Ok(unsupported(eqs, messages, sink));
    };
    let Some(dimension) = om_poly::ideal_dimension(&grevlex, axes.len(), ctx)? else {
        return Ok(unsupported(eqs, messages, sink));
    };
    if dimension == IdealDimension::Empty {
        return Ok(PolynomialRoots {
            set: SolutionSet::Finite(vec![]),
            assumptions: vec![],
            messages,
        });
    }
    let IdealDimension::Dimension(d) = dimension else {
        unreachable!("invariant: nonempty ideal dimension")
    };
    let lex = if d == 0 {
        let Some(lex) = om_poly::fglm(&grevlex, ctx)? else {
            return Ok(unsupported(eqs, messages, sink));
        };
        convert::record_basis(&grevlex, &lex, &axes, MonoOrder::Lex, ctx, sink)?;
        lex
    } else {
        let Some(lex) = convert::basis(&polys, &axes, MonoOrder::Lex, ctx, sink)? else {
            return Ok(unsupported(eqs, messages, sink));
        };
        lex
    };
    let mut assumptions = vec![];
    let roots = if d == 0 {
        finite::solve(&lex, &axes, opts, ctx, sink)?
    } else {
        message(
            "svars",
            "The polynomial system leaves independent variables or unresolved relations.",
            eqs,
            &mut messages,
            sink,
        );
        positive::solve(&lex, &axes, vars.len(), d, opts, ctx, sink)?.map(|solved| {
            assumptions = solved.assumptions;
            solved.roots
        })
    };
    let Some(roots) = roots else {
        return Ok(unsupported(eqs, messages, sink));
    };
    let mut kept = vec![];
    for mut root in roots {
        ctx.tick()?;
        let computed_rules = root.rules.clone();
        for (v, value) in &computed_rules {
            if !vars.contains(v) {
                root.condition = condition(
                    root.condition.take(),
                    Expr::call(B::EQUAL, [v.clone(), value.clone()]),
                );
            }
        }
        root.rules.retain(|(v, _)| vars.contains(v));
        for (v, domain) in &branch.domains {
            if *domain == Domain::Reals && !root.rules.iter().any(|(var, _)| var == v) {
                root.condition = condition(
                    root.condition.take(),
                    Expr::call(B::ELEMENT, [v.clone(), Expr::sym(B::REALS)]),
                );
            }
        }
        root.rules.sort_by_key(|(v, _)| {
            vars.iter()
                .position(|a| a == v)
                .expect("invariant: requested rule order")
        });
        let mut drop = false;
        let mut exact = true;
        for exclusion in &branch.exclusions {
            let value = exclusion.value.replace_all(&computed_rules);
            match is_zero_with(&value, ctx)? {
                Tri::Zero => {
                    drop = true;
                    break;
                }
                Tri::NonZero => {
                    if !value.free_symbols().is_empty() {
                        root.condition = condition(
                            root.condition.take(),
                            Expr::call(B::UNEQUAL, [value, Expr::int(0)]),
                        )
                    }
                }
                Tri::Unknown(_) => {
                    if value.free_symbols().is_empty() {
                        if let Some(z) = om_simplify::numeval::enclose(&value, 512, ctx)? {
                            if !z.re.excludes_zero() && !z.im.excludes_zero() {
                                return Ok(unsupported(eqs, messages, sink));
                            }
                        } else {
                            return Ok(unsupported(eqs, messages, sink));
                        }
                    } else {
                        root.condition = condition(
                            root.condition.take(),
                            Expr::call(B::UNEQUAL, [value, Expr::int(0)]),
                        )
                    }
                }
            }
        }
        if drop {
            sink.record(|| {
                Step::new(
                    StepKind::DropExtraneous {
                        candidate: root.rules.clone(),
                        why: "original_exclusion".into(),
                    },
                    vec![eqs.clone()],
                    vec![],
                    Level::Major,
                )
            });
            continue;
        }
        for original in &branch.original {
            let residual = original.replace_all(&computed_rules);
            let outcome = is_zero_with(&residual, ctx)?;
            sink.record(|| {
                Step::new(
                    StepKind::Verify {
                        candidate: root.rules.clone(),
                        outcome,
                        residual: Some(residual.clone()),
                    },
                    vec![original.clone()],
                    vec![residual.clone()],
                    Level::Minor,
                )
            });
            match outcome {
                Tri::Zero => {}
                Tri::Unknown(_) => exact = false,
                Tri::NonZero => {
                    if d > 0 {
                        root.condition = condition(
                            root.condition.take(),
                            Expr::call(B::EQUAL, [residual, Expr::int(0)]),
                        );
                        exact = false
                    } else {
                        return Ok(unsupported(eqs, messages, sink));
                    }
                }
            }
        }
        for c in &branch.conditions {
            root.condition = condition(root.condition.take(), c.replace_all(&root.rules))
        }
        let mut numeric = vec![];
        let mut all_numeric = true;
        for (var, value) in &root.rules {
            let domain = branch
                .domains
                .iter()
                .find(|(v, _)| v == var)
                .map_or(opts.domain, |(_, d)| *d);
            let ball = om_simplify::numeval::enclose(value, 512, ctx)?;
            if domain == Domain::Reals {
                if ball.as_ref().is_some_and(|z| z.im.excludes_zero()) {
                    drop = true;
                    break;
                }
                if !ball.as_ref().is_some_and(|z| {
                    z.im.mid == om_num::BigFloat::ZERO && z.im.rad == om_num::BigFloat::ZERO
                }) {
                    root.condition = condition(
                        root.condition.take(),
                        Expr::call(B::ELEMENT, [value.clone(), Expr::sym(B::REALS)]),
                    )
                }
            }
            if let Some(z) = ball {
                let v = (z.re.to_f64(), z.im.to_f64());
                if v.0.is_finite() && v.1.is_finite() {
                    numeric.push(v)
                } else {
                    all_numeric = false
                }
            } else {
                all_numeric = false
            }
        }
        if drop {
            continue;
        }
        root.numeric = all_numeric.then_some(numeric);
        root.verification = if exact {
            Verification::Exact
        } else {
            Verification::ByConstruction
        };
        if !kept
            .iter()
            .any(|r: &crate::Solution| r.rules == root.rules && r.condition == root.condition)
        {
            kept.push(root)
        }
    }
    let mut coordinates = kept
        .iter()
        .map(|root| {
            root.rules
                .iter()
                .map(|(_, v)| crate::univariate::order::coordinate(v))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for i in 1..kept.len() {
        let mut j = i;
        while j > 0 {
            ctx.tick()?;
            let mut ordering = std::cmp::Ordering::Equal;
            for (a, b) in coordinates[j - 1].iter().zip(&coordinates[j]) {
                ordering = crate::univariate::order::compare_coordinates(a, b, ctx)?;
                if ordering != std::cmp::Ordering::Equal {
                    break;
                }
            }
            if ordering != std::cmp::Ordering::Greater {
                break;
            }
            kept.swap(j - 1, j);
            coordinates.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(PolynomialRoots {
        set: SolutionSet::Finite(kept),
        assumptions,
        messages,
    })
}
