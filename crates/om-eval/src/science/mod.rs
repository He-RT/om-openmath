//! Pure scientific/data callbacks share validated arguments and portable budgets.
mod basics;
mod calculus_registry;
mod calculus_source;
pub(crate) mod csv_data;
mod data;
mod data_number;
mod fitted_model;
mod fitting;
mod fitting_affine;
mod fitting_data;
mod fitting_registry;
mod fitting_report;
mod integral_poly;
mod integral_rational;
mod integral_rules;
mod integration_registry;
mod interpolation_data;
mod io_registry;
pub(crate) mod json_data;
mod limit_rational;
mod limits;
mod matrix;
mod matrix_numeric;
mod numeric_callback;
mod numeric_integration;
mod ode;
mod ode_registry;
mod optimization;
mod optimization_global;
mod optimization_quadratic;
mod optimization_registry;
mod ordering;
mod probability;
mod probability_registry;
mod reflection;
mod reflection_registry;
mod registry;
mod root_registry;
mod roots;
mod series_registry;
mod special;
mod statistics;
mod symbolic_integration;
mod table_data;
mod taylor;
mod unit_parse;
mod unit_registry;
mod units;
mod vector_calculus;
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel};
use om_num::{Number, Rational};
pub(crate) use registry::register;
pub(crate) use special::is_machine_special;
pub(crate) fn calculus_terminal(s: om_core::Symbol) -> bool {
    matches!(
        s.name(),
        "Grad"
            | "Jacobian"
            | "Hessian"
            | "Divergence"
            | "Curl"
            | "Laplacian"
            | "Integrate"
            | "Series"
            | "Normal"
            | "Limit"
    )
}
use std::collections::BTreeMap;
pub(super) struct Args<'a> {
    pub values: Vec<&'a Expr>,
    pub options: BTreeMap<&'static str, &'a Expr>,
}
impl<'a> Args<'a> {
    fn parse(name: &str, args: &'a [Expr]) -> Result<Self, EvalError> {
        let schema = om_core::catalog::by_runtime(name).ok_or_else(|| error("缺少实际函数描述"))?;
        let mut values = vec![];
        let mut options = BTreeMap::new();
        for (i, arg) in args.iter().enumerate() {
            if name == "Limit"
                && i == 1
                && arg.is_head(B::RULE)
                && arg.args().len() == 2
                && arg.args()[0].as_symbol().is_some_and(|s| {
                    !schema
                        .options
                        .iter()
                        .any(|p| p.runtime_name.as_deref() == Some(s.name()))
                })
            {
                values.push(arg);
                continue;
            }
            if arg.is_head(B::RULE) && arg.args().len() == 2 {
                let key = arg.args()[0]
                    .as_symbol()
                    .ok_or_else(|| error("选项键需要符号"))?
                    .name();
                if !schema
                    .options
                    .iter()
                    .any(|p| p.runtime_name.as_deref() == Some(key))
                {
                    return Err(error("函数不支持此选项"));
                }
                if options.insert(key, &arg.args()[1]).is_some() {
                    return Err(error("选项重复"));
                }
            } else {
                values.push(arg);
            }
        }
        let minimum = schema
            .parameters
            .iter()
            .filter(|p| p.required && !p.variadic)
            .count();
        if values.len() < minimum
            || (!schema.parameters.iter().any(|p| p.variadic)
                && values.len() > schema.parameters.len())
        {
            return Err(error("位置参数个数不符合实际接口"));
        }
        Ok(Self { values, options })
    }
    pub fn boolean(&self, key: &str, default: bool) -> Result<bool, EvalError> {
        match self.options.get(key).and_then(|e| e.as_symbol()) {
            Some(B::TRUE) => Ok(true),
            Some(B::FALSE) => Ok(false),
            None if !self.options.contains_key(key) => Ok(default),
            _ => Err(error("选项需要布尔值")),
        }
    }
}
pub(super) fn error(s: &str) -> EvalError {
    EvalError::Other(s.into())
}
pub(super) fn number(e: &Expr) -> Result<Number, EvalError> {
    if matches!(e.as_number(),Some(Number::Real(om_num::Real::Big(value))) if value.repr().exponent().unsigned_abs()>20000)
    {
        return Err(error("数值指数超过本批科学函数的资源界限"));
    }
    e.as_number()
        .cloned()
        .filter(|n| !matches!(n, Number::Complex(_)))
        .ok_or_else(|| error("需要有限实数，不能把符号输入静默近似"))
}
pub(super) fn rational(e: &Expr) -> Result<Rational, EvalError> {
    crate::scalar::rational(&number(e)?).ok_or_else(|| error("需要可表示的有限实数"))
}
pub(super) fn vector(e: &Expr, ctx: &Interrupt) -> Result<Vec<Number>, EvalError> {
    if !e.is_head(B::LIST) || e.args().len() > 100_000 {
        return Err(error("需要最多100000项的列表"));
    }
    e.args()
        .iter()
        .map(|e| {
            ctx.tick()?;
            number(e)
        })
        .collect()
}
pub(super) fn list(values: impl IntoIterator<Item = Expr>) -> Expr {
    Expr::call(B::LIST, values)
}
pub(super) fn record(values: impl IntoIterator<Item = (&'static str, Expr)>) -> Expr {
    Expr::call(
        B::RECORD,
        values
            .into_iter()
            .map(|(key, value)| Expr::call(B::RULE, [Expr::string(key), value])),
    )
}
pub(super) fn real(value: f64) -> Result<Expr, EvalError> {
    if !value.is_finite() {
        return Err(error("结果超出机器有限值范围"));
    }
    Ok(Expr::number(Number::Real(om_num::Real::Machine(value))))
}
pub(super) fn dispatch(
    ev: &mut Evaluator,
    name: &str,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let result = (|| {
        ctx.tick()?;
        let args = Args::parse(name, args)?;
        if let Some(result) = fitting::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = optimization::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = ode::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = limits::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = taylor::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = vector_calculus::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = numeric_integration::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = reflection::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if matches!(name, "CubeRoot" | "NthRoot") {
            return roots::dispatch(name, &args, ctx)?.ok_or_else(|| error("根接口未匹配"));
        }
        if let Some(result) = special::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = probability::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = io_registry::dispatch(name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = table_data::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = units::dispatch(name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = statistics::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if matrix_numeric::wants_numeric(name, &args)? {
            return matrix_numeric::dispatch(name, &args, ctx);
        }
        if let Some(result) = matrix::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        if let Some(result) = data::dispatch(ev, name, &args, ctx)? {
            return Ok(result);
        }
        basics::dispatch(ev, name, &args, ctx)?.ok_or_else(|| error("未匹配实际回调"))
    })();
    match result {
        Ok(value) => Ok(Some(value)),
        Err(EvalError::Other(reason)) => {
            ev.message(name, "domain", reason, MsgLevel::Warning);
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
pub(crate) fn quantity_arithmetic(
    ev: &mut Evaluator,
    head: om_core::Symbol,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    match units::arithmetic(head, args, ctx) {
        Err(EvalError::Other(reason)) => {
            ev.message("Quantity", "units", reason, MsgLevel::Warning);
            Ok(Some(Expr::call(head, args.iter().cloned())))
        }
        result => result,
    }
}
pub(crate) fn table_structure(
    ev: &mut Evaluator,
    name: &str,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let adapted = Args {
        values: args.iter().collect(),
        options: BTreeMap::new(),
    };
    match table_data::dispatch(ev, name, &adapted, ctx) {
        Err(EvalError::Other(reason)) => {
            ev.message(name, "shape", reason, MsgLevel::Warning);
            Ok(None)
        }
        result => result,
    }
}
pub(crate) fn table_field(
    ev: &mut Evaluator,
    value: &Expr,
    key: &str,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    match table_data::field(value, key, ctx) {
        Ok(value) => Ok(Some(value)),
        Err(EvalError::Other(reason)) => {
            ev.message("Part", "shape", reason, MsgLevel::Warning);
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
pub(super) fn string(e: &Expr) -> Result<&str, EvalError> {
    if let ExprKind::String(s) = e.kind() {
        Ok(s)
    } else {
        Err(error("需要字符串"))
    }
}
pub(super) fn machine(e: &Expr) -> Result<f64, EvalError> {
    let n = number(e)?;
    if matches!(n.precision(), om_num::Precision::Bits(_)) {
        return Err(error("此算法只有机器精度路径，不能静默降低高精度输入"));
    }
    n.to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| error("需要机器范围内的有限实数"))
}
pub(super) fn analysis_failure(e: om_analysis::Error) -> EvalError {
    match e {
        om_analysis::Error::Abort(e) => e.into(),
        e => error(&e.to_string()),
    }
}
pub(crate) fn interpolation_call(
    ev: &mut Evaluator,
    object: &Expr,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    match interpolation_data::call(object, args, ctx) {
        Ok(value) => Ok(Some(value)),
        Err(EvalError::Other(reason)) => {
            ev.message("InterpolationData", "domain", reason, MsgLevel::Warning);
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
pub(crate) fn fitted_call(
    ev: &mut Evaluator,
    object: &Expr,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    match fitted_model::call(ev, object, args, ctx) {
        Ok(value) => Ok(Some(value)),
        Err(EvalError::Other(reason)) => {
            ev.message("FittedModelData", "domain", reason, MsgLevel::Warning);
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
pub(crate) fn fitted_source(
    ev: &mut Evaluator,
    object: &Expr,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    fitted_model::source(ev, object, args, ctx)
}
