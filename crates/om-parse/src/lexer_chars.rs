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
