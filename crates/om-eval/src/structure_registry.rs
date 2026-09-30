//! Registry for implemented structural and finite-iteration functions.

use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {
        ($f:ident,$name:literal,$arity:expr,$attrs:expr,$zh:literal,$en:literal,$ex:expr) => {{
            fn $f(
                ev: &mut Evaluator,
                args: &[Expr],
                ctx: &Interrupt,
            ) -> Result<Option<Expr>, EvalError> {
                crate::structure::dispatch(ev, Symbol::intern($name), args, ctx)
            }
            specs.insert(
                $name,
                BuiltinSpec {
                    symbol: Symbol::intern($name),
                    f: $f,
                    attrs: $attrs | A::PROTECTED,
                    arity: $arity,
                    doc: DocEntry {
                        name: $name,
                        modern: concat!($name, "(…)"),
                        wolfram: concat!($name, "[…]"),
                        summary_zh: $zh,
                        summary_en: $en,
                        examples: $ex,
                        category: "Structure",
                    },
                },
            );
        }};
    }
    entry!(
        list,
        "List",
        Arity::Any,
        A::default(),
        "构造有序列表。",
        "Construct an ordered list.",
        &["{1, 2}", "{}"]
    );
    entry!(
        part,
        "Part",
        Arity::AtLeast(2),
        A::default(),
        "取第 1 起始的参数，负数从末尾取。",
        "Extract a one-based argument; negative indices count from the end.",
        &["Part[{1, 2}, 1]", "Part[{1, 2}, -1]"]
    );
    entry!(
        length,
        "Length",
        Arity::Exactly(1),
        A::default(),
        "返回直接参数数量。",
        "Return the number of immediate arguments.",
        &["Length[{1, 2}]", "Length[x]"]
    );
    entry!(
        first,
        "First",
        Arity::Exactly(1),
        A::default(),
        "返回首个参数。",
        "Return the first argument.",
        &["First[{1, 2}]", "First[f[a, b]]"]
    );
    entry!(
        last,
        "Last",
        Arity::Exactly(1),
        A::default(),
        "返回末个参数。",
        "Return the last argument.",
        &["Last[{1, 2}]", "Last[f[a, b]]"]
    );
    entry!(
        rest,
        "Rest",
        Arity::Exactly(1),
        A::default(),
        "移除首个参数并保留头。",
        "Remove the first argument and retain the head.",
        &["Rest[{1, 2}]", "Rest[f[a, b]]"]
    );
    entry!(
        append,
        "Append",
        Arity::Exactly(2),
        A::default(),
        "在表达式末尾添加参数。",
        "Append an argument to an expression.",
        &["Append[{1}, 2]", "Append[f[a], b]"]
    );
    entry!(
        range,
        "Range",
        Arity::Range(1, 3),
        A::default(),
        "构造有限数值等差序列。",
        "Construct a finite numeric arithmetic sequence.",
        &["Range[3]", "Range[3, 1, -1]"]
    );
    entry!(
        map,
        "Map",
        Arity::Exactly(2),
        A::default(),
        "对直接参数逐个应用函数。",
        "Apply a function to each immediate argument.",
        &["Map[f, {1, 2}]", "Map[f, g[a, b]]"]
    );
    entry!(
        apply,
        "Apply",
        Arity::Exactly(2),
        A::default(),
        "用指定函数替换表达式头。",
        "Replace an expression head with a function.",
        &["Apply[Plus, {1, 2}]", "Apply[f, g[a, b]]"]
    );
    entry!(
        table,
        "Table",
        Arity::AtLeast(2),
        A::HOLD_ALL,
        "在有限迭代区间构造结果列表。",
        "Build results over finite iteration ranges.",
        &["Table[i^2, {i, 3}]", "Table[a, {3}]"]
    );
    entry!(
        sum,
        "Sum",
        Arity::AtLeast(2),
        A::HOLD_ALL,
        "在有限迭代区间累加。",
        "Add terms over finite iteration ranges.",
        &["Sum[i, {i, 3}]", "Sum[i^2, {i, 3}]"]
    );
    entry!(
        product,
        "Product",
        Arity::AtLeast(2),
        A::HOLD_ALL,
        "在有限迭代区间累乘。",
        "Multiply terms over finite iteration ranges.",
        &["Product[i, {i, 4}]", "Product[i+1, {i, 3}]"]
    );
    entry!(
        function,
        "Function",
        Arity::Range(1, 2),
        A::HOLD_ALL,
        "构造 Slot 或命名参数的纯函数。",
        "Construct a pure function with slots or named parameters.",
        &["(#^2&)[3]", "Function[x, x^2][3]"]
    );
    entry!(
        slot,
        "Slot",
        Arity::Exactly(1),
        A::HOLD_ALL,
        "纯函数参数占位符。",
        "Represent an argument placeholder in a pure function.",
        &["Slot[1]", "Slot[2]"]
    );
    entry!(
        rule,
        "Rule",
        Arity::Exactly(2),
        A::default(),
        "构造立即替换规则。",
        "Construct an immediate replacement rule.",
        &["x -> 1", "y -> 2"]
    );
    entry!(
        replace,
        "ReplaceAll",
        Arity::Exactly(2),
        A::default(),
        "同时替换所有原始子树。",
        "Replace original subtrees simultaneously.",
        &["x /. x -> 2", "{x,y} /. {x->y,y->1}"]
    );
    entry!(
        repeat,
        "ReplaceRepeated",
        Arity::Exactly(2),
        A::default(),
        "有界重复应用替换直到稳定。",
        "Repeat replacement to a fixed point within the iteration limit.",
        &["x //. {x->y,y->2}", "f[f[1]] //. f[x_]:>x"]
    );
    entry!(
        element,
        "Element",
        Arity::Exactly(2),
        A::default(),
        "保持集合成员关系供后续求解。",
        "Retain a set-membership relation for solving.",
        &["Element[x, Reals]", "Element[x, Integers]"]
    );
}
