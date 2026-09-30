//! Sorting then merging keeps Plus construction O(n log n).

use super::order::canonical_cmp;
use crate::{BUILTIN as B, Expr};
use om_num::Number;

/// Construct a canonical sum, flattening Plus and collecting numeric coefficients.
pub fn add(terms: impl IntoIterator<Item = Expr>) -> Expr {
    let mut stack: Vec<_> = terms.into_iter().collect();
    stack.reverse();
    let mut sum = Number::Integer(0.into());
    let mut symbolic = Vec::new();
    let mut infinity: Option<Option<Expr>> = None;
    while let Some(term) = stack.pop() {
        if term.is_head(B::PLUS) {
            stack.extend(term.args().iter().rev().cloned());
        } else if term.as_symbol() == Some(B::INDETERMINATE) {
            return Expr::sym(B::INDETERMINATE);
        } else if let Some(direction) = infinity_direction(&term) {
            if let Some(previous) = &infinity {
                if direction.is_none() || previous.is_none() || *previous != direction {
                    return Expr::sym(B::INDETERMINATE);
                }
            } else {
                infinity = Some(direction);
            }
        } else if let Some(n) = term.as_number() {
            sum = sum.add(n);
        } else {
            let (coefficient, rest) = split_term(term);
            symbolic.push((rest, coefficient));
        }
    }
    if let Some(direction) = infinity {
        return Expr::call(B::DIRECTED_INFINITY, direction);
    }
    symbolic.sort_by(|(a, _), (b, _)| canonical_cmp(a, b));
    let mut merged: Vec<(Expr, Number)> = Vec::with_capacity(symbolic.len());
    for (rest, coefficient) in symbolic {
        if let Some((last, c)) = merged.last_mut()
            && *last == rest
        {
            *c = c.add(&coefficient);
        } else {
            merged.push((rest, coefficient));
        }
    }
    let mut result = Vec::with_capacity(merged.len() + 1);
    for (rest, coefficient) in merged {
        if coefficient.is_exact() && coefficient.is_zero() {
            continue;
        }
        let term = scaled_term(coefficient, rest);
        // An approximate cancelled coefficient becomes literal 0., not a dropped term.
        if let Some(n) = term.as_number() {
            sum = sum.add(n);
        } else {
            result.push(term);
        }
    }
    if !sum.is_exact() || !sum.is_zero() {
        result.push(Expr::number(sum));
    }
    result.sort_by(canonical_cmp);
    match result.len() {
        0 => Expr::int(0),
        1 => result.remove(0),
        _ => Expr::call(B::PLUS, result),
    }
}

fn infinity_direction(e: &Expr) -> Option<Option<Expr>> {
    match e.as_symbol() {
        Some(B::INFINITY) => return Some(Some(Expr::int(1))),
        Some(B::COMPLEX_INFINITY) => return Some(None),
        _ => {}
    }
    if e.is_head(B::DIRECTED_INFINITY) {
        match e.args() {
            [] => Some(None),
            [direction] => Some(Some(direction.clone())),
            _ => None,
        }
    } else {
        None
    }
}

fn split_term(term: Expr) -> (Number, Expr) {
    if term.is_head(B::TIMES) {
        let args = term.args();
        if let Some(coefficient) = args.first().and_then(Expr::as_number) {
            let rest = match &args[1..] {
                [] => Expr::int(1),
                [one] => one.clone(),
                args => Expr::call(B::TIMES, args.iter().cloned()),
            };
            return (coefficient.clone(), rest);
        }
    }
    (Number::Integer(1.into()), term)
}

// M2.5 replaces this narrow coefficient assembly with the complete mul constructor.
fn scaled_term(coefficient: Number, rest: Expr) -> Expr {
    if coefficient.is_zero() {
        return Expr::number(coefficient);
    }
    if let Some(n) = rest.as_number() {
        return Expr::number(coefficient.mul(n));
    }
    if coefficient.is_exact() && coefficient.is_one() {
        return rest;
    }
    let mut factors = vec![Expr::number(coefficient)];
    if rest.is_head(B::TIMES) {
        factors.extend(rest.args().iter().cloned());
    } else {
        factors.push(rest);
    }
    Expr::call(B::TIMES, factors)
}
