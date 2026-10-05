//! Executable root names have distinct real/principal mathematical meanings.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$arity:expr,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:A::PROTECTED|A::LISTABLE|A::NUMERIC_FUNCTION,arity:$arity,doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Exact/guarded root evaluation with an explicit real or principal branch contract.",examples:&[$example],category:"Mathematics"}});
    };}
    entry!(
        cube,
        "CubeRoot",
        Arity::Exactly(1),
        "cbrt(value)",
        "CubeRoot[value]",
        "实数立方根；负实数结果为负，不是复主值。",
        "CubeRoot[-8]"
    );
    entry!(
        nth,
        "NthRoot",
        Arity::Range(2, 3),
        "nth_root(value,degree,branch:\"principal\")",
        "NthRoot[value,degree]",
        "正整数阶主值根，与认证代数Root独立。",
        "NthRoot[81,4]"
    );
}
