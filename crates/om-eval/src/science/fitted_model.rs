//! Held source is compiled data: stored models cannot run definitions and numeric predictions stay consistent.
use super::numeric_callback as numeric;
use super::*;
use om_core::Symbol;
fn validate(
    object: &Expr,
    ctx: &Interrupt,
) -> Result<(Vec<Symbol>, crate::numeric::CompiledFn), EvalError> {
    if !object.is_head(B::FITTED_MODEL_DATA)
        || object.args().len() != 3
        || object.args()[2] != Expr::string("machine")
    {
        return Err(error("FittedModelData形状/精度无效"));
    }
    let variables = super::fitting_data::axes(&object.args()[0])?;
    numeric::machine_source(&object.args()[1], ctx)?;
    // Compilation validates every stored head, variable and raw domain structure, without evaluating source.
    let program = numeric::compile(&object.args()[1], &variables, ctx)?;
    Ok((variables, program))
}
fn beta(
    ev: &mut Evaluator,
    object: &Expr,
    variables: &[Symbol],
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    if args.len() != variables.len() {
        return Err(error("拟合模型调用参数维度与输入变量不一致"));
    }
    for arg in args {
        numeric::machine_source(arg, ctx)?;
        if matches!(
            arg.head_symbol(),
            Some(B::LIST | B::RECORD | B::DATA_TABLE | B::SERIES_DATA)
        ) {
            return Err(error("拟合模型输入需要独立标量参数，不能把容器当坐标"));
        }
    }
    let function = Expr::call(
        B::FUNCTION,
        [object.args()[0].clone(), object.args()[1].clone()],
    );
    crate::pure::apply(ev, &function, args, ctx)?.ok_or_else(|| error("拟合模型符号代入失败"))
}
pub(super) fn source(
    ev: &mut Evaluator,
    object: &Expr,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    let (variables, _) = validate(object, ctx)?;
    beta(ev, object, &variables, args, ctx)
}
pub(super) fn call(
    ev: &mut Evaluator,
    object: &Expr,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    let (variables, program) = validate(object, ctx)?;
    if args.len() != variables.len() {
        return Err(error("拟合模型调用参数个数无效"));
    }
    if args.iter().all(|a| a.as_number().is_some()) {
        let point = args.iter().map(machine).collect::<Result<Vec<_>, _>>()?;
        return real(program.eval_with_ctx(&point, &mut vec![], ctx)?);
    }
    beta(ev, object, &variables, args, ctx)
}
