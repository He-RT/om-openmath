//! Real readonly model compilation feeds tall QR or scaled LM; unsupported models remain diagnostic.
use super::*;
use super::{
    fitting_data as data,
    fitting_report::{self as report, Progress},
    numeric_callback as source,
};
use om_analysis::fitting::{self as analysis, Options, Termination};
fn failed(f: analysis::Failure) -> EvalError {
    if let om_analysis::Error::Abort(e) = f.reason {
        return e.into();
    }
    error(&if let Some(a) = f.partial {
        format!(
            "{}；拟合未收敛，实际参数={:?}，残差范数={}，梯度夹角={}，迭代={}，调用={}",
            f.reason, a.parameters, a.residual_norm, a.gradient_cosine, a.iterations, a.evaluations
        )
    } else {
        f.reason.to_string()
    })
}
pub(super) fn dispatch(
    ev: &Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if name != "Fit" {
        return Ok(None);
    }
    let mut fork = ev.fork_readonly();
    source::precision(&mut fork, args, ctx)?;
    let method = args
        .options
        .get("Method")
        .map(|e| fork.evaluate(e, ctx))
        .transpose()?;
    let method = method.as_ref().map(string).transpose()?.unwrap_or("linear");
    let nonlinear = match method {
        "linear" => false,
        "nonlinear" | "levenberg_marquardt" => true,
        _ => {
            return Err(error(
                "fit method只支持linear/nonlinear/levenberg_marquardt",
            ));
        }
    };
    let default_axis = Expr::symbol("x");
    let vars = data::axes(
        args.options
            .get("Variables")
            .copied()
            .unwrap_or(&default_axis),
    )?;
    let parameters = data::parameters(
        &mut fork,
        args.options
            .get("Parameters")
            .ok_or_else(|| error("fit需要parameters参数声明"))?,
        nonlinear,
        &vars,
        ctx,
    )?;
    let target = args
        .options
        .get("Target")
        .map(|e| fork.evaluate(e, ctx))
        .transpose()?;
    let target = target.as_ref().map(string).transpose()?.unwrap_or("y");
    let data = data::read(
        &mut fork,
        args.values[0],
        &vars,
        target,
        args.options.contains_key("Target"),
        ctx,
    )?;
    let (m, n) = (data.inputs.len(), parameters.names.len());
    if m < n || m.saturating_mul(n) > 100000 {
        return Err(error("fit样本数必须≥参数数，设计矩阵最多100000标量"));
    }
    let mut options = Options::default();
    if !nonlinear
        && [
            "AbsTolerance",
            "RelTolerance",
            "GradientTolerance",
            "MaxIterations",
            "InitialDamping",
        ]
        .iter()
        .any(|k| args.options.contains_key(k))
    {
        return Err(error(
            "linear是直接QR求解，不能忽略nonlinear容差/阻尼/迭代选项",
        ));
    }
    if nonlinear {
        for (key, out) in [
            ("AbsTolerance", &mut options.abs_tol),
            ("RelTolerance", &mut options.rel_tol),
            ("GradientTolerance", &mut options.gradient_tol),
            ("InitialDamping", &mut options.initial_damping),
        ] {
            if let Some(e) = args.options.get(key) {
                *out = source::scalar(&mut fork, e, ctx)?;
            }
        }
        if let Some(e) = args.options.get("MaxIterations") {
            options.max_iterations = source::uint(&mut fork, e, ctx)?;
        }
    }
    let mut variables = vars.clone();
    variables.extend(&parameters.names);
    fork.scopes
        .push(variables.iter().map(|s| (*s, None)).collect());
    let raw = fork.prepare_numeric(
        args.options
            .get("Model")
            .ok_or_else(|| error("fit需要model表达式"))?,
        &[],
        ctx,
    )?;
    source::machine_source(&raw, ctx)?;
    let model = source::compile(&raw, &variables, ctx)?;
    let mut model_evaluations = 0;
    let mut jacobian_evaluations = 0;
    let mut work = vec![];
    let mut values = vec![0.; variables.len()];
    let mut affine_check = None;
    let progress = if nonlinear {
        let gradients = parameters
            .names
            .iter()
            .map(|s| {
                source::compile(
                    &crate::algebra::differentiate(&raw, &Expr::sym(*s), ctx)?,
                    &variables,
                    ctx,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let answer = analysis::nonlinear(
            |p, r, j, ctx| {
                for (i, x) in data.inputs.iter().enumerate() {
                    ctx.tick()?;
                    values[..vars.len()].copy_from_slice(x);
                    values[vars.len()..].copy_from_slice(p);
                    model_evaluations += 1;
                    r[i] = model.eval_with_ctx(&values, &mut work, ctx)? - data.observed[i];
                    for (k, g) in gradients.iter().enumerate() {
                        jacobian_evaluations += 1;
                        j[i * n + k] = g.eval_with_ctx(&values, &mut work, ctx)?;
                    }
                }
                Ok(())
            },
            parameters.initial.clone(),
            m,
            &options,
            ctx,
        )
        .map_err(failed)?;
        Progress {
            point: answer.parameters,
            iterations: answer.iterations,
            accepted: answer.accepted_steps,
            rejected: answer.rejected_steps,
            evaluations: answer.evaluations,
            damping: Some(answer.damping),
            gradient: Some(answer.gradient_cosine),
            termination: match answer.termination {
                Termination::ResidualTolerance => "residual_tolerance",
                Termination::GradientTolerance => "gradient_tolerance",
            },
            model_evaluations,
            jacobian_evaluations,
        }
    } else {
        let coefficients = super::fitting_affine::extract(&raw, &parameters.names, ctx)?;
        let coefficients = coefficients[1..]
            .iter()
            .map(|e| source::compile(e, &vars, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut design = vec![];
        let mut rhs = vec![];
        let mut baselines = vec![];
        for (i, x) in data.inputs.iter().enumerate() {
            ctx.tick()?;
            values[..vars.len()].copy_from_slice(x);
            values[vars.len()..].fill(0.);
            model_evaluations += 1;
            let baseline = model.eval_with_ctx(&values, &mut work, ctx)?;
            if !baseline.is_finite() {
                return Err(error(
                    "linear模型在样本或零参数基点有原始奇点，不能声明全参数仿射",
                ));
            }
            rhs.push(data.observed[i] - baseline);
            baselines.push(baseline);
            for coefficient in &coefficients {
                jacobian_evaluations += 1;
                design.push(coefficient.eval_with_ctx(x, &mut work, ctx)?);
            }
        }
        let point = analysis::least_squares(m, n, &design, &rhs, ctx).map_err(analysis_failure)?;
        affine_check = Some((design, baselines));
        Progress {
            point,
            iterations: 0,
            accepted: 0,
            rejected: 0,
            evaluations: 0,
            damping: None,
            gradient: None,
            termination: "linear_qr",
            model_evaluations,
            jacobian_evaluations,
        }
    };
    let mut predicted = vec![];
    let mut progress = progress;
    for (i, x) in data.inputs.iter().enumerate() {
        ctx.tick()?;
        values[..vars.len()].copy_from_slice(x);
        values[vars.len()..].copy_from_slice(&progress.point);
        progress.model_evaluations += 1;
        let y = model.eval_with_ctx(&values, &mut work, ctx)?;
        if !y.is_finite() {
            return Err(error("拟合最终模型在真实样本不有限"));
        }
        if let Some((design, baselines)) = &affine_check {
            let mut expected = baselines[i];
            for k in 0..n {
                ctx.tick()?;
                expected = design[i * n + k].mul_add(progress.point[k], expected);
            }
            let scale = y
                .abs()
                .max(expected.abs())
                .max(data.observed[i].abs())
                .max(1e-300);
            if !expected.is_finite() || (y - expected).abs() > 128. * f64::EPSILON * scale {
                return Err(error(
                    "原模型与仿射设计在当前机器尺度下求值不一致，不能伪造线性拟合成功；请重写消去或重新缩放",
                ));
            }
        }
        predicted.push(y);
    }
    Ok(Some(report::build(
        &raw,
        &vars,
        &parameters,
        &data,
        report::Outcome {
            progress,
            predicted,
            nonlinear,
        },
        ctx,
    )?))
}
