//! Guarded principal log/exp, with a single final precision rounding.

use om_num::{Ball, BigFloat, CBall, Complex, Number, Precision, Rational, Real};

fn precision(a: Precision, b: Precision) -> Precision {
    match (a, b) {
        (Precision::Machine, _) | (_, Precision::Machine) => Precision::Machine,
        (Precision::Bits(a), Precision::Bits(b)) => Precision::Bits(a.min(b)),
        (Precision::Bits(bits), _) | (_, Precision::Bits(bits)) => Precision::Bits(bits),
        _ => Precision::Machine,
    }
}
fn bits(p: Precision) -> u32 {
    if let Precision::Bits(p) = p { p } else { 53 }
}
fn scalar(n: &Number) -> Rational {
    match n {
        Number::Integer(n) => Rational::from(n.clone()),
        Number::Rational(n) => n.clone(),
        Number::Real(Real::Machine(x)) => {
            Rational::try_from(*x).expect("invariant: normalized machine value is finite")
        }
        Number::Real(Real::Big(x)) => {
            Rational::try_from(x.clone()).expect("invariant: normalized big value is finite")
        }
        Number::Complex(_) => unreachable!("invariant: complex components are scalar"),
    }
}
fn ball(n: &Number, work: u32) -> CBall {
    if let Number::Complex(c) = n {
        CBall::exact(&scalar(&c.re), &scalar(&c.im), work)
    } else {
        CBall::exact(&scalar(n), &Rational::ZERO, work)
    }
}
fn in_exp_range(z: &CBall) -> bool {
    let mid = if z.re.mid < BigFloat::ZERO {
        -&z.re.mid
    } else {
        z.re.mid.clone()
    };
    let magnitude = mid + &z.re.rad;
    let max_log = (isize::MAX - 16_384) as f64 * std::f64::consts::LN_2;
    let x = magnitude.to_f64().value();
    x.is_finite() && x <= max_log
}
fn guarded_exp(z: &CBall) -> Option<CBall> {
    in_exp_range(z).then(|| z.exp())
}
fn finish(re: BigFloat, im: Option<BigFloat>, p: Precision) -> Number {
    let re = re.with_precision(bits(p) as usize).value();
    let im = im.map(|x| x.with_precision(bits(p) as usize).value());
    let machine = p == Precision::Machine
        && re.to_f64().value().is_finite()
        && im.as_ref().is_none_or(|x| x.to_f64().value().is_finite());
    let component = |x: BigFloat| {
        if machine {
            Number::Real(Real::Machine(x.to_f64().value()))
        } else {
            Number::Real(Real::Big(x))
        }
    };
    if let Some(im) = im {
        Number::Complex(Box::new(Complex {
            re: component(re),
            im: component(im),
        }))
        .normalize()
    } else {
        component(re)
    }
}

pub(super) fn numeric_power(base: &Number, exp: &Number) -> Option<Number> {
    let p = precision(base.precision(), exp.precision());
    let work = bits(p).saturating_add(32);
    if let Number::Integer(e) = exp {
        let logarithm = ball(base, work).ln().mul(&ball(exp, work));
        return if in_exp_range(&logarithm) {
            base.pow_int(e).ok()
        } else {
            None
        };
    }
    if !matches!(base, Number::Complex(_)) && !matches!(exp, Number::Complex(_)) {
        let b = scalar(base);
        let e = scalar(exp);
        let half = Rational::from(1) / Rational::from(2);
        if e == half {
            let root = Ball::exact(&if b < Rational::ZERO { -&b } else { b.clone() }, work).sqrt();
            return Some(if b < Rational::ZERO {
                finish(BigFloat::ZERO, Some(root.mid), p)
            } else {
                finish(root.mid, None, p)
            });
        }
        if b > Rational::ZERO || e.denominator().is_one() {
            let magnitude = if b < Rational::ZERO { -&b } else { b.clone() };
            let log = Ball::exact(&magnitude, work)
                .ln()
                .mul(&Ball::exact(&e, work));
            let result = guarded_exp(&CBall {
                re: log,
                im: Ball::exact(&Rational::ZERO, work),
            })?;
            let negative = b < Rational::ZERO && e.numerator() % 2 != 0;
            return Some(finish(
                if negative {
                    -result.re.mid
                } else {
                    result.re.mid
                },
                None,
                p,
            ));
        }
    }
    let z = ball(base, work).ln().mul(&ball(exp, work));
    let result = guarded_exp(&z)?;
    Some(finish(result.re.mid, Some(result.im.mid), p))
}
pub(super) fn numeric_exp(exp: &Number) -> Option<Number> {
    let p = precision(Precision::Exact, exp.precision());
    let result = guarded_exp(&ball(exp, bits(p).saturating_add(32)))?;
    let im = matches!(exp, Number::Complex(_)).then_some(result.im.mid);
    Some(finish(result.re.mid, im, p))
}
