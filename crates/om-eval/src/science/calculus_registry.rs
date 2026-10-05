//! Executable Cartesian calculus names share the actual derivative engine.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:A::PROTECTED|A::HOLD_ALL,arity:Arity::Exactly(2),doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Actual Cartesian symbolic derivatives with localized coordinates and checked dimensions.",examples:&[$example],category:"Calculus"}});
    };}
    entry!(
        grad,
        "Grad",
        "grad(expr,variables)",
        "Grad[expr,variables]",
        "笛卡尔标量梯度。",
        "Grad[x^2+y^2,{x,y}]"
    );
    entry!(
        jacobian,
        "Jacobian",
        "jacobian(expressions,variables)",
        "Jacobian[expressions,variables]",
        "向量Jacobian，行对应分量、列对应坐标。",
        "Jacobian[{x*y,Sin[x]},{x,y}]"
    );
    entry!(
        hessian,
        "Hessian",
        "hessian(expr,variables)",
        "Hessian[expr,variables]",
        "标量Hessian，按坐标有序返回。",
        "Hessian[x^2*y,{x,y}]"
    );
    entry!(
        divergence,
        "Divergence",
        "div(vector,variables)",
        "Divergence[vector,variables]",
        "笛卡尔向量散度，维度必须相同。",
        "Divergence[{x,y,z},{x,y,z}]"
    );
    entry!(
        curl,
        "Curl",
        "curl(vector,variables)",
        "Curl[vector,variables]",
        "三维笛卡尔向量旋度。",
        "Curl[{-y,x,0},{x,y,z}]"
    );
    entry!(
        laplacian,
        "Laplacian",
        "laplacian(expr,variables)",
        "Laplacian[expr,variables]",
        "笛卡尔标量拉普拉斯算子。",
        "Laplacian[x^2+y^2+z^2,{x,y,z}]"
    );
}
