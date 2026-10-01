//! Conditional families are sampled only within their original domains.
mod condition;
mod sampling;
#[cfg(test)]
mod tests;
use super::{
    super::{extract, radical::verify},
    inverse,
};
use crate::{
    Domain, Level, Solution, SolveError, SolveOptions, Step, StepKind, StepSink, Verification,
    normalize::Exclusion,
};
pub(crate) use condition::{allows, nonzero, periodic_nonzero, residual};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel};
use om_num::{BigFloat, ctx::Interrupt};
use om_simplify::{
    numeval::enclose,
    zero::{Tri, UnknownReason},
};
use sampling::{period_vectors, sample};
pub(crate) struct Source<'a> {
    pub original: &'a Expr,
    pub exclusions: &'a [Exclusion],
    pub guards: &'a [Expr],
}
pub(crate) fn candidates(
    source: Source<'_>,
    roots: &mut Vec<Solution>,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
    messages: &mut Vec<Message>,
) -> Result<(), SolveError> {
    let Source {
        original,
        exclusions,
        guards,
    } = source;
    let guards = if guards.is_empty() {
        None
    } else if guards.len() == 1 {
        Some(guards[0].clone())
    } else {
        Some(Expr::call(B::AND, guards.iter().cloned()))
    };
    let mut kept = vec![];
    for mut root in roots.drain(..) {
        let values = exclusions
            .iter()
            .map(|e| e.value.replace_all(&root.rules))
            .collect::<Vec<_>>();
        for v in &values {
            let mut conditional = !v.free_symbols().is_empty();
            for (c, _) in &root.constants {
                conditional |= extract::depends(v, c, ctx)?;
            }
            if conditional {
                root.condition = inverse::and(
                    root.condition.take(),
                    Some(Expr::call(B::UNEQUAL, [v.clone(), Expr::int(0)])),
                )
            }
        }
        let original_residual = original.replace_all(&root.rules);
        let mut accepted = true;
        let numeric = verify::zero(&residual(&original_residual, ctx)?, ctx)? != Tri::Zero;
        let mut tested = 0;
        for point in period_vectors(root.constants.len(), ctx)? {
            let rules = root
                .constants
                .iter()
                .zip(point)
                .map(|((c, _), k)| (c.clone(), Expr::int(k)))
                .collect::<Vec<_>>();
            let condition =
                inverse::and(root.condition.clone(), guards.clone()).map(|c| c.replace_all(&rules));
            if let Some(c) = &condition
                && allows(c, ctx)? == Some(false)
            {
                continue;
            }
            let sampled = residual(&original_residual.replace_all(&rules), ctx)?;
            let exclusions = values
                .iter()
                .map(|v| v.replace_all(&rules))
                .collect::<Vec<_>>();
            let outcome = sample(&sampled, &exclusions, condition.as_ref(), opts, ctx)?;
            sink.record(|| {
                Step::new(
                    StepKind::Verify {
                        candidate: root.rules.clone(),
                        outcome: match outcome {
                            Some(Verification::Exact) => Tri::Zero,
                            Some(_) => Tri::Unknown(UnknownReason::ProbablyZero),
                            None => Tri::Unknown(UnknownReason::NoInfo),
                        },
                        residual: Some(sampled.clone()),
                    },
                    vec![original.clone()],
                    vec![sampled],
                    Level::Minor,
                )
            });
            let Some(_) = outcome else {
                accepted = false;
                break;
            };
            tested += 1;
        }
        if accepted && tested > 0 {
            root.verification = if numeric {
                Verification::Numeric { digits: 33 }
            } else {
                Verification::Exact
            };
            kept.push(root)
        } else {
            sink.record(|| {
                Step::new(
                    StepKind::DropExtraneous {
                        candidate: root.rules.clone(),
                        why: "principal_branch_or_exclusion".into(),
                    },
                    vec![original_residual],
                    vec![],
                    Level::Major,
                )
            });
            messages.push(Message {
                symbol: "Solve".into(),
                tag: "verify".into(),
                text: "A transcendental candidate failed mandatory original-equation verification."
                    .into(),
                level: MsgLevel::Warning,
            });
        }
    }
    *roots = kept;
    Ok(())
}
pub(super) fn real_filter(
    roots: &mut Vec<Solution>,
    known: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<(), SolveError> {
    if opts.domain != Domain::Reals {
        return Ok(());
    }
    let before = roots
        .iter()
        .map(|r| r.rules[0].1.clone())
        .collect::<Vec<_>>();
    let mut kept = vec![];
    for mut r in roots.drain(..) {
        let value = r.rules[0].1.clone();
        let mut certain = known.contains(&value);
        let mut reject = false;
        if r.constants.len() == 1 {
            let c = r.constants[0].0.clone();
            if let Some(p) = extract::coefficients(&value, &c, ctx)?
                && p.values.len() <= 2
            {
                let offset = p.values.first().cloned().unwrap_or_else(|| Expr::int(0));
                let step = p.values.get(1).cloned().unwrap_or_else(|| Expr::int(0));
                if let (Some(a), Some(b)) = (enclose(&offset, 256, ctx)?, enclose(&step, 256, ctx)?)
                {
                    let ar = a.im.mid == BigFloat::ZERO && a.im.rad == BigFloat::ZERO;
                    let br = b.im.mid == BigFloat::ZERO && b.im.rad == BigFloat::ZERO;
                    if ar && br {
                        certain = true
                    } else if ar
                        && b.re.mid == BigFloat::ZERO
                        && b.re.rad == BigFloat::ZERO
                        && b.im.excludes_zero()
                    {
                        r.rules[0].1 = offset;
                        r.constants.clear();
                        certain = true
                    } else if a.im.excludes_zero() && br {
                        reject = true
                    }
                }
            }
        } else if r.constants.is_empty()
            && !certain
            && let Some(z) = enclose(&value, 256, ctx)?
        {
            reject = z.im.excludes_zero();
            certain = z.im.mid == BigFloat::ZERO && z.im.rad == BigFloat::ZERO
        }
        if !reject {
            if !certain {
                r.condition = inverse::and(
                    r.condition.take(),
                    Some(Expr::call(
                        B::ELEMENT,
                        [r.rules[0].1.clone(), Expr::sym(B::REALS)],
                    )),
                )
            }
            kept.push(r)
        }
    }
    let kept_count = kept.len();
    let rejected = before.len() - kept_count;
    let after = kept
        .iter()
        .map(|r| r.rules[0].1.clone())
        .collect::<Vec<_>>();
    sink.record(|| {
        Step::new(
            StepKind::DomainFilter {
                domain: Domain::Reals,
                kept: kept_count,
                dropped: rejected,
            },
            before,
            after,
            Level::Major,
        )
    });
    *roots = kept;
    Ok(())
}
