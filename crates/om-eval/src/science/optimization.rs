//! Local raw compiled search and separate exact quadratic certification share only validated inputs.
use super::*;
use super::{
    numeric_callback as source, optimization_global as global, optimization_quadratic as q,
};
use om_analysis::optimization::{self as analysis, Options};
use om_core::Symbol;
use std::collections::BTreeSet;
fn variables(e: &Expr) -> Result<Vec<Symbol>, EvalError> {
    let values = if e.is_head(B::LIST) {
        e.args().to_vec()
    } else {
        vec![e.clone()]
    };
    if !(1..=64).contains(&values.len()) {
        return Err(error("优化需要1..64个坐标变量"));
    }
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|e| {
            let s = super::calculus_source::axis(e)?;
            if !seen.insert(s) {
                return Err(error("优化坐标必须互异"));
            }
            Ok(s)
        })
        .collect()
}
fn pairs<'a>(
    e: &'a Expr,
    variables: &[Symbol],
    ctx: &Interrupt,
) -> Result<Vec<[&'a Expr; 2]>, EvalError> {
    fn pair(e: &Expr) -> Result<[&Expr; 2], EvalError> {
        if !(e.is_head(B::LIST) || e.is_head(B::SPAN)) || e.args().len() != 2 {
            return Err(error("每个bounds需要两端点范围"));
        }
        Ok([&e.args()[0], &e.args()[1]])
    }
    if e.is_head(B::RECORD) {
        let entries = super::csv_data::entries(e, ctx)?;
        if entries.len() != variables.len() {
            return Err(error("bounds记录字段必须恰好对应变量"));
        }
        return variables
            .iter()
            .map(|s| {
                pair(
                    entries
                        .get(s.name())
                        .copied()
                        .ok_or_else(|| error("bounds记录缺少坐标"))?,
                )
            })
            .collect();
    }
    if variables.len() == 1
        && e.args().len() == 2
        && !e
            .args()
            .iter()
            .any(|e| e.is_head(B::LIST) || e.is_head(B::SPAN))
    {
        return Ok(vec![pair(e)?]);
    }
    if !e.is_head(B::LIST) || e.args().len() != variables.len() {
        return Err(error("bounds需要与变量等长的范围列表或坐标记录"));
    }
    e.args().iter().map(pair).collect()
}
fn choice(
    ev: &mut Evaluator,
    args: &Args<'_>,
    key: &str,
    default: &str,
    allowed: &[&str],
    ctx: &Interrupt,
) -> Result<String, EvalError> {
    let value = args
        .options
        .get(key)
        .map(|e| ev.evaluate(e, ctx))
        .transpose()?;
    let name = value.as_ref().map(string).transpose()?.unwrap_or(default);
    if !allowed.contains(&name) {
        return Err(error("优化选项枚举不受支持"));
    }
    Ok(name.into())
}
fn failed(f: analysis::Failure, maximize: bool) -> EvalError {
    if let om_analysis::Error::Abort(e) = f.reason {
        return e.into();
    }
    error(&if let Some(a) = f.partial {
        format!(
            "{}；未收敛候选point={:?}，value={}，实际迭代={}，求值={}，投影梯度={:?}",
            f.reason,
            a.point,
            if maximize { -a.value } else { a.value },
            a.iterations,
            a.evaluations,
            a.gradient_norm
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
    if name != "Optimize" {
        return Ok(None);
    }
    let axes = variables(args.values[1])?;
    let n = axes.len();
    let mut fork = ev.fork_readonly();
    let maximize = choice(&mut fork, args, "Goal", "min", &["min", "max"], ctx)? == "max";
    let scope = choice(&mut fork, args, "Scope", "local", &["local", "global"], ctx)?;
    let method = choice(
        &mut fork,
        args,
        "Method",
        "auto",
        &["auto", "brent", "bfgs", "exact_ldlt"],
        ctx,
    )?;
    let limit = args
        .options
        .get("MaxIterations")
        .map(|e| source::uint(&mut fork, e, ctx))
        .transpose()?
        .unwrap_or(1000);
    if !(1..=100000).contains(&limit) {
        return Err(error("优化max_iterations需要1..100000"));
    }
    let bounds = args
        .options
        .get("Bounds")
        .map(|e| fork.evaluate(e, ctx))
        .transpose()?;
    let bound_pairs = bounds.as_ref().map(|e| pairs(e, &axes, ctx)).transpose()?;
    let bound_pairs = bound_pairs.as_deref();
    let result = if scope == "global" {
        if !matches!(method.as_str(), "auto" | "exact_ldlt") {
            return Err(error("global必须走精确凸二次认证，不能用局部方法代替"));
        }
        if [
            "Initial",
            "AbsTolerance",
            "RelTolerance",
            "GradientTolerance",
        ]
        .iter()
        .any(|k| args.options.contains_key(k))
        {
            return Err(error("global认证不接受局部初值或数值容差"));
        }
        if let Some(p) = args.options.get("WorkingPrecision") {
            let p = fork.evaluate(p, ctx)?;
            if !matches!(p.kind(),ExprKind::String(s) if &**s=="exact") {
                return Err(error(
                    "global认证仅接受precision:exact，不使用机器或高精度近似作为证明",
                ));
            }
        }
        let bounds = bound_pairs
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|[a, b]| {
                        let a = q::scalar(&mut fork, a, ctx)?;
                        let b = q::scalar(&mut fork, b, ctx)?;
                        if a > b {
                            return Err(error("全局bounds必须有序"));
                        }
                        Ok((a, b))
                    })
                    .collect::<Result<Vec<_>, EvalError>>()
            })
            .transpose()?;
        fork.scopes.push(axes.iter().map(|s| (*s, None)).collect());
        let raw = fork.prepare_numeric(args.values[0], &[], ctx)?;
        let polynomial = q::Quadratic::new(q::extract(&mut fork, &raw, &axes, ctx)?, n, ctx)?;
        global::apply(polynomial, bounds, maximize, limit, ctx)?
    } else {
        source::precision(&mut fork, args, ctx)?;
        if method == "exact_ldlt" {
            return Err(error("exact_ldlt需要显式scope:global"));
        }
        let bounds = bound_pairs
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|[a, b]| {
                        let a = source::scalar(&mut fork, a, ctx)?;
                        let b = source::scalar(&mut fork, b, ctx)?;
                        if a > b {
                            return Err(error("局部bounds必须有序"));
                        }
                        Ok((a, b))
                    })
                    .collect::<Result<Vec<_>, EvalError>>()
            })
            .transpose()?;
        let brent = method == "brent"
            || (method == "auto"
                && n == 1
                && bounds.is_some()
                && !args.options.contains_key("Initial"));
        if brent && (n != 1 || bounds.is_none() || args.options.contains_key("Initial")) {
            return Err(error("brent只接受一维有限bounds，不接受initial"));
        }
        let mut options = Options {
            max_iterations: limit,
            ..Options::default()
        };
        if brent {
            if args.options.contains_key("GradientTolerance") {
                return Err(error("brent使用abs_tol/rel_tol而不是gradient_tol"));
            }
            if let Some(e) = args.options.get("AbsTolerance") {
                options.abs_tol = source::scalar(&mut fork, e, ctx)?;
            }
            if let Some(e) = args.options.get("RelTolerance") {
                options.rel_tol = source::scalar(&mut fork, e, ctx)?;
            }
        } else {
            if args.options.contains_key("AbsTolerance")
                || args.options.contains_key("RelTolerance")
            {
                return Err(error("bfgs使用gradient_tol，不能忽略坐标容差请求"));
            }
            if let Some(e) = args.options.get("GradientTolerance") {
                options.gradient_tol = source::scalar(&mut fork, e, ctx)?;
            }
        }
        let initial = if brent {
            None
        } else {
            let e = args
                .options
                .get("Initial")
                .ok_or_else(|| error("局部bfgs需要initial"))?;
            source::machine_source(e, ctx)?;
            let e = fork.evaluate(e, ctx)?;
            let row = if e.is_head(B::LIST) {
                e.args().to_vec()
            } else {
                vec![e]
            };
            if row.len() != n {
                return Err(error("initial维数与变量不一致"));
            }
            Some(
                row.iter()
                    .map(|e| source::scalar(&mut fork, e, ctx))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        };
        fork.scopes.push(axes.iter().map(|s| (*s, None)).collect());
        let raw = fork.prepare_numeric(args.values[0], &[], ctx)?;
        source::machine_source(&raw, ctx)?;
        let objective = source::compile(&raw, &axes, ctx)?;
        let sign = if maximize { -1. } else { 1. };
        let mut work = vec![];
        let answer = if brent {
            let (a, b) = bounds.as_ref().unwrap()[0];
            analysis::bounded(
                |x, ctx| Ok(sign * objective.eval_with_ctx(&[x], &mut work, ctx)?),
                a,
                b,
                &options,
                ctx,
            )
        } else {
            let gradients = axes
                .iter()
                .map(|s| {
                    let derivative = crate::algebra::differentiate(&raw, &Expr::sym(*s), ctx)?;
                    source::compile(&derivative, &axes, ctx)
                })
                .collect::<Result<Vec<_>, _>>()?;
            analysis::minimize(
                |point, gradient, ctx| {
                    let value = sign * objective.eval_with_ctx(point, &mut work, ctx)?;
                    for (out, p) in gradient.iter_mut().zip(&gradients) {
                        *out = sign * p.eval_with_ctx(point, &mut work, ctx)?;
                    }
                    Ok(value)
                },
                initial.unwrap(),
                bounds.as_deref(),
                &options,
                ctx,
            )
        }
        .map_err(|e| failed(e, maximize))?;
        record([
            (
                "point",
                list(
                    answer
                        .point
                        .into_iter()
                        .map(real)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
            ("value", real(sign * answer.value)?),
            ("scope", Expr::string("local")),
            ("goal", Expr::string(if maximize { "max" } else { "min" })),
            ("converged", Expr::sym(B::TRUE)),
            (
                "guarantee",
                Expr::string(if brent {
                    "numerical_bounded_candidate"
                } else {
                    "numerical_stationary_candidate"
                }),
            ),
            ("method", Expr::string(if brent { "brent" } else { "bfgs" })),
            ("iterations", Expr::int(answer.iterations as i64)),
            ("evaluations", Expr::int(answer.evaluations as i64)),
            (
                "projected_gradient_norm",
                answer
                    .gradient_norm
                    .map(real)
                    .transpose()?
                    .unwrap_or_else(|| Expr::sym(B::NULL)),
            ),
            (
                "bracket_width",
                answer
                    .bracket_width
                    .map(real)
                    .transpose()?
                    .unwrap_or_else(|| Expr::sym(B::NULL)),
            ),
            ("null_space", Expr::sym(B::NULL)),
            ("certificate", Expr::sym(B::NULL)),
        ])
    };
    let point = result
        .args()
        .iter()
        .find(|r| r.args()[0] == Expr::string("point"))
        .ok_or_else(|| error("优化结果缺少point"))?
        .args()[1]
        .clone();
    let mut rows = result.args().to_vec();
    rows.push(Expr::call(
        B::RULE,
        [
            Expr::string("variables"),
            list(axes.iter().map(|s| Expr::string(s.name()))),
        ],
    ));
    rows.push(Expr::call(
        B::RULE,
        [
            Expr::string("bindings"),
            Expr::call(
                B::RECORD,
                axes.iter()
                    .zip(point.args())
                    .map(|(s, x)| Expr::call(B::RULE, [Expr::string(s.name()), x.clone()])),
            ),
        ],
    ));
    Ok(Some(Expr::call(B::RECORD, rows)))
}
