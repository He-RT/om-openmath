//! Held plot syntax becomes the same request used by SamplePlot.
use super::{PlotError, axis, range};
use crate::protocol::{PlotKind, PlotRequest};
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
pub(super) fn number(e: &Expr, eval: &Evaluator, ctx: &Interrupt) -> Result<f64, PlotError> {
    let value = eval
        .fork_readonly()
        .evaluate(&Expr::call(B::N, [e.clone()]), ctx)?;
    value
        .as_number()
        .and_then(om_num::Number::to_f64)
        .ok_or_else(|| PlotError::Invalid("plot bounds must be finite real numbers".into()))
}
pub(super) fn pair(e: &Expr, eval: &Evaluator, ctx: &Interrupt) -> Result<(f64, f64), PlotError> {
    if !e.is_head(B::LIST) || e.args().len() != 2 {
        return Err(PlotError::Invalid(
            "PlotRange needs a pair of finite endpoints".into(),
        ));
    }
    let r = (
        number(&e.args()[0], eval, ctx)?,
        number(&e.args()[1], eval, ctx)?,
    );
    range(r)?;
    Ok(r)
}
pub(super) fn iterator(
    e: &Expr,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<(String, (f64, f64)), PlotError> {
    if !e.is_head(B::LIST) || e.args().len() != 3 {
        return Err(PlotError::Invalid(
            "plot iterator needs {symbol,min,max}".into(),
        ));
    }
    let s = e.args()[0]
        .as_symbol()
        .ok_or_else(|| PlotError::Invalid("plot axis must be a user symbol".into()))?;
    axis(s.name())?;
    let r = (
        number(&e.args()[1], eval, ctx)?,
        number(&e.args()[2], eval, ctx)?,
    );
    range(r)?;
    Ok((s.name().into(), r))
}
pub(crate) fn from_expr(
    e: &Expr,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<PlotRequest>, PlotError> {
    if let Some(request) = super::extended::from_expr(e, eval, ctx)? {
        return Ok(Some(request));
    }
    let kind = match e.head_symbol() {
        Some(B::PLOT) => PlotKind::Function,
        Some(B::CONTOUR_PLOT) => PlotKind::Implicit,
        _ => return Ok(None),
    };
    let count = if kind == PlotKind::Function { 2 } else { 3 };
    if e.args().len() < count {
        return Err(PlotError::Invalid(
            "missing plot expression or iterator".into(),
        ));
    }
    let args = e.args();
    let (var_x, mut x_range) = iterator(&args[1], eval, ctx)?;
    let (var_y, mut y_range) = if kind == PlotKind::Implicit {
        let (v, r) = iterator(&args[2], eval, ctx)?;
        (Some(v), Some(r))
    } else {
        (None, None)
    };
    for option in &args[count..] {
        if !option.is_head(B::RULE)
            || option.args().len() != 2
            || option.args()[0]
                .as_symbol()
                .is_none_or(|s| s.name() != "PlotRange")
            || kind != PlotKind::Function
        {
            return Err(PlotError::Invalid("unsupported plot option".into()));
        }
        let rhs = &option.args()[1];
        if rhs.is_head(B::LIST) && rhs.args().len() == 2 && rhs.args()[0].is_head(B::LIST) {
            x_range = pair(&rhs.args()[0], eval, ctx)?;
            y_range = Some(pair(&rhs.args()[1], eval, ctx)?);
        } else {
            y_range = Some(pair(rhs, eval, ctx)?);
        }
    }
    let exprs = if args[0].is_head(B::LIST) {
        args[0].args().iter().map(om_format::input_form).collect()
    } else {
        vec![om_format::input_form(&args[0])]
    };
    Ok(Some(PlotRequest {
        options: None,
        kind,
        exprs,
        var_x,
        var_y,
        x_range,
        y_range,
        params: Default::default(),
        points: vec![],
        shade: vec![],
        param_ranges: Default::default(),
        solve: None,
    }))
}
