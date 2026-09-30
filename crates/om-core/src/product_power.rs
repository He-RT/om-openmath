//! Private first-consumer rebuild; full Power semantics arrive in M2.6.

use super::special::rational;
use crate::{BUILTIN as B, Expr};
use om_num::{NumError, Number, Rational, exact_root};

pub(super) fn power_for_product(base: Expr, exp: Expr) -> Expr {
    if exp.as_number().is_some_and(|n| n.is_exact() && n.is_zero()) {
        return if base.is_zero() {
            Expr::sym(B::INDETERMINATE)
        } else {
            Expr::int(1)
        };
    }
    if exp == Expr::int(1) {
        return base;
    }
    if base == Expr::int(1) {
        return base;
    }
    if let (Some(n), Some(Number::Integer(e))) = (base.as_number(), exp.as_number()) {
        match n.pow_int(e) {
            Ok(value) => return Expr::number(value),
            Err(NumError::DivByZero) => return Expr::call(B::DIRECTED_INFINITY, []),
            Err(NumError::Indeterminate) => return Expr::sym(B::INDETERMINATE),
            Err(NumError::ExactOverflow) => {}
        }
    }
    if let (Some(r), Some(Number::Rational(e))) =
        (base.as_number().and_then(rational), exp.as_number())
        && r > Rational::ZERO
        && let Ok(q) = u32::try_from(e.denominator().clone())
        && let Some(a) = exact_root(r.numerator(), q)
        && let Some(b) = exact_root(&r.denominator().clone().into(), q)
        && let Ok(value) =
            Number::Rational(Rational::from(a) / Rational::from(b)).pow_int(e.numerator())
    {
        return Expr::number(value);
    }
    Expr::call(B::POWER, [base, exp])
}
