//! Actual exact rule and numeric algorithms; unsupported requests retain source and diagnose their boundary.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:A::PROTECTED|A::HOLD_ALL,arity:Arity::Range(2,9),doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Exact supported rules and adaptive numeric GK15/7; conditions, estimates and failures retain their true scope.",examples:&[$example],category:"Calculus"}});
    };}
    entry!(
        unified,
        "Integrate",
        "integrate(expr,variable_or_range,mode:\"exact\")",
        "Integrate[expr,variable_or_range]",
        "精确有界规则原函数与区间验证；numeric显式使用GK15/7。",
        "Integrate[x^2,x]"
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
