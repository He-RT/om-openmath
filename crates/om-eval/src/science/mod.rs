//! Pure scientific/data callbacks share validated arguments and portable budgets.
mod basics;
mod data;
mod matrix;
mod matrix_numeric;
mod ordering;
mod registry;
mod statistics;
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel};
use om_num::{Number, Rational};
pub(crate) use registry::register;
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
        for arg in args {
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
        if args.values.is_empty() && !matches!(name, "Min" | "Max") {
            return Err(error("缺少位置参数"));
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
pub(super) fn string(e: &Expr) -> Result<&str, EvalError> {
    if let ExprKind::String(s) = e.kind() {
        Ok(s)
    } else {
        Err(error("需要字符串"))
    }
}
