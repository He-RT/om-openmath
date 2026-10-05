//! Explicit numeric integration and a partial unified interface; unsupported exact requests retain source.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:A::PROTECTED|A::HOLD_ALL,arity:Arity::Range(2,9),doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Actual adaptive GK15/7 machine integration, error estimates and real failures; no symbolic fallback.",examples:&[$example],category:"Calculus"}});
    };}
    entry!(
        unified,
        "Integrate",
        "integrate(expr,axis,mode:\"numeric\")",
        "Integrate[expr,axis,Mode->\"numeric\"]",
        "统一积分当前仅显式numeric；精确默认保留并诊断。",
        "Integrate[x^2,{x,0,1},Mode->\"numeric\"]"
    );
    entry!(
        numeric,
        "NIntegrate",
        "n_integrate(expr,axis)",
        "NIntegrate[expr,axis]",
        "一维有限/无限积分，真实误差估计与中断。",
        "NIntegrate[x^2,{x,0,1}]"
    );
}
