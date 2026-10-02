//! Protocol output is a rendering of actual evaluator results and solver evidence.
mod steps;
use crate::{notebook::StatementRecord, protocol::*};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
use om_eval::Evaluator;
use om_solve::{Bound, Domain, Solution, SolutionSet};
pub(crate) use steps::render as render_steps;

pub(crate) fn pack(
    record: &StatementRecord,
    eval: &Evaluator,
    ctx: &Interrupt,
    auto_plot: bool,
) -> Result<OutputItem, crate::plot::PlotError> {
    if let Some(request) = crate::plot::from_expr(&record.value, eval, ctx)? {
        let data = crate::plot::sample(&request, eval, ctx)?;
        return Ok(OutputItem::Plot { request, data });
    }
    let input_form = om_format::input_form(&record.value);
    let modern_form = om_format::modern_form(&record.value);
    if let Some(result) = &record.solver {
        let mut readonly = eval.fork_readonly();
        if let Some(view) = view(&result.set, &result.vars, &mut readonly, ctx) {
            return Ok(OutputItem::Solutions {
                out_index: record.out_index,
                input_form,
                modern_form,
                view,
                steps: record.steps.as_ref().map(steps::render),
                plot: if auto_plot {
                    crate::plot::automatic(result, eval, ctx)?
                } else {
                    None
                },
            });
        }
    }
    Ok(OutputItem::Expr {
        out_index: record.out_index,
        input_form,
        modern_form,
        latex: om_format::latex(&record.value),
    })
}

fn numeric(value: &Expr, eval: &mut Evaluator, ctx: &Interrupt) -> Option<Expr> {
    let result = eval
        .evaluate(&Expr::call(B::N, [value.clone(), Expr::int(10)]), ctx)
        .ok()?;
    result.as_number().map(|_| result.clone())
}
fn exact(value: &Expr) -> bool {
    let mut work = vec![value];
    while let Some(e) = work.pop() {
        if e.as_number().is_some_and(|n| !n.is_exact()) {
            return false;
        }
        if let ExprKind::Normal(n) = e.kind() {
            work.push(&n.head);
            work.extend(n.args.iter());
        }
    }
    true
}
fn binding(var: &Expr, value: &Expr, eval: &mut Evaluator, ctx: &Interrupt) -> BindingView {
    BindingView {
        var: om_format::input_form(var),
        latex: om_format::latex(value),
        input_form: om_format::input_form(value),
        modern_form: om_format::modern_form(value),
        numeric: exact(value)
            .then(|| numeric(value, eval, ctx))
            .flatten()
            .map(|n| om_format::input_form(&n)),
    }
}
fn condition(s: &Solution) -> Option<Expr> {
    let mut cond: Vec<_> = s.condition.iter().cloned().collect();
    cond.extend(s.constants.iter().map(|(c, d)| {
        Expr::call(
            B::ELEMENT,
            [
                c.clone(),
                Expr::sym(match d {
                    Domain::Complexes => B::COMPLEXES,
                    Domain::Reals => B::REALS,
                    Domain::Integers => B::INTEGERS,
                    Domain::Rationals => B::RATIONALS,
                }),
            ],
        )
    }));
    match cond.len() {
        0 => None,
        1 => cond.pop(),
        _ => Some(Expr::call(B::AND, cond)),
    }
}
fn endpoint(
    bound: &Bound,
    eval: &mut Evaluator,
    ctx: &Interrupt,
) -> (Option<String>, Option<f64>, bool) {
    match bound {
        Bound::NegInf | Bound::PosInf => (None, None, false),
        Bound::Open(e) | Bound::Closed(e) => (
            Some(om_format::latex(e)),
            numeric(e, eval, ctx).and_then(|n| n.as_number().and_then(om_num::Number::to_f64)),
            matches!(bound, Bound::Closed(_)),
        ),
    }
}
fn view(
    set: &SolutionSet,
    vars: &[Expr],
    eval: &mut Evaluator,
    ctx: &Interrupt,
) -> Option<SolutionSetView> {
    let mut view = SolutionSetView {
        kind: SolutionKind::Finite,
        vars: vars.iter().map(om_format::input_form).collect(),
        solutions: vec![],
        region_latex: None,
        intervals: vec![],
    };
    match set {
        SolutionSet::All => view.kind = SolutionKind::All,
        SolutionSet::Finite(rows) => {
            if rows.is_empty() {
                view.kind = SolutionKind::None;
            }
            for row in rows {
                let solution = SolutionView {
                    bindings: row
                        .rules
                        .iter()
                        .map(|(v, a)| binding(v, a, eval, ctx))
                        .collect(),
                    condition_latex: condition(row).as_ref().map(om_format::latex),
                    verified: row.verification.into(),
                };
                for _ in 0..row.multiplicity {
                    view.solutions.push(solution.clone());
                }
            }
        }
        SolutionSet::Region { cond, intervals } => {
            view.kind = SolutionKind::Region;
            view.region_latex = Some(om_format::latex(cond));
            for i in intervals {
                let (lo, lo_value, lo_closed) = endpoint(&i.lo, eval, ctx);
                let (hi, hi_value, hi_closed) = endpoint(&i.hi, eval, ctx);
                view.intervals.push(IntervalView {
                    lo,
                    hi,
                    lo_closed,
                    hi_closed,
                    lo_value,
                    hi_value,
                });
            }
        }
        // An unsupported callback never enters SolverResult; preserve this exhaustive guard.
        SolutionSet::Unevaluated => return None,
    }
    Some(view)
}
