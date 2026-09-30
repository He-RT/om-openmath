//! Additional elementary functions; each entry has a real callback and examples.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;

pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {
        ($f:ident,$name:literal,$zh:literal,$en:literal,$examples:expr) => {{
            fn $f(
                ev: &mut Evaluator,
                args: &[Expr],
                ctx: &Interrupt,
            ) -> Result<Option<Expr>, EvalError> {
                crate::scalar::dispatch(ev, Symbol::intern($name), args, ctx)
            }
            specs.insert(
                $name,
                BuiltinSpec {
                    symbol: Symbol::intern($name),
                    f: $f,
                    attrs: A::LISTABLE | A::NUMERIC_FUNCTION | A::PROTECTED,
                    arity: Arity::Exactly(1),
                    doc: DocEntry {
                        name: $name,
                        modern: concat!($name, "(x)"),
                        wolfram: concat!($name, "[x]"),
                        summary_zh: $zh,
                        summary_en: $en,
                        examples: $examples,
                        category: "Mathematics",
                    },
                },
            );
        }};
    }
    entry!(
        cot,
        "Cot",
        "计算余切。",
        "Compute the cotangent.",
        &["Cot[Pi/4]", "Cot[0.5]"]
    );
    entry!(
        sec,
        "Sec",
        "计算正割。",
        "Compute the secant.",
        &["Sec[Pi/3]", "Sec[0.5]"]
    );
    entry!(
        csc,
        "Csc",
        "计算余割。",
        "Compute the cosecant.",
        &["Csc[Pi/6]", "Csc[0.5]"]
    );
    entry!(
        acot,
        "ArcCot",
        "计算主值反余切。",
        "Compute the principal inverse cotangent.",
        &["ArcCot[1]", "ArcCot[0.5]"]
    );
    entry!(
        asec,
        "ArcSec",
        "计算主值反正割。",
        "Compute the principal inverse secant.",
        &["ArcSec[2]", "ArcSec[2.0]"]
    );
    entry!(
        acsc,
        "ArcCsc",
        "计算主值反余割。",
        "Compute the principal inverse cosecant.",
        &["ArcCsc[2]", "ArcCsc[2.0]"]
    );
    entry!(
        sinh,
        "Sinh",
        "计算双曲正弦。",
        "Compute the hyperbolic sine.",
        &["Sinh[0]", "Sinh[0.5]"]
    );
    entry!(
        cosh,
        "Cosh",
        "计算双曲余弦。",
        "Compute the hyperbolic cosine.",
        &["Cosh[0]", "Cosh[0.5]"]
    );
    entry!(
        tanh,
        "Tanh",
        "计算双曲正切。",
        "Compute the hyperbolic tangent.",
        &["Tanh[0]", "Tanh[0.5]"]
    );
    entry!(
        coth,
        "Coth",
        "计算双曲余切。",
        "Compute the hyperbolic cotangent.",
        &["Coth[0]", "Coth[0.5]"]
    );
    entry!(
        sech,
        "Sech",
        "计算双曲正割。",
        "Compute the hyperbolic secant.",
        &["Sech[0]", "Sech[0.5]"]
    );
    entry!(
        csch,
        "Csch",
        "计算双曲余割。",
        "Compute the hyperbolic cosecant.",
        &["Csch[0]", "Csch[0.5]"]
    );
    entry!(
        asinh,
        "ArcSinh",
        "计算主值反双曲正弦。",
        "Compute the principal inverse hyperbolic sine.",
        &["ArcSinh[0]", "ArcSinh[0.5]"]
    );
    entry!(
        acosh,
        "ArcCosh",
        "计算主值反双曲余弦。",
        "Compute the principal inverse hyperbolic cosine.",
        &["ArcCosh[1]", "ArcCosh[2.0]"]
    );
    entry!(
        atanh,
        "ArcTanh",
        "计算主值反双曲正切。",
        "Compute the principal inverse hyperbolic tangent.",
        &["ArcTanh[0]", "ArcTanh[0.5]"]
    );
}
