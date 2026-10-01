//! One shared chart makes conjunction, union and endpoint coalescing exact.
use super::model::{self, Constraint, Point};
use crate::{Bound, Interval, Level, Sign, SolveError, Step, StepKind, StepSink};
use om_core::{BUILTIN as B, Expr};
use om_num::ctx::Interrupt;
pub(super) fn intervals(
    branches: &[Vec<Constraint>],
    points: &[Point],
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Interval>, SolveError> {
    let samples = model::samples(points, ctx)?;
    let mut accepted = vec![false; 2 * points.len() + 1];
    for branch in branches {
        let mut cells = vec![true; accepted.len()];
        for c in branch {
            let signs = samples
                .iter()
                .map(|q| Ok((model::sign(&c.n, q, ctx)?, model::sign(&c.d, q, ctx)?)))
                .collect::<Result<Vec<_>, SolveError>>()?;
            for (i, (n, d)) in signs.iter().enumerate() {
                cells[2 * i] &= *d != 0 && model::holds(c.relation, n * d);
            }
            for (i, p) in points.iter().enumerate() {
                ctx.tick()?;
                let n = if model::zero_at(&c.n, p, ctx)? {
                    0
                } else {
                    signs[i].0
                };
                let d = if model::zero_at(&c.d, p, ctx)? {
                    0
                } else {
                    signs[i].1
                };
                cells[2 * i + 1] &= d != 0 && model::holds(c.relation, n * d);
            }
            sink.record(|| {
                Step::new(
                    StepKind::SignChart {
                        points: points.iter().map(|p| p.expr.clone()).collect(),
                        signs: signs
                            .iter()
                            .map(|(n, d)| match n * d {
                                -1 => Sign::Negative,
                                0 => Sign::Zero,
                                _ => Sign::Positive,
                            })
                            .collect(),
                    },
                    vec![c.original.clone()],
                    vec![],
                    Level::Major,
                )
            });
        }
        for (all, branch) in accepted.iter_mut().zip(cells) {
            *all |= branch;
        }
    }
    let mut intervals = vec![];
    let mut i = 0;
    while i < accepted.len() {
        ctx.tick()?;
        if !accepted[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i + 1 < accepted.len() && accepted[i + 1] {
            i += 1;
        }
        let end = i;
        let lo = if start == 0 {
            Bound::NegInf
        } else if start % 2 == 0 {
            Bound::Open(points[start / 2 - 1].expr.clone())
        } else {
            Bound::Closed(points[start / 2].expr.clone())
        };
        let hi = if end == 2 * points.len() {
            Bound::PosInf
        } else if end % 2 == 0 {
            Bound::Open(points[end / 2].expr.clone())
        } else {
            Bound::Closed(points[end / 2].expr.clone())
        };
        intervals.push(Interval { lo, hi });
        i += 1;
    }
    Ok(intervals)
}
pub(super) fn boolean(intervals: &[Interval], x: &Expr) -> Expr {
    let branches = intervals
        .iter()
        .map(|iv| {
            if let (Bound::Closed(lo), Bound::Closed(hi)) = (&iv.lo, &iv.hi)
                && lo == hi
            {
                return Expr::call(B::EQUAL, [x.clone(), lo.clone()]);
            }
            let lo = match &iv.lo {
                Bound::Open(v) => Some((v.clone(), B::LESS)),
                Bound::Closed(v) => Some((v.clone(), B::LESS_EQUAL)),
                _ => None,
            };
            let hi = match &iv.hi {
                Bound::Open(v) => Some((B::LESS, v.clone())),
                Bound::Closed(v) => Some((B::LESS_EQUAL, v.clone())),
                _ => None,
            };
            match (lo, hi) {
                (None, None) => Expr::sym(B::TRUE),
                (Some((v, rel)), None) => Expr::call(
                    if rel == B::LESS {
                        B::GREATER
                    } else {
                        B::GREATER_EQUAL
                    },
                    [x.clone(), v],
                ),
                (None, Some((rel, v))) => Expr::call(rel, [x.clone(), v]),
                (Some((a, left)), Some((right, b))) => Expr::call(
                    B::INEQUALITY,
                    [a, Expr::sym(left), x.clone(), Expr::sym(right), b],
                ),
            }
        })
        .collect::<Vec<_>>();
    match branches.len() {
        0 => Expr::sym(B::FALSE),
        1 => branches[0].clone(),
        _ => Expr::call(B::OR, branches),
    }
}
