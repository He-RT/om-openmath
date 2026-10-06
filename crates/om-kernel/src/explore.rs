//! Actual isolated computations and credential-free readonly contexts; no source mutation/replay.
use crate::{notebook::StatementRecord, plot::PlotError, protocol::*};
use om_core::{Expr, Interrupt};
use om_eval::{
    Evaluator,
    explore::{ExplorePlan, ReadonlyState},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
fn error(s: &str) -> PlotError {
    PlotError::Invalid(format!("explore: {s}"))
}
/// In-memory snapshot retained only with the current output.
pub(crate) struct Snapshot {
    pub eval: Evaluator,
    pub plan: ExplorePlan,
}
pub(crate) fn capture(
    value: &Expr,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<Snapshot>, PlotError> {
    if value.head_symbol().is_none_or(|s| s.name() != "Explore") {
        return Ok(None);
    }
    let mut snapshot = ev.fork_readonly();
    if snapshot
        .history
        .last()
        .is_some_and(|(_, last)| last == value)
    {
        snapshot.history.pop();
    }
    Ok(Some(Snapshot {
        plan: om_eval::explore::plan(value, ev, ctx)?,
        eval: snapshot,
    }))
}
fn values(s: &Snapshot) -> BTreeMap<String, f64> {
    s.plan
        .controls
        .iter()
        .map(|c| (c.name.clone(), c.initial))
        .collect()
}
pub(crate) fn compute(
    s: &Snapshot,
    view_id: &str,
    out_index: u32,
    parameters: BTreeMap<String, f64>,
    revision: u32,
    ctx: &Interrupt,
) -> Result<ExploreResult, PlotError> {
    let started = ctx.clock.as_ref().map(|clock| clock.now_ms());
    if parameters.len() != s.plan.controls.len() {
        return Err(error("参数集合必须与controls相同"));
    }
    let mut locals = vec![];
    for c in &s.plan.controls {
        ctx.tick()?;
        let v = parameters
            .get(&c.name)
            .copied()
            .ok_or_else(|| error("缺少参数"))?;
        if !v.is_finite() || v < c.range.0 || v > c.range.1 {
            return Err(error("参数非有限或超出范围"));
        }
        let name = crate::plot::axis(&c.name)?;
        locals.push((name, Expr::real(v)));
    }
    let mut ev = s.eval.fork_with_locals(&locals);
    ev.settings.record_steps = false;
    let value = ev.evaluate(&s.plan.expression, ctx)?;
    if value.head_symbol().is_some_and(|s| s.name() == "Explore") {
        return Err(error("不支持返回嵌套explore"));
    }
    let mut record = StatementRecord {
        input: s.plan.expression.clone(),
        value,
        steps: None,
        suppress_output: false,
        out_index,
        solver: ev.take_solver_result(),
        scientific: ev.take_scientific_result(),
        view_id: view_id.into(),
        exploration: None,
    };
    let value_token = if matches!(
        crate::output::values::kind(&record.value),
        ValueKind::List | ValueKind::Matrix | ValueKind::Record | ValueKind::Table
    ) {
        Some(value_token(&record.value, ctx)?)
    } else {
        None
    };
    let mut item = crate::output::pack(&mut record, &ev, ctx, false)?;
    // Ephemeral slider results are not stored in notebook history; do not expose pages bound to the held Explore record.
    if let OutputItem::Expr { presentation, .. } = &mut item {
        *presentation = None;
    }
    let messages = ev.messages.take().iter().map(Message::from).collect();
    Ok(ExploreResult {
        view_id: view_id.into(),
        revision,
        values: parameters,
        item: Box::new(item),
        messages,
        value_token,
        timing_ms: started
            .and_then(|start| ctx.clock.as_ref().map(|clock| clock.now_ms() - start))
            .filter(|v| v.is_finite() && *v >= 0.)
            .unwrap_or(0.),
    })
}
pub(crate) fn initial(record: &StatementRecord, ctx: &Interrupt) -> Result<OutputItem, PlotError> {
    let s = record
        .exploration
        .as_ref()
        .ok_or_else(|| error("缺少真实快照"))?;
    let result = compute(s, &record.view_id, record.out_index, values(s), 0, ctx)?;
    Ok(OutputItem::Explore {
        out_index: record.out_index,
        view_id: record.view_id.clone(),
        input_form: om_format::input_form(&record.value),
        modern_form: om_format::modern_form(&record.value),
        controls: s
            .plan
            .controls
            .iter()
            .map(|c| ExploreControl {
                name: c.name.clone(),
                range: c.range,
                initial: c.initial,
            })
            .collect(),
        result: Box::new(result),
    })
}
#[derive(Serialize, Deserialize)]
struct Context {
    version: u32,
    view_id: String,
    out_index: u32,
    source: u32,
    nodes: Vec<Node>,
    own: Vec<(String, u32)>,
    down: Vec<(String, Vec<StoredRule>)>,
    attributes: Vec<(String, u16)>,
    history: Vec<(u32, u32)>,
    settings: om_eval::EvalSettings,
    random_state: u64,
}
#[derive(Serialize, Deserialize)]
enum Node {
    Number(om_num::Number),
    Symbol(String),
    String(String),
    Normal { head: u32, args: Vec<u32> },
}
#[derive(Serialize, Deserialize)]
struct StoredRule {
    lhs: u32,
    rhs: u32,
    delayed: bool,
}
fn encode(e: &Expr, nodes: &mut Vec<Node>, ctx: &Interrupt) -> Result<u32, PlotError> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Exit(usize),
    }
    let mut work = vec![Frame::Enter(e)];
    let mut stack = vec![];
    while let Some(frame) = work.pop() {
        ctx.tick()?;
        let node = match frame {
            Frame::Enter(e) => match e.kind() {
                om_core::ExprKind::Number(n) => Node::Number(n.clone()),
                om_core::ExprKind::Symbol(s) => Node::Symbol(s.name().into()),
                om_core::ExprKind::String(s) => Node::String(s.to_string()),
                om_core::ExprKind::Normal(n) => {
                    work.push(Frame::Exit(n.args.len()));
                    for arg in n.args.iter().rev() {
                        work.push(Frame::Enter(arg));
                    }
                    work.push(Frame::Enter(&n.head));
                    continue;
                }
            },
            Frame::Exit(count) => {
                let start = stack
                    .len()
                    .checked_sub(count + 1)
                    .ok_or_else(|| error("结构栈无效"))?;
                let head = stack[start];
                let args = stack[start + 1..].to_vec();
                stack.truncate(start);
                Node::Normal { head, args }
            }
        };
        if nodes.len() >= 100000 {
            return Err(error("只读状态超过100000结构节点"));
        }
        stack.push(nodes.len() as u32);
        nodes.push(node);
    }
    stack.pop().ok_or_else(|| error("空表达式状态"))
}
pub(crate) fn context(record: &StatementRecord, ctx: &Interrupt) -> Result<String, PlotError> {
    let s = record
        .exploration
        .as_ref()
        .ok_or_else(|| error("输出不是explore"))?;
    let state = s.eval.readonly_state();
    let mut nodes = vec![];
    let source = encode(&record.value, &mut nodes, ctx)?;
    let own = state
        .own
        .into_iter()
        .map(|(symbol, e)| Ok((symbol.name().into(), encode(&e, &mut nodes, ctx)?)))
        .collect::<Result<_, PlotError>>()?;
    let down = state
        .down
        .into_iter()
        .map(|(symbol, rules)| {
            Ok((
                symbol.name().into(),
                rules
                    .into_iter()
                    .map(|r| {
                        Ok(StoredRule {
                            lhs: encode(&r.lhs, &mut nodes, ctx)?,
                            rhs: encode(&r.rhs, &mut nodes, ctx)?,
                            delayed: r.delayed,
                        })
                    })
                    .collect::<Result<Vec<_>, PlotError>>()?,
            ))
        })
        .collect::<Result<_, PlotError>>()?;
    let history = state
        .history
        .iter()
        .map(|(a, b)| Ok((encode(a, &mut nodes, ctx)?, encode(b, &mut nodes, ctx)?)))
        .collect::<Result<_, PlotError>>()?;
    let encoded = serde_json::to_string(&Context {
        version: 1,
        view_id: record.view_id.clone(),
        out_index: record.out_index,
        source,
        nodes,
        own,
        down,
        attributes: state
            .attributes
            .iter()
            .map(|(s, a)| (s.name().into(), *a))
            .collect(),
        history,
        settings: state.settings,
        random_state: state.random_state,
    })
    .map_err(|_| error("无法编码只读状态"))?;
    if encoded.len() > 524288 {
        return Err(error("只读状态超过512KiB预算"));
    }
    Ok(encoded)
}
fn source_expression(source: &str, ctx: &Interrupt) -> Result<Expr, PlotError> {
    ctx.tick()?;
    if source.len() > 65536 {
        return Err(error("表达式源码超限"));
    }
    om_parse::parse_expr(source, om_parse::Dialect::Wolfram).map_err(|_| error("表达式源码无效"))
}
fn restore(source: &str, ctx: &Interrupt) -> Result<(Snapshot, String, u32), PlotError> {
    if source.len() > 524288 {
        return Err(error("只读状态超过512KiB"));
    }
    let c: Context = serde_json::from_str(source).map_err(|_| error("只读状态格式无效"))?;
    if c.version != 1
        || c.nodes.len() > 100000
        || c.own.len() > 4096
        || c.down.len() > 4096
        || c.history.len() > 4096
        || c.attributes.len() > 4096
    {
        return Err(error("只读状态版本/限额无效"));
    }
    if !(1..=1_000_000).contains(&c.settings.iteration_limit)
        || !(1..=1024).contains(&c.settings.recursion_limit)
    {
        return Err(error("只读状态求值限制无效"));
    }
    let exprs = decode_nodes(c.nodes, ctx)?;
    let get = |i: u32| {
        exprs
            .get(i as usize)
            .cloned()
            .ok_or_else(|| error("只读状态索引越界"))
    };
    let own = c
        .own
        .iter()
        .map(|(s, e)| Ok((crate::plot::axis(s)?, get(*e)?)))
        .collect::<Result<_, PlotError>>()?;
    let down = c
        .down
        .iter()
        .map(|(s, rules)| {
            ctx.tick()?;
            if rules.len() > 4096 {
                return Err(error("函数定义超过限额"));
            }
            Ok((
                crate::plot::axis(s)?,
                rules
                    .iter()
                    .map(|r| {
                        Ok(om_eval::Rule {
                            lhs: get(r.lhs)?,
                            rhs: get(r.rhs)?,
                            delayed: r.delayed,
                        })
                    })
                    .collect::<Result<Vec<_>, PlotError>>()?,
            ))
        })
        .collect::<Result<_, PlotError>>()?;
    let history = c
        .history
        .iter()
        .map(|(a, b)| Ok((get(*a)?, get(*b)?)))
        .collect::<Result<_, PlotError>>()?;
    let attributes = c
        .attributes
        .iter()
        .map(|(s, b)| Ok((crate::plot::axis(s)?, *b)))
        .collect::<Result<_, PlotError>>()?;
    let ev = Evaluator::from_readonly_state(ReadonlyState {
        own,
        down,
        history,
        attributes,
        settings: c.settings,
        random_state: c.random_state,
    })?;
    let held = get(c.source)?;
    let plan = om_eval::explore::plan(&held, &ev, ctx)?;
    Ok((Snapshot { eval: ev, plan }, c.view_id, c.out_index))
}
pub(crate) fn detached(
    source: &str,
    values: BTreeMap<String, f64>,
    revision: u32,
    ctx: &Interrupt,
) -> Result<ExploreResult, PlotError> {
    let (s, id, index) = restore(source, ctx)?;
    compute(&s, &id, index, values, revision, ctx)
}
pub(crate) fn detached_plot(
    source: &str,
    values: BTreeMap<String, f64>,
    request: PlotRequest,
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    let (s, _, _) = restore(source, ctx)?;
    if values.len() != s.plan.controls.len() {
        return Err(error("参数集合必须与controls相同"));
    }
    let mut locals = vec![];
    for c in &s.plan.controls {
        ctx.tick()?;
        let v = values
            .get(&c.name)
            .copied()
            .ok_or_else(|| error("缺少参数"))?;
        if !v.is_finite() || v < c.range.0 || v > c.range.1 {
            return Err(error("参数非有限/越界"));
        }
        locals.push((crate::plot::axis(&c.name)?, Expr::real(v)));
    }
    crate::plot::sample(&request, &s.eval.fork_with_locals(&locals), ctx)
}

pub(crate) fn detached_expression(
    source: &str,
    values: BTreeMap<String, f64>,
    expression: &str,
    numeric: bool,
    ctx: &Interrupt,
) -> Result<crate::views::ExpressionView, PlotError> {
    let (s, _, _) = restore(source, ctx)?;
    if values.len() != s.plan.controls.len() {
        return Err(error("参数集合必须与controls相同"));
    }
    let mut locals = vec![];
    for c in &s.plan.controls {
        ctx.tick()?;
        let v = values
            .get(&c.name)
            .copied()
            .ok_or_else(|| error("缺少参数"))?;
        if !v.is_finite() || v < c.range.0 || v > c.range.1 {
            return Err(error("参数非有限/越界"));
        }
        locals.push((crate::plot::axis(&c.name)?, Expr::real(v)));
    }
    let mut ev = s.eval.fork_with_locals(&locals);
    let expr = source_expression(expression, ctx)?;
    let expr = if numeric {
        Expr::call(om_core::BUILTIN::N, [expr, Expr::int(20)])
    } else {
        expr
    };
    let value = ev.evaluate(&expr, ctx)?;
    Ok(crate::output::expression_view(&value))
}

fn decode_nodes(nodes: Vec<Node>, ctx: &Interrupt) -> Result<Vec<Expr>, PlotError> {
    let mut exprs: Vec<Expr> = vec![];
    for node in nodes {
        ctx.tick()?;
        let expr = match node {
            Node::Number(n) => {
                fn valid(n: &om_num::Number) -> bool {
                    match n {
                        om_num::Number::Real(om_num::Real::Machine(v)) => v.is_finite(),
                        om_num::Number::Real(om_num::Real::Big(v)) => {
                            v.repr().is_finite() && v.precision() <= 16384
                        }
                        om_num::Number::Complex(c) => {
                            !matches!(c.re, om_num::Number::Complex(_))
                                && !matches!(c.im, om_num::Number::Complex(_))
                                && valid(&c.re)
                                && valid(&c.im)
                        }
                        _ => true,
                    }
                }
                if !valid(&n) {
                    return Err(error("只读状态包含无效数值/精度"));
                }
                Expr::number(n)
            }
            Node::Symbol(s) => Expr::symbol(&s),
            Node::String(s) => Expr::string(&s),
            Node::Normal { head, args } => {
                let head = exprs
                    .get(head as usize)
                    .ok_or_else(|| error("无效向前/循环结构引用"))?
                    .clone();
                let args = args
                    .iter()
                    .map(|i| {
                        ctx.tick()?;
                        exprs
                            .get(*i as usize)
                            .cloned()
                            .ok_or_else(|| error("无效结构引用"))
                    })
                    .collect::<Result<Vec<_>, PlotError>>()?;
                Expr::normal(head, args)
            }
        };
        exprs.push(expr);
    }
    Ok(exprs)
}
#[derive(Serialize, Deserialize)]
struct ValueToken {
    version: u32,
    root: u32,
    nodes: Vec<Node>,
}
pub(crate) fn value_token(value: &Expr, ctx: &Interrupt) -> Result<String, PlotError> {
    let mut nodes = vec![];
    let root = encode(value, &mut nodes, ctx)?;
    let out = serde_json::to_string(&ValueToken {
        version: 1,
        root,
        nodes,
    })
    .map_err(|_| error("无法编码值"))?;
    if out.len() > 8 * 1024 * 1024 {
        return Err(error("值图超过8MiB"));
    }
    Ok(out)
}
pub(crate) fn token_value(token: &str, ctx: &Interrupt) -> Result<Expr, PlotError> {
    if token.len() > 8 * 1024 * 1024 {
        return Err(error("值图超过8MiB"));
    }
    let value: ValueToken = serde_json::from_str(token).map_err(|_| error("值图格式无效"))?;
    if value.version != 1 || value.nodes.len() > 100000 {
        return Err(error("值图版本/节点数无效"));
    }
    let nodes = decode_nodes(value.nodes, ctx)?;
    nodes
        .get(value.root as usize)
        .cloned()
        .ok_or_else(|| error("值图索引无效"))
}
