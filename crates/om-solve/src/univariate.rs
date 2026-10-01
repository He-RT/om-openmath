//! Polynomial candidates with real-domain filtering; original validation belongs to the dispatcher.
mod extract;
mod formulas;
mod irreducible;
use formulas::formula;
pub(crate) mod order;
mod radical;
mod real;
pub(crate) use radical::candidates as radical_candidates;
pub use radical::radical_path;
pub(crate) use radical::verify::zero as verification_zero;
mod reductions;
mod symbolic;
pub(crate) mod transcendental;
use crate::{
    Domain, ExclReason, Level, MaxExtra, Solution, SolutionSet, SolveError, SolveOptions, Step,
    StepKind, StepSink, Verification,
};
use om_core::{BUILTIN as B, Expr, Message, add, div, mul, pow};
use om_num::{Integer, Number, Rational, ctx::Interrupt, gcd};
use om_poly::{FactorStatus, UPoly};
use om_simplify::{
    algebra::cancel_with,
    zero::{Tri, is_zero_with},
};
pub(crate) use transcendental::candidates as transcendental_candidates;
pub use transcendental::transcendental_path;
/// Candidate solutions with the generic conditions under which they were constructed.
#[derive(Clone, Debug)]
pub struct PolynomialRoots {
    /// All/empty/finite polynomial candidates, or wholly unsupported input.
    pub set: SolutionSet,
    /// Nonzero conditions for parameter coefficients and their denominators.
    pub assumptions: Vec<Expr>,
    /// Domain diagnostics, including retained candidates of unknown realness.
    pub messages: Vec<Message>,
}
struct Candidate {
    value: Expr,
    multiplicity: u32,
    key: Option<order::Key>,
}
/// Solve exact polynomial factors with radicals or Root objects, retaining multiplicity.
/// Reals is filtered with certified enclosures/root counts. Original exclusions,
/// verification and the remaining domain filters belong to the dispatcher.
pub fn poly_uni(
    e: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    if !opts.record_steps {
        let result = poly_uni_impl(e, x, opts, ctx, &mut crate::NoSteps)?;
        return crate::domain::filter(
            result,
            &[(x.clone(), opts.domain)],
            ctx,
            &mut crate::NoSteps,
        );
    }
    let result = poly_uni_impl(e, x, opts, ctx, sink)?;
    crate::domain::filter(result, &[(x.clone(), opts.domain)], ctx, sink)
}
fn poly_uni_impl(
    e: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    ctx.tick()?;
    let Some(mut coefficients) = extract::coefficients(e, x, ctx)? else {
        return Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions: vec![],
            messages: vec![],
        });
    };
    let mut assumptions = vec![];
    if coefficients.values.is_empty() {
        return Ok(PolynomialRoots {
            set: SolutionSet::All,
            assumptions,
            messages: vec![],
        });
    }
    if extract::exact(&coefficients.denominator).is_none() {
        let cond = Expr::call(B::UNEQUAL, [coefficients.denominator.clone(), Expr::int(0)]);
        sink.record(|| {
            Step::new(
                StepKind::RecordExclusion {
                    cond: cond.clone(),
                    reason: ExclReason::ZeroDenominator,
                },
                vec![coefficients.denominator.clone()],
                vec![],
                Level::Minor,
            )
        });
        assumptions.push(cond);
    }
    while let Some(lead) = coefficients.values.last() {
        ctx.tick()?;
        if is_zero_with(lead, ctx)? == Tri::Zero {
            coefficients.values.pop();
        } else {
            break;
        }
    }
    if coefficients.values.is_empty() {
        return Ok(PolynomialRoots {
            set: SolutionSet::All,
            assumptions,
            messages: vec![],
        });
    }
    let lead = coefficients
        .values
        .last()
        .expect("invariant: nonempty coefficient vector");
    if !lead.free_symbols().is_empty() {
        let cond = Expr::call(B::UNEQUAL, [lead.clone(), Expr::int(0)]);
        sink.record(|| {
            Step::new(
                StepKind::GenericAssumption { cond: cond.clone() },
                vec![lead.clone()],
                vec![],
                Level::Minor,
            )
        });
        if !assumptions.contains(&cond) {
            assumptions.push(cond);
        }
    } else if !matches!(is_zero_with(lead, ctx)?, Tri::NonZero) {
        return Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions,
            messages: vec![],
        });
    }
    if coefficients.values.len() == 1 {
        return Ok(PolynomialRoots {
            set: SolutionSet::Finite(vec![]),
            assumptions,
            messages: vec![],
        });
    }
    let mut primitive = e.clone();
    if !coefficients.content.is_one() {
        for c in &mut coefficients.values {
            ctx.tick()?;
            *c = cancel_with(&div(c.clone(), coefficients.content.clone()), &[], ctx)?.ok_or_else(
                || SolveError::Unsupported("coefficient content division failed".into()),
            )?;
        }
        if sink.enabled() {
            let mut terms = vec![];
            for (i, c) in coefficients.values.iter().enumerate() {
                ctx.tick()?;
                terms.push(mul([
                    c.clone(),
                    pow(x.clone(), Expr::integer(Integer::from(i))),
                ]));
            }
            primitive = add(terms);
            sink.record(|| {
                Step::new(
                    StepKind::Factor {
                        factors: vec![(coefficients.content.clone(), 1), (primitive.clone(), 1)],
                    },
                    vec![e.clone()],
                    vec![mul([coefficients.content.clone(), primitive.clone()])],
                    Level::Major,
                )
            });
        }
    }
    let rational = coefficients
        .values
        .iter()
        .map(extract::exact)
        .collect::<Option<Vec<_>>>();
    let mut candidates = if let Some(q) = rational {
        numeric(&UPoly::new(q), &primitive, x, opts, ctx, sink)?
    } else {
        symbolic::roots(&coefficients.values, &primitive, x, opts, ctx, sink)?
    };
    let Some(ref mut candidates) = candidates else {
        return Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions,
            messages: vec![],
        });
    };
    order::sort(candidates, ctx)?;
    let messages = if opts.domain == Domain::Reals {
        real::filter(candidates, &coefficients.values, ctx, sink)?
    } else {
        vec![]
    };
    let show = match opts.max_extra_conditions {
        MaxExtra::Zero => false,
        MaxExtra::All => true,
        MaxExtra::Count(n) => assumptions.len() <= n as usize,
    };
    let condition = if !show || assumptions.is_empty() {
        None
    } else if assumptions.len() == 1 {
        Some(assumptions[0].clone())
    } else {
        Some(Expr::call(B::AND, assumptions.iter().cloned()))
    };
    let mut solutions = vec![];
    for root in candidates.drain(..) {
        ctx.tick()?;
        solutions.push(Solution {
            rules: vec![(x.clone(), root.value)],
            condition: condition.clone(),
            constants: vec![],
            multiplicity: root.multiplicity,
            verification: Verification::ByConstruction,
            numeric: None,
        });
    }
    Ok(PolynomialRoots {
        set: SolutionSet::Finite(solutions),
        assumptions,
        messages,
    })
}
fn numeric(
    p: &UPoly<Rational>,
    original: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    let mut scale = Integer::ONE;
    for c in &p.coeffs {
        ctx.tick()?;
        let d = Integer::from(c.denominator().clone());
        scale = (&scale / gcd(&scale, &d)) * d;
    }
    let mut ints = vec![];
    for c in &p.coeffs {
        ctx.tick()?;
        ints.push(c.numerator() * (&scale / Integer::from(c.denominator().clone())));
    }
    let z = UPoly::new(ints);
    if scale != Integer::ONE && sink.enabled() {
        let cleared = order::polynomial(&z, x, ctx)?;
        sink.record(|| {
            Step::new(
                StepKind::ClearDenominators {
                    factor: Expr::integer(scale.clone()),
                },
                vec![original.clone()],
                vec![cleared],
                Level::Minor,
            )
        });
    }
    let sf = z
        .square_free(ctx)?
        .expect("invariant: nonzero polynomial admits Yun decomposition");
    let mut parts = vec![];
    for (f, m) in &sf.factors {
        ctx.tick()?;
        parts.push((order::polynomial(f, x, ctx)?, *m));
    }
    sink.record(|| {
        Step::new(
            StepKind::SquareFree {
                parts: parts.clone(),
            },
            vec![original.clone()],
            parts.iter().map(|(f, _)| f.clone()).collect(),
            Level::Minor,
        )
    });
    let mut factors = vec![];
    for (part, m) in sf.factors {
        ctx.tick()?;
        let Some(f) = part.factor_z(ctx)? else {
            return Ok(None);
        };
        if f.status != FactorStatus::Complete {
            return Ok(None);
        }
        for (poly, n) in f.factors {
            ctx.tick()?;
            let multiplicity = m
                .checked_mul(n)
                .ok_or_else(|| SolveError::Unsupported("root multiplicity overflow".into()))?;
            factors.push((poly, multiplicity));
        }
    }
    let content = Expr::number(Number::Rational(
        Rational::from(sf.content) / Rational::from(scale),
    ));
    let mut factor_events = vec![];
    let mut product = vec![content];
    for (f, m) in &factors {
        ctx.tick()?;
        let e = order::polynomial(f, x, ctx)?;
        factor_events.push((e.clone(), *m));
        product.push(pow(e, Expr::integer(Integer::from(*m))));
    }
    sink.record(|| {
        Step::new(
            StepKind::Factor {
                factors: factor_events.clone(),
            },
            vec![original.clone()],
            vec![mul(product)],
            Level::Major,
        )
    });
    if factors.len() > 1 {
        sink.record(|| {
            Step::new(
                StepKind::ZeroProduct,
                vec![original.clone()],
                factor_events.iter().map(|(f, _)| f.clone()).collect(),
                Level::Major,
            )
        });
    }
    let mut roots = vec![];
    for (f, m) in factors {
        ctx.tick()?;
        let polynomial = order::polynomial(&f, x, ctx)?;
        if sink.enabled() {
            sink.enter("polynomial_factor");
        }
        sink.record(|| {
            Step::new(
                StepKind::SplitComponent {
                    factor: polynomial.clone(),
                },
                vec![original.clone()],
                vec![polynomial.clone()],
                Level::Minor,
            )
        });
        let coefficients = f
            .coeffs
            .iter()
            .map(|c| Expr::integer(c.clone()))
            .collect::<Vec<_>>();
        let computed = irreducible::roots(&coefficients, &polynomial, x, opts, ctx, sink);
        if sink.enabled() {
            sink.exit();
        }
        let Some(mut found) = computed? else {
            return Ok(None);
        };
        if let Err(error) = order::assign(&f, &mut found, ctx) {
            if matches!(error, SolveError::Unsupported(_))
                && found.iter().all(|r| !r.value.is_head(B::ROOT))
            {
                found = irreducible::root_objects(&coefficients, &polynomial, ctx, sink)?;
                order::assign(&f, &mut found, ctx)?;
            } else {
                return Err(error);
            }
        }
        for root in &mut found {
            ctx.tick()?;
            root.multiplicity = root
                .multiplicity
                .checked_mul(m)
                .ok_or_else(|| SolveError::Unsupported("root multiplicity overflow".into()))?;
        }
        roots.extend(found);
    }
    Ok(Some(roots))
}
