//! Actual project Fit signature, without pretending Wolfram's basis-list syntax is already adapted.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    fn fit(ev: &mut Evaluator, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
        super::dispatch(ev, "Fit", args, ctx)
    }
    specs.insert("Fit",BuiltinSpec{symbol:Symbol::intern("Fit"),f:fit,attrs:A::PROTECTED|A::HOLD_ALL,arity:Arity::Range(3,12),doc:DocEntry{name:"Fit",modern:"fit(data,model:expr,parameters:starts,method:\"linear\")",wolfram:"Fit[data,Model->expr,Parameters->starts]",summary_zh:"真实满列秩QR/LM拟合、可调用模型、残差与工作，不虚构置信区间。",summary_en:"Actual full-rank QR/LM fit with a callable model, residuals and numerical work; no invented confidence statistics.",examples:&["Fit[{{0,1},{1,3},{2,5}},Model->a*x+b,Parameters->{a,b}]"],category:"Analysis"}});
}
