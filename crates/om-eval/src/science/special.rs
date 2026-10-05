//! Exact recognized values remain exact; general real numeric calls are explicitly machine only.
use super::*;
use om_core::Symbol;
use om_num::Precision;
pub(crate) fn is_machine_special(name: &str) -> bool {
    matches!(
        name,
        "Erf" | "Erfc" | "Gamma" | "LogGamma" | "Beta" | "ArcCoth" | "ArcSech" | "ArcCsch"
    )
}
pub(super) fn dispatch(
    ev: &mut Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if !is_machine_special(name) {
        return Ok(None);
    }
    let x = args.values[0];
    if matches!(name, "Erf" | "Erfc") && x.is_zero() {
        return Ok(Some(Expr::int(i64::from(name == "Erfc"))));
    }
    if name == "ArcSech" && x == &Expr::int(1) {
        return Ok(Some(Expr::int(0)));
    }
    if name == "Gamma"
        && let Some(Number::Integer(n)) = x.as_number()
    {
        let n = u32::try_from(n)
            .ok()
            .filter(|n| (1..=1_000_001).contains(n))
            .ok_or_else(|| error("精确Gamma只支持1..1000001的正整数"))?;
        return Ok(Some(ev.evaluate(
            &Expr::call(Symbol::intern("Factorial"), [Expr::int(i64::from(n - 1))]),
            ctx,
        )?));
    }
    // Symbolic calls and unsupported exact values retain their head. N later supplies machine arguments.
    if args.values.iter().any(|x| x.as_number().is_none()) {
        return Ok(Some(Expr::call(
            Symbol::intern(name),
            args.values.iter().map(|x| (*x).clone()),
        )));
    }
    let exact = args.values.iter().all(|x| {
        x.as_number()
            .is_some_and(|n| n.precision() == Precision::Exact)
    });
    if exact {
        let q = args
            .values
            .iter()
            .map(|x| rational(x))
            .collect::<Result<Vec<_>, _>>()?;
        if matches!(name, "Gamma" | "LogGamma" | "Beta") && q.iter().any(|x| *x <= Rational::ZERO) {
            return Err(error("此函数首版只支持正实参数"));
        }
        if (name == "ArcCoth" && q[0] >= -Rational::ONE && q[0] <= Rational::ONE)
            || (name == "ArcSech" && (q[0] <= Rational::ZERO || q[0] > Rational::ONE))
            || (name == "ArcCsch" && q[0] == Rational::ZERO)
        {
            return Err(error("参数超出首版实数主值定义域"));
        }
        return Ok(Some(Expr::call(
            Symbol::intern(name),
            args.values.iter().map(|x| (*x).clone()),
        )));
    }
    let values = args
        .values
        .iter()
        .map(|x| machine(x))
        .collect::<Result<Vec<_>, _>>()?;
    let x = values[0];
    if matches!(name, "Gamma" | "LogGamma" | "Beta") && values.iter().any(|x| *x <= 0.) {
        return Err(error("此函数首版只支持正实参数"));
    }
    if (name == "ArcCoth" && x.abs() <= 1.)
        || (name == "ArcSech" && !(0. < x && x <= 1.))
        || (name == "ArcCsch" && x == 0.)
    {
        return Err(error("参数超出首版实数主值定义域"));
    }
    let value = match name {
        "Erf" => om_analysis::special::erf(x, ctx).map_err(analysis_failure)?,
        "Erfc" => om_analysis::special::erfc(x, ctx).map_err(analysis_failure)?,
        "Gamma" => om_analysis::special::gamma(x, ctx).map_err(analysis_failure)?,
        "LogGamma" => om_analysis::special::log_gamma(x, ctx).map_err(analysis_failure)?,
        "Beta" => om_analysis::special::beta(x, values[1], ctx).map_err(analysis_failure)?,
        "ArcCoth" => (1. / x).atanh(),
        "ArcSech" => (1. - x * x).sqrt().ln_1p() - x.ln(),
        "ArcCsch" => {
            if x.abs() >= 1. {
                (1. / x).asinh()
            } else {
                x.signum() * (x.hypot(1.).ln_1p() - x.abs().ln())
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(real(value)?))
}
