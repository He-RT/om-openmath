//! Finite display coordinates come from actual solution assignments and intervals.
use super::{PlotError, shape::Shape};
use crate::protocol::PlotHighlights;
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol};
use om_eval::{Evaluator, numeric::compile_f64_with_ctx};
use om_solve::{Bound, SolutionSet};
pub(super) fn real(
    value: &Expr,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<f64>, PlotError> {
    ctx.tick()?;
    let n = eval
        .fork_readonly()
        .evaluate(&Expr::call(B::N, [value.clone()]), ctx)?;
    Ok(n.as_number().and_then(om_num::Number::to_f64))
}
fn bound(b: &Bound, eval: &Evaluator, ctx: &Interrupt) -> Result<Option<f64>, PlotError> {
    match b {
        Bound::NegInf => Ok(Some(-1e308)),
        Bound::PosInf => Ok(Some(1e308)),
        Bound::Closed(e) | Bound::Open(e) => real(e, eval, ctx),
    }
}
pub(super) fn from_set(
    set: &SolutionSet,
    shape: &Shape,
    vars: &[Symbol],
    eval: &Evaluator,
    ctx: &Interrupt,
    strict_initial: bool,
) -> Result<Option<PlotHighlights>, PlotError> {
    let mut highlights = PlotHighlights {
        points: vec![],
        shade: vec![],
    };
    if shape.region() {
        match set {
            SolutionSet::All => highlights.shade.push((-1e308, 1e308)),
            SolutionSet::Finite(rows) if rows.is_empty() => {}
            SolutionSet::Region { intervals, .. } => {
                for interval in intervals {
                    let (Some(lo), Some(hi)) = (
                        bound(&interval.lo, eval, ctx)?,
                        bound(&interval.hi, eval, ctx)?,
                    ) else {
                        return Ok(None);
                    };
                    highlights.shade.push((lo, hi));
                }
            }
            _ => return Ok(None),
        }
        return Ok(Some(highlights));
    }
    let SolutionSet::Finite(rows) = set else {
        return Ok(None);
    };
    let lhs = if vars.len() == 1 {
        Some(compile_f64_with_ctx(&shape.equations[0].0, vars, ctx)?)
    } else {
        None
    };
    let mut work = vec![];
    for row in rows {
        ctx.tick()?;
        if !row.constants.is_empty() || row.condition.is_some() || row.rules.len() != vars.len() {
            return Ok(None);
        }
        let mut values = vec![];
        let mut nonreal = false;
        for var in vars {
            let Some((_, value)) = row.rules.iter().find(|(v, _)| v.as_symbol() == Some(*var))
            else {
                return Ok(None);
            };
            if let Some(value) = real(value, eval, ctx)? {
                values.push(value);
            } else {
                nonreal = true;
                break;
            }
        }
        if nonreal {
            if vars.len() == 1 && strict_initial {
                return Ok(None);
            }
            continue;
        }
        let point = if let Some(lhs) = &lhs {
            let y = lhs.eval_with_ctx(&values, &mut work, ctx)?;
            if !y.is_finite() {
                return Ok(None);
            }
            (values[0], y)
        } else {
            (values[0], values[1])
        };
        if !highlights.points.contains(&point) {
            highlights.points.push(point);
        }
    }
    if strict_initial && vars.len() == 2 && highlights.points.is_empty() {
        return Ok(None);
    }
    Ok(Some(highlights))
}
pub(super) fn viewport(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    let values: Vec<_> = values.collect();
    if values.is_empty() {
        return Some((-5.0, 5.0));
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (mut lo, mut hi) = (min - 2.0, max + 2.0);
    if hi - lo < 6.0 {
        let center = min * 0.5 + max * 0.5;
        lo = center - 3.0;
        hi = center + 3.0;
    }
    super::range((lo, hi)).ok().map(|()| (lo, hi))
}
