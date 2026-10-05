//! Real distributions, exact uniform values and isolated deterministic random streams.
use super::*;
use om_core::Symbol;
use om_num::{Integer, rng::SplitMix64};
fn argument<'a>(
    args: &'a Args<'a>,
    index: usize,
    option: &str,
) -> Result<Option<&'a Expr>, EvalError> {
    if args.values.get(index).is_some() && args.options.contains_key(option) {
        return Err(error("同一参数不能同时使用位置和命名写法"));
    }
    Ok(args
        .values
        .get(index)
        .copied()
        .or_else(|| args.options.get(option).copied()))
}
fn distribution(e: &Expr) -> Result<(&str, &Expr, &Expr), EvalError> {
    match e.head_symbol().map(Symbol::name) {
        Some("NormalDistribution") if e.args().len() == 2 => {
            Ok(("normal", &e.args()[0], &e.args()[1]))
        }
        Some("UniformDistribution")
            if e.args().len() == 1
                && e.args()[0].is_head(B::LIST)
                && e.args()[0].args().len() == 2 =>
        {
            uniform_bounds(&e.args()[0])?;
            Ok(("uniform", &e.args()[0].args()[0], &e.args()[0].args()[1]))
        }
        _ => Err(error("需要有效的正态或均匀分布对象")),
    }
}
fn uniform_bounds(e: &Expr) -> Result<(&Expr, &Expr), EvalError> {
    if !e.is_head(B::LIST)
        || e.args().len() != 2
        || rational(&e.args()[0])? >= rational(&e.args()[1])?
    {
        return Err(error("范围需要两个严格递增的有限实端点"));
    }
    machine(&e.args()[0])?;
    machine(&e.args()[1])?;
    Ok((&e.args()[0], &e.args()[1]))
}
fn sample_size(args: &Args<'_>) -> Result<usize, EvalError> {
    match args.options.get("Count") {
        Some(e) => {
            if let Some(Number::Integer(n)) = e.as_number() {
                usize::try_from(n)
                    .ok()
                    .filter(|n| *n <= 100_000)
                    .ok_or_else(|| error("样本数须为0..100000整数"))
            } else {
                Err(error("样本数须为整数"))
            }
        }
        None => Ok(1),
    }
}
fn seed(e: &Expr) -> Result<u64, EvalError> {
    if let Some(Number::Integer(n)) = e.as_number() {
        u64::try_from(n).map_err(|_| error("种子须为0..2^64-1整数"))
    } else {
        Err(error("种子须为整数"))
    }
}
fn random(
    ev: &mut Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    if name == "SeedRandom" {
        let value = seed(args.values[0])?;
        if ev.readonly {
            return Err(error("只读计算不能修改随机种子"));
        }
        ev.random = SplitMix64::new(value);
        return Ok(Expr::sym(B::NULL));
    }
    let count = sample_size(args)?;
    let mut rng = if let Some(e) = args.options.get("Seed") {
        SplitMix64::new(seed(e)?)
    } else {
        ev.random.clone()
    };
    let bounds = args
        .options
        .get("Bounds")
        .copied()
        .cloned()
        .unwrap_or_else(|| list([Expr::int(0), Expr::int(1)]));
    let (lo, hi) = if name == "RandomUniform" {
        let (lo, hi) = uniform_bounds(&bounds)?;
        let bounds = (machine(lo)?, machine(hi)?);
        if bounds.0 >= bounds.1 {
            return Err(error("随机范围端点在机器精度下不可区分"));
        }
        bounds
    } else {
        (0., 1.)
    };
    let mu = args
        .options
        .get("Mean")
        .map(|e| machine(e))
        .transpose()?
        .unwrap_or(0.);
    let sigma = args
        .options
        .get("StandardDeviation")
        .map(|e| machine(e))
        .transpose()?
        .unwrap_or(1.);
    if sigma <= 0. {
        return Err(error("标准差须为正数"));
    }
    let choices = if name == "RandomChoice" {
        let e = args.values[0];
        if !e.is_head(B::LIST) || e.args().is_empty() || e.args().len() > 100_000 {
            return Err(error("随机选择需要非空且最多100000项的列表"));
        }
        e.args()
    } else {
        &[]
    };
    let mut output = Vec::with_capacity(count);
    for _ in 0..count {
        ctx.tick()?;
        output.push(match name {
            "RandomUniform" => {
                let u = (rng.next_u64() >> 11) as f64 / 9007199254740992.;
                real(((1. - u) * lo + u * hi).clamp(lo, hi.next_down()))?
            }
            "RandomNormal" => {
                let u = ((rng.next_u64() >> 11) as f64 + 1.) / 9007199254740992.;
                let v = (rng.next_u64() >> 11) as f64 / 9007199254740992.;
                real(mu + sigma * (-2. * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos())?
            }
            "RandomChoice" => choices[rng.next_range(0, choices.len() as u64) as usize].clone(),
            _ => return Err(error("未知随机入口")),
        });
    }
    ctx.tick()?;
    // Commit only a complete successful request. Readonly forks use their own snapshot stream.
    if !args.options.contains_key("Seed") {
        ev.random = rng;
    }
    Ok(if args.options.contains_key("Count") {
        list(output)
    } else {
        output.remove(0)
    })
}
pub(super) fn dispatch(
    ev: &mut Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let value = match name {
        "NormalDistribution" => {
            let mu = argument(args, 0, "Mean")?
                .cloned()
                .unwrap_or_else(|| Expr::int(0));
            let sigma = argument(args, 1, "StandardDeviation")?
                .cloned()
                .unwrap_or_else(|| Expr::int(1));
            machine(&mu)?;
            if machine(&sigma)? <= 0. {
                return Err(error("标准差须为正数"));
            }
            Expr::call(Symbol::intern(name), [mu, sigma])
        }
        "UniformDistribution" => {
            let bounds = argument(args, 0, "Bounds")?
                .cloned()
                .unwrap_or_else(|| list([Expr::int(0), Expr::int(1)]));
            uniform_bounds(&bounds)?;
            Expr::call(Symbol::intern(name), [bounds])
        }
        "PDF" | "CDF" | "Quantile" if name != "Quantile" || !args.values[0].is_head(B::LIST) => {
            let (kind, a, b) = distribution(args.values[0])?;
            let x = args.values[1];
            if kind == "uniform" {
                let (a, b) = (number(a)?, number(b)?);
                let x = number(x)?;
                let q = crate::scalar::rational(&x).ok_or_else(|| error("需要有限实数"))?;
                let qa = crate::scalar::rational(&a).ok_or_else(|| error("范围无效"))?;
                let qb = crate::scalar::rational(&b).ok_or_else(|| error("范围无效"))?;
                let width = b.add(&a.neg());
                let inverse = width.recip().map_err(|e| error(&e.to_string()))?;
                Expr::number(match name {
                    "PDF" => {
                        if q < qa || q > qb {
                            Number::Integer(Integer::ZERO)
                        } else {
                            inverse.clone()
                        }
                    }
                    "CDF" => {
                        if q <= qa {
                            Number::Integer(Integer::ZERO)
                        } else if q >= qb {
                            Number::Integer(Integer::ONE)
                        } else {
                            x.add(&a.neg()).mul(&inverse)
                        }
                    }
                    _ => {
                        if q < Rational::ZERO || q > Rational::ONE {
                            return Err(error("概率须在0..1内"));
                        }
                        a.add(&width.mul(&x))
                    }
                })
            } else {
                if name == "Quantile" {
                    let p = rational(x)?;
                    if p < Rational::ZERO || p > Rational::ONE {
                        return Err(error("概率须在0..1内"));
                    }
                    if p != Rational::ZERO && p != Rational::ONE && matches!(machine(x)?, 0. | 1.) {
                        return Err(error("概率与端点的距离超出机器表示范围"));
                    }
                }
                let (mu, sigma, x) = (machine(a)?, machine(b)?, machine(x)?);
                if sigma <= 0. {
                    return Err(error("标准差须为正数"));
                }
                if name == "Quantile" {
                    let z =
                        om_analysis::special::normal_quantile(x, ctx).map_err(analysis_failure)?;
                    if z.is_infinite() {
                        Expr::call(
                            B::DIRECTED_INFINITY,
                            [Expr::int(if z.is_sign_negative() { -1 } else { 1 })],
                        )
                    } else {
                        real(mu + sigma * z)?
                    }
                } else {
                    let z = (x - mu) / sigma;
                    real(if name == "PDF" {
                        (-0.5 * z * z).exp() / std::f64::consts::TAU.sqrt() / sigma
                    } else if z.is_infinite() {
                        if z.is_sign_negative() { 0. } else { 1. }
                    } else {
                        0.5 * om_analysis::special::erfc(-z / std::f64::consts::SQRT_2, ctx)
                            .map_err(analysis_failure)?
                    })?
                }
            }
        }
        "SeedRandom" | "RandomUniform" | "RandomNormal" | "RandomChoice" => {
            random(ev, name, args, ctx)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
