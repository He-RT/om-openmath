//! Bounded extraction of principal exact rational powers.

use super::{mul, pow};
use crate::{BUILTIN as B, Expr};
use om_num::{
    BitTest, Complex, Integer, Number, Rational, extract_root_factor, gcd, perfect_power,
};

pub(super) fn rational_power(r: Rational, e: Rational) -> Result<Expr, ()> {
    if r < Rational::ZERO {
        let phase = minus_one_power(e.clone());
        if r == Rational::from(-1) {
            return Ok(phase);
        }
        return Ok(mul([phase, rational_power(-r, e)?]));
    }
    let p = Integer::from(e.numerator().clone().into_parts().1);
    let q = Integer::from(e.denominator().clone());
    let k = &p / &q;
    let s = &p % &q;
    let negative = e < Rational::ZERO;
    let signed = |n: Integer| if negative { -n } else { n };
    let mut coefficient = Number::Rational(r.clone())
        .pow_int(&signed(k))
        .map_err(|_| ())?;
    let t = Rational::from(signed(s.clone())) / Rational::from(q.clone());
    let Ok(degree) = u32::try_from(e.denominator().clone()) else {
        return Ok(with_coefficient(coefficient, raw(r, t)));
    };
    let (c, a) = extract_root_factor(r.numerator(), degree);
    let (d, b) = extract_root_factor(&Integer::from(r.denominator().clone()), degree);
    let outside = Number::Rational(Rational::from(c) / Rational::from(d))
        .pow_int(&signed(s))
        .map_err(|_| ())?;
    coefficient = coefficient.mul(&outside);
    if exact_bits(&coefficient) > 1 << 24 {
        return Err(());
    }
    let root = reduced_root(a, b, t, &q);
    Ok(with_coefficient(coefficient, root))
}

fn with_coefficient(coefficient: Number, root: Expr) -> Expr {
    if coefficient.is_exact() && coefficient.is_one() {
        root
    } else if root == Expr::int(1) {
        Expr::number(coefficient)
    } else {
        mul([Expr::number(coefficient), root])
    }
}

fn raw(r: Rational, e: Rational) -> Expr {
    Expr::call(
        B::POWER,
        [
            Expr::number(Number::Rational(r)),
            Expr::number(Number::Rational(e)),
        ],
    )
}
fn reduced_root(a: Integer, b: Integer, t: Rational, q: &Integer) -> Expr {
    if a.is_one() && b.is_one() {
        return Expr::int(1);
    }
    let ap = perfect_power(&a);
    let bp = perfect_power(&b);
    if a.is_one()
        && let Some((base, m)) = bp
    {
        return pow(
            Expr::integer(base),
            Expr::number(Number::Rational(-t * Rational::from(m))),
        );
    }
    if b.is_one()
        && let Some((base, m)) = ap
    {
        return pow(
            Expr::integer(base),
            Expr::number(Number::Rational(t * Rational::from(m))),
        );
    }
    if let (Some((ar, am)), Some((br, bm))) = (&ap, &bp) {
        let common = gcd(&gcd(&Integer::from(*am), &Integer::from(*bm)), q);
        if common > Integer::ONE {
            // common divides u32 perfect-power degrees, hence fits u32.
            let g = u32::try_from(common).expect("invariant: common degree divides u32 degrees");
            let r = Rational::from(ar.pow((*am / g) as usize))
                / Rational::from(br.pow((*bm / g) as usize));
            return pow(
                Expr::number(Number::Rational(r)),
                Expr::number(Number::Rational(t * Rational::from(g))),
            );
        }
    }
    let reducible = |power: &Option<(Integer, u32)>| {
        power
            .as_ref()
            .is_some_and(|(_, m)| gcd(&Integer::from(*m), q) > Integer::ONE)
    };
    if !a.is_one() && !b.is_one() && (reducible(&ap) || reducible(&bp)) {
        return mul([
            pow(Expr::integer(a), Expr::number(Number::Rational(t.clone()))),
            pow(Expr::integer(b), Expr::number(Number::Rational(-t))),
        ]);
    }
    if b.is_one() {
        raw(Rational::from(a), t)
    } else if a.is_one() {
        raw(Rational::from(b), -t)
    } else {
        raw(Rational::from(a) / Rational::from(b), t)
    }
}

pub(super) fn minus_one_power(e: Rational) -> Expr {
    let q = Integer::from(e.denominator().clone());
    let modulus = &q * 2;
    let mut p = e.numerator() % &modulus;
    if p < Integer::ZERO {
        p += modulus;
    }
    let reduced = Rational::from(p) / Rational::from(q);
    if reduced.is_zero() {
        return Expr::int(1);
    }
    if reduced.is_one() {
        return Expr::int(-1);
    }
    let half = Rational::from(1) / Rational::from(2);
    if reduced == half || reduced == Rational::from(3) / Rational::from(2) {
        return Expr::number(Number::Complex(Box::new(Complex {
            re: Number::Integer(0.into()),
            im: Number::Integer(if reduced == half {
                1.into()
            } else {
                (-1).into()
            }),
        })));
    }
    if reduced > Rational::ONE {
        mul([Expr::int(-1), minus_one_power(reduced - Rational::ONE)])
    } else {
        raw(Rational::from(-1), reduced)
    }
}

fn exact_bits(n: &Number) -> usize {
    match n {
        Number::Integer(n) => n.bit_len(),
        Number::Rational(q) => q.numerator().bit_len().max(q.denominator().bit_len()),
        _ => 0,
    }
}
