//! Exact discrete membership never uses a floating-point proximity heuristic.
mod integer;
use crate::{
    Domain, Level, NoSteps, SolutionSet, SolveError, SolveOptions, Step, StepKind, StepSink,
    normalize, univariate::PolynomialRoots,
};
pub(crate) use integer::solve as integer_linear;
use om_core::{BUILTIN as B, Expr, ExprKind, Message, MsgLevel};
use om_num::{Number, Rational, ctx::Interrupt};
use om_poly::Algebraic;

pub(crate) fn condition(a: Option<Expr>, b: Expr) -> Option<Expr> {
    Some(match a {
        Some(a) if a != b => Expr::call(B::AND, [a, b]),
        Some(a) => a,
        None => b,
    })
}
fn domain_symbol(d: Domain) -> om_core::Symbol {
    match d {
        Domain::Integers => B::INTEGERS,
        Domain::Rationals => B::RATIONALS,
        Domain::Reals => B::REALS,
        Domain::Complexes => B::COMPLEXES,
    }
}
fn integral_polynomial(value: &Expr, known: &[Expr], ctx: &Interrupt) -> Result<bool, SolveError> {
    let Some(view) = om_simplify::convert::to_rational_function_with(value, known, ctx)? else {
        return Ok(false);
    };
    if view.gens != known || view.den.terms.len() != 1 || view.den.terms[0].0.deg != 0 {
        return Ok(false);
    }
    let denominator = &view.den.terms[0].1;
    if denominator == &Rational::ZERO {
        return Ok(false);
    }
    Ok(view
        .num
        .terms
        .iter()
        .all(|(_, c)| (c / denominator).denominator() == &1_u8.into()))
}
pub(crate) fn filter(
    mut result: PolynomialRoots,
    domains: &[(Expr, Domain)],
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    if !domains
        .iter()
        .any(|(_, d)| matches!(d, Domain::Integers | Domain::Rationals))
    {
        return Ok(result);
    }
    let SolutionSet::Finite(roots) = result.set else {
        return Ok(result);
    };
    let before = roots
        .iter()
        .map(|r| Expr::call(B::LIST, r.rules.iter().map(|(_, v)| v.clone())))
        .collect::<Vec<_>>();
    let total = roots.len();
    let mut kept = vec![];
    for mut root in roots {
        ctx.tick()?;
        let integer_axes = domains
            .iter()
            .filter(|(_, d)| *d == Domain::Integers)
            .map(|(v, _)| v.clone())
            .chain(
                root.constants
                    .iter()
                    .filter(|(_, d)| *d == Domain::Integers)
                    .map(|(v, _)| v.clone()),
            )
            .collect::<Vec<_>>();
        let mut reject = false;
        let mut changed = false;
        for (var, value) in &mut root.rules {
            let domain = domains
                .iter()
                .find(|(v, _)| v == var)
                .map_or(Domain::Complexes, |(_, d)| *d);
            if !matches!(domain, Domain::Integers | Domain::Rationals) {
                continue;
            }
            if integral_polynomial(value, &integer_axes, ctx)? {
                continue;
            }
            match om_simplify::root_reduce::to_algebraic(value, ctx)? {
                Some(Algebraic::Rational(q)) => {
                    if domain == Domain::Integers && q.denominator() != &1_u8.into() {
                        reject = true;
                        break;
                    }
                    let reduced = Expr::number(Number::Rational(q));
                    changed |= *value != reduced;
                    *value = reduced;
                }
                Some(_) => {
                    reject = true;
                    break;
                }
                None => {
                    let periodic = root.constants.iter().any(|(c, _)| !value.free_of(c));
                    if value.free_symbols().is_empty() && !periodic {
                        let msg = Message {
                            symbol: "Solve".into(),
                            tag: "dom".into(),
                            text: "Exact integer/rational membership could not be certified."
                                .into(),
                            level: MsgLevel::Warning,
                        };
                        sink.record(|| {
                            Step::new(
                                StepKind::Note { msg: msg.clone() },
                                vec![value.clone()],
                                vec![],
                                Level::Minor,
                            )
                        });
                        result.messages.push(msg);
                        result.set = SolutionSet::Unevaluated;
                        return Ok(result);
                    }
                    root.condition = condition(
                        root.condition.take(),
                        Expr::call(
                            B::ELEMENT,
                            [value.clone(), Expr::sym(domain_symbol(domain))],
                        ),
                    );
                }
            }
        }
        if reject {
            continue;
        }
        for (var, domain) in domains {
            if matches!(domain, Domain::Integers | Domain::Rationals)
                && !root.rules.iter().any(|(v, _)| v == var)
            {
                root.condition = condition(
                    root.condition.take(),
                    Expr::call(B::ELEMENT, [var.clone(), Expr::sym(domain_symbol(*domain))]),
                );
            }
        }
        if changed {
            root.numeric = None
        }
        kept.push(root);
    }
    let after = kept
        .iter()
        .map(|r| Expr::call(B::LIST, r.rules.iter().map(|(_, v)| v.clone())))
        .collect::<Vec<_>>();
    let domain = domains
        .iter()
        .find(|(_, d)| matches!(d, Domain::Integers | Domain::Rationals))
        .expect("invariant: discrete domain present")
        .1;
    sink.record(|| {
        Step::new(
            StepKind::DomainFilter {
                domain,
                kept: kept.len(),
                dropped: total - kept.len(),
            },
            before,
            after,
            Level::Major,
        )
    });
    result.set = SolutionSet::Finite(kept);
    Ok(result)
}
pub(crate) fn filter_input(
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    result: PolynomialRoots,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    if matches!(result.set, SolutionSet::Unevaluated) {
        return Ok(result);
    }
    let prepared = normalize::normalize(eqs, Some(vars), opts.domain, ctx, &mut NoSteps)?;
    if let Some(branch) = prepared.branches.first() {
        filter(result, &branch.domains, ctx, sink)
    } else {
        Ok(result)
    }
}
/// Probe the integer lattice path silently; an accepted path records its real run.
pub(crate) fn affine(
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<PolynomialRoots>, SolveError> {
    let prepared = normalize::normalize(eqs, Some(vars), opts.domain, ctx, &mut NoSteps)?;
    if prepared.unsupported
        || prepared.branches.len() != 1
        || prepared.vars.is_empty()
        || !prepared.branches[0]
            .domains
            .iter()
            .all(|(_, d)| *d == Domain::Integers)
    {
        return Ok(None);
    }
    let result = crate::linear_system(eqs, vars, opts, ctx, &mut NoSteps)?;
    if matches!(result.set, SolutionSet::Unevaluated) {
        return Ok(None);
    }
    if sink.enabled() {
        return Ok(Some(crate::linear_system(eqs, vars, opts, ctx, sink)?));
    }
    Ok(Some(result))
}
pub(crate) fn periods(
    e: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
) -> Result<(Vec<Expr>, u32), SolveError> {
    let mut names = vec![];
    let mut next = 1;
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if e.head_symbol() == Some(opts.generated_parameter)
            && e.args().len() == 1
            && let Some(Number::Integer(i)) = e.args()[0].as_number()
            && let Ok(i) = u32::try_from(i)
        {
            next = next.max(i.checked_add(1).ok_or_else(|| {
                SolveError::Unsupported("integer parameter index exhaustion".into())
            })?);
            if !names.contains(e) {
                names.push(e.clone())
            }
        }
        if let ExprKind::Normal(n) = e.kind() {
            stack.push(&n.head);
            stack.extend(&n.args)
        }
    }
    Ok((names, next))
}
