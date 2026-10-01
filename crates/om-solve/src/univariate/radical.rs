//! Radical elimination generates candidates; original principal values accept them.
mod collect;
mod eliminate;
pub(crate) mod verify;
use super::{PolynomialRoots, extract, poly_uni, reductions};
use crate::{
    Level, MaxExtra, NoSteps, SolutionSet, SolveError, SolveOptions, Step, StepKind, StepSink,
    normalize,
};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel, add, mul, neg, pow, sub};
use om_num::ctx::Interrupt;
use om_simplify::{algebra::expand_with, convert::canonicalize_with};
use std::collections::BTreeMap;

/// Solve rational-base radical equations, checking every original principal branch.
/// Original exclusions are preserved before normalization. Mandatory checks cannot
/// be disabled with VerifyMode::Never; unsupported continuous cases stay unevaluated.
pub fn radical_path(
    e: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    if !opts.record_steps {
        let result = run(e, x, opts, ctx, &mut NoSteps, true)?;
        return crate::domain::filter(result, &[(x.clone(), opts.domain)], ctx, &mut NoSteps);
    }
    let result = run(e, x, opts, ctx, sink, true)?;
    crate::domain::filter(result, &[(x.clone(), opts.domain)], ctx, sink)
}
/// Internal candidates defer principal checks until all system parameters are solved.
pub(crate) fn candidates(
    e: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    run(e, x, opts, ctx, sink, false)
}
fn unsupported(e: &Expr, mut messages: Vec<Message>, sink: &mut impl StepSink) -> PolynomialRoots {
    let msg = Message {
        symbol: "Solve".into(),
        tag: "nsmet".into(),
        text: "This radical equation requires unsupported elimination or branch conditions.".into(),
        level: MsgLevel::Warning,
    };
    sink.record(|| {
        Step::new(
            StepKind::Note { msg: msg.clone() },
            vec![e.clone()],
            vec![],
            Level::Minor,
        )
    });
    messages.push(msg);
    PolynomialRoots {
        set: SolutionSet::Unevaluated,
        assumptions: vec![],
        messages,
    }
}
fn run(
    e: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
    verify_original: bool,
) -> Result<PolynomialRoots, SolveError> {
    ctx.tick()?;
    let normalized = normalize::normalize(
        &equation(e.clone(), Expr::int(0)),
        Some(std::slice::from_ref(x)),
        opts.domain,
        ctx,
        sink,
    )?;
    if normalized.unsupported {
        return Ok(unsupported(e, normalized.messages, sink));
    }
    let Some(branch) = normalized.branches.first() else {
        return Ok(PolynomialRoots {
            set: SolutionSet::Finite(vec![]),
            assumptions: vec![],
            messages: normalized.messages,
        });
    };
    let Some(work) = branch.equations.first() else {
        if verify::zero(e, ctx)? == om_simplify::zero::Tri::Zero
            && verify::global_exclusions(&branch.exclusions, ctx)?
        {
            return Ok(PolynomialRoots {
                set: SolutionSet::All,
                assumptions: vec![],
                messages: normalized.messages,
            });
        }
        return Ok(unsupported(e, normalized.messages, sink));
    };
    let polynomial = extract::coefficients(work, x, ctx)?.is_some();
    let eliminated = if polynomial {
        Some(work.clone())
    } else {
        let short = isolate(work, x, ctx, sink);
        match short {
            Ok(Some(p)) => Some(p),
            Ok(None) | Err(SolveError::Unsupported(_)) => eliminate::general(work, x, ctx, sink)?,
            Err(error) => return Err(error),
        }
    };
    let Some(p) = eliminated else {
        return Ok(unsupported(e, normalized.messages, sink));
    };
    let mut result = poly_uni(&p, x, opts, ctx, sink)?;
    result.messages.splice(0..0, normalized.messages);
    for exclusion in &branch.exclusions {
        ctx.tick()?;
        if !extract::depends(&exclusion.value, x, ctx)?
            && !exclusion.value.free_symbols().is_empty()
        {
            let guard = Expr::call(B::UNEQUAL, [exclusion.value.clone(), Expr::int(0)]);
            if !result.assumptions.contains(&guard) {
                result.assumptions.push(guard);
            }
        }
    }
    match &mut result.set {
        SolutionSet::Finite(roots) => {
            if verify_original {
                verify::candidates(
                    verify::Source {
                        original: e,
                        exclusions: &branch.exclusions,
                        guards: &result.assumptions,
                    },
                    roots,
                    &mut result.messages,
                    opts,
                    ctx,
                    sink,
                )?;
            }
            let expose = match opts.max_extra_conditions {
                MaxExtra::Zero => false,
                MaxExtra::All => true,
                MaxExtra::Count(n) => result.assumptions.len() <= n as usize,
            };
            let condition = if !expose || result.assumptions.is_empty() {
                None
            } else if result.assumptions.len() == 1 {
                Some(result.assumptions[0].clone())
            } else {
                Some(Expr::call(B::AND, result.assumptions.iter().cloned()))
            };
            for root in roots {
                ctx.tick()?;
                root.condition = condition.clone();
                if !polynomial {
                    root.multiplicity = 1;
                    if !verify_original {
                        root.verification = crate::Verification::Unverified;
                    }
                }
            }
        }
        SolutionSet::All => {
            if verify::zero(e, ctx)? != om_simplify::zero::Tri::Zero
                || !verify::global_exclusions(&branch.exclusions, ctx)?
            {
                return Ok(unsupported(e, result.messages, sink));
            }
        }
        _ => {}
    }
    Ok(result)
}
fn equation(left: Expr, right: Expr) -> Expr {
    Expr::call(B::EQUAL, [left, right])
}
fn isolate(
    original: &Expr,
    x: &Expr,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Expr>, SolveError> {
    let mut current = canonicalize_with(original, ctx)?;
    for _ in 0..4 {
        ctx.tick()?;
        let radicals = collect::discover(&current, x, false, ctx)?;
        if radicals.is_empty() {
            return Ok(extract::coefficients(&current, x, ctx)?.map(|_| current));
        }
        if radicals.len() > 2 {
            return Ok(None);
        }
        let selected = radicals
            .iter()
            .max_by_key(|r| r.degree)
            .expect("invariant: nonempty radical list");
        if !collect::rational(&selected.base, std::slice::from_ref(x), ctx)? {
            return Ok(None);
        }
        let axis = reductions::fresh(&current, x, ctx)?;
        let rewritten = collect::rewrite(
            &current,
            std::slice::from_ref(selected),
            std::slice::from_ref(&axis),
            ctx,
        )?;
        let Some(coefficients) = extract::coefficients(&rewritten, &axis, ctx)? else {
            return Ok(None);
        };
        let mut reduced: BTreeMap<u32, Vec<Expr>> = BTreeMap::new();
        for (i, c) in coefficients.values.into_iter().enumerate() {
            ctx.tick()?;
            if c.is_zero() {
                continue;
            }
            let i = u32::try_from(i).expect("invariant: dense degree <=4096");
            reduced.entry(i % selected.degree).or_default().push(mul([
                c,
                pow(
                    selected.base.clone(),
                    Expr::int(i64::from(i / selected.degree)),
                ),
            ]));
        }
        let mut reduced = reduced
            .into_iter()
            .map(|(i, terms)| (i, add(terms)))
            .collect::<BTreeMap<_, _>>();
        if reduced.iter().any(|(i, c)| *i > 1 && !c.is_zero()) {
            return Ok(None);
        }
        let c = reduced.remove(&1).unwrap_or_else(|| Expr::int(0));
        let rest = reduced.remove(&0).unwrap_or_else(|| Expr::int(0));
        if c.is_zero() {
            current = rest;
            continue;
        }
        let term = mul([c.clone(), selected.expression()]);
        let isolated = equation(term.clone(), neg(rest.clone()));
        sink.record(|| {
            Step::new(
                StepKind::IsolateTerm { term },
                vec![equation(current.clone(), Expr::int(0))],
                vec![isolated.clone()],
                Level::Major,
            )
        });
        let n = Expr::int(i64::from(selected.degree));
        let left = mul([pow(c, n.clone()), selected.base.clone()]);
        let right = pow(neg(rest), n);
        let raised = equation(left.clone(), right.clone());
        sink.record(|| {
            Step::new(
                StepKind::RaiseToPower { n: selected.degree },
                vec![isolated],
                vec![raised],
                Level::Major,
            )
        });
        let Some(expanded) = expand_with(&sub(left, right), ctx)? else {
            return Ok(None);
        };
        current = expanded;
    }
    Ok(extract::coefficients(&current, x, ctx)?.map(|_| current))
}
