//! Actual ordered records, closed slices and small exact/symbolic matrix products.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel};
use om_num::{Integer, Number};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    for (symbol, f, arity, modern, wolfram, zh, examples) in [
        (
            B::RECORD,
            record as crate::BuiltinFn,
            Arity::Any,
            "record(...entries)",
            "Record[...entries]",
            "构造具有文字键的有序记录。",
            &["Record[\"a\"->2]"][..],
        ),
        (
            B::DOT,
            dot as crate::BuiltinFn,
            Arity::Exactly(2),
            "dot(a,b)",
            "Dot[a,b]",
            "计算矩阵乘积；向量内积共轭第一向量。",
            &["Dot[{{1,2},{3,4}},{5,6}]", "Dot[{1,2},{3,4}]"][..],
        ),
    ] {
        let name = symbol.name();
        specs.insert(
            name,
            BuiltinSpec {
                symbol,
                f,
                arity,
                attrs: A::PROTECTED,
                doc: DocEntry {
                    name,
                    modern,
                    wolfram,
                    summary_zh: zh,
                    summary_en: if symbol == B::RECORD {
                        "Construct an ordered record with literal string keys."
                    } else {
                        "Multiply matrices; vector inner products conjugate the first vector."
                    },
                    examples,
                    category: "Structure",
                },
            },
        );
    }
}
fn invalid(ev: &mut Evaluator, name: &str, reason: &str) -> Option<Expr> {
    ev.message(name, "shape", reason.into(), MsgLevel::Warning);
    None
}
fn record(ev: &mut Evaluator, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let mut keys = BTreeSet::new();
    for entry in args {
        ctx.tick()?;
        if !entry.is_head(B::RULE) || entry.args().len() != 2 {
            return Ok(invalid(ev, "Record", "记录需要文字键值规则。"));
        }
        let ExprKind::String(key) = entry.args()[0].kind() else {
            return Ok(invalid(ev, "Record", "记录键必须是文字字符串。"));
        };
        if !keys.insert(key.to_string()) {
            return Ok(invalid(ev, "Record", "记录键重复。"));
        }
    }
    Ok(None)
}
pub(crate) fn part(
    ev: &mut Evaluator,
    value: &Expr,
    index: &Expr,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if let ExprKind::String(key) = index.kind() {
        if !value.is_head(B::RECORD) {
            return Ok(invalid(ev, "Part", "文字键索引需要记录。"));
        }
        return Ok(value
            .args()
            .iter()
            .find(|entry| {
                entry.is_head(B::RULE)
                    && entry.args().len() == 2
                    && entry.args()[0] == Expr::string(key.as_ref())
            })
            .map(|entry| entry.args()[1].clone())
            .or_else(|| invalid(ev, "Part", "记录缺少请求的键。")));
    }
    if index.is_head(B::SPAN) {
        let [start, end] = index.args() else {
            return Ok(invalid(ev, "Part", "切片需要两个端点。"));
        };
        let convert = |e: &Expr| -> Option<usize> {
            let Some(Number::Integer(n)) = e.as_number() else {
                return None;
            };
            if n == &Integer::ZERO {
                return None;
            }
            if n < &Integer::ZERO {
                usize::try_from(n.clone().into_parts().1)
                    .ok()
                    .and_then(|n| value.args().len().checked_sub(n))
                    .map(|n| n + 1)
            } else {
                usize::try_from(n).ok().filter(|n| *n <= value.args().len())
            }
        };
        let Some((start, end)) = convert(start).zip(convert(end)) else {
            return Ok(invalid(ev, "Part", "切片端点必须存在，且不能为0。"));
        };
        let count = start.abs_diff(end) + 1;
        let mut values = Vec::with_capacity(count);
        for i in 0..count {
            ctx.tick()?;
            let offset = if start <= end {
                start + i - 1
            } else {
                start - i - 1
            };
            values.push(value.args()[offset].clone());
        }
        return Ok(Some(Expr::normal(value.head(), values)));
    }
    Ok(None)
}
fn shape(value: &Expr) -> Option<(usize, usize, bool)> {
    if !value.is_head(B::LIST) || value.args().is_empty() || value.args().len() > 64 {
        return None;
    }
    if value.args().iter().all(|v| !v.is_head(B::LIST)) {
        return Some((1, value.args().len(), true));
    }
    let columns = value.args()[0].args().len();
    (columns > 0
        && columns <= 64
        && value.args().iter().all(|v| {
            v.is_head(B::LIST)
                && v.args().len() == columns
                && v.args().iter().all(|e| !e.is_head(B::LIST))
        }))
    .then_some((value.args().len(), columns, false))
}
fn dot(ev: &mut Evaluator, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let Some((ar, ac, av)) = shape(&args[0]) else {
        return Ok(invalid(ev, "Dot", "需要非空向量或最多64×64的矩阵。"));
    };
    let Some((br, bc, bv)) = shape(&args[1]) else {
        return Ok(invalid(ev, "Dot", "需要非空向量或最多64×64的矩阵。"));
    };
    let inner = ac;
    let b_inner = if bv { bc } else { br };
    if inner != b_inner {
        return Ok(invalid(ev, "Dot", "矩阵或向量维度不匹配。"));
    }
    let rows = if av { 1 } else { ar };
    let columns = if bv { 1 } else { bc };
    let mut result = vec![];
    for i in 0..rows {
        let mut row = vec![];
        for j in 0..columns {
            let mut sum = vec![];
            for k in 0..inner {
                ctx.tick()?;
                let mut a = if av {
                    args[0].args()[k].clone()
                } else {
                    args[0].args()[i].args()[k].clone()
                };
                if av && bv {
                    a = ev.evaluate(&Expr::call(B::CONJUGATE, [a]), ctx)?;
                }
                let b = if bv {
                    args[1].args()[k].clone()
                } else {
                    args[1].args()[k].args()[j].clone()
                };
                sum.push(om_core::mul([a, b]));
            }
            row.push(om_core::add(sum));
        }
        result.push(if bv {
            row.remove(0)
        } else {
            Expr::call(B::LIST, row)
        });
    }
    Ok(Some(if av {
        result.remove(0)
    } else {
        Expr::call(B::LIST, result)
    }))
}
