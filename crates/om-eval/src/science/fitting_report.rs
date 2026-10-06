//! Actual model/residual metrics and explicit unavailable machine summaries; no invented confidence data.
use super::fitting_data::{Data, Parameters};
use super::*;
use om_core::Symbol;
pub(super) struct Progress {
    pub point: Vec<f64>,
    pub iterations: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub evaluations: usize,
    pub damping: Option<f64>,
    pub gradient: Option<f64>,
    pub termination: &'static str,
    pub model_evaluations: usize,
    pub jacobian_evaluations: usize,
}
pub(super) struct Outcome {
    pub progress: Progress,
    pub predicted: Vec<f64>,
    pub nonlinear: bool,
}
fn metric(value: f64, nonzero: bool) -> Result<(Expr, Expr), EvalError> {
    if !value.is_finite() {
        return Ok((Expr::sym(B::NULL), Expr::string("overflow")));
    }
    if value == 0. && nonzero {
        return Ok((Expr::sym(B::NULL), Expr::string("underflow")));
    }
    Ok((real(value)?, Expr::string("finite")))
}
pub(super) fn build(
    raw: &Expr,
    vars: &[Symbol],
    params: &Parameters,
    data: &Data,
    outcome: Outcome,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    let Outcome {
        progress,
        predicted,
        nonlinear,
    } = outcome;
    let mut residuals = vec![];
    let mut norm = 0_f64;
    let mut rows = vec![];
    for ((x, y), prediction) in data.inputs.iter().zip(&data.observed).zip(&predicted) {
        ctx.tick()?;
        let residual = prediction - y;
        if !residual.is_finite() {
            return Err(error("拟合残差超出有限机器范围"));
        }
        norm = norm.hypot(residual);
        residuals.push(real(residual)?);
        rows.push(record([
            (
                "input",
                if vars.len() == 1 {
                    real(x[0])?
                } else {
                    list(x.iter().map(|x| real(*x)).collect::<Result<Vec<_>, _>>()?)
                },
            ),
            ("observed", real(*y)?),
            ("predicted", real(*prediction)?),
            ("residual", real(residual)?),
        ]));
    }
    let (sse, sse_status) = metric(norm * norm, norm != 0.)?;
    let (rms, rms_status) = metric(norm / (data.inputs.len() as f64).sqrt(), norm != 0.)?;
    let mut model = raw.clone();
    let mut bindings = vec![];
    for (s, p) in params.names.iter().zip(&progress.point) {
        ctx.tick()?;
        let p = real(*p)?;
        model = super::symbolic_integration::substitute(&model, &Expr::sym(*s), &p, ctx)?;
        bindings.push(Expr::call(B::RULE, [Expr::string(s.name()), p]));
    }
    let model = Expr::call(
        B::FITTED_MODEL_DATA,
        [
            list(vars.iter().map(|s| Expr::sym(*s))),
            model,
            Expr::string("machine"),
        ],
    );
    Ok(record([
        ("model", model),
        ("parameters", Expr::call(B::RECORD, bindings)),
        (
            "point",
            list(
                progress
                    .point
                    .into_iter()
                    .map(real)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ),
        (
            "parameter_names",
            list(params.names.iter().map(|s| Expr::string(s.name()))),
        ),
        (
            "variables",
            list(vars.iter().map(|s| Expr::string(s.name()))),
        ),
        ("converged", Expr::sym(B::TRUE)),
        (
            "method",
            Expr::string(if nonlinear {
                "levenberg_marquardt"
            } else {
                "linear_qr"
            }),
        ),
        ("termination", Expr::string(progress.termination)),
        (
            "guarantee",
            Expr::string(if nonlinear {
                "numerical_local_fit"
            } else {
                "numerical_linear_least_squares"
            }),
        ),
        ("precision", Expr::string("machine")),
        ("residuals", list(residuals)),
        ("residual_norm", real(norm)?),
        ("sum_squares", sse),
        ("sum_squares_status", sse_status),
        ("rms", rms),
        ("rms_status", rms_status),
        ("sample_count", Expr::int(data.inputs.len() as i64)),
        ("parameter_count", Expr::int(params.names.len() as i64)),
        (
            "degrees_of_freedom",
            Expr::int((data.inputs.len() - params.names.len()) as i64),
        ),
        ("numerical_rank", Expr::int(params.names.len() as i64)),
        ("rank_tolerance", real(1e-12)?),
        ("iterations", Expr::int(progress.iterations as i64)),
        ("accepted_steps", Expr::int(progress.accepted as i64)),
        ("rejected_steps", Expr::int(progress.rejected as i64)),
        ("evaluations", Expr::int(progress.evaluations as i64)),
        (
            "model_evaluations",
            Expr::int(progress.model_evaluations as i64),
        ),
        (
            "jacobian_evaluations",
            Expr::int(progress.jacobian_evaluations as i64),
        ),
        (
            "damping",
            progress
                .damping
                .map(real)
                .transpose()?
                .unwrap_or_else(|| Expr::sym(B::NULL)),
        ),
        (
            "gradient_cosine",
            progress
                .gradient
                .map(real)
                .transpose()?
                .unwrap_or_else(|| Expr::sym(B::NULL)),
        ),
        (
            "data",
            Expr::call(
                B::DATA_TABLE,
                [
                    list(
                        ["input", "observed", "predicted", "residual"]
                            .into_iter()
                            .map(Expr::string),
                    ),
                    list(rows),
                ],
            ),
        ),
    ]))
}
