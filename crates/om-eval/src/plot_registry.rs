//! Plot syntax stays held until the shared kernel performs actual sampling.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, Interrupt};
use std::collections::BTreeMap;
fn held(_: &mut Evaluator, _: &[Expr], _: &Interrupt) -> Result<Option<Expr>, EvalError> {
    Ok(None)
}
pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    for (name, symbol, arity, modern, wolfram, example, zh, en) in [
        (
            "Plot",
            B::PLOT,
            2,
            "plot(expr, [x,min,max])",
            "Plot[expr,{x,min,max}]",
            &["Plot[Sin[x],{x,0,2*Pi}]"],
            "采样一元实函数图像。",
            "Sample a real one-variable function.",
        ),
        (
            "ContourPlot",
            B::CONTOUR_PLOT,
            3,
            "implicitplot(equation, [x,min,max], [y,min,max])",
            "ContourPlot[eq,{x,min,max},{y,min,max}]",
            &["ContourPlot[x^2+y^2==1,{x,-2,2},{y,-2,2}]"],
            "采样二元隐函数等值轮廓。",
            "Sample a two-variable zero contour.",
        ),
    ] {
        specs.insert(
            name,
            BuiltinSpec {
                symbol,
                f: held,
                attrs: A::HOLD_ALL | A::PROTECTED,
                arity: Arity::AtLeast(arity),
                doc: DocEntry {
                    name,
                    modern,
                    wolfram,
                    summary_zh: zh,
                    summary_en: en,
                    examples: example,
                    category: "Visualization",
                },
            },
        );
    }
}
