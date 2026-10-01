//! Exact root counts and directed imaginary enclosures certify real candidates.
use super::{Candidate, extract};
use crate::{Domain, Level, SolveError, Step, StepKind, StepSink};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel};
use om_num::{BigFloat, Integer, ctx::Interrupt, gcd};
use om_poly::{UPoly, isolate};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Realness {
    Real,
    Nonreal,
    Unknown,
}

fn real_count(c: &[Expr], ctx: &Interrupt) -> Result<Option<usize>, SolveError> {
    let Some(q) = c.iter().map(extract::exact).collect::<Option<Vec<_>>>() else {
        return Ok(None);
    };
    let mut scale = Integer::ONE;
    for a in &q {
        ctx.tick()?;
        let den = Integer::from(a.denominator().clone());
        scale = (&scale / gcd(&scale, &den)) * den;
    }
    let mut integers = vec![];
    for a in q {
        ctx.tick()?;
        integers.push(a.numerator() * (&scale / Integer::from(a.denominator().clone())));
    }
    let p = UPoly::new(integers);
    let Some(parts) = p.square_free(ctx)? else {
        return Ok(None);
    };
    let mut count = 0;
    for (part, _) in parts.factors {
        ctx.tick()?;
        let Some(intervals) = isolate(&part, ctx)? else {
            return Ok(None);
        };
        count += intervals.len();
    }
    Ok(Some(count))
}
struct RootCount {
    function: Expr,
    data: Option<(usize, usize)>,
}
fn function_count(f: &Expr, ctx: &Interrupt) -> Result<Option<(usize, usize)>, SolveError> {
    let slot = Expr::call(B::SLOT, [Expr::int(1)]);
    let Some(coefficients) = extract::coefficients(&f.args()[0], &slot, ctx)? else {
        return Ok(None);
    };
    let Some(degree) = coefficients.values.len().checked_sub(1) else {
        return Ok(None);
    };
    Ok(real_count(&coefficients.values, ctx)?.map(|r| (degree, r)))
}
fn root_realness(
    e: &Expr,
    cache: &mut Vec<RootCount>,
    ctx: &Interrupt,
) -> Result<Realness, SolveError> {
    if !e.is_head(B::ROOT) || e.args().len() != 2 {
        return Ok(Realness::Unknown);
    }
    let f = &e.args()[0];
    if !f.is_head(B::FUNCTION) || f.args().len() != 1 {
        return Ok(Realness::Unknown);
    }
    let Some(om_num::Number::Integer(k)) = e.args()[1].as_number() else {
        return Ok(Realness::Unknown);
    };
    let Ok(k) = usize::try_from(k) else {
        return Ok(Realness::Unknown);
    };
    if k == 0 {
        return Ok(Realness::Unknown);
    }
    let data = if let Some(entry) = cache.iter().find(|entry| entry.function == *f) {
        entry.data
    } else {
        let data = function_count(f, ctx)?;
        cache.push(RootCount {
            function: f.clone(),
            data,
        });
        data
    };
    let Some((degree, r)) = data else {
        return Ok(Realness::Unknown);
    };
    if k > degree {
        return Ok(Realness::Unknown);
    }
    Ok(if k <= r {
        Realness::Real
    } else {
        Realness::Nonreal
    })
}
pub(super) fn filter(
    roots: &mut Vec<Candidate>,
    c: &[Expr],
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Message>, SolveError> {
    ctx.tick()?;
    let count = real_count(c, ctx)?;
    let mut status = vec![];
    let mut cache = vec![];
    for root in roots.iter() {
        ctx.tick()?;
        status.push(root_realness(&root.value, &mut cache, ctx)?);
    }
    for bits in [64, 256, 1024, 4096] {
        for (root, state) in roots.iter().zip(&mut status) {
            ctx.tick()?;
            if *state != Realness::Unknown {
                continue;
            }
            let Some(z) = om_simplify::numeval::enclose(&root.value, bits, ctx)? else {
                continue;
            };
            if z.im.excludes_zero() {
                *state = Realness::Nonreal;
            } else if z.im.mid == BigFloat::ZERO && z.im.rad == BigFloat::ZERO {
                *state = Realness::Real;
            }
        }
        // Completeness comes from the polynomial construction, not numerical proximity.
        if count == Some(status.iter().filter(|s| **s != Realness::Nonreal).count()) {
            for state in &mut status {
                ctx.tick()?;
                if *state == Realness::Unknown {
                    *state = Realness::Real;
                }
            }
        }
        if !status.contains(&Realness::Unknown) {
            break;
        }
    }
    let before = if sink.enabled() {
        roots.iter().map(|r| r.value.clone()).collect()
    } else {
        vec![]
    };
    let mut messages = vec![];
    let mut kept = vec![];
    let mut dropped = 0;
    for (root, state) in roots.drain(..).zip(status) {
        ctx.tick()?;
        match state {
            Realness::Nonreal => dropped += 1,
            Realness::Real => kept.push(root),
            Realness::Unknown => {
                let msg = Message {
                    symbol: "Solve".into(),
                    tag: "real".into(),
                    text: "Could not certify whether a polynomial candidate is real; retaining it."
                        .into(),
                    level: MsgLevel::Warning,
                };
                sink.record(|| {
                    Step::new(
                        StepKind::Note { msg: msg.clone() },
                        vec![root.value.clone()],
                        vec![root.value.clone()],
                        Level::Minor,
                    )
                });
                messages.push(msg);
                kept.push(root);
            }
        }
    }
    sink.record(|| {
        Step::new(
            StepKind::DomainFilter {
                domain: Domain::Reals,
                kept: kept.len(),
                dropped,
            },
            before,
            kept.iter().map(|r| r.value.clone()).collect(),
            Level::Major,
        )
    });
    *roots = kept;
    Ok(messages)
}
