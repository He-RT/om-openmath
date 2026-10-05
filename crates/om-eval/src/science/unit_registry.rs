//! Actual quantity and unit conversion entry points.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(super) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {($f:ident,$name:literal,$arity:expr,$modern:literal,$wolfram:literal,$zh:literal,$example:literal)=>{
        fn $f(ev:&mut Evaluator,args:&[Expr],ctx:&Interrupt)->Result<Option<Expr>,EvalError>{super::dispatch(ev,$name,args,ctx)}
        specs.insert($name,BuiltinSpec{symbol:Symbol::intern($name),f:$f,attrs:A::PROTECTED,arity:$arity,doc:DocEntry{name:$name,modern:$modern,wolfram:$wolfram,summary_zh:$zh,summary_en:"Real scalar quantities, exact SI unit factors and dimension-checked conversion.",examples:&[$example],category:"Units"}});
    };}
    entry!(
        quantity,
        "Quantity",
        Arity::Range(2, 3),
        "quantity(value,unit:\"m\")",
        "Quantity[value,unit]",
        "标量实数单位量，必填单位。",
        "Quantity[1,\"km\"]"
    );
    entry!(
        convert,
        "UnitConvert",
        Arity::Exactly(2),
        "convert_units(quantity,target_unit)",
        "UnitConvert[quantity,target_unit]",
        "精确因子换算，量纲不一致拒绝。",
        "UnitConvert[Quantity[1,\"km\"],\"m\"]"
    );
    entry!(
        magnitude,
        "QuantityMagnitude",
        Arity::Exactly(1),
        "magnitude(quantity)",
        "QuantityMagnitude[quantity]",
        "取当前单位下的数值。",
        "QuantityMagnitude[Quantity[1,\"km\"]]"
    );
    entry!(
        unit,
        "QuantityUnit",
        Arity::Exactly(1),
        "unit(quantity)",
        "QuantityUnit[quantity]",
        "取当前单位文字。",
        "QuantityUnit[Quantity[1,\"km\"]]"
    );
}
