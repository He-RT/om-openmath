//! Actual help and capability entry points, independent of agent frameworks and host IO.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$arity:expr,$attrs:expr,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:$attrs|A::PROTECTED,arity:$arity,doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Audited function documentation and actual kernel capabilities; plans never grant execution.",examples:&[$example],category:"Runtime"}});
    };}
    entry!(
        help,
        "Help",
        Arity::Exactly(1),
        A::HOLD_FIRST,
        "help(name)",
        "Help[name]",
        "查询稳定身份、实际接口与规划边界。",
        "Help[\"map\"]"
    );
    entry!(
        options,
        "Options",
        Arity::Exactly(1),
        A::HOLD_FIRST,
        "options(name)",
        "Options[name]",
        "当前可执行选项与仅文档参数分开。",
        "Options[\"solve\"]"
    );
    entry!(
        functions,
        "Functions",
        Arity::Range(0, 2),
        A::default(),
        "functions(category:...,stage:\"current\")",
        "Functions[Stage->\"current\"]",
        "默认仅列实际回调身份，可浏览规划/后续目录。",
        "Functions[Stage->\"current\"]"
    );
    entry!(
        capabilities,
        "Capabilities",
        Arity::Exactly(0),
        A::default(),
        "capabilities()",
        "Capabilities[]",
        "报告实际内核接口版本、身份与计算平台；宿主权限保持未知。",
        "Capabilities[]"
    );
}
