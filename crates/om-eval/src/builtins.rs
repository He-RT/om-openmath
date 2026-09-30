//! The first functional registry: definitions, sequencing and output history.

use crate::{Arity, Attributes as A, BuiltinFn, BuiltinSpec, DocEntry, EvalError, Evaluator, Rule};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel, Symbol};
use om_num::Number;
use std::{collections::BTreeMap, sync::OnceLock};

/// Process-wide immutable registry of implemented callbacks and help.
pub struct BuiltinTable {
    specs: BTreeMap<&'static str, BuiltinSpec>,
}
impl BuiltinTable {
    pub(crate) fn get(&self, s: Symbol) -> Option<&BuiltinSpec> {
        self.specs.get(s.name())
    }
    pub(crate) fn docs(&self) -> impl Iterator<Item = &DocEntry> {
        self.specs.values().map(|s| &s.doc)
    }
}
type Entry = (
    Symbol,
    BuiltinFn,
    A,
    Arity,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

pub(crate) fn table() -> &'static BuiltinTable {
    static TABLE: OnceLock<BuiltinTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let entries: [Entry; 6] = [
            (
                B::SET,
                set,
                A::HOLD_FIRST,
                Arity::Exactly(2),
                "let x = value",
                "x = value",
                "立即计算右侧并赋值。",
                "Evaluate the right-hand side and assign it.",
            ),
            (
                B::SET_DELAYED,
                set_delayed,
                A::HOLD_ALL,
                Arity::Exactly(2),
                "SetDelayed(x, value)",
                "x := value",
                "保存右侧，在使用定义时求值。",
                "Store the right-hand side for evaluation when used.",
            ),
            (
                Symbol::intern("Unset"),
                unset,
                A::HOLD_ALL,
                Arity::Exactly(1),
                "Unset(x)",
                "x =.",
                "移除一个定义。",
                "Remove one definition.",
            ),
            (
                B::CLEAR,
                clear,
                A::HOLD_ALL,
                Arity::AtLeast(1),
                "Clear(x, …)",
                "Clear[x, …]",
                "清除符号的值与函数定义。",
                "Clear symbol values and function definitions.",
            ),
            (
                B::COMPOUND_EXPRESSION,
                compound,
                A::HOLD_ALL,
                Arity::AtLeast(1),
                "CompoundExpression(expr, …)",
                "expr; …",
                "顺序求值并返回最后的结果。",
                "Evaluate in order and return the last result.",
            ),
            (
                B::OUT,
                out,
                A::default(),
                Arity::Range(0, 1),
                "out(n)",
                "Out[n]",
                "读取已记录的输出；省略序号则取最近一次。",
                "Read recorded output; omit the index for the most recent result.",
            ),
        ];
        BuiltinTable {
            specs: entries
                .into_iter()
                .map(
                    |(symbol, f, attrs, arity, modern, wolfram, summary_zh, summary_en)| {
                        let name = symbol.name();
                        (
                            name,
                            BuiltinSpec {
                                symbol,
                                f,
                                attrs: attrs | A::PROTECTED,
                                arity,
                                doc: DocEntry {
                                    name,
                                    modern,
                                    wolfram,
                                    summary_zh,
                                    summary_en,
                                    examples: match name {
                                        "Set" => &["x = 2"],
                                        "SetDelayed" => &["x := 2 + 2"],
                                        "Unset" => &["x =."],
                                        "Clear" => &["Clear[x]"],
                                        "Out" => &["Out[]"],
                                        _ => &["1; 2"],
                                    },
                                    category: "Evaluation",
                                },
                            },
                        )
                    },
                )
                .collect(),
        }
    })
}
fn writable(ev: &Evaluator) -> Result<(), EvalError> {
    if ev.readonly {
        Err(EvalError::Other(
            "read-only evaluator cannot change definitions".into(),
        ))
    } else {
        Ok(())
    }
}
fn target(ev: &mut Evaluator, e: &Expr, operation: &str) -> Option<Symbol> {
    let symbol = e.as_symbol().or_else(|| e.head_symbol());
    if let Some(symbol) = symbol {
        if !ev.attributes(symbol).contains(A::PROTECTED) {
            return Some(symbol);
        }
        ev.message(
            operation,
            if e.as_symbol().is_some() {
                "wrsym"
            } else {
                "write"
            },
            format!("Symbol {} is Protected.", symbol.name()),
            MsgLevel::Warning,
        );
    } else {
        ev.message(
            operation,
            "setraw",
            "Cannot assign to this expression.".into(),
            MsgLevel::Warning,
        );
    }
    None
}
fn assign(ev: &mut Evaluator, args: &[Expr], delayed: bool) -> Result<Option<Expr>, EvalError> {
    writable(ev)?;
    let operation = if delayed { "SetDelayed" } else { "Set" };
    let Some(symbol) = target(ev, &args[0], operation) else {
        return Ok(None);
    };
    if args[0].as_symbol().is_some() {
        ev.defs.own.insert(symbol, args[1].clone());
    } else {
        ev.defs.set_down(
            symbol,
            Rule {
                lhs: args[0].clone(),
                rhs: args[1].clone(),
                delayed,
            },
        );
    }
    Ok(Some(if delayed {
        Expr::sym(B::NULL)
    } else {
        args[1].clone()
    }))
}
fn set(ev: &mut Evaluator, args: &[Expr], _: &Interrupt) -> Result<Option<Expr>, EvalError> {
    assign(ev, args, false)
}
fn set_delayed(
    ev: &mut Evaluator,
    args: &[Expr],
    _: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    assign(ev, args, true)
}
fn unset(ev: &mut Evaluator, args: &[Expr], _: &Interrupt) -> Result<Option<Expr>, EvalError> {
    writable(ev)?;
    let Some(symbol) = target(ev, &args[0], "Unset") else {
        return Ok(None);
    };
    if args[0].as_symbol().is_some() {
        ev.defs.own.remove(&symbol);
    } else if let Some(rules) = ev.defs.down.get_mut(&symbol) {
        rules.retain(|r| r.lhs != args[0]);
    }
    Ok(Some(Expr::sym(B::NULL)))
}
fn clear(ev: &mut Evaluator, args: &[Expr], _: &Interrupt) -> Result<Option<Expr>, EvalError> {
    writable(ev)?;
    for e in args {
        if !matches!(e.kind(), ExprKind::Symbol(_)) {
            ev.message(
                "Clear",
                "ssym",
                "Clear expects symbol arguments.".into(),
                MsgLevel::Warning,
            );
            continue;
        }
        if let Some(symbol) = target(ev, e, "Clear") {
            ev.defs.clear(symbol);
        }
    }
    Ok(Some(Expr::sym(B::NULL)))
}
fn compound(ev: &mut Evaluator, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let mut result = Expr::sym(B::NULL);
    for e in args {
        result = ev.evaluate(e, ctx)?;
    }
    Ok(Some(result))
}
fn out(ev: &mut Evaluator, args: &[Expr], _: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let index = if args.is_empty() {
        ev.history.len().checked_sub(1)
    } else if let Some(Number::Integer(n)) = args[0].as_number() {
        if n > &om_num::Integer::ZERO {
            usize::try_from(n).ok().and_then(|i| i.checked_sub(1))
        } else if n < &om_num::Integer::ZERO {
            usize::try_from(n.clone().into_parts().1)
                .ok()
                .and_then(|i| ev.history.len().checked_sub(i))
        } else {
            None
        }
    } else {
        None
    };
    Ok(index
        .and_then(|i| ev.history.get(i))
        .map(|(_, out)| out.clone()))
}
