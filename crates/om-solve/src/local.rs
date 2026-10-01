//! Local numerical proposals become solutions only after raw source validation.
mod arithmetic;
mod brent;
mod derivative;
mod newton;
use crate::{
    Domain, FindRootOptions, NoSteps, Solution, SolutionSet, SolveError, SolveOutcome,
    StepRecorder, StepSink, Verification, normalize,
};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel};
use om_num::{BigFloat, Number, Precision, ctx::Interrupt};
use om_simplify::zero::{Tri, UnknownReason};
pub(super) fn record(
    eqs: &[Expr],
    vars: &[Expr],
    before: &[Number],
    after: &[Number],
    residual: &[Number],
    sink: &mut impl StepSink,
) {
    for (e, r) in eqs.iter().zip(residual) {
        sink.record(|| {
            crate::Step::new(
                crate::StepKind::Verify {
                    candidate: vars
                        .iter()
                        .cloned()
                        .zip(after.iter().cloned().map(Expr::number))
                        .collect(),
                    outcome: Tri::Unknown(UnknownReason::NoInfo),
                    residual: Some(Expr::number(r.clone())),
                },
                vec![
                    e.clone(),
                    Expr::call(B::LIST, before.iter().cloned().map(Expr::number)),
                ],
                vec![Expr::call(B::LIST, after.iter().cloned().map(Expr::number))],
                crate::Level::Minor,
            )
        });
    }
}
pub(crate) fn verify(
    branch: &normalize::NormalizedBranch,
    rules: &[(Expr, Expr)],
    bits: u32,
    ctx: &Interrupt,
) -> Result<bool, SolveError> {
    let threshold = BigFloat::from_parts(1.into(), -i64::from(bits.saturating_sub(8)) as isize);
    for e in &branch.original {
        let Some(z) = om_simplify::numeval::enclose(&e.replace_all(rules), bits + 32, ctx)? else {
            return Ok(false);
        };
        if [z.re, z.im].iter().any(|b| {
            (if b.mid < BigFloat::ZERO {
                -&b.mid
            } else {
                b.mid.clone()
            }) + &b.rad
                >= threshold
        }) {
            return Ok(false);
        }
    }
    for e in &branch.exclusions {
        if !crate::univariate::transcendental::check::nonzero(&e.value.replace_all(rules), ctx)? {
            return Ok(false);
        }
    }
    for c in &branch.conditions {
        if crate::univariate::transcendental::check::allows(&c.replace_all(rules), ctx)?
            != Some(true)
        {
            return Ok(false);
        }
    }
    for (v, d) in &branch.domains {
        if *d != Domain::Complexes {
            let value = rules
                .iter()
                .find(|(a, _)| a == v)
                .map_or_else(|| v.clone(), |(_, b)| b.clone());
            if crate::univariate::transcendental::check::allows(
                &Expr::call(
                    B::ELEMENT,
                    [
                        value,
                        Expr::sym(match d {
                            Domain::Reals => B::REALS,
                            Domain::Integers => B::INTEGERS,
                            Domain::Rationals => B::RATIONALS,
                            Domain::Complexes => B::COMPLEXES,
                        }),
                    ],
                ),
                ctx,
            )? != Some(true)
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
pub(crate) fn verified_step(original: &Expr, rules: &[(Expr, Expr)]) -> crate::Step {
    let residual = original.replace_all(rules);
    crate::Step::new(
        crate::StepKind::Verify {
            candidate: rules.to_vec(),
            outcome: Tri::Unknown(UnknownReason::ProbablyZero),
            residual: Some(residual.clone()),
        },
        vec![original.clone()],
        vec![residual],
        crate::Level::Major,
    )
}
fn finite(n: &Number) -> bool {
    match n {
        Number::Real(om_num::Real::Machine(f)) => f.is_finite(),
        Number::Real(om_num::Real::Big(f)) => f.repr().is_finite(),
        Number::Complex(c) => finite(&c.re) && finite(&c.im),
        _ => true,
    }
}
fn scaled(e: &Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    let Some(view) = om_simplify::convert::to_rational_function_with(e, &[], ctx)? else {
        return Ok(e.clone());
    };
    let maximum = |p: &om_poly::MPoly<om_num::Rational>| {
        p.terms
            .iter()
            .map(|(_, q)| {
                if q < &om_num::Rational::ZERO {
                    -q
                } else {
                    q.clone()
                }
            })
            .max()
            .unwrap_or(om_num::Rational::ZERO)
    };
    let (n, d) = (maximum(&view.num), maximum(&view.den));
    if n == om_num::Rational::ZERO || d == om_num::Rational::ZERO {
        return Ok(e.clone());
    }
    Ok(om_core::mul([
        e.clone(),
        Expr::number(Number::Rational(d / n)),
    ]))
}
/// Find one local root by damped Newton, or bracketed Brent for one real variable.
/// Nonconvergence or failed raw source validation returns Unevaluated with diagnostics.
pub fn find_root(
    eqs: &Expr,
    starts: &[(Expr, Number)],
    opts: &FindRootOptions,
    ctx: &Interrupt,
) -> Result<SolveOutcome, SolveError> {
    ctx.tick()?;
    crate::numeric::bits(opts.precision)?;
    if starts.is_empty() || starts.len() > 64 || starts.iter().any(|(_, n)| !finite(n)) {
        return Err(SolveError::Invalid(
            "local starts must contain 1..64 finite numbers".into(),
        ));
    }
    if let Some((a, b)) = &opts.bracket
        && (!finite(a) || !finite(b))
    {
        return Err(SolveError::Invalid(
            "bracket endpoints must be finite".into(),
        ));
    }
    if opts.record_steps {
        let mut sink = StepRecorder::new();
        let (set, messages) = run(eqs, starts, opts, ctx, &mut sink)?;
        Ok(SolveOutcome {
            set,
            messages,
            steps: Some(sink.finish()),
        })
    } else {
        let (set, messages) = run(eqs, starts, opts, ctx, &mut NoSteps)?;
        Ok(SolveOutcome {
            set,
            messages,
            steps: None,
        })
    }
}
fn run(
    eqs: &Expr,
    starts: &[(Expr, Number)],
    opts: &FindRootOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<(SolutionSet, Vec<Message>), SolveError> {
    let vars = starts.iter().map(|(v, _)| v.clone()).collect::<Vec<_>>();
    let target = crate::numeric::bits(opts.precision)?;
    let mut messages = vec![];
    let work = (|| {
        let prepared = normalize::normalize(eqs, Some(&vars), Domain::Complexes, ctx, sink)?;
        if prepared.vars.len() != starts.len() {
            return Err(SolveError::Invalid(
                "duplicate local starting variables".into(),
            ));
        }
        messages = prepared.messages;
        if prepared.unsupported || prepared.branches.len() != 1 {
            return Err(SolveError::Unsupported(
                "FindRoot requires one numerical conjunction".into(),
            ));
        }
        let branch = &prepared.branches[0];
        if !branch.inequalities.is_empty() || branch.original.len() != vars.len() {
            return Err(SolveError::Unsupported(
                "FindRoot requires one equation per starting variable".into(),
            ));
        }
        let originals = branch
            .original
            .iter()
            .map(|e| {
                scaled(
                    &crate::univariate::transcendental::check::residual(e, ctx)?,
                    ctx,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let values = starts
            .iter()
            .map(|(_, n)| crate::numeric::rounded(n, Precision::Bits(target + 32), ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let found = if let Some(bracket) = &opts.bracket {
            if vars.len() != 1 {
                return Err(SolveError::Invalid(
                    "Brent requires one starting variable".into(),
                ));
            }
            brent::iterate(&originals[0], &prepared.vars[0], bracket, opts, ctx, sink)?
        } else {
            newton::iterate(&originals, &prepared.vars, values, opts, ctx, sink)?
        };
        let rounded = found
            .iter()
            .map(|n| crate::numeric::rounded(n, opts.precision, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let rules = prepared
            .vars
            .iter()
            .cloned()
            .zip(rounded.iter().cloned().map(Expr::number))
            .collect::<Vec<_>>();
        if !verify(branch, &rules, target, ctx)? {
            return Err(SolveError::Unsupported(
                "rounded local root failed original-equation or pole verification".into(),
            ));
        }
        for original in &branch.original {
            sink.record(|| verified_step(original, &rules));
        }
        let display = rounded
            .iter()
            .map(Number::to_complex_f64)
            .collect::<Vec<_>>();
        Ok(SolutionSet::Finite(vec![Solution {
            rules,
            condition: None,
            constants: vec![],
            multiplicity: 1,
            verification: Verification::Numeric {
                digits: target.saturating_sub(8) * 301 / 1000,
            },
            numeric: display
                .iter()
                .all(|(re, im)| re.is_finite() && im.is_finite())
                .then_some(display),
        }]))
    })();
    for m in &mut messages {
        m.symbol = "FindRoot".into();
    }
    match work {
        Err(SolveError::Unsupported(reason)) => {
            let msg = Message {
                symbol: "FindRoot".into(),
                tag: "cvmit".into(),
                text: reason,
                level: MsgLevel::Warning,
            };
            sink.record(|| {
                crate::Step::new(
                    crate::StepKind::Note { msg: msg.clone() },
                    vec![eqs.clone()],
                    vec![],
                    crate::Level::Major,
                )
            });
            messages.push(msg);
            Ok((SolutionSet::Unevaluated, messages))
        }
        result => Ok((result?, messages)),
    }
}
