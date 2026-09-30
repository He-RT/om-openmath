//! Exact elementary values; numerical evaluation has a separate precision policy.

use om_core::{BUILTIN as B, Expr, Symbol, add, div, mul, neg, pow, sqrt};
use om_num::{Complex, Integer, Number, Rational};

/// Return an exact special value or symbolic identity for an elementary call.
/// Unknown inputs remain unevaluated. Approximate transcendental inputs belong
/// to the numerical evaluator rather than these exact-angle tables.
pub fn eval(e: &Expr) -> Option<Expr> {
    let [x] = e.args() else {
        return None;
    };
    let head = e.head_symbol()?;
    match head {
        B::SIN | B::COS | B::TAN => {
            if let Some(index) = pi_grid(x) {
                return Some(trig(head, index));
            }
            let positive = positive(x)?;
            Some(if head == B::COS {
                Expr::call(head, [positive])
            } else {
                neg(Expr::call(head, [positive]))
            })
        }
        B::ARCSIN | B::ARCCOS | B::ARCTAN => inverse(head, x),
        B::LOG => {
            if *x == Expr::int(1) {
                return Some(Expr::int(0));
            }
            if x.as_symbol() == Some(B::E) {
                return Some(Expr::int(1));
            }
            if x.is_head(B::POWER)
                && x.args().len() == 2
                && x.args()[0].as_symbol() == Some(B::E)
                && x.args()[1]
                    .as_number()
                    .is_some_and(|n| !matches!(n, Number::Complex(_)))
            {
                return Some(x.args()[1].clone());
            }
            if let Some(n) = x.as_number()
                && let Some(q) = rational(n)
                && q < Rational::ZERO
            {
                return Some(add([
                    Expr::call(B::LOG, [Expr::number(n.neg())]),
                    mul([imaginary_unit(), Expr::sym(B::PI)]),
                ]));
            }
            None
        }
        B::EXP => Some(pow(Expr::sym(B::E), x.clone())),
        B::SQRT => Some(sqrt(x.clone())),
        B::ABS => {
            if let Some(n) = x.as_number() {
                if !n.is_exact() {
                    return None;
                }
                return Some(if let Number::Complex(c) = n {
                    sqrt(Expr::number(c.re.mul(&c.re).add(&c.im.mul(&c.im))))
                } else {
                    Expr::number(
                        if n.cmp_real(&Number::Integer(0.into())) == Some(std::cmp::Ordering::Less)
                        {
                            n.neg()
                        } else {
                            n.clone()
                        },
                    )
                });
            }
            positive(x).map(|p| Expr::call(B::ABS, [p]))
        }
        B::RE | B::IM | B::CONJUGATE => {
            let n = x.as_number()?;
            Some(match (head, n) {
                (B::RE, Number::Complex(c)) => Expr::number(c.re.clone()),
                (B::IM, Number::Complex(c)) => Expr::number(c.im.clone()),
                (B::CONJUGATE, Number::Complex(c)) => {
                    Expr::number(Number::Complex(Box::new(Complex {
                        re: c.re.clone(),
                        im: c.im.neg(),
                    })))
                }
                (B::IM, _) => Expr::int(0),
                _ => x.clone(),
            })
        }
        B::PRODUCT_LOG => {
            if *x == Expr::int(0) {
                Some(Expr::int(0))
            } else if x.as_symbol() == Some(B::E) {
                Some(Expr::int(1))
            } else if *x == neg(pow(Expr::sym(B::E), Expr::int(-1))) {
                Some(Expr::int(-1))
            } else {
                None
            }
        }
        _ => None,
    }
}
fn imaginary_unit() -> Expr {
    Expr::number(Number::Complex(Box::new(Complex {
        re: Number::Integer(0.into()),
        im: Number::Integer(1.into()),
    })))
}
fn rational(n: &Number) -> Option<Rational> {
    match n {
        Number::Integer(n) => Some(Rational::from(n.clone())),
        Number::Rational(q) => Some(q.clone()),
        _ => None,
    }
}
fn pi_grid(x: &Expr) -> Option<u8> {
    let coefficient = if *x == Expr::int(0) {
        Rational::ZERO
    } else if x.as_symbol() == Some(B::PI) {
        Rational::ONE
    } else if x.is_head(B::TIMES) && x.args().len() == 2 && x.args()[1].as_symbol() == Some(B::PI) {
        rational(x.args()[0].as_number()?)?
    } else {
        return None;
    };
    let denominator = u32::try_from(coefficient.denominator()).ok()?;
    if ![1, 2, 3, 4, 6, 12].contains(&denominator) {
        return None;
    }
    let units = coefficient * Rational::from(12);
    let mut index = units.numerator() % Integer::from(24);
    if index < Integer::ZERO {
        index += 24;
    }
    u8::try_from(index).ok()
}
fn sine(index: u8) -> Expr {
    let negative = index > 12;
    let index = index % 12;
    let reduced = if index > 6 { 12 - index } else { index };
    let result = match reduced {
        0 => Expr::int(0),
        1 => mul([
            Expr::rational(1, 4),
            add([sqrt(Expr::int(6)), neg(sqrt(Expr::int(2)))]),
        ]),
        2 => Expr::rational(1, 2),
        3 => mul([Expr::rational(1, 2), sqrt(Expr::int(2))]),
        4 => mul([Expr::rational(1, 2), sqrt(Expr::int(3))]),
        5 => mul([
            Expr::rational(1, 4),
            add([sqrt(Expr::int(6)), sqrt(Expr::int(2))]),
        ]),
        _ => Expr::int(1),
    };
    if negative { neg(result) } else { result }
}
fn trig(head: Symbol, index: u8) -> Expr {
    if head == B::SIN {
        return sine(index);
    }
    if head == B::COS {
        return sine((index + 6) % 24);
    }
    let index = index % 12;
    let negative = index > 6;
    let reduced = if negative { 12 - index } else { index };
    let value = match reduced {
        0 => Expr::int(0),
        1 => add([Expr::int(2), neg(sqrt(Expr::int(3)))]),
        2 => div(sqrt(Expr::int(3)), Expr::int(3)),
        3 => Expr::int(1),
        4 => sqrt(Expr::int(3)),
        5 => add([Expr::int(2), sqrt(Expr::int(3))]),
        _ => Expr::call(B::DIRECTED_INFINITY, []),
    };
    if negative { neg(value) } else { value }
}
fn inverse(head: Symbol, x: &Expr) -> Option<Expr> {
    let positive = positive(x);
    let x = positive.as_ref().unwrap_or(x);
    let table = if head == B::ARCTAN {
        vec![
            (Expr::int(0), Expr::int(0)),
            (div(Expr::int(1), sqrt(Expr::int(3))), Expr::rational(1, 6)),
            (Expr::int(1), Expr::rational(1, 4)),
            (sqrt(Expr::int(3)), Expr::rational(1, 3)),
        ]
    } else {
        vec![
            (Expr::int(0), Expr::int(0)),
            (Expr::rational(1, 2), Expr::rational(1, 6)),
            (div(sqrt(Expr::int(2)), Expr::int(2)), Expr::rational(1, 4)),
            (div(sqrt(Expr::int(3)), Expr::int(2)), Expr::rational(1, 3)),
            (Expr::int(1), Expr::rational(1, 2)),
        ]
    };
    let (_, fraction) = table.into_iter().find(|(value, _)| value == x)?;
    let mut fraction = if positive.is_some() {
        neg(fraction)
    } else {
        fraction
    };
    if head == B::ARCCOS {
        fraction = add([Expr::rational(1, 2), neg(fraction)]);
    }
    Some(mul([fraction, Expr::sym(B::PI)]))
}
fn positive(x: &Expr) -> Option<Expr> {
    if let Some(n) = x.as_number()
        && rational(n).is_some_and(|q| q < Rational::ZERO)
    {
        return Some(Expr::number(n.neg()));
    }
    if x.is_head(B::TIMES)
        && let Some(first) = x.args().first()
        && let Some(first) = positive(first)
    {
        return Some(mul(
            std::iter::once(first).chain(x.args()[1..].iter().cloned())
        ));
    }
    None
}
