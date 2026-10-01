//! Numeric polynomial coordinates are associated with complete certified Aberth batches.
mod exactify;
use crate::{SolutionSet, SolveError, SolveOptions, SolveOutcome, Verification};
use om_core::Expr;
use om_num::{BigFloat, Complex, Number, Precision, Rational, Real, ctx::Interrupt};
use om_poly::{Algebraic, RootDisk, UPoly};
pub(crate) fn bits(p: Precision) -> Result<u32, SolveError> {
    match p {
        Precision::Machine => Ok(53),
        Precision::Bits(n) if (16..=8192).contains(&n) => Ok(n),
        _ => Err(SolveError::Invalid(
            "numeric precision must be Machine or 16..8192 bits".into(),
        )),
    }
}
pub(crate) fn rounded(n: &Number, p: Precision, ctx: &Interrupt) -> Result<Number, SolveError> {
    om_simplify::numeval::approximate(&Expr::number(n.clone()), p, ctx)?
        .ok_or_else(|| SolveError::Unsupported("numeric output rounding unavailable".into()))
}
fn dyadic(n: &BigFloat) -> Rational {
    let q = Rational::from(n.repr().significand().clone());
    if n.repr().exponent() >= 0 {
        q * Rational::from(om_num::Integer::ONE << n.repr().exponent() as usize)
    } else {
        q / Rational::from(om_num::Integer::ONE << n.repr().exponent().unsigned_abs())
    }
}
fn coordinate(
    e: &Expr,
    p: Precision,
    cache: &mut Vec<(UPoly<om_num::Integer>, Vec<RootDisk>)>,
    ctx: &Interrupt,
) -> Result<Number, SolveError> {
    let value = om_simplify::root_reduce::to_algebraic(e, ctx)?.ok_or_else(|| {
        SolveError::Unsupported("NSolve requires closed algebraic coordinates".into())
    })?;
    if let Algebraic::Rational(q) = &value {
        return rounded(&Number::Rational(q.clone()), p, ctx);
    }
    let f = value.minimal_polynomial(ctx)?;
    let index = if let Some(i) = cache.iter().position(|(poly, _)| poly == &f) {
        i
    } else {
        let disks = om_poly::complex_roots(&f, bits(p)? + 32, ctx)?.ok_or_else(|| {
            SolveError::Unsupported("NSolve Aberth certification unavailable".into())
        })?;
        cache.push((f, disks));
        cache.len() - 1
    };
    let ball = value
        .enclosure(bits(p)? + 32, ctx)?
        .ok_or_else(|| SolveError::Unsupported("NSolve root association unavailable".into()))?;
    let bounds = |b: &om_num::Ball| {
        let m = dyadic(&b.mid);
        let r = dyadic(&b.rad);
        (&m - &r, m + r)
    };
    let mut selected = None;
    for disk in &cache[index].1 {
        ctx.tick()?;
        let d = disk.to_cball();
        let overlap = [(&ball.re, &d.re), (&ball.im, &d.im)].iter().all(|(a, b)| {
            let (al, ah) = bounds(a);
            let (bl, bh) = bounds(b);
            al <= bh && bl <= ah
        });
        if overlap {
            if selected.is_some() {
                return Err(SolveError::Unsupported(
                    "ambiguous NSolve root association".into(),
                ));
            }
            selected = Some(disk);
        }
    }
    let disk =
        selected.ok_or_else(|| SolveError::Unsupported("NSolve root association failed".into()))?;
    let n = Number::Complex(Box::new(Complex {
        re: Number::Real(Real::Big(disk.re.clone())),
        im: if matches!(value, Algebraic::Real(_)) {
            Number::Integer(0.into())
        } else {
            Number::Real(Real::Big(disk.im.clone()))
        },
    }))
    .normalize();
    rounded(&n, p, ctx)
}
/// Compute every finite algebraic assignment numerically at the requested precision.
/// Incomplete, conditional or parameterized systems remain Unevaluated with diagnostics.
pub fn nsolve(
    eqs: &Expr,
    vars: &[Expr],
    precision: Precision,
    ctx: &Interrupt,
) -> Result<SolveOutcome, SolveError> {
    ctx.tick()?;
    let bits = bits(precision)?;
    let prepared = crate::normalize::normalize(
        eqs,
        Some(vars),
        crate::Domain::Complexes,
        ctx,
        &mut crate::NoSteps,
    )?;
    let exact = exactify::input(eqs, ctx)?;
    let mut result = crate::solve(&exact, vars, &SolveOptions::default(), ctx)?;
    let SolutionSet::Finite(roots) = &mut result.set else {
        for m in &mut result.messages {
            m.symbol = "NSolve".into();
        }
        return Ok(result);
    };
    let mut cache = vec![];
    let work = (|| {
        for root in roots {
            if !root.constants.is_empty()
                || root.condition.is_some()
                || root.rules.len() != prepared.vars.len()
            {
                return Err(SolveError::Unsupported(
                    "NSolve requires complete finite assignments".into(),
                ));
            }
            for (_, v) in &mut root.rules {
                *v = Expr::number(coordinate(v, precision, &mut cache, ctx)?);
            }
            let mut valid = false;
            for branch in &prepared.branches {
                if crate::local::verify(branch, &root.rules, bits, ctx)? {
                    valid = true;
                    for original in &branch.original {
                        append(&mut result.steps, || {
                            crate::local::verified_step(original, &root.rules)
                        });
                    }
                    break;
                }
            }
            if !valid {
                return Err(SolveError::Unsupported(
                    "rounded NSolve root failed original residual or pole verification".into(),
                ));
            }
            root.verification = Verification::Numeric {
                digits: bits.saturating_sub(8) * 301 / 1000,
            };
            let values = root
                .rules
                .iter()
                .map(|(_, v)| {
                    v.as_number()
                        .expect("invariant: numerical coordinate")
                        .to_complex_f64()
                })
                .collect::<Vec<_>>();
            root.numeric = values
                .iter()
                .all(|(re, im)| re.is_finite() && im.is_finite())
                .then_some(values);
        }
        Ok(())
    })();
    for m in &mut result.messages {
        m.symbol = "NSolve".into();
    }
    match work {
        Err(SolveError::Unsupported(reason)) => {
            result.set = SolutionSet::Unevaluated;
            let msg = om_core::Message {
                symbol: "NSolve".into(),
                tag: "nsmet".into(),
                text: reason,
                level: om_core::MsgLevel::Warning,
            };
            append(&mut result.steps, || {
                crate::Step::new(
                    crate::StepKind::Note { msg: msg.clone() },
                    vec![eqs.clone()],
                    vec![],
                    crate::Level::Major,
                )
            });
            result.messages.push(msg);
        }
        result => result?,
    }
    Ok(result)
}

fn append(steps: &mut Option<crate::Steps>, create: impl FnOnce() -> crate::Step) {
    if let Some(steps) = steps {
        let mut step = create();
        step.id = format!("S{}", steps.root.len() + 1);
        steps.root.push(step);
    }
}
