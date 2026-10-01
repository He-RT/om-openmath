//! P0/P1 preparation; the dispatcher owns subsequent solution and union semantics.
mod boolean;
mod exclusions;
mod variables;
use crate::{Domain, ExclReason, Level, SolveError, Step, StepKind, StepSink};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel, Symbol};
use om_num::ctx::Interrupt;
use om_simplify::convert::{canonicalize_with, from_mpoly_with, to_rational_function_with};

/// Nonzero restriction from original syntax or explicit Unequal input.
#[derive(Clone, Debug)]
pub struct Exclusion {
    /// Expression that must be nonzero at a solution.
    pub value: Expr,
    /// Reason this restriction is required.
    pub reason: ExclReason,
}
/// One conjunction ready for solving, with original verification data.
#[derive(Clone, Debug)]
pub struct NormalizedBranch {
    /// Raw original residuals, retaining uncanceled syntax for verification.
    pub original: Vec<Expr>,
    /// Common-denominator numerator equations, without polynomial GCD cancellation.
    pub equations: Vec<Expr>,
    /// Original domain exclusions and explicit nonzero conditions.
    pub exclusions: Vec<Exclusion>,
    /// Domains in solving-variable order.
    pub domains: Vec<(Expr, Domain)>,
    /// Ordered one-variable real inequalities for the Reduce path.
    pub inequalities: Vec<Expr>,
    /// Conditions on parameters not represented by solving-variable domains.
    pub conditions: Vec<Expr>,
}
/// Prepared disjunction; an empty branch list means False unless unsupported is true.
#[derive(Clone, Debug)]
pub struct Normalized {
    /// Requested or inferred variables in their deterministic priority order.
    pub vars: Vec<Expr>,
    /// Source-order conjunction branches; union/deduplication happens after solving.
    pub branches: Vec<NormalizedBranch>,
    /// Diagnostics produced during preparation.
    pub messages: Vec<Message>,
    /// Preparation cannot be completed within supported semantics/resources.
    pub unsupported: bool,
}
/// Normalize original Solve input before arithmetic can erase its exclusions.
/// None infers axes; Some preserves explicit axes, including an empty list.
pub fn normalize(
    input: &Expr,
    vars: Option<&[Expr]>,
    domain: Domain,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Normalized, SolveError> {
    ctx.tick()?;
    let axes = if let Some(v) = vars {
        variables::requested(v, ctx)?
    } else {
        variables::infer(input, ctx)?
    };
    let mut result = Normalized {
        vars: axes,
        branches: vec![],
        messages: vec![],
        unsupported: false,
    };
    let Some(branches) = boolean::branches(input, ctx)? else {
        diagnostic(&mut result, "nsmet", "Too many logical branches", sink);
        result.unsupported = true;
        return Ok(result);
    };
    if vars.is_none() {
        let mut count = 0;
        for branch in &branches {
            let mut n = 0;
            for e in branch {
                ctx.tick()?;
                if e.is_head(B::EQUAL) || inequality(e.head_symbol()) {
                    n += e.args().len().saturating_sub(1);
                }
            }
            count = count.max(n);
        }
        if count > 0 && result.vars.len() > count {
            diagnostic(
                &mut result,
                "svars",
                "More variables than equations; solving the first variables in name order",
                sink,
            );
            result.vars.truncate(count);
        }
    }
    let branched = branches.len() > 1;
    for leaves in branches {
        ctx.tick()?;
        if branched && sink.enabled() {
            sink.enter("normalization_branch");
        }
        let prepared = prepare(&leaves, &result.vars, domain, ctx, sink);
        if branched && sink.enabled() {
            sink.exit();
        }
        match prepared? {
            Prepared::Branch(b) => result.branches.push(b),
            Prepared::False => {}
            Prepared::Unsupported(tag) => {
                diagnostic(
                    &mut result,
                    tag,
                    "Unsupported normalization condition",
                    sink,
                );
                result.unsupported = true;
            }
        }
    }
    Ok(result)
}
enum Prepared {
    Branch(NormalizedBranch),
    False,
    Unsupported(&'static str),
}
fn prepare(
    leaves: &[Expr],
    vars: &[Expr],
    domain: Domain,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Prepared, SolveError> {
    let mut b = NormalizedBranch {
        original: vec![],
        equations: vec![],
        exclusions: vec![],
        domains: vars.iter().map(|v| (v.clone(), domain)).collect(),
        inequalities: vec![],
        conditions: vec![],
    };
    // Keep the entire raw restriction list before the first structural rewrite.
    let mut raw_restrictions = vec![];
    for e in leaves {
        ctx.tick()?;
        raw_restrictions.extend(exclusions::collect(e, ctx)?);
    }
    for (value, reason) in raw_restrictions {
        if !exclude(&mut b, value, reason, ctx, sink)? {
            return Ok(Prepared::False);
        }
    }
    for e in leaves {
        ctx.tick()?;
        if e.is_head(B::EQUAL) {
            for pair in e.args().windows(2) {
                ctx.tick()?;
                b.original.push(residual(&pair[0], &pair[1]));
            }
        } else if e.is_head(B::UNEQUAL) {
            for i in 0..e.args().len() {
                for j in i + 1..e.args().len() {
                    ctx.tick()?;
                    if !exclude(
                        &mut b,
                        residual(&e.args()[i], &e.args()[j]),
                        ExclReason::BranchRestriction,
                        ctx,
                        sink,
                    )? {
                        return Ok(Prepared::False);
                    }
                }
            }
        } else if e.is_head(B::ELEMENT) && e.args().len() == 2 {
            let Some(d) = variables::domain(&e.args()[1]) else {
                return Ok(Prepared::Unsupported("bdom"));
            };
            let group = &e.args()[0];
            let members = if group.is_head(B::LIST)
                || group
                    .head_symbol()
                    .is_some_and(|h| h.name() == "Alternatives")
            {
                group.args().to_vec()
            } else {
                vec![group.clone()]
            };
            for member in members {
                ctx.tick()?;
                let member = canonicalize_with(&member, ctx)?;
                if let Some((_, current)) = b.domains.iter_mut().find(|(v, _)| v == &member) {
                    *current = variables::meet(*current, d);
                } else {
                    b.conditions
                        .push(Expr::call(B::ELEMENT, [member, e.args()[1].clone()]));
                }
            }
        } else if inequality(e.head_symbol()) {
            if vars.len() != 1 {
                return Ok(Prepared::Unsupported("ineq"));
            }
            b.domains[0].1 = variables::meet(b.domains[0].1, Domain::Reals);
            for pair in e.args().windows(2) {
                ctx.tick()?;
                b.inequalities
                    .push(Expr::normal(e.head(), pair.iter().cloned()));
            }
        } else if e.is_head(B::INEQUALITY) && e.args().len() >= 3 && e.args().len() % 2 == 1 {
            if vars.len() != 1 {
                return Ok(Prepared::Unsupported("ineq"));
            }
            b.domains[0].1 = variables::meet(b.domains[0].1, Domain::Reals);
            for i in (1..e.args().len()).step_by(2) {
                ctx.tick()?;
                if !inequality(e.args()[i].as_symbol()) {
                    return Err(SolveError::Invalid("invalid inequality operator".into()));
                }
                b.inequalities.push(Expr::normal(
                    e.args()[i].clone(),
                    [e.args()[i - 1].clone(), e.args()[i + 1].clone()],
                ));
            }
        } else {
            return Err(SolveError::Invalid(
                "expected equations or Boolean conditions".into(),
            ));
        }
    }
    let mut normalized = vec![];
    for original in &b.original {
        ctx.tick()?;
        normalized.push(canonicalize_with(original, ctx)?);
    }
    sink.record(|| {
        Step::new(
            StepKind::Normalize,
            leaves.to_vec(),
            normalized.clone(),
            Level::Major,
        )
    });
    for e in normalized {
        ctx.tick()?;
        let Some(view) = to_rational_function_with(&e, &[], ctx)? else {
            return Err(SolveError::Unsupported(
                "equation degree cannot be represented".into(),
            ));
        };
        let Some(num) = from_mpoly_with(&view.num, &view.gens, ctx)? else {
            return Err(SolveError::Unsupported("invalid equation numerator".into()));
        };
        if !view.den.is_one() {
            let Some(den) = from_mpoly_with(&view.den, &view.gens, ctx)? else {
                return Err(SolveError::Unsupported(
                    "invalid equation denominator".into(),
                ));
            };
            sink.record(|| {
                Step::new(
                    StepKind::ClearDenominators { factor: den },
                    vec![e],
                    vec![num.clone()],
                    Level::Major,
                )
            });
        }
        if !num.is_zero() {
            b.equations.push(num);
        }
    }
    Ok(Prepared::Branch(b))
}
fn residual(a: &Expr, b: &Expr) -> Expr {
    Expr::call(
        B::PLUS,
        [a.clone(), Expr::call(B::TIMES, [Expr::int(-1), b.clone()])],
    )
}
fn exclude(
    b: &mut NormalizedBranch,
    value: Expr,
    reason: ExclReason,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<bool, SolveError> {
    let value = canonicalize_with(&value, ctx)?;
    if value.is_zero() {
        sink.record(|| {
            Step::new(
                StepKind::RecordExclusion {
                    cond: Expr::call(B::UNEQUAL, [value.clone(), Expr::int(0)]),
                    reason,
                },
                vec![value.clone()],
                vec![],
                Level::Minor,
            )
        });
        return Ok(false);
    }
    if value.as_number().is_some() {
        return Ok(true);
    }
    for previous in &b.exclusions {
        ctx.tick()?;
        if previous.value == value {
            return Ok(true);
        }
    }
    sink.record(|| {
        Step::new(
            StepKind::RecordExclusion {
                cond: Expr::call(B::UNEQUAL, [value.clone(), Expr::int(0)]),
                reason,
            },
            vec![value.clone()],
            vec![],
            Level::Minor,
        )
    });
    b.exclusions.push(Exclusion { value, reason });
    Ok(true)
}
fn inequality(head: Option<Symbol>) -> bool {
    matches!(
        head,
        Some(B::LESS | B::LESS_EQUAL | B::GREATER | B::GREATER_EQUAL)
    )
}
fn diagnostic(result: &mut Normalized, tag: &str, text: &str, sink: &mut impl StepSink) {
    let msg = Message {
        symbol: "Solve".into(),
        tag: tag.into(),
        text: text.into(),
        level: MsgLevel::Warning,
    };
    sink.record(|| {
        Step::new(
            StepKind::Note { msg: msg.clone() },
            vec![],
            vec![],
            Level::Minor,
        )
    });
    result.messages.push(msg);
}
