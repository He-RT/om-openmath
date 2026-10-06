//! Only genuinely implemented ODE/interpolation/sampling callbacks enter executable metadata.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$arity:expr,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:A::PROTECTED|A::HOLD_ALL,arity:$arity,doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Actual machine nonstiff ODE, finite-domain interpolation and readonly sampling with honest diagnostics.",examples:&[$example],category:"Analysis"}});
    };}
    entry!(
        ode,
        "Ode",
        Arity::Range(3, 11),
        "ode(rhs,initial:values,t:start..end)",
        "Ode[rhs,{t,start,end},Initial->values]",
        "非刚性机器DP5(4)，真实连续输出与简单终止事件。",
        "Ode[Function[{t,y},y],{t,0,1},Initial->1]"
    );
    entry!(
        interpolate,
        "Interpolate",
        Arity::Range(1, 3),
        "interpolate(points,method:\"linear\")",
        "Interpolate[points,Method->\"linear\"]",
        "有限域线性/Hermite插值，默认拒绝外推。",
        "Interpolate[{{0,0},{1,2},{2,4}}]"
    );
    entry!(
        sample,
        "Sample",
        Arity::Range(2, 4),
        "sample(fn,x:a..b,count:100)",
        "Sample[fn,{x,a,b},Count->100]",
        "真实只读标量/向量采样，返回x/value表格。",
        "Sample[Function[x,x^2],{x,0,1},Count->5]"
    );
}
