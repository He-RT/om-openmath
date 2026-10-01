//! P3 preserves trusted construction evidence without relaxing principal checks.
use crate::{
    Domain, Level, MaxExtra, Solution, SolutionSet, SolveError, SolveOptions, Step, StepKind,
    StepSink, Verification, domain,
    normalize::NormalizedBranch,
    univariate::{PolynomialRoots, transcendental::check, verification_zero},
};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel, sub};
use om_num::ctx::Interrupt;
use om_simplify::zero::Tri;
#[derive(Clone, Copy)]
pub(crate) enum Origin {
    Constructed,
    BranchSensitive,
}
pub(crate) fn empty() -> Solution {
    Solution {
        rules: vec![],
        condition: None,
        constants: vec![],
        multiplicity: 1,
        verification: Verification::Exact,
        numeric: None,
    }
}
pub(crate) fn simplify_condition(c: &Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    ctx.tick()?;
    if c.is_head(B::AND) || c.is_head(B::OR) {
        let all = c.is_head(B::AND);
        let mut conditions = vec![];
        let absorbing = if all { B::FALSE } else { B::TRUE };
        let neutral = if all { B::TRUE } else { B::FALSE };
        for arg in c.args() {
            let v = simplify_condition(arg, ctx)?;
            if v.as_symbol() == Some(absorbing) {
                return Ok(v);
            }
            if v.as_symbol() == Some(neutral) {
                continue;
            }
            if v.head() == c.head() {
                conditions.extend_from_slice(v.args())
            } else {
                conditions.push(v)
            }
        }
        conditions.sort_by(om_core::canonical_cmp);
        conditions.dedup();
        return Ok(match conditions.len() {
            0 => Expr::sym(neutral),
            1 => conditions.remove(0),
            _ => Expr::normal(c.head(), conditions),
        });
    }
    Ok(match check::allows(c, ctx)? {
        Some(b) => Expr::sym(if b { B::TRUE } else { B::FALSE }),
        None => c.clone(),
    })
}
fn conditional_zero(r: &Expr, c: Option<&Expr>, ctx: &Interrupt) -> Result<bool, SolveError> {
    let mut stack = c.into_iter().collect::<Vec<_>>();
    while let Some(c) = stack.pop() {
        ctx.tick()?;
        if c.is_head(B::AND) {
            stack.extend(c.args());
            continue;
        }
        if c.is_head(B::EQUAL) && c.args().len() == 2 {
            let d = sub(c.args()[0].clone(), c.args()[1].clone());
            if verification_zero(&sub(r.clone(), d.clone()), ctx)? == Tri::Zero {
                return Ok(true);
            }
            if let Some(q) =
                om_simplify::algebra::cancel_with(&om_core::div(r.clone(), d), &[], ctx)?
                && q.as_number().is_some_and(|n| n.is_exact())
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn symbolic(e: &Expr, root: &Solution) -> bool {
    !e.free_symbols().is_empty() || root.constants.iter().any(|(c, _)| !e.free_of(c))
}
fn constructed(
    branch: &NormalizedBranch,
    guards: &[Expr],
    roots: &mut Vec<Solution>,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<(), SolveError> {
    let mut kept = vec![];
    for mut root in roots.drain(..) {
        let mut reject = false;
        let mut exact = true;
        for exclusion in &branch.exclusions {
            let value = exclusion.value.replace_all(&root.rules);
            match verification_zero(&value, ctx)? {
                Tri::Zero => {
                    reject = true;
                    break;
                }
                Tri::NonZero | Tri::Unknown(_) if symbolic(&value, &root) => {
                    root.condition = domain::condition(
                        root.condition.take(),
                        Expr::call(B::UNEQUAL, [value, Expr::int(0)]),
                    )
                }
                _ => {
                    if !check::nonzero(&value, ctx)? {
                        return Err(SolveError::Unsupported(
                            "source exclusion lacks a nonzero certificate".into(),
                        ));
                    }
                }
            }
        }
        for original in &branch.original {
            if reject {
                break;
            }
            let r = check::residual(&original.replace_all(&root.rules), ctx)?;
            let outcome = verification_zero(&r, ctx)?;
            sink.record(|| {
                Step::new(
                    StepKind::Verify {
                        candidate: root.rules.clone(),
                        outcome,
                        residual: Some(r.clone()),
                    },
                    vec![original.clone()],
                    vec![r.clone()],
                    Level::Minor,
                )
            });
            match outcome {
                Tri::Zero => {}
                Tri::Unknown(_) => exact = false,
                Tri::NonZero => {
                    if conditional_zero(&r, root.condition.as_ref(), ctx)? {
                        exact = false
                    } else {
                        reject = true
                    }
                }
            }
        }
        for guard in guards {
            if check::allows(&guard.replace_all(&root.rules), ctx)? == Some(false) {
                reject = true
            }
        }
        if reject {
            sink.record(|| {
                Step::new(
                    StepKind::DropExtraneous {
                        candidate: root.rules.clone(),
                        why: "original_equation_or_exclusion".into(),
                    },
                    branch.original.clone(),
                    vec![],
                    Level::Major,
                )
            });
        } else {
            root.verification = if exact {
                Verification::Exact
            } else {
                Verification::ByConstruction
            };
            kept.push(root)
        }
    }
    *roots = kept;
    Ok(())
}
pub(crate) fn apply(
    branch: &NormalizedBranch,
    mut result: PolynomialRoots,
    origin: Origin,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    let all = matches!(result.set, SolutionSet::All);
    let mut roots = match result.set {
        SolutionSet::All => vec![empty()],
        SolutionSet::Finite(r) => r,
        _ => return Ok(result),
    };
    for root in &mut roots {
        root.rules
            .sort_by_key(|(v, _)| branch.domains.iter().position(|(a, _)| a == v));
        for c in &branch.conditions {
            root.condition = domain::condition(root.condition.take(), c.replace_all(&root.rules))
        }
        for (v, d) in &branch.domains {
            if *d != Domain::Reals {
                continue;
            }
            if let Some((_, value)) = root.rules.iter().find(|(a, _)| a == v) {
                let member = Expr::call(B::ELEMENT, [value.clone(), Expr::sym(B::REALS)]);
                root.condition = domain::condition(root.condition.take(), member);
            } else if !all || *d != opts.domain {
                root.condition = domain::condition(
                    root.condition.take(),
                    Expr::call(B::ELEMENT, [v.clone(), Expr::sym(B::REALS)]),
                )
            }
        }
    }
    match origin {
        Origin::Constructed => constructed(branch, &result.assumptions, &mut roots, ctx, sink)?,
        Origin::BranchSensitive => {
            let identity = Expr::int(0);
            let mut numeric = vec![false; roots.len()];
            // Check one candidate independently so provenance survives each residual.
            let mut kept = vec![];
            for (i, root) in roots.drain(..).enumerate() {
                let mut root = root;
                for original in &branch.original {
                    let r = check::residual(&original.replace_all(&root.rules), ctx)?;
                    if r.free_symbols()
                        .iter()
                        .any(|s| *s != opts.generated_parameter)
                        && verification_zero(&r, ctx)? != Tri::Zero
                    {
                        root.condition = domain::condition(
                            root.condition.take(),
                            Expr::call(B::EQUAL, [r, Expr::int(0)]),
                        );
                    }
                }
                let mut candidate = vec![root];
                for original in branch.original.iter().chain(std::iter::once(&identity)) {
                    check::candidates(
                        check::Source {
                            original,
                            exclusions: &branch.exclusions,
                            guards: &result.assumptions,
                        },
                        &mut candidate,
                        opts,
                        ctx,
                        sink,
                        &mut result.messages,
                    )?;
                    if candidate.is_empty() {
                        break;
                    }
                    numeric[i] |= matches!(candidate[0].verification, Verification::Numeric { .. });
                }
                if let Some(mut root) = candidate.pop() {
                    if numeric[i] {
                        root.verification = Verification::Numeric { digits: 33 }
                    };
                    kept.push(root)
                }
            }
            roots = kept;
        }
    }
    let expose = match opts.max_extra_conditions {
        MaxExtra::Zero => false,
        MaxExtra::All => true,
        MaxExtra::Count(n) => result.assumptions.len() <= n as usize,
    };
    let mut kept = vec![];
    for mut root in roots {
        if all && root.condition.is_some() {
            for (v, d) in &branch.domains {
                if *d == Domain::Reals {
                    root.condition = domain::condition(
                        root.condition.take(),
                        Expr::call(B::ELEMENT, [v.clone(), Expr::sym(B::REALS)]),
                    )
                }
            }
        }
        if expose {
            for guard in &result.assumptions {
                root.condition =
                    domain::condition(root.condition.take(), guard.replace_all(&root.rules))
            }
        }
        if let Some(c) = root.condition.take() {
            let c = simplify_condition(&c, ctx)?;
            if c.as_symbol() == Some(B::FALSE) {
                continue;
            }
            if c.as_symbol() != Some(B::TRUE) {
                root.condition = Some(c)
            }
        }
        let mut values = vec![];
        let period_zero = root
            .constants
            .iter()
            .map(|(c, _)| (c.clone(), Expr::int(0)))
            .collect::<Vec<_>>();
        for (_, v) in &root.rules {
            let Some(z) = om_simplify::numeval::enclose(&v.replace_all(&period_zero), 128, ctx)?
            else {
                values.clear();
                break;
            };
            let v = (z.re.to_f64(), z.im.to_f64());
            if !v.0.is_finite() || !v.1.is_finite() {
                values.clear();
                break;
            };
            values.push(v);
        }
        root.numeric = (values.len() == root.rules.len()).then_some(values);
        kept.push(root);
    }
    result.set = SolutionSet::Finite(kept);
    if all
        && branch.domains.iter().all(|(_, d)| *d == opts.domain)
        && let SolutionSet::Finite(r) = &result.set
        && r.len() == 1
        && r[0].rules.is_empty()
        && r[0].condition.is_none()
    {
        result.set = SolutionSet::All
    } else {
        result = domain::filter(result, &branch.domains, ctx, sink)?
    }
    Ok(result)
}
pub(crate) fn note(
    input: &Expr,
    tag: &str,
    text: &str,
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
    messages.push(msg);
}
