//! Special reductions precede option-controlled formulas and atomic Root fallback.
use super::{Candidate, extract, formulas, reductions};
use crate::{Level, SolveError, SolveOptions, Step, StepKind, StepSink};
use om_core::{BUILTIN as B, Expr};
use om_num::{Integer, ctx::Interrupt, gcd};
use om_poly::{UPoly, isolate};

pub(super) fn roots(
    c: &[Expr],
    p: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    if let Some(roots) = reductions::roots(c, p, x, opts, ctx, sink)? {
        return Ok(Some(roots));
    }
    if c.iter().all(|a| extract::exact(a).is_some()) {
        let computed = match c.len() {
            4 if opts.cubics => Some(formulas::cardano(c, p, x, ctx, sink)),
            5 if opts.quartics => Some(formulas::ferrari(c, p, x, opts, ctx, sink)),
            _ => None,
        };
        if let Some(computed) = computed {
            match computed {
                Ok(roots) => return Ok(Some(roots)),
                Err(SolveError::Unsupported(_)) => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(Some(root_objects(c, p, ctx, sink)?))
}
pub(super) fn root_objects(
    c: &[Expr],
    p: &Expr,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Candidate>, SolveError> {
    let slot = Expr::call(B::SLOT, [Expr::int(1)]);
    for a in c {
        if extract::depends(a, &slot, ctx)? {
            return Err(SolveError::Unsupported(
                "Root coefficients would capture Slot[1]".into(),
            ));
        }
    }
    let body = reductions::polynomial(c, &slot, ctx)?;
    let function = Expr::call(B::FUNCTION, [body]);
    let mut roots = vec![];
    for index in 1..c.len() {
        ctx.tick()?;
        roots.push(Candidate {
            value: Expr::call(
                B::ROOT,
                [function.clone(), Expr::integer(Integer::from(index))],
            ),
            multiplicity: 1,
            key: None,
        });
    }
    if sink.enabled() {
        let q = c.iter().map(extract::exact).collect::<Option<Vec<_>>>();
        let kind = if let Some(q) = q {
            let mut scale = Integer::ONE;
            for a in &q {
                ctx.tick()?;
                let d = Integer::from(a.denominator().clone());
                scale = (&scale / gcd(&scale, &d)) * d;
            }
            let mut ints = vec![];
            for a in q {
                ctx.tick()?;
                ints.push(a.numerator() * (&scale / Integer::from(a.denominator().clone())));
            }
            let isolated = isolate(&UPoly::new(ints), ctx)?.ok_or_else(|| {
                SolveError::Unsupported("Root real-count isolation failed".into())
            })?;
            StepKind::RootObjects {
                poly: p.clone(),
                real_count: u32::try_from(isolated.len()).expect("invariant: dense degree <=4096"),
            }
        } else {
            StepKind::SplitComponent { factor: p.clone() }
        };
        sink.record(|| {
            Step::new(
                kind,
                vec![p.clone()],
                roots.iter().map(|r| r.value.clone()).collect(),
                Level::Major,
            )
        });
    }
    Ok(roots)
}
