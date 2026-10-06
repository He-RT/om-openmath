//! Real compiled IVPs, interpolation and sampling, with readonly inputs and portable interruption.
use super::*;
use super::{interpolation_data as data, numeric_callback as source};
use om_analysis::{
    interpolation::{Interpolation, Method},
    ode::{self, Options, Termination},
};
fn state(ev: &mut Evaluator, e: &Expr, ctx: &Interrupt) -> Result<(Vec<f64>, bool), EvalError> {
    source::machine_source(e, ctx)?;
    let e = ev.evaluate(e, ctx)?;
    let scalar = !e.is_head(B::LIST);
    let row = if scalar { vec![e] } else { e.args().to_vec() };
    if !(1..=64).contains(&row.len()) {
        return Err(error("状态需要有限实标量或1..64维向量"));
    }
    Ok((
        row.iter()
            .map(|e| source::scalar(ev, e, ctx))
            .collect::<Result<_, _>>()?,
        scalar,
    ))
}
fn ode(ev: &mut Evaluator, args: &Args<'_>, ctx: &Interrupt) -> Result<Expr, EvalError> {
    source::precision(ev, args, ctx)?;
    if let Some(e) = args.options.get("Method")
        && !matches!(
            string(&ev.evaluate(e, ctx)?)?,
            "dormand_prince" | "dormand_prince_5_4"
        )
    {
        return Err(error("ODE首版method仅支持dormand_prince"));
    }
    let (start, end) = source::axis(ev, args.values[1], ctx)?;
    let initial = args
        .options
        .get("Initial")
        .ok_or_else(|| error("ODE需要initial初值"))?;
    let (initial, scalar) = state(ev, initial, ctx)?;
    let mut options = Options::default();
    for (key, target) in [
        ("AbsTolerance", &mut options.abs_tol),
        ("RelTolerance", &mut options.rel_tol),
        ("MaxStep", &mut options.max_step),
    ] {
        if let Some(e) = args.options.get(key) {
            *target = source::scalar(ev, e, ctx)?;
        }
    }
    if let Some(e) = args.options.get("MaxSteps") {
        options.max_steps = source::uint(ev, e, ctx)?;
    }
    if let Some(e) = args.options.get("InitialStep") {
        options.initial_step = Some(source::scalar(ev, e, ctx)?);
    }
    let d = initial.len();
    let source_input = list(
        std::iter::once(args.values[0].clone())
            .chain(args.options.get("Event").map(|e| (*e).clone())),
    );
    let vars = source::variables(ev, &source_input, d + 1, ctx)?;
    let state = if scalar {
        Expr::sym(vars[1])
    } else {
        list(vars[1..].iter().map(|s| Expr::sym(*s)))
    };
    let arguments = vec![Expr::sym(vars[0]), state];
    let raw = source::callback(ev, args.values[0], arguments.clone(), &vars, ctx)?;
    let expressions = if scalar && !raw.is_head(B::LIST) {
        vec![raw]
    } else if !scalar && raw.is_head(B::LIST) && raw.args().len() == d {
        raw.args().to_vec()
    } else {
        return Err(error("ODE右端返回形状必须与initial一致"));
    };
    let programs = expressions
        .iter()
        .map(|e| source::compile(e, &vars, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    let event_program = if let Some(e) = args.options.get("Event") {
        let raw = source::callback(ev, e, arguments, &vars, ctx)?;
        Some(source::compile(&raw, &vars, ctx)?)
    } else {
        None
    };
    let mut event_work = vec![];
    let mut event_count = 0usize;
    let mut event = |t: f64, y: &[f64], ctx: &Interrupt| {
        event_count += 1;
        let values: Vec<_> = std::iter::once(t).chain(y.iter().copied()).collect();
        event_program
            .as_ref()
            .ok_or(om_analysis::Error::Input("缺少事件回调"))?
            .eval_with_ctx(&values, &mut event_work, ctx)
            .map_err(om_analysis::Error::Abort)
    };
    let mut work = vec![];
    let answer = ode::solve(
        |t, y, out, ctx| {
            let values: Vec<_> = std::iter::once(t).chain(y.iter().copied()).collect();
            for (i, p) in programs.iter().enumerate() {
                out[i] = p.eval_with_ctx(&values, &mut work, ctx)?;
            }
            Ok(())
        },
        if event_program.is_some() {
            Some(&mut event as &mut ode::Event<'_>)
        } else {
            None
        },
        start,
        end,
        initial,
        &options,
        ctx,
    )
    .map_err(|failure| {
        if let om_analysis::Error::Abort(e) = failure.reason {
            return e.into();
        }
        let reason = failure.reason.to_string();
        error(&if let Some(p) = failure.partial {
            format!(
                "{reason}；ODE未收敛，最后时间={}，状态={:?}，接受步={}，拒绝步={}，右端求值={}",
                p.solution.times().last().unwrap(),
                p.solution.values().last().unwrap(),
                p.accepted_steps,
                p.rejected_steps,
                p.evaluations
            )
        } else {
            reason
        })
    })?;
    let times = answer.solution.times();
    Ok(record([
        (
            "solution",
            data::encode(&answer.solution, scalar, "rhs", ctx)?,
        ),
        (
            "domain",
            list([real(times[0])?, real(*times.last().unwrap())?]),
        ),
        ("converged", Expr::sym(B::TRUE)),
        (
            "termination",
            Expr::string(match answer.termination {
                Termination::End => "end",
                Termination::Event => "event",
            }),
        ),
        ("accepted_steps", Expr::int(answer.accepted_steps as i64)),
        ("rejected_steps", Expr::int(answer.rejected_steps as i64)),
        ("evaluations", Expr::int(answer.evaluations as i64)),
        ("event_evaluations", Expr::int(event_count as i64)),
        ("abs_tol", real(options.abs_tol)?),
        ("rel_tol", real(options.rel_tol)?),
        ("method", Expr::string("dormand_prince_5_4")),
    ]))
}
fn interpolate(ev: &mut Evaluator, args: &Args<'_>, ctx: &Interrupt) -> Result<Expr, EvalError> {
    source::precision(ev, args, ctx)?;
    let method = if let Some(e) = args.options.get("Method") {
        match string(&ev.evaluate(e, ctx)?)? {
            "linear" => Method::Linear,
            "hermite" => Method::Hermite,
            _ => return Err(error("interpolate支持linear/hermite")),
        }
    } else {
        Method::Linear
    };
    source::machine_source(args.values[0], ctx)?;
    let points = ev.evaluate(args.values[0], ctx)?;
    if !points.is_head(B::LIST) || points.args().len() < 2 || points.args().len() > 16666 {
        return Err(error("interpolate需要至少两个有序点，最多100000存储标量"));
    }
    let first = &points.args()[0];
    if !first.is_head(B::LIST) || !matches!(first.args().len(), 2 | 3) {
        return Err(error("插值节点格式为[x,value]或Hermite[x,value,slope]"));
    }
    let supplied = first.args().len() == 3;
    if supplied && method == Method::Linear {
        return Err(error("linear不接受Hermite导数"));
    }
    let mut times = vec![];
    let mut values = vec![];
    let mut slopes = vec![];
    let (first_value, scalar) = state(ev, &first.args()[1], ctx)?;
    let d = first_value.len();
    if points.args().len().saturating_mul(5 * d + 1) > 100000 {
        return Err(error("插值数据超出100000标量存储限额"));
    }
    for row in points.args() {
        ctx.tick()?;
        if !row.is_head(B::LIST) || row.args().len() != first.args().len() {
            return Err(error("所有插值节点必须使用相同格式"));
        }
        times.push(source::scalar(ev, &row.args()[0], ctx)?);
        let (value, is_scalar) = state(ev, &row.args()[1], ctx)?;
        if is_scalar != scalar || value.len() != d {
            return Err(error("插值值维度不一致"));
        }
        values.push(value);
        if supplied {
            let (value, is_scalar) = state(ev, &row.args()[2], ctx)?;
            if is_scalar != scalar || value.len() != d {
                return Err(error("插值导数维度不一致"));
            }
            slopes.push(value);
        }
    }
    let answer = Interpolation::from_points(times, values, supplied.then_some(slopes), method, ctx)
        .map_err(analysis_failure)?;
    data::encode(
        &answer,
        scalar,
        if method == Method::Linear {
            "none"
        } else if supplied {
            "supplied"
        } else {
            "estimated"
        },
        ctx,
    )
}
fn sample(ev: &mut Evaluator, args: &Args<'_>, ctx: &Interrupt) -> Result<Expr, EvalError> {
    source::precision(ev, args, ctx)?;
    let (a, b) = source::axis(ev, args.values[1], ctx)?;
    let count = if let Some(e) = args.options.get("Count") {
        source::uint(ev, e, ctx)?
    } else {
        100
    };
    if a == b || !(2..=10000).contains(&count) {
        return Err(error("sample需要非退化有限范围与count=2..10000"));
    }
    let function = ev.evaluate(args.values[0], ctx)?;
    let interpolation = if function.is_head(B::INTERPOLATION_DATA) {
        Some(data::decode(&function, ctx)?)
    } else {
        None
    };
    let (programs, scalar) = if let Some((_, scalar)) = &interpolation {
        (vec![], *scalar)
    } else {
        let vars = source::variables(ev, &function, 1, ctx)?;
        let raw = source::callback(ev, &function, vec![Expr::sym(vars[0])], &vars, ctx)?;
        let scalar = !raw.is_head(B::LIST);
        let exprs = if scalar {
            vec![raw]
        } else {
            raw.args().to_vec()
        };
        if !(1..=64).contains(&exprs.len()) {
            return Err(error("sample函数返回实标量或1..64维向量"));
        }
        (
            exprs
                .iter()
                .map(|e| source::compile(e, &vars, ctx))
                .collect::<Result<Vec<_>, _>>()?,
            scalar,
        )
    };
    let d = interpolation
        .as_ref()
        .map_or(programs.len(), |(i, _)| i.values()[0].len());
    if count.saturating_mul(d + 1) > 100000 {
        return Err(error("sample结果超过100000标量限额"));
    }
    let mut rows = vec![];
    let mut work = vec![];
    let mut previous = None;
    for i in 0..count {
        ctx.tick()?;
        let x = if i == 0 {
            a
        } else if i == count - 1 {
            b
        } else {
            a + (b - a) * (i as f64 / (count - 1) as f64)
        };
        if previous.is_some_and(|p| if b > a { x <= p } else { x >= p }) {
            return Err(error("sample节点在机器精度下重复或逆序"));
        }
        previous = Some(x);
        let values = if let Some((data, _)) = &interpolation {
            data.evaluate(x, ctx).map_err(analysis_failure)?
        } else {
            programs
                .iter()
                .map(|p| {
                    p.eval_with_ctx(&[x], &mut work, ctx)
                        .map_err(EvalError::from)
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        rows.push(record([
            ("x", real(x)?),
            ("value", data::value(values, scalar)?),
        ]));
    }
    Ok(Expr::call(
        B::DATA_TABLE,
        [list([Expr::string("x"), Expr::string("value")]), list(rows)],
    ))
}
pub(super) fn dispatch(
    ev: &Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if !matches!(name, "Ode" | "Interpolate" | "Sample") {
        return Ok(None);
    }
    let mut fork = ev.fork_readonly();
    let value = match name {
        "Ode" => ode(&mut fork, args, ctx)?,
        "Interpolate" => interpolate(&mut fork, args, ctx)?,
        _ => sample(&mut fork, args, ctx)?,
    };
    Ok(Some(value))
}
