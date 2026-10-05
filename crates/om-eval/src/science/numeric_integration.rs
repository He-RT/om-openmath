//! Readonly raw expressions become a compiled real callback. No symbolic-to-numeric fallback.
use super::*;
use om_analysis::integration::{Failure, FailureKind, Options, integrate};
fn scalar(
    ev: &mut Evaluator,
    e: &Expr,
    allow_infinite: bool,
    ctx: &Interrupt,
) -> Result<f64, EvalError> {
    let e = ev.evaluate(e, ctx)?;
    if allow_infinite && e.is_head(B::DIRECTED_INFINITY) && e.args().len() == 1 {
        if e.args()[0] == Expr::int(1) {
            return Ok(f64::INFINITY);
        }
        if e.args()[0] == Expr::int(-1) {
            return Ok(f64::NEG_INFINITY);
        }
    }
    let n = if let Some(n) = e.as_number() {
        n.clone()
    } else {
        om_simplify::numeval::approximate(&e, om_num::Precision::Machine, ctx)?
            .ok_or_else(|| error("积分参数需要实数数值"))?
    };
    if matches!(n, Number::Complex(_)) {
        return Err(error("积分首版只支持实数参数"));
    }
    n.to_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| error("积分参数超出有限机器范围"))
}
fn integer(ev: &mut Evaluator, e: &Expr, ctx: &Interrupt) -> Result<usize, EvalError> {
    let e = ev.evaluate(e, ctx)?;
    if let Some(Number::Integer(n)) = e.as_number() {
        usize::try_from(n).map_err(|_| error("区间限额需要非负整数"))
    } else {
        Err(error("区间限额需要整数"))
    }
}
fn failed(e: Failure) -> EvalError {
    if let FailureKind::Abort(e) = e.kind {
        return e.into();
    }
    let reason = match e.kind {
        FailureKind::Input(reason) => reason.to_owned(),
        FailureKind::NonFiniteSample => "数值积分遇到非有限样本/原始极点或数值溢出".into(),
        FailureKind::Callback(e) => e.to_string(),
        FailureKind::IntervalLimit => "数值积分在区间限额内未收敛".into(),
        FailureKind::Roundoff => "数值积分误差停滞或达到机器分辨率".into(),
        FailureKind::Abort(_) => "积分已取消".into(),
    };
    error(&if let Some(p) = e.partial {
        format!(
            "{reason}；未收敛估计={}，误差估计={}，实际采样={}，区间={}",
            p.value, p.error_estimate, p.evaluations, p.intervals
        )
    } else {
        reason
    })
}
pub(super) fn dispatch(
    ev: &Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if !matches!(name, "Integrate" | "NIntegrate") {
        return Ok(None);
    }
    let mut fork = ev.fork_readonly();
    let mode = args
        .options
        .get("Mode")
        .map(|e| fork.evaluate(e, ctx))
        .transpose()?;
    let mode = mode
        .as_ref()
        .map(|e| string(e))
        .transpose()?
        .unwrap_or(if name == "NIntegrate" {
            "numeric"
        } else {
            "exact"
        });
    if !matches!(mode, "exact" | "numeric") || name == "NIntegrate" && mode != "numeric" {
        return Err(error("积分模式不受支持：NIntegrate只支持numeric"));
    }
    if mode != "numeric" {
        return Err(error(
            "符号积分尚在后续批次实施；保留原式，不自动进行数值近似",
        ));
    }
    if let Some(p) = args.options.get("WorkingPrecision") {
        let p = fork.evaluate(p, ctx)?;
        let supported = matches!(p.kind(),ExprKind::String(s) if matches!(&**s,"machine"|"MachinePrecision"))
            || p.as_symbol()
                .is_some_and(|s| s.name() == "MachinePrecision");
        if !supported {
            return Err(error("数值积分首版仅机器精度，不提供虚假的高精度路径"));
        }
    }
    if let Some(method) = args.options.get("Method") {
        let method = fork.evaluate(method, ctx)?;
        if !matches!(string(&method)?, "gauss_kronrod" | "gauss_kronrod_15_7") {
            return Err(error("积分method仅支持gauss_kronrod"));
        }
    }
    let axis = args.values[1];
    if !axis.is_head(B::LIST) || axis.args().len() != 3 {
        return Err(error("数值积分需要一维范围[x,a,b]或x:a..b"));
    }
    let variable = axis.args()[0]
        .as_symbol()
        .filter(|s| {
            !om_core::builtins::names().contains(&s.name())
                && om_core::catalog::by_runtime(s.name()).is_none()
        })
        .ok_or_else(|| error("积分坐标需要用户符号"))?;
    let a = scalar(&mut fork, &axis.args()[1], true, ctx)?;
    let b = scalar(&mut fork, &axis.args()[2], true, ctx)?;
    let mut options = Options::default();
    if let Some(e) = args.options.get("AbsTolerance") {
        options.abs_tol = scalar(&mut fork, e, false, ctx)?;
    }
    if let Some(e) = args.options.get("RelTolerance") {
        options.rel_tol = scalar(&mut fork, e, false, ctx)?;
    }
    if let Some(e) = args.options.get("MaxIntervals") {
        options.max_intervals = integer(&mut fork, e, ctx)?;
    }
    if let Some(e) = args.options.get("Breakpoints") {
        let e = fork.evaluate(e, ctx)?;
        if !e.is_head(B::LIST) || e.args().len() > 4096 {
            return Err(error("积分分段点需要最多4096项实数列表"));
        }
        for e in e.args() {
            ctx.tick()?;
            options.breakpoints.push(scalar(&mut fork, e, false, ctx)?);
        }
    }
    let expression = fork.prepare_numeric(args.values[0], &[(variable, None)], ctx)?;
    let program = crate::numeric::compile_f64_with_ctx(&expression, &[variable], ctx).map_err(
        |e| match e {
            crate::numeric::CompileError::Abort(e) => e.into(),
            e => error(&e.to_string()),
        },
    )?;
    let mut work = vec![];
    let result = integrate(
        |x, ctx| {
            program
                .eval_with_ctx(&[x], &mut work, ctx)
                .map_err(om_analysis::Error::Abort)
        },
        a,
        b,
        &options,
        ctx,
    )
    .map_err(failed)?;
    Ok(Some(record([
        ("value", real(result.value)?),
        ("error_estimate", real(result.error_estimate)?),
        ("converged", Expr::sym(B::TRUE)),
        ("evaluations", Expr::int(result.evaluations as i64)),
        ("intervals", Expr::int(result.intervals as i64)),
        ("method", Expr::string("gauss_kronrod_15_7")),
        ("precision", Expr::string("machine")),
        ("certified", Expr::sym(B::FALSE)),
    ])))
}
