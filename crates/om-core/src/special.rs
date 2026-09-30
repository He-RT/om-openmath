//! Shared recognition of the symbolic infinity forms.

use crate::{BUILTIN as B, Expr};
use om_num::{Number, Rational};

pub(super) fn infinity_direction(e: &Expr) -> Option<Option<Expr>> {
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

pub(super) fn rational(n: &Number) -> Option<Rational> {
    match n {
        Number::Integer(i) => Some(Rational::from(i.clone())),
        Number::Rational(q) => Some(q.clone()),
        _ => None,
    }
}
