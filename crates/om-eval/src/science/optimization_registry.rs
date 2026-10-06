//! Actual Optimize callback; no unimplemented Wolfram return shape is claimed as compatible.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    fn optimize(
        ev: &mut Evaluator,
        args: &[Expr],
        ctx: &Interrupt,
    ) -> Result<Option<Expr>, EvalError> {
        super::dispatch(ev, "Optimize", args, ctx)
    }
    specs.insert("Optimize",BuiltinSpec{symbol:Symbol::intern("Optimize"),f:optimize,attrs:A::PROTECTED|A::HOLD_ALL,arity:Arity::Range(2,12),doc:DocEntry{name:"Optimize",modern:"optimize(expr,variables,initial:values,scope:\"local\")",wolfram:"Optimize[expr,variables,Initial->values]",summary_zh:"局部数值候选与精确有理凸二次global认证分开，返回真实工作与证书。",summary_en:"Real local candidates or exact rational convex quadratic global certificates, with explicit guarantees and work.",examples:&["Optimize[(x-2)^2,x,Initial->0]"],category:"Analysis"}});
}
