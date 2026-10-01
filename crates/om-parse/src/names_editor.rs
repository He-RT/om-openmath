//! One executable identifier mapping for editor insertion and ordinary parsing.
use crate::{ConstantMode, Dialect};
use om_core::Symbol;
/// Resolve an actual identifier according to source dialect, constants and call position.
pub fn identifier_symbol(
    name: &str,
    dialect: Dialect,
    constants: ConstantMode,
    call: bool,
) -> Option<Symbol> {
    if dialect == Dialect::Wolfram {
        super::names::wolfram(name).ok()
    } else if call && name != "Root" && name.eq_ignore_ascii_case("root") {
        Some(om_core::BUILTIN::POWER)
    } else if call {
        Some(super::names::modern(name, constants).unwrap_or_else(|| Symbol::intern(name)))
    } else {
        Some(super::names::atom(name, constants))
    }
}
/// Preferred executable Modern callable spelling; user symbols retain their original names.
pub fn modern_name(symbol: Symbol) -> String {
    let alias = match symbol.name() {
        "Root" => "Root",
        "And" => "And",
        "Or" => "Or",
        "Not" => "Not",
        "D" => "diff",
        "FindRoot" => "find_root",
        "ContourPlot" => "implicitplot",
        "ArcSin" => "asin",
        "ArcCos" => "acos",
        "ArcTan" => "atan",
        "ArcSinh" => "asinh",
        "ArcCosh" => "acosh",
        "ArcTanh" => "atanh",
        _ => "",
    };
    if !alias.is_empty() {
        alias.into()
    } else if super::names::is_function(symbol) {
        symbol.name().to_ascii_lowercase()
    } else {
        symbol.name().into()
    }
}
/// Already implemented callable names outside core's fixed stable symbol table.
pub(super) const EXTRA: &[&str] = &[
    "Subtract",
    "Divide",
    "Minus",
    "CubeRoot",
    "Quotient",
    "FactorInteger",
    "PrimeQ",
    "Numerator",
    "Denominator",
    "First",
    "Last",
    "Rest",
    "Append",
    "Sum",
    "Product",
    "ReplaceRepeated",
    "Unset",
    "SameQ",
    "Coefficient",
    "CoefficientList",
    "Exponent",
    "PolynomialQ",
    "PolynomialGCD",
    "PolynomialLCM",
    "PolynomialQuotient",
    "PolynomialRemainder",
    "Resultant",
    "Discriminant",
    "Variables",
    "RootReduce",
    "ToRadicals",
    "NSolveValues",
    "Roots",
];
