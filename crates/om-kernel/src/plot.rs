//! Real readonly preparation, compilation and finite plot geometry.
mod dataset;
mod explicit;
mod extended;
mod field;
mod function;
mod grid;
mod highlights;
mod implicit;
mod parametric;
mod predicate;
mod shape;
mod visualization;
use crate::protocol::*;
pub(crate) use explicit::from_expr;
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol};
use om_eval::{Evaluator, numeric::compile_f64_with_ctx};
pub(crate) use visualization::automatic;

#[derive(Debug, thiserror::Error)]
pub(crate) enum PlotError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Compile(#[from] om_eval::numeric::CompileError),
    #[error(transparent)]
    Evaluation(#[from] om_eval::EvalError),
    #[error(transparent)]
    Abort(#[from] om_core::Abort),
}
impl PlotError {
    pub(crate) fn abort(&self) -> Option<om_core::Abort> {
        match self {
            Self::Abort(e)
            | Self::Compile(om_eval::numeric::CompileError::Abort(e))
            | Self::Evaluation(om_eval::EvalError::Abort(e)) => Some(e.clone()),
            _ => None,
        }
    }
}
pub(super) type Point = (f64, f64);
pub(super) fn axis(name: &str) -> Result<Symbol, PlotError> {
    let e = parse(name)?;
    let s = e
        .as_symbol()
        .filter(|s| !om_core::builtins::names().contains(&s.name()) && Evaluator::doc(*s).is_none())
        .ok_or_else(|| {
            PlotError::Invalid("axes and parameters must be user symbol names".into())
        })?;
    Ok(s)
}
fn parse(source: &str) -> Result<Expr, PlotError> {
    om_parse::parse_expr(source, om_parse::Dialect::Wolfram)
        .map_err(|e| PlotError::Invalid(format!("invalid plot source: {e:?}")))
}
pub(super) fn range(r: (f64, f64)) -> Result<(), PlotError> {
    if !r.0.is_finite() || !r.1.is_finite() || r.0 >= r.1 || !(r.1 - r.0).is_finite() {
        return Err(PlotError::Invalid(
            "ranges need finite ordered endpoints and finite positive width".into(),
        ));
    }
    Ok(())
}
pub(crate) fn sample(
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    ctx.tick()?;
    if r.options.is_some() || !matches!(r.kind, PlotKind::Function | PlotKind::Implicit) {
        return extended::sample(r, eval, ctx);
    }
    range(r.x_range)?;
    if let Some(y) = r.y_range {
        range(y)?;
    }
    if r.exprs.is_empty() || r.exprs.len() > 64 {
        return Err(PlotError::Invalid(
            "one to 64 plot expressions are required".into(),
        ));
    }
    let x = axis(&r.var_x)?;
    let mut vars = vec![x];
    match (r.kind, r.var_y.as_deref(), r.y_range) {
        (PlotKind::Implicit, Some(y), Some(_)) => {
            let y = axis(y)?;
            if x == y {
                return Err(PlotError::Invalid("implicit axes must differ".into()));
            }
            vars.push(y);
        }
        (PlotKind::Implicit, _, _) => {
            return Err(PlotError::Invalid(
                "implicit plots need two axes and ranges".into(),
            ));
        }
        (PlotKind::Function, None, _) => {}
        _ => return Err(PlotError::Invalid("function plots need one axis".into())),
    }
    let mut locals: Vec<_> = vars.iter().map(|s| (*s, None)).collect();
    for (name, value) in &r.params {
        ctx.tick()?;
        let s = axis(name)?;
        if vars.contains(&s) || !value.is_finite() {
            return Err(PlotError::Invalid(
                "parameters must be finite and distinct from axes".into(),
            ));
        }
        locals.push((
            s,
            Some(Expr::number(om_num::Number::Real(om_num::Real::Machine(
                *value,
            )))),
        ));
    }
    for (name, bounds) in &r.param_ranges {
        ctx.tick()?;
        axis(name)?;
        range(*bounds)?;
    }
    if r.points
        .iter()
        .any(|p| !p.0.is_finite() || !p.1.is_finite())
        || r.shade
            .iter()
            .any(|p| !p.0.is_finite() || !p.1.is_finite() || p.0 > p.1)
    {
        return Err(PlotError::Invalid(
            "points and shading must be finite and ordered".into(),
        ));
    }
    let mut compiled = vec![];
    for source in &r.exprs {
        ctx.tick()?;
        let raw = parse(source)?;
        let raw = if r.kind == PlotKind::Implicit && raw.is_head(B::EQUAL) && raw.args().len() == 2
        {
            Expr::call(
                B::PLUS,
                [
                    raw.args()[0].clone(),
                    Expr::call(B::TIMES, [Expr::int(-1), raw.args()[1].clone()]),
                ],
            )
        } else {
            raw
        };
        let prepared = eval.prepare_numeric(&raw, &locals, ctx)?;
        compiled.push(compile_f64_with_ctx(&prepared, &vars, ctx)?);
    }
    let highlights = visualization::refresh(r, eval, ctx)?;
    let mut data = match r.kind {
        PlotKind::Function => function::sample(r, &compiled, ctx),
        PlotKind::Implicit => implicit::sample(r, &compiled, ctx),
        _ => return Err(PlotError::Invalid("未接入的绘图类型".into())),
    }?;
    data.highlights = highlights;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    fn request(kind: PlotKind) -> PlotRequest {
        PlotRequest {
            options: None,
            kind,
            exprs: vec![
                if kind == PlotKind::Implicit {
                    "x^2+y^2==1"
                } else {
                    "Sin[x]"
                }
                .into(),
            ],
            var_x: "x".into(),
            var_y: (kind == PlotKind::Implicit).then(|| "y".into()),
            x_range: (-2.0, 2.0),
            y_range: (kind == PlotKind::Implicit).then_some((-2.0, 2.0)),
            params: Default::default(),
            points: vec![],
            shade: vec![],
            param_ranges: Default::default(),
            solve: None,
        }
    }
    #[test]
    fn preparation_instructions_sampling_and_stitching_share_one_budget() {
        for kind in [PlotKind::Function, PlotKind::Implicit] {
            let r = request(kind);
            let ev = Evaluator::new();
            let ctx = Interrupt::default();
            let before = ctx.steps_left.get();
            sample(&r, &ev, &ctx).unwrap();
            let cost = before - ctx.steps_left.get();
            assert!(cost > 1000);
            for budget in [0, 100, cost - 1] {
                let ctx = Interrupt {
                    steps_left: Cell::new(budget),
                    ..Interrupt::default()
                };
                let err = sample(&r, &ev, &ctx).unwrap_err();
                assert_eq!(err.abort(), Some(om_core::Abort::Budget));
                assert_eq!(ctx.steps_left.get(), 0);
            }
            let ctx = Interrupt {
                steps_left: Cell::new(cost),
                ..Interrupt::default()
            };
            assert!(sample(&r, &ev, &ctx).is_ok());
        }
    }
}
