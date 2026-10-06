//! Protocol output is a rendering of actual evaluator results and solver evidence.
mod steps;
pub(crate) mod values;
use crate::{notebook::StatementRecord, protocol::*};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
use om_eval::Evaluator;
use om_solve::{Bound, Domain, Solution, SolutionSet};
use std::collections::BTreeSet;
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
    let presentation = if values::kind(&record.value) != ValueKind::Scalar {
        let query = ValueQuery {
            cell_id: String::new(),
            out_index: record.out_index,
            view_id: record.view_id.clone(),
            path: vec![],
            offset: 0,
            limit: 32,
            column_offset: 0,
            column_limit: 8,
            include_source: false,
        };
        match values::page(record, &query, ctx) {
            Ok(p) => Some(p),
            Err(e) if e.abort().is_some() => return Err(e),
            Err(_) => None,
        }
    } else {
        None
    };
    Ok(OutputItem::Expr {
        out_index: record.out_index,
        input_form,
        modern_form,
        latex: om_format::latex(&record.value),
        presentation,
    })
}

pub(crate) fn expression_view(value: &Expr) -> ExpressionView {
    ExpressionView {
        input_form: om_format::input_form(value),
        modern_form: om_format::modern_form(value),
        latex: om_format::latex(value),
    }
}
fn numeric(value: &Expr, eval: &mut Evaluator, ctx: &Interrupt, digits: i64) -> Option<Expr> {
    let projection = if value.free_symbols().is_empty() {
        om_simplify::algebra::expand_with(value, ctx)
            .ok()
            .flatten()
            .unwrap_or_else(|| value.clone())
    } else {
        value.clone()
    };
    let result = eval
        .evaluate(&Expr::call(B::N, [projection, Expr::int(digits)]), ctx)
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
fn binding(
    var: &Expr,
    value: &Expr,
    display: &Expr,
    eval: &mut Evaluator,
    ctx: &Interrupt,
) -> BindingView {
    let root_index = if value.is_head(B::ROOT) {
        value
            .args()
            .get(1)
            .and_then(|i| i.as_number())
            .and_then(om_num::Number::to_f64)
            .filter(|i| *i >= 1.0 && *i <= u32::MAX as f64 && i.fract() == 0.0)
            .map(|i| i as u32)
    } else {
        None
    };
    let radicals = root_index.and_then(|_| {
        let converted = eval
            .evaluate(
                &Expr::call(om_core::Symbol::intern("ToRadicals"), [value.clone()]),
                ctx,
            )
            .ok()?;
        if converted == *value
            || converted.is_head(om_core::Symbol::intern("ToRadicals"))
            || contains_root(&converted)
        {
            None
        } else {
            Some(expression_view(&converted))
        }
    });
    BindingView {
        var: om_format::input_form(var),
        latex: om_format::latex(display),
        input_form: om_format::input_form(value),
        modern_form: om_format::modern_form(value),
        numeric: exact(value)
            .then(|| numeric(value, eval, ctx, 20))
            .flatten()
            .map(|n| om_format::input_form(&n)),
        var_latex: Some(om_format::latex(var)),
        root_index,
        radicals,
    }
}
fn contains_root(value: &Expr) -> bool {
    let mut work = vec![value];
    while let Some(e) = work.pop() {
        if e.is_head(B::ROOT) {
            return true;
        }
        if let ExprKind::Normal(n) = e.kind() {
            work.push(&n.head);
            work.extend(n.args.iter());
        }
    }
    false
}
fn display_aliases(rows: &[Solution], vars: &[Expr]) -> Vec<(Expr, Expr)> {
    let mut used = BTreeSet::new();
    for e in vars
        .iter()
        .chain(rows.iter().flat_map(|r| r.rules.iter().map(|(_, v)| v)))
        .chain(rows.iter().filter_map(|r| r.condition.as_ref()))
    {
        used.extend(e.free_symbols().into_iter().map(|s| s.name().to_string()));
    }
    let mut aliases: Vec<(Expr, Expr)> = vec![];
    for (constant, _) in rows.iter().flat_map(|r| r.constants.iter()) {
        if aliases.iter().any(|(c, _)| c == constant) {
            continue;
        }
        let mut index = 0;
        loop {
            let name = match index {
                0 => "k".into(),
                1 => "m".into(),
                2 => "n".into(),
                i => format!("k{}", i - 1),
            };
            index += 1;
            if used.insert(name.clone()) {
                aliases.push((constant.clone(), Expr::sym(om_core::Symbol::intern(&name))));
                break;
            }
        }
    }
    aliases
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
fn condition_latex(e: &Expr) -> String {
    if e.is_head(B::ELEMENT) && e.args().len() == 2 {
        let domain = match e.args()[1].as_symbol() {
            Some(B::INTEGERS) => Some("Z"),
            Some(B::REALS) => Some("R"),
            Some(B::COMPLEXES) => Some("C"),
            Some(B::RATIONALS) => Some("Q"),
            _ => None,
        };
        if let Some(domain) = domain {
            return format!(
                "{} \\in \\mathbb{{{domain}}}",
                om_format::latex(&e.args()[0])
            );
        }
    }
    if e.is_head(B::AND) {
        return e
            .args()
            .iter()
            .map(condition_latex)
            .collect::<Vec<_>>()
            .join(" \\land ");
    }
    om_format::latex(e)
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
            numeric(e, eval, ctx, 10).and_then(|n| n.as_number().and_then(om_num::Number::to_f64)),
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
            let aliases = display_aliases(rows, vars);
            if rows.is_empty() {
                view.kind = SolutionKind::None;
            }
            for row in rows {
                let solution = SolutionView {
                    bindings: row
                        .rules
                        .iter()
                        .map(|(v, a)| binding(v, a, &a.replace_all(&aliases), eval, ctx))
                        .collect(),
                    condition_latex: condition(row)
                        .as_ref()
                        .map(|c| om_format::latex(&c.replace_all(&aliases))),
                    condition_display_latex: condition(row)
                        .as_ref()
                        .map(|c| condition_latex(&c.replace_all(&aliases))),
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_projection_of_recovered_real_coordinate_is_bounded_and_real() {
        let value = om_core::canonicalize(
            &om_parse::parse_expr(
                "((-I/2*Sqrt[3]-5/2)^3/126)-((-I/2*Sqrt[3]-5/2)^2/21)-8*(-I/2*Sqrt[3]-5/2)/21+7/18",
                om_parse::Dialect::Wolfram,
            )
            .unwrap(),
        );
        let ctx = Interrupt {
            steps_left: std::cell::Cell::new(4096),
            ..Interrupt::default()
        };
        let mut eval = Evaluator::new().fork_readonly();
        let value = numeric(&value, &mut eval, &ctx, 20).unwrap();
        let number = value.as_number().unwrap();
        assert_eq!(number.to_complex_f64(), (1.0, 0.0));
        assert!(!matches!(number, om_num::Number::Complex(_)));
        assert!(eval.history.is_empty());
    }
}
