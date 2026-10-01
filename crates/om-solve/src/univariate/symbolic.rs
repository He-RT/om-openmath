//! Extract the exact zero-root valuation before applying parameter coefficient formulas.
use super::{Candidate, irreducible};
use crate::{Level, SolveError, SolveOptions, Step, StepKind, StepSink};
use om_core::{Expr, add, mul, pow};
use om_num::{Integer, ctx::Interrupt};
use om_simplify::zero::{Tri, is_zero_with};

pub(super) fn roots(
    c: &[Expr],
    original: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    let mut valuation = 0;
    while valuation + 1 < c.len() && is_zero_with(&c[valuation], ctx)? == Tri::Zero {
        valuation += 1;
    }
    let rest = &c[valuation..];
    let quotient = if sink.enabled() && valuation > 0 {
        let mut terms = vec![];
        for (i, a) in rest.iter().enumerate() {
            ctx.tick()?;
            terms.push(mul([
                a.clone(),
                pow(x.clone(), Expr::integer(Integer::from(i))),
            ]));
        }
        let quotient = add(terms);
        let exponent =
            u32::try_from(valuation).expect("invariant: dense degree is limited to 4096");
        sink.record(|| {
            Step::new(
                StepKind::Factor {
                    factors: vec![(x.clone(), exponent), (quotient.clone(), 1)],
                },
                vec![original.clone()],
                vec![mul([
                    pow(x.clone(), Expr::integer(Integer::from(valuation))),
                    quotient.clone(),
                ])],
                Level::Major,
            )
        });
        if rest.len() > 1 {
            sink.record(|| {
                Step::new(
                    StepKind::ZeroProduct,
                    vec![original.clone()],
                    vec![x.clone(), quotient.clone()],
                    Level::Major,
                )
            });
        }
        quotient
    } else {
        original.clone()
    };
    let mut roots = if rest.len() == 1 {
        vec![]
    } else {
        let Some(roots) = irreducible::roots(rest, &quotient, x, opts, ctx, sink)? else {
            return Ok(None);
        };
        roots
    };
    if valuation > 0 {
        roots.push(Candidate {
            value: Expr::int(0),
            multiplicity: u32::try_from(valuation)
                .expect("invariant: dense degree is limited to 4096"),
            key: None,
        });
    }
    Ok(Some(roots))
}
