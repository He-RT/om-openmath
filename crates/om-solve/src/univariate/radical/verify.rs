//! Original zero and pole certificates are mandatory, independent of VerifyMode.
use crate::{
    Level, Solution, SolveError, SolveOptions, Step, StepKind, StepSink, Verification,
    normalize::Exclusion,
};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel};
use om_num::{
    BigFloat, Complex, Number, Rational,
    ctx::{Abort, Interrupt},
    rng::SplitMix64,
};
use om_simplify::zero::{Tri, UnknownReason, is_zero_with};
use std::cell::Cell;

pub(in crate::univariate) fn zero(e: &Expr, ctx: &Interrupt) -> Result<Tri, SolveError> {
    const LIMIT: u64 = 16_384;
    let available = ctx.steps_left.get();
    let budget = available.min(LIMIT);
    let local = Interrupt {
        flag: ctx.flag.clone(),
        deadline_ms: ctx.deadline_ms,
        clock: ctx.clock.clone(),
        steps_left: Cell::new(budget),
    };
    let result = is_zero_with(e, &local);
    ctx.steps_left
        .set(available - (budget - local.steps_left.get()));
    match result {
        Err(Abort::Budget) if available > LIMIT => Ok(Tri::Unknown(UnknownReason::NoInfo)),
        result => Ok(result?),
    }
}
pub(in crate::univariate) fn numeric(e: &Expr, ctx: &Interrupt) -> Result<bool, SolveError> {
    for bits in [128, 512] {
        ctx.tick()?;
        let Some(z) = om_simplify::numeval::enclose(e, bits, ctx)? else {
            return Ok(false);
        };
        let limit = BigFloat::from_parts(1.into(), -(i64::from(bits) - 16) as isize);
        if [z.re, z.im]
            .iter()
            .any(|b| !b.contains_zero() || b.rad >= limit)
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn nonzero(e: &Expr, ctx: &Interrupt) -> Result<bool, SolveError> {
    match zero(e, ctx)? {
        Tri::Zero => Ok(false),
        Tri::NonZero => Ok(true),
        Tri::Unknown(_) => {
            let Some(z) = om_simplify::numeval::enclose(e, 512, ctx)? else {
                return Ok(false);
            };
            Ok(z.re.excludes_zero() || z.im.excludes_zero())
        }
    }
}
pub(in crate::univariate) fn global_exclusions(
    exclusions: &[Exclusion],
    ctx: &Interrupt,
) -> Result<bool, SolveError> {
    for exclusion in exclusions {
        ctx.tick()?;
        if !exclusion.value.free_symbols().is_empty() || !nonzero(&exclusion.value, ctx)? {
            return Ok(false);
        }
    }
    Ok(true)
}
fn parameter_numeric(
    e: &Expr,
    exclusions: &[Expr],
    guards: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
) -> Result<bool, SolveError> {
    let mut vars = e.free_symbols().into_iter().collect::<Vec<_>>();
    for expr in exclusions.iter().chain(guards) {
        for var in expr.free_symbols() {
            if !vars.contains(&var) {
                vars.push(var);
            }
        }
    }
    vars.sort_by_key(|s| s.name().to_string());
    if vars.is_empty() {
        return numeric(e, ctx);
    }
    let mut rng = SplitMix64::new(opts.seed);
    let mut passed = 0;
    for _ in 0..24 {
        ctx.tick()?;
        let mut rules = vec![];
        for var in &vars {
            let re = Rational::from(rng.next_range(0, 7) as i64 - 3)
                / Rational::from(rng.next_range(1, 5));
            let im = Rational::from(rng.next_range(0, 7) as i64 - 3)
                / Rational::from(rng.next_range(1, 5));
            rules.push((
                Expr::sym(*var),
                Expr::number(Number::Complex(Box::new(Complex {
                    re: Number::Rational(re),
                    im: Number::Rational(im),
                }))),
            ));
        }
        let mut allowed = true;
        for value in exclusions {
            if !nonzero(&value.replace_all(&rules), ctx)? {
                allowed = false;
                break;
            }
        }
        for guard in guards {
            if guard.is_head(B::UNEQUAL)
                && guard.args().len() == 2
                && !nonzero(
                    &om_core::sub(guard.args()[0].clone(), guard.args()[1].clone())
                        .replace_all(&rules),
                    ctx,
                )?
            {
                allowed = false;
                break;
            }
        }
        if !allowed {
            continue;
        }
        if !numeric(&e.replace_all(&rules), ctx)? {
            return Ok(false);
        }
        passed += 1;
        if passed == 3 {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) struct Source<'a> {
    pub original: &'a Expr,
    pub exclusions: &'a [Exclusion],
    pub guards: &'a [Expr],
}
pub(super) fn candidates(
    source: Source<'_>,
    roots: &mut Vec<Solution>,
    messages: &mut Vec<Message>,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<(), SolveError> {
    let Source {
        original,
        exclusions,
        guards,
    } = source;
    let mut kept = vec![];
    for mut root in roots.drain(..) {
        ctx.tick()?;
        let residual = original.replace_all(&root.rules);
        let outcome = zero(&residual, ctx)?;
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
        let values = exclusions
            .iter()
            .map(|e| e.value.replace_all(&root.rules))
            .collect::<Vec<_>>();
        let mut poles = false;
        for value in &values {
            ctx.tick()?;
            if value.free_symbols().is_empty() && !nonzero(value, ctx)? {
                poles = true;
                break;
            }
            if zero(value, ctx)? == Tri::Zero {
                poles = true;
                break;
            }
        }
        let accepted = if poles {
            false
        } else {
            match outcome {
                Tri::Zero => {
                    root.verification = Verification::Exact;
                    true
                }
                Tri::NonZero => false,
                Tri::Unknown(_) => {
                    if parameter_numeric(&residual, &values, guards, opts, ctx)? {
                        root.verification = Verification::Numeric { digits: 33 };
                        true
                    } else {
                        false
                    }
                }
            }
        };
        if accepted {
            kept.push(root);
        } else {
            let why = if poles {
                "original_exclusion"
            } else {
                "principal_branch"
            };
            sink.record(|| {
                Step::new(
                    StepKind::DropExtraneous {
                        candidate: root.rules.clone(),
                        why: why.into(),
                    },
                    vec![residual.clone()],
                    vec![],
                    Level::Major,
                )
            });
            if matches!(outcome, Tri::Unknown(_)) && !poles {
                let msg = Message {
                    symbol: "Solve".into(),
                    tag: "verify".into(),
                    text: "A radical candidate failed mandatory original-equation verification."
                        .into(),
                    level: MsgLevel::Warning,
                };
                sink.record(|| {
                    Step::new(
                        StepKind::Note { msg: msg.clone() },
                        vec![residual],
                        vec![],
                        Level::Minor,
                    )
                });
                messages.push(msg);
            }
        }
    }
    *roots = kept;
    Ok(())
}
