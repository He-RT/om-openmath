//! Complex polynomial candidates; original-equation validation belongs to the dispatcher.
mod extract;
mod order;
mod reductions;
mod symbolic;
use crate::{
    ExclReason, Formula, Level, MaxExtra, Solution, SolutionSet, SolveError, SolveOptions, Step,
    StepKind, StepSink, Verification,
};
use om_core::{BUILTIN as B, Expr, add, div, mul, neg, pow, sqrt, sub};
use om_num::{Integer, Number, Rational, ctx::Interrupt, gcd};
use om_poly::{FactorStatus, UPoly};
use om_simplify::{
    algebra::cancel_with,
    zero::{Tri, is_zero_with},
};
/// Candidate solutions with the generic conditions under which they were constructed.
#[derive(Clone, Debug)]
pub struct PolynomialRoots {
    /// All/empty/finite polynomial candidates, or wholly unsupported input.
    pub set: SolutionSet,
    /// Nonzero conditions for parameter coefficients and their denominators.
    pub assumptions: Vec<Expr>,
}
struct Candidate {
    value: Expr,
    multiplicity: u32,
    key: Option<order::Key>,
}
/// Solve Q or symbolic degree-one/two polynomial factors, retaining multiplicity.
/// This is a complex candidate kernel; the caller owns original exclusions and domain filters.
pub fn poly_uni(
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
        });
    };
    let mut assumptions = vec![];
    if coefficients.values.is_empty() {
        return Ok(PolynomialRoots {
            set: SolutionSet::All,
            assumptions,
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
        });
    }
    if coefficients.values.len() == 1 {
        return Ok(PolynomialRoots {
            set: SolutionSet::Finite(vec![]),
            assumptions,
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
        numeric(&UPoly::new(q), &primitive, x, ctx, sink)?
    } else {
        symbolic::roots(&coefficients.values, &primitive, x, ctx, sink)?
    };
    let Some(ref mut candidates) = candidates else {
        return Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions,
        });
    };
    order::sort(candidates, ctx)?;
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
    })
}
fn numeric(
    p: &UPoly<Rational>,
    original: &Expr,
    x: &Expr,
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
        let computed = reductions::roots(&coefficients, &polynomial, x, ctx, sink);
        if sink.enabled() {
            sink.exit();
        }
        let Some(mut found) = computed? else {
            return Ok(None);
        };
        order::assign(&f, &mut found, ctx)?;
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
fn simplified(e: Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    let canceled = cancel_with(&e, &[], ctx)?
        .ok_or_else(|| SolveError::Unsupported("formula simplification failed".into()))?;
    // Generic formulas retain their canonical numerator/denominator presentation.
    // Cancel is still useful when it proves a numeric value or reduces a numeric radical.
    Ok(
        if canceled.as_number().is_some() || e.free_symbols().is_empty() {
            canceled
        } else {
            e
        },
    )
}
fn formula(
    c: &[Expr],
    polynomial: &Expr,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    let (family, bindings, values) = match c.len() {
        2 => {
            let value = simplified(div(neg(c[0].clone()), c[1].clone()), ctx)?;
            (
                Formula::Linear,
                vec![("a".into(), c[1].clone()), ("b".into(), c[0].clone())],
                vec![(value, 1)],
            )
        }
        3 => {
            let (a, b, constant) = (&c[2], &c[1], &c[0]);
            let d = simplified(
                sub(
                    pow(b.clone(), Expr::int(2)),
                    mul([Expr::int(4), a.clone(), constant.clone()]),
                ),
                ctx,
            )?;
            let sign = extract::exact(&d).map(|q| {
                if q < Rational::ZERO {
                    crate::Sign::Negative
                } else if q == Rational::ZERO {
                    crate::Sign::Zero
                } else {
                    crate::Sign::Positive
                }
            });
            sink.record(|| {
                Step::new(
                    StepKind::Discriminant {
                        value: d.clone(),
                        sign,
                    },
                    vec![polynomial.clone()],
                    vec![d.clone()],
                    Level::Minor,
                )
            });
            let denominator = mul([Expr::int(2), a.clone()]);
            let values = if is_zero_with(&d, ctx)? == Tri::Zero {
                vec![(simplified(div(neg(b.clone()), denominator), ctx)?, 2)]
            } else {
                let s = sqrt(d);
                vec![
                    (
                        simplified(
                            div(sub(neg(b.clone()), s.clone()), denominator.clone()),
                            ctx,
                        )?,
                        1,
                    ),
                    (
                        simplified(div(add([neg(b.clone()), s]), denominator), ctx)?,
                        1,
                    ),
                ]
            };
            (
                Formula::Quadratic,
                vec![
                    ("a".into(), a.clone()),
                    ("b".into(), b.clone()),
                    ("c".into(), constant.clone()),
                ],
                values,
            )
        }
        _ => return Ok(None),
    };
    sink.record(|| {
        Step::new(
            StepKind::ApplyFormula {
                formula: family,
                bindings,
                results: values.iter().map(|(e, _)| e.clone()).collect(),
            },
            vec![polynomial.clone()],
            values.iter().map(|(e, _)| e.clone()).collect(),
            Level::Major,
        )
    });
    Ok(Some(
        values
            .into_iter()
            .map(|(value, multiplicity)| Candidate {
                value,
                multiplicity,
                key: None,
            })
            .collect(),
    ))
}
