//! Read-only function documentation comes from the same audited catalog; planning cannot grant execution.
use super::*;
use om_core::{Symbol, catalog};
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn entries() -> Result<&'static [Value], EvalError> {
    catalog::documentation()["functions"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| error("功能文档目录结构无效"))
}
fn name(ev: &Evaluator, e: &Expr) -> Result<String, EvalError> {
    if let ExprKind::String(s) = e.kind() {
        return Ok(s.to_string());
    }
    let symbol = e
        .as_symbol()
        .ok_or_else(|| error("功能名需要文字或符号，不执行名称表达式"))?;
    if let Some(value) = ev.own(symbol)
        && let ExprKind::String(s) = value.kind()
    {
        return Ok(s.to_string());
    }
    Ok(symbol.name().into())
}
fn docs(
    ev: &Evaluator,
    name: &str,
    ctx: &Interrupt,
) -> Result<(&'static Value, Option<&'static catalog::FunctionDescriptor>), EvalError> {
    let symbol = Symbol::intern(name);
    if ev.own(symbol).is_some() || !ev.defs.down_values(symbol).is_empty() {
        return Err(error(
            "该名字已被用户绑定遮蔽；内置文档请使用未遮蔽的兼容名称",
        ));
    }
    let runtime = catalog::by_runtime(name).or_else(|| catalog::by_alias(name));
    for entry in entries()? {
        ctx.tick()?;
        if runtime.is_some_and(|r| entry["id"].as_str() == Some(&r.id))
            || runtime.is_none()
                && (entry["name"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
                    || entry["compatibility_names"]
                        .as_array()
                        .is_some_and(|names| {
                            names
                                .iter()
                                .any(|n| n.as_str().is_some_and(|n| n.eq_ignore_ascii_case(name)))
                        }))
        {
            return Ok((entry, runtime));
        }
    }
    Err(error("功能目录没有这个名称"))
}
fn convert(value: Value, ctx: &Interrupt) -> Result<Expr, EvalError> {
    let text = serde_json::to_string(&value).map_err(|_| error("无法编码功能描述"))?;
    super::json_data::parse(&text, ctx)
}
pub(super) fn dispatch(
    ev: &Evaluator,
    callee: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let value = match callee {
        "Help" | "Options" => {
            let wanted = name(ev, args.values[0])?;
            let (entry, runtime) = docs(ev, &wanted, ctx)?;
            let executable = runtime.is_some();
            let mut value = json!({"id":entry["id"],"name":entry["name"],"status":entry["status"],"category":entry["category"],"executable":executable});
            if callee == "Help" {
                value["documentation"] = entry.clone();
                value["runtime"] = runtime
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|_| error("无法编码当前接口"))?
                    .unwrap_or(Value::Null);
                value["summary"] = runtime
                    .and_then(|r| Evaluator::doc(Symbol::intern(&r.name)))
                    .map(|doc| Value::String(doc.summary_zh.into()))
                    .unwrap_or_else(|| entry["target_support"].clone());
            } else {
                value["options"] = runtime
                    .map(|r| serde_json::to_value(&r.options))
                    .transpose()
                    .map_err(|_| error("无法编码选项"))?
                    .unwrap_or_else(|| json!([]));
                value["documentation_parameters"] = entry["parameters"].clone();
                value["documentation_is_not_executable_schema"] = Value::Bool(true);
            }
            value
        }
        "Functions" => {
            let category = args
                .options
                .get("Category")
                .map(|e| string(e))
                .transpose()?;
            if let Some(category) = category
                && !catalog::documentation()["categories"]
                    .as_array()
                    .is_some_and(|all| all.iter().any(|c| c["id"].as_str() == Some(category)))
            {
                return Err(error("未知功能分类"));
            }
            let stage = args
                .options
                .get("Stage")
                .map(|e| string(e))
                .transpose()?
                .unwrap_or("current");
            if !matches!(stage, "current" | "planned" | "deferred") {
                return Err(error("stage只支持current/planned/deferred"));
            }
            let mut output = vec![];
            for entry in entries()? {
                ctx.tick()?;
                let executable = entry["runtime_names"]
                    .as_array()
                    .is_some_and(|n| !n.is_empty());
                let selected = if stage == "current" {
                    executable
                } else {
                    entry["status"].as_str() == Some(stage)
                };
                if !selected || category.is_some_and(|c| entry["category"].as_str() != Some(c)) {
                    continue;
                }
                output.push(json!({"id":entry["id"],"name":entry["name"],"category":entry["category"],"status":entry["status"],"kind":entry["kind"],"executable":executable,"runtime_names":entry["runtime_names"],"signatures":if executable{&entry["current_modern_signatures"]}else{&entry["signatures"]}}));
            }
            Value::Array(output)
        }
        "Capabilities" => {
            let mut ids = BTreeSet::new();
            let mut platforms = BTreeSet::new();
            for function in catalog::functions() {
                ctx.tick()?;
                ids.insert(function.id.clone());
                platforms.extend(function.platforms.iter().cloned());
            }
            json!({"kernel_version":env!("CARGO_PKG_VERSION"),"metadata_version":catalog::METADATA_VERSION,"schema_version":catalog::SCHEMA_VERSION,
                "function_ids":ids,"callback_count":catalog::functions().len(),"computation_platforms":platforms,"host_presentation":null,"task_permissions":null,
                "note":"仅报告实际内核接口；宿主展示与任务权限需由宿主查询，规划目录不授予执行能力。"})
        }
        _ => return Ok(None),
    };
    Ok(Some(convert(value, ctx)?))
}
