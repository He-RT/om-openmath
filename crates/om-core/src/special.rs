//! Shared recognition of the symbolic infinity forms.

use crate::{BUILTIN as B, Expr};
use om_num::{Complex, Number, Rational};

pub(super) fn literal_alias(e: Expr) -> Expr {
    match e.as_symbol() {
        Some(B::I) => Expr::number(Number::Complex(Box::new(Complex {
            re: Number::Integer(0.into()),
            im: Number::Integer(1.into()),
        }))),
        Some(B::INFINITY) => Expr::call(B::DIRECTED_INFINITY, [Expr::int(1)]),
        Some(B::COMPLEX_INFINITY) => Expr::call(B::DIRECTED_INFINITY, []),
        _ => e,
    }
}

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
