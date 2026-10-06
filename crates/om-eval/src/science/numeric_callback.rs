//! Raw, readonly callbacks compile once. Lexical arguments never capture caller definitions.
use super::*;
use crate::numeric::{CompileError, CompiledFn, compile_f64_with_ctx};
use om_core::Symbol;
use std::collections::BTreeSet;
pub(super) fn machine_source(e: &Expr, ctx: &Interrupt) -> Result<(), EvalError> {
    let mut pending = vec![e];
    while let Some(e) = pending.pop() {
        ctx.tick()?;
        if let Some(n) = e.as_number()
            && matches!(n.precision(), om_num::Precision::Bits(_))
        {
            return Err(error("此算法只有机器精度路径，不能静默降低高精度输入"));
        }
        if let ExprKind::Normal(n) = e.kind() {
            pending.push(&n.head);
            pending.extend(n.args.iter());
        }
    }
    Ok(())
}
pub(super) fn scalar(ev: &mut Evaluator, e: &Expr, ctx: &Interrupt) -> Result<f64, EvalError> {
    machine_source(e, ctx)?;
    let e = ev.evaluate(e, ctx)?;
    machine_source(&e, ctx)?;
    if e.as_number().is_some() {
        return machine(&e);
    }
    let n = om_simplify::numeval::approximate(&e, om_num::Precision::Machine, ctx)?
        .ok_or_else(|| error("需要可数值化的有限实标量"))?;
    machine(&Expr::number(n))
}
pub(super) fn uint(ev: &mut Evaluator, e: &Expr, ctx: &Interrupt) -> Result<usize, EvalError> {
    let e = ev.evaluate(e, ctx)?;
    if let Some(Number::Integer(n)) = e.as_number() {
        usize::try_from(n).map_err(|_| error("需要非负整数控制参数"))
    } else {
        Err(error("需要整数控制参数"))
    }
}
pub(super) fn precision(
    ev: &mut Evaluator,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<(), EvalError> {
    if let Some(e) = args.options.get("WorkingPrecision") {
        let e = ev.evaluate(e, ctx)?;
        if !matches!(e.kind(), ExprKind::String(s) if matches!(&**s, "machine" | "MachinePrecision"))
            && e.as_symbol().is_none_or(|s| s.name() != "MachinePrecision")
        {
            return Err(error("此算法首版仅支持机器精度"));
        }
    }
    Ok(())
}
pub(super) fn axis(ev: &mut Evaluator, e: &Expr, ctx: &Interrupt) -> Result<(f64, f64), EvalError> {
    if !e.is_head(B::LIST) || e.args().len() != 3 {
        return Err(error("需要一维范围[坐标,起点,终点]或坐标:起点..终点"));
    }
    super::calculus_source::axis(&e.args()[0])?;
    let a = scalar(ev, &e.args()[1], ctx)?;
    let b = scalar(ev, &e.args()[2], ctx)?;
    if !(b - a).is_finite() {
        return Err(error("区间跨度超出机器范围"));
    }
    Ok((a, b))
}
/// Fresh names avoid every stored definition, lexical scope and input symbol.
pub(super) fn variables(
    ev: &Evaluator,
    input: &Expr,
    n: usize,
    ctx: &Interrupt,
) -> Result<Vec<Symbol>, EvalError> {
    let mut used = BTreeSet::new();
    used.extend(ev.defs.own.keys().copied());
    used.extend(ev.defs.attrs.keys().copied());
    let mut pending = vec![input];
    pending.extend(ev.defs.own.values());
    for (s, rules) in &ev.defs.down {
        ctx.tick()?;
        used.insert(*s);
        for rule in rules {
            pending.push(&rule.lhs);
            pending.push(&rule.rhs);
        }
    }
    for scope in &ev.scopes {
        used.extend(scope.keys().copied());
        pending.extend(scope.values().filter_map(Option::as_ref));
    }
    while let Some(e) = pending.pop() {
        ctx.tick()?;
        if let Some(s) = e.as_symbol() {
            used.insert(s);
        }
        if let ExprKind::Normal(e) = e.kind() {
            pending.push(&e.head);
            pending.extend(e.args.iter());
        }
    }
    let mut variables = vec![];
    let mut index = 0;
    while variables.len() < n {
        ctx.tick()?;
        let s = Symbol::intern(&format!("$om$analysis${index}"));
        index += 1;
        if used.insert(s) {
            variables.push(s);
        }
    }
    Ok(variables)
}
/// Prepare without canonical arithmetic so removable source poles still fail during sampling.
pub(super) fn callback(
    ev: &Evaluator,
    function: &Expr,
    arguments: Vec<Expr>,
    variables: &[Symbol],
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    let call = Expr::normal(function.clone(), arguments);
    let locals: Vec<_> = variables.iter().map(|s| (*s, None)).collect();
    let raw = ev.prepare_numeric(&call, &locals, ctx)?;
    machine_source(&raw, ctx)?;
    Ok(raw)
}
pub(super) fn compile(
    e: &Expr,
    variables: &[Symbol],
    ctx: &Interrupt,
) -> Result<CompiledFn, EvalError> {
    compile_f64_with_ctx(e, variables, ctx).map_err(|e| match e {
        CompileError::Abort(e) => e.into(),
        e => error(&e.to_string()),
    })
}
