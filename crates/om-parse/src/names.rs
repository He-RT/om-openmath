//! Modern names share the core's authoritative built-in table.

use crate::ConstantMode;
use om_core::{Symbol, builtins};

pub(crate) fn modern(name: &str, mode: ConstantMode) -> Option<Symbol> {
    let lower = name.to_ascii_lowercase();
    if mode == ConstantMode::Strict && matches!(name, "e" | "i") {
        return None;
    }
    let target = match lower.as_str() {
        "e" => "E",
        "i" => "I",
        "π" => "Pi",
        "∞" | "inf" | "infinity" => "Infinity",
        "find_root" => "FindRoot",
        "asin" => "ArcSin",
        "acos" => "ArcCos",
        "atan" | "atan2" => "ArcTan",
        "asinh" => "ArcSinh",
        "acosh" => "ArcCosh",
        "atanh" => "ArcTanh",
        "ln" | "log10" | "log2" => "Log",
        "cbrt" | "cuberoot" => "CubeRoot",
        "sgn" => "Sign",
        "conj" => "Conjugate",
        "ceil" => "Ceiling",
        "binom" | "choose" => "Binomial",
        "diff" | "derivative" => "D",
        "numeric" => "N",
        "implicitplot" => "ContourPlot",
        "len" => "Length",
        "lambertw" => "ProductLog",
        _ => {
            return builtins::names()
                .iter()
                .find(|n| {
                    n.eq_ignore_ascii_case(name)
                        && (name.len() > 1 || **n == name || matches!(name, "n" | "d"))
                })
                .map(|n| Symbol::intern(n));
        }
    };
    Some(Symbol::intern(target))
}

pub(crate) fn is_function(symbol: Symbol) -> bool {
    let name = symbol.name();
    (builtins::names().contains(&name) || name == "CubeRoot")
        && !matches!(
            name,
            "True"
                | "False"
                | "Null"
                | "Pi"
                | "E"
                | "I"
                | "Infinity"
                | "ComplexInfinity"
                | "Indeterminate"
                | "Reals"
                | "Integers"
                | "Complexes"
                | "Rationals"
                | "Algebraics"
                | "Primes"
                | "Booleans"
                | "Automatic"
                | "All"
                | "None"
                | "Integer"
                | "Rational"
                | "Real"
                | "Complex"
                | "Symbol"
                | "String"
        )
}

pub(crate) fn known_lowercase(name: &str) -> bool {
    name.bytes().all(|c| !c.is_ascii_uppercase())
        && modern(name, ConstantMode::Math).is_some_and(is_function)
}

pub(crate) fn atom(name: &str, mode: ConstantMode) -> Symbol {
    if builtins::names().contains(&name) {
        return Symbol::intern(name);
    }
    let constant = matches!(
        name.to_ascii_lowercase().as_str(),
        "pi" | "π"
            | "e"
            | "i"
            | "inf"
            | "∞"
            | "infinity"
            | "true"
            | "false"
            | "null"
            | "reals"
            | "integers"
            | "complexes"
            | "rationals"
            | "automatic"
            | "all"
            | "none"
    );
    if constant && let Some(symbol) = modern(name, mode) {
        return symbol;
    }
    Symbol::intern(name)
}

pub(crate) fn option(name: &str) -> Symbol {
    let name = match name.to_ascii_lowercase().as_str() {
        "precision" => return Symbol::intern("WorkingPrecision"),
        "steps" => return Symbol::intern("RecordSteps"),
        "method" => return Symbol::intern("Method"),
        _ => name,
    };
    let mut camel = String::new();
    for word in name.split('_') {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            camel.extend(first.to_uppercase());
            camel.extend(chars);
        }
    }
    Symbol::intern(&camel)
}

pub(crate) fn wolfram(name: &str) -> Result<Symbol, &'static str> {
    let expanded = crate::lexer::named::expand(name)?;
    let name = match expanded.as_str() {
        "π" => "Pi",
        "∞" => "Infinity",
        other => other,
    };
    Ok(Symbol::intern(name))
}
