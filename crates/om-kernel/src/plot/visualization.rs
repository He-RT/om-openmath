//! Automatic plots and serialized slider refreshes retain genuine solving source.
use super::{
    PlotError, highlights,
    shape::{self, Shape},
};
use crate::protocol::*;
use om_core::{Expr, Interrupt, Symbol};
use om_eval::{Evaluator, SolverResult, numeric::compile_f64_with_ctx};
use om_solve::{Domain, SolutionSet, SolveOptions};
use std::collections::BTreeMap;
fn solve_error(e: om_solve::SolveError) -> PlotError {
    match e {
        om_solve::SolveError::Abort(e) => PlotError::Abort(e),
        e => PlotError::Invalid(e.to_string()),
    }
}
fn locals(vars: &[Symbol], params: &BTreeMap<String, f64>) -> Vec<(Symbol, Option<Expr>)> {
    vars.iter()
        .map(|s| (*s, None))
        .chain(params.iter().map(|(name, value)| {
            (
                Symbol::intern(name),
                Some(Expr::number(om_num::Number::Rational(
                    om_num::Rational::try_from(*value).expect(
                        "invariant: validated finite parameter has an exact binary rational",
                    ),
                ))),
            )
        }))
        .collect()
}
fn substitute(
    source: &Expr,
    vars: &[Symbol],
    params: &BTreeMap<String, f64>,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<Expr, PlotError> {
    eval.prepare_numeric(source, &locals(vars, params), ctx)
        .map_err(PlotError::from)
}
fn defaults(
    source: &Expr,
    vars: &[Symbol],
    domain: Domain,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<BTreeMap<String, f64>>, PlotError> {
    let parameters = shape::parameters(source, vars);
    let mut params: BTreeMap<_, _> = parameters.keys().map(|s| (s.clone(), 1.0)).collect();
    let axes: Vec<_> = vars.iter().map(|s| Expr::sym(*s)).collect();
    let prepared =
        om_solve::normalize::normalize(source, Some(&axes), domain, ctx, &mut om_solve::NoSteps)
            .map_err(solve_error)?;
    for (name, symbol) in parameters {
        ctx.tick()?;
        let mut invalid = false;
        for branch in &prepared.branches {
            for restriction in &branch.exclusions {
                if !restriction.value.free_symbols().contains(&symbol) {
                    continue;
                }
                let value = om_core::canonicalize(&substitute(
                    &restriction.value,
                    vars,
                    &params,
                    eval,
                    ctx,
                )?);
                if value.free_symbols().is_empty()
                    && highlights::real(&value, eval, ctx)?.is_none_or(|v| v == 0.0)
                {
                    invalid = true;
                }
            }
        }
        if invalid {
            params.insert(name, 2.0);
        }
    }
    let substituted = substitute(source, vars, &params, eval, ctx)?;
    let check = om_solve::normalize::normalize(
        &substituted,
        Some(&axes),
        domain,
        ctx,
        &mut om_solve::NoSteps,
    )
    .map_err(solve_error)?;
    if check.unsupported || check.branches.is_empty() && !prepared.branches.is_empty() {
        return Ok(None);
    }
    Ok(Some(params))
}
fn resolve_set(
    source: &Expr,
    vars: &[Symbol],
    domain: Domain,
    region: bool,
    ctx: &Interrupt,
) -> Result<SolutionSet, PlotError> {
    let axes: Vec<_> = vars.iter().map(|s| Expr::sym(*s)).collect();
    let options = SolveOptions {
        domain,
        record_steps: false,
        ..Default::default()
    };
    let result = if region {
        om_solve::reduce_with_options(source, &axes, &options, ctx)
    } else {
        om_solve::nsolve_with_options(source, &axes, om_num::Precision::Machine, &options, ctx)
    }
    .map_err(solve_error)?;
    if matches!(result.set, SolutionSet::Unevaluated) {
        return Err(PlotError::Invalid(
            "plot highlight solving is unsupported for these parameters".into(),
        ));
    }
    Ok(result.set)
}
pub(crate) fn automatic(
    result: &SolverResult,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<PlotRequest>, PlotError> {
    // Unsupported optional visualization is not a failure of the completed solver call.
    match build(result, eval, ctx) {
        Err(e) if e.abort().is_none() => Ok(None),
        r => r,
    }
}
fn build(
    result: &SolverResult,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<PlotRequest>, PlotError> {
    if !(1..=2).contains(&result.vars.len()) {
        return Ok(None);
    }
    let Some(vars) = result
        .vars
        .iter()
        .map(Expr::as_symbol)
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(None);
    };
    let Some(shape) = Shape::parse(&result.source, vars.len(), ctx)? else {
        return Ok(None);
    };
    let Some(params) = defaults(&result.source, &vars, result.domain, eval, ctx)? else {
        return Ok(None);
    };
    let substituted = substitute(&result.source, &vars, &params, eval, ctx)?;
    let Some(concrete) = Shape::parse(&substituted, vars.len(), ctx)? else {
        return Ok(None);
    };
    let set = if params.is_empty() {
        result.set.clone()
    } else {
        resolve_set(&substituted, &vars, result.domain, shape.region(), ctx)?
    };
    let Some(highlights) = highlights::from_set(&set, &concrete, &vars, eval, ctx, true)? else {
        return Ok(None);
    };
    // Check each raw curve now, so an unsupported shape does not emit an unusable request.
    for curve in concrete.curves(vars.len()) {
        compile_f64_with_ctx(&curve, &vars, ctx)?;
    }
    let extents = if shape.region() {
        highlights
            .shade
            .iter()
            .flat_map(|(a, b)| [*a, *b])
            .filter(|v| v.abs() < 1e308)
            .collect::<Vec<_>>()
    } else {
        highlights.points.iter().map(|p| p.0).collect()
    };
    let Some(x_range) = highlights::viewport(extents.into_iter()) else {
        return Ok(None);
    };
    let y_range = if vars.len() == 2 {
        let Some(r) = highlights::viewport(highlights.points.iter().map(|p| p.1)) else {
            return Ok(None);
        };
        Some(r)
    } else {
        None
    };
    Ok(Some(PlotRequest {
        kind: shape.kind(vars.len()),
        exprs: shape
            .curves(vars.len())
            .iter()
            .map(om_format::input_form)
            .collect(),
        var_x: vars[0].name().into(),
        var_y: vars.get(1).map(|s| s.name().into()),
        x_range,
        y_range,
        param_ranges: params.keys().map(|s| (s.clone(), (-5.0, 5.0))).collect(),
        params,
        points: highlights.points,
        shade: highlights.shade,
        solve: Some(PlotSolveSource {
            source: om_format::input_form(&result.source),
            domain: shape::from_domain(result.domain),
        }),
    }))
}
fn parameter_domain(
    source: &Expr,
    vars: &[Symbol],
    domain: Domain,
    params: &BTreeMap<String, f64>,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<(), PlotError> {
    let axes: Vec<_> = vars.iter().map(|s| Expr::sym(*s)).collect();
    let prepared =
        om_solve::normalize::normalize(source, Some(&axes), domain, ctx, &mut om_solve::NoSteps)
            .map_err(solve_error)?;
    if prepared.branches.is_empty() {
        return Ok(());
    }
    for branch in prepared.branches {
        let mut valid = true;
        for exclusion in branch.exclusions {
            let value =
                om_core::canonicalize(&substitute(&exclusion.value, vars, params, eval, ctx)?);
            if value.free_symbols().is_empty()
                && highlights::real(&value, eval, ctx)?.is_none_or(|v| v == 0.0)
            {
                valid = false;
                break;
            }
        }
        if valid {
            return Ok(());
        }
    }
    Err(PlotError::Invalid(
        "plot parameter values violate all original source branches".into(),
    ))
}
pub(super) fn refresh(
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<PlotHighlights>, PlotError> {
    let Some(solve) = &r.solve else {
        return Ok(
            (!r.points.is_empty() || !r.shade.is_empty()).then(|| PlotHighlights {
                points: r.points.clone(),
                shade: r.shade.clone(),
            }),
        );
    };
    let source = super::parse(&solve.source)?;
    let mut vars = vec![super::axis(&r.var_x)?];
    if let Some(y) = &r.var_y {
        vars.push(super::axis(y)?);
    }
    let Some(shape) = Shape::parse(&source, vars.len(), ctx)? else {
        return Err(PlotError::Invalid("unsupported plot solving source".into()));
    };
    if shape.kind(vars.len()) != r.kind
        || shape
            .curves(vars.len())
            .iter()
            .map(om_format::input_form)
            .collect::<Vec<_>>()
            != r.exprs
    {
        return Err(PlotError::Invalid(
            "plot curves must match their transported solving source".into(),
        ));
    }
    if shape::parameters(&source, &vars).keys().ne(r.params.keys()) {
        return Err(PlotError::Invalid(
            "all transported source parameters must be supplied exactly once".into(),
        ));
    }
    parameter_domain(
        &source,
        &vars,
        shape::to_domain(solve.domain),
        &r.params,
        eval,
        ctx,
    )?;
    let source = substitute(&source, &vars, &r.params, eval, ctx)?;
    let Some(concrete) = Shape::parse(&source, vars.len(), ctx)? else {
        return Err(PlotError::Invalid("invalid substituted plot source".into()));
    };
    let set = resolve_set(
        &source,
        &vars,
        shape::to_domain(solve.domain),
        shape.region(),
        ctx,
    )?;
    highlights::from_set(&set, &concrete, &vars, eval, ctx, false)?
        .ok_or_else(|| {
            PlotError::Invalid("plot highlights need finite real complete assignments".into())
        })
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn transported_parameter_solving_shares_the_sampling_budget_without_writing_history() {
        let mut ev = Evaluator::new();
        let ctx = Interrupt::default();
        let raw = om_parse::parse_expr("Solve[x^2==a,x]", om_parse::Dialect::Wolfram).unwrap();
        ev.evaluate_statement(&raw, &ctx).unwrap();
        let result = ev.take_solver_result().unwrap();
        let request = automatic(&result, &ev, &ctx).unwrap().unwrap();
        let history = ev.history.clone();
        let ctx = Interrupt::default();
        let before = ctx.steps_left.get();
        super::super::sample(&request, &ev, &ctx).unwrap();
        let cost = before - ctx.steps_left.get();
        assert!(cost > 2000);
        for budget in [0, 100, cost - 1] {
            let ctx = Interrupt {
                steps_left: Cell::new(budget),
                ..Interrupt::default()
            };
            let error = super::super::sample(&request, &ev, &ctx).unwrap_err();
            assert_eq!(error.abort(), Some(om_core::Abort::Budget));
            assert_eq!(ctx.steps_left.get(), 0);
        }
        let ctx = Interrupt {
            steps_left: Cell::new(cost),
            ..Interrupt::default()
        };
        assert!(super::super::sample(&request, &ev, &ctx).is_ok());
        assert_eq!(ev.history, history);
    }
}
