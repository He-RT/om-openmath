//! Principal powers and their exact structural rewrites.

use super::{
    mul,
    power_numeric::{numeric_exp, numeric_power},
    power_roots::{minus_one_power, rational_power},
    special::{infinity_direction, rational},
};
use crate::{BUILTIN as B, Expr, message::emit};
use om_num::{NumError, Number, Rational};

/// Construct a canonical principal power, retaining unsupported or oversized powers.
pub fn pow(base: Expr, exp: Expr) -> Expr {
    if base.as_symbol() == Some(B::INDETERMINATE) || exp.as_symbol() == Some(B::INDETERMINATE) {
        return Expr::sym(B::INDETERMINATE);
    }
    let infinity = infinity_direction(&base);
    if ((base.is_zero() || infinity.is_some()) && exp.is_zero())
        || (base.is_one() && infinity_direction(&exp).is_some())
    {
        return Expr::sym(B::INDETERMINATE);
    }
    if exp == Expr::int(0) {
        return Expr::int(1);
    }
    if exp == Expr::int(1) {
        return base;
    }
    if base == Expr::int(1) && exp.as_number().is_none_or(Number::is_exact) {
        return base;
    }
    if base.is_zero()
        && let Some(e) = exp.as_number()
    {
        let re = if let Number::Complex(c) = e { &c.re } else { e };
        if re.is_negative() {
            emit(
                "Power",
                "infy",
                "A negative power of zero has infinite magnitude.",
            );
            return Expr::call(B::DIRECTED_INFINITY, []);
        }
        if re.is_zero() {
            return Expr::sym(B::INDETERMINATE);
        }
        return Expr::number(
            base.as_number()
                .expect("invariant: literal zero is numeric")
                .mul(e),
        );
    }
    if let Some(direction) = infinity
        && let Some(e) = exp.as_number()
    {
        if e.is_negative() {
            return Expr::int(0);
        }
        if let Number::Integer(n) = e
            && n > &0.into()
        {
            return if let Some(direction) = direction {
                mul([Expr::call(B::DIRECTED_INFINITY, [pow(direction, exp)])])
            } else {
                Expr::call(B::DIRECTED_INFINITY, [])
            };
        }
    }
    if let (Some(b), Some(e)) = (base.as_number(), exp.as_number()) {
        if let Number::Integer(n) = e
            && b.is_exact()
        {
            match b.pow_int(n) {
                Ok(n) => return Expr::number(n),
                Err(NumError::DivByZero) => return Expr::call(B::DIRECTED_INFINITY, []),
                Err(NumError::Indeterminate) => return Expr::sym(B::INDETERMINATE),
                Err(NumError::ExactOverflow) => return overflow(base, exp),
            }
        }
        if let (Some(b), Number::Rational(e)) = (rational(b), e) {
            return rational_power(b, e.clone()).unwrap_or_else(|_| overflow(base, exp));
        }
        return numeric_power(b, e)
            .map(Expr::number)
            .unwrap_or_else(|| overflow(base, exp));
    }
    if base.as_symbol() == Some(B::E) {
        if exp.is_head(B::LOG)
            && let [z] = exp.args()
        {
            return z.clone();
        }
        if let Some(n) = exp.as_number()
            && !n.is_exact()
        {
            return numeric_exp(n)
                .map(Expr::number)
                .unwrap_or_else(|| overflow(base, exp));
        }
        if exp.is_head(B::TIMES)
            && let [coefficient, pi] = exp.args()
            && pi.as_symbol() == Some(B::PI)
            && let Some(Number::Complex(c)) = coefficient.as_number()
            && c.re.is_zero()
            && let Some(r) = rational(&c.im)
            && (r.denominator().is_one() || r.denominator() == &2u32.into())
        {
            return minus_one_power(r);
        }
    }
    if base.is_head(B::POWER)
        && let [x, a] = base.args()
    {
        let safe = a
            .as_number()
            .and_then(rational)
            .is_some_and(|q| q > Rational::from(-1) && q < Rational::ONE);
        if matches!(exp.as_number(), Some(Number::Integer(_))) || safe {
            return pow(x.clone(), mul([a.clone(), exp]));
        }
    }
    if base.is_head(B::TIMES) {
        if matches!(exp.as_number(), Some(Number::Integer(_))) {
            return mul(base
                .args()
                .iter()
                .map(|factor| pow(factor.clone(), exp.clone())));
        }
        if let Some(first) = base.args().first()
            && first
                .as_number()
                .and_then(rational)
                .is_some_and(|q| q > Rational::ZERO)
        {
            let rest = mul(base.args()[1..].iter().cloned());
            return mul([pow(first.clone(), exp.clone()), pow(rest, exp)]);
        }
    }
    Expr::call(B::POWER, [base, exp])
}
fn overflow(base: Expr, exp: Expr) -> Expr {
    emit(
        "General",
        "ovfl",
        "The power exceeds the numeric resource or exponent limit.",
    );
    Expr::call(B::POWER, [base, exp])
}
