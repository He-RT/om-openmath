//! The named characters required by the Wolfram subset.

use super::TokenKind as K;

pub(super) fn kind(name: &str) -> Option<K> {
    match name {
        "Element" => Some(K::Element),
        "Equal" => Some(K::Equal),
        "LessEqual" => Some(K::LessEqual),
        "GreaterEqual" => Some(K::GreaterEqual),
        "NotEqual" => Some(K::Unequal),
        "Rule" => Some(K::Rule),
        "Pi" | "Infinity" => Some(K::Identifier),
        _ if GREEK.iter().any(|n| name == *n || name == n.to_lowercase()) => Some(K::Identifier),
        _ => None,
    }
}

const GREEK: &[&str] = &[
    "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota", "Kappa",
    "Lambda", "Mu", "Nu", "Xi", "Omicron", "Pi", "Rho", "Sigma", "Tau", "Upsilon", "Phi", "Chi",
    "Psi", "Omega",
];

pub(crate) fn value(name: &str) -> Option<char> {
    match name {
        "Infinity" => Some('∞'),
        "Element" => Some('∈'),
        "Equal" => Some('\u{f431}'),
        "LessEqual" => Some('≤'),
        "GreaterEqual" => Some('≥'),
        "NotEqual" => Some('≠'),
        "Rule" => Some('\u{f522}'),
        _ => GREEK
            .iter()
            .position(|n| name == *n || name == n.to_lowercase())
            .and_then(|index| "αβγδεζηθικλμνξοπρστυφχψω".chars().nth(index)),
    }
}

pub(crate) fn expand(src: &str) -> Result<String, &'static str> {
    let mut result = String::new();
    let mut chars = src.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        if chars.next() != Some('[') {
            return Err("非法具名字符");
        }
        let mut name = String::new();
        let mut closed = false;
        for ch in chars.by_ref() {
            if ch == ']' {
                closed = true;
                break;
            }
            name.push(ch);
        }
        if !closed {
            return Err("不完整的具名字符");
        }
        result.push(value(&name).ok_or("未知具名字符")?);
    }
    Ok(result)
}
