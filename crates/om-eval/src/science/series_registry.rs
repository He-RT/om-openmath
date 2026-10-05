//! Genuine finite Taylor and exact-limit callbacks; planning cannot enter the registry.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$arity:expr,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:A::PROTECTED|A::HOLD_ALL,arity:$arity,doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Actual exact limits or derivative-based finite Taylor coefficients within stated analytic and resource bounds.",examples:&[$example],category:"Calculus"}});
    };}
    entry!(
        limit,
        "Limit",
        Arity::Range(2, 4),
        "limit(expr,x,at:point,direction:\"both\")",
        "Limit[expr,x->point]",
        "精确实单侧/双侧/无限极限；振荡或不可证不猜测。",
        "Limit[Sin[x]/x,x->0]"
    );
    entry!(
        series,
        "Series",
        Arity::Range(2, 4),
        "series(expr,x,at:0,order:6)",
        "Series[expr,{x,point,order}]",
        "真实导数普通Taylor，0..64阶与明确截断。",
        "Series[Sin[x],{x,0,7}]"
    );
    entry!(
        coefficient,
        "SeriesCoefficient",
        Arity::Exactly(2),
        "series_coefficient(series,order)",
        "SeriesCoefficient[series,order]",
        "已知截断内系数；外部未知项不返回零。",
        "SeriesCoefficient[Series[Exp[x],{x,0,4}],3]"
    );
    entry!(
        normal,
        "Normal",
        Arity::Exactly(1),
        "normal(value)",
        "Normal[value]",
        "显式去Taylor截断，保留已知多项式。",
        "Normal[Series[Exp[x],{x,0,4}]]"
    );
}
