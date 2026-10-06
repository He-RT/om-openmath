//! Finite real observations; pure CSV/JSON strings are never interpreted as executable input.
use super::numeric_callback as source;
use super::*;
use om_core::Symbol;
use std::collections::BTreeSet;
pub(super) struct Data {
    pub inputs: Vec<Vec<f64>>,
    pub observed: Vec<f64>,
}
pub(super) struct Parameters {
    pub names: Vec<Symbol>,
    pub initial: Vec<f64>,
}
pub(super) fn axes(e: &Expr) -> Result<Vec<Symbol>, EvalError> {
    let values = if e.is_head(B::LIST) {
        e.args().to_vec()
    } else {
        vec![e.clone()]
    };
    if !(1..=16).contains(&values.len()) {
        return Err(error("fit需要1..16个有序输入变量"));
    }
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|e| {
            let s = super::calculus_source::axis(e)?;
            if !seen.insert(s) {
                return Err(error("fit输入变量必须互异"));
            }
            Ok(s)
        })
        .collect()
}
pub(super) fn parameters(
    ev: &mut Evaluator,
    e: &Expr,
    nonlinear: bool,
    vars: &[Symbol],
    ctx: &Interrupt,
) -> Result<Parameters, EvalError> {
    if e.is_head(B::LIST) && e.args().iter().all(|p| p.as_symbol().is_some()) {
        if nonlinear {
            return Err(error("nonlinear需要参数起点记录，例如parameters:{a:1,b:0}"));
        }
        let names = axes(e)?;
        if names.iter().any(|p| vars.contains(p)) {
            return Err(error("拟合参数与输入变量不能重名"));
        }
        return Ok(Parameters {
            initial: vec![0.; names.len()],
            names,
        });
    }
    source::machine_source(e, ctx)?;
    let e = ev.evaluate(e, ctx)?;
    if !e.is_head(B::RECORD) || !(1..=16).contains(&e.args().len()) {
        return Err(error(
            "parameters需要1..16个文字键的起点记录；linear也接受参数符号列表",
        ));
    }
    super::csv_data::entries(&e, ctx)?;
    let mut names = vec![];
    let mut initial = vec![];
    for entry in e.args() {
        let name = Symbol::intern(string(&entry.args()[0])?);
        super::calculus_source::axis(&Expr::sym(name))?;
        if name.name().is_empty() || vars.contains(&name) {
            return Err(error("拟合参数不能空名或与输入变量重名"));
        }
        names.push(name);
        initial.push(source::scalar(ev, &entry.args()[1], ctx)?);
    }
    Ok(Parameters { names, initial })
}
pub(super) fn read(
    ev: &mut Evaluator,
    e: &Expr,
    vars: &[Symbol],
    target: &str,
    explicit_target: bool,
    ctx: &Interrupt,
) -> Result<Data, EvalError> {
    source::machine_source(e, ctx)?;
    let e = ev.evaluate(e, ctx)?;
    let mut inputs = vec![];
    let mut observed = vec![];
    if e.is_head(B::DATA_TABLE) {
        super::csv_data::validate_table(&e, ctx)?;
        if vars.iter().any(|v| v.name() == target) {
            return Err(error("表格的target列不能同时作为输入变量列"));
        }
        if e.args()[1].args().is_empty() || e.args()[1].args().len() > 10000 {
            return Err(error("拟合表格需要1..10000样本"));
        }
        for row in e.args()[1].args() {
            let row = super::csv_data::entries(row, ctx)?;
            let mut x = vec![];
            for v in vars {
                x.push(source::scalar(
                    ev,
                    row.get(v.name())
                        .ok_or_else(|| error("拟合表格缺少输入变量列"))?,
                    ctx,
                )?);
            }
            inputs.push(x);
            observed.push(source::scalar(
                ev,
                row.get(target)
                    .ok_or_else(|| error("拟合表格缺少target列"))?,
                ctx,
            )?);
        }
    } else {
        if explicit_target {
            return Err(error(
                "target选项只选择DataTable观测列，普通样本行使用最后一列",
            ));
        }
        if !e.is_head(B::LIST) || e.args().is_empty() || e.args().len() > 10000 {
            return Err(error("fit数据需要1..10000行有限实数样本"));
        }
        for row in e.args() {
            ctx.tick()?;
            if !row.is_head(B::LIST) || row.args().len() != vars.len() + 1 {
                return Err(error("每行数据需要输入变量值，最后一项为观测值"));
            }
            inputs.push(
                row.args()[..vars.len()]
                    .iter()
                    .map(|e| source::scalar(ev, e, ctx))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            observed.push(source::scalar(ev, &row.args()[vars.len()], ctx)?);
        }
    }
    if inputs.len().saturating_mul(vars.len() + 1) > 100000 {
        return Err(error("拟合输入超过100000标量限额"));
    }
    Ok(Data { inputs, observed })
}
