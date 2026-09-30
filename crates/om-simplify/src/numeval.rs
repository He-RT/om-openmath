//! Guarded ball evaluation of numeric trees, shared by N and solver consumers.

use om_core::{Abort, BUILTIN as B, Expr, ExprKind, Interrupt};
use om_num::{Ball, BigFloat, BitTest, CBall, Complex, Integer, Number, Precision, Rational, Real};

#[path = "numeval_elementary.rs"]
mod elementary;

const MAX_BITS: u32 = 16_384;

/// Evaluate a fully numeric expression at the requested working bit precision.
/// Unsupported trees, singularities and exponent/resource overflow return None.
pub fn evaluate(e: &Expr, bits: u32, ctx: &Interrupt) -> Result<Option<Number>, Abort> {
    if !(1..=MAX_BITS).contains(&bits) {
        ctx.tick()?;
        return Ok(None);
    }
    Ok(evaluate_ball(e, bits, ctx)?.map(|z| midpoint(&z, Precision::Bits(bits))))
}

/// Approximate with guarded work, precision doubling and certified final rounding.
/// Exact mode preserves exact numeric atoms; invalid precision returns None.
pub fn approximate(
    e: &Expr,
    precision: Precision,
    ctx: &Interrupt,
) -> Result<Option<Number>, Abort> {
    ctx.tick()?;
    if precision == Precision::Exact {
        return Ok(e.as_number().filter(|n| n.is_exact()).cloned());
    }
    let target = match precision {
        Precision::Bits(bits) if (1..=MAX_BITS).contains(&bits) => bits,
        Precision::Machine => 64,
        _ => return Ok(None),
    };
    let mut work = target + 32;
    for _ in 0..4 {
        ctx.tick()?;
        let Some(a) = evaluate_ball(e, work, ctx)? else {
            return Ok(None);
        };
        let Some(b) = evaluate_ball(e, 2 * work, ctx)? else {
            return Ok(None);
        };
        if let (Some(a), Some(b)) = (rounded(&a, precision), rounded(&b, precision))
            && a == b
        {
            return Ok(Some(b));
        }
        work *= 2;
    }
    Ok(None)
}

enum Frame<'a> {
    Visit(&'a Expr),
    Apply(&'a Expr),
}

fn evaluate_ball(e: &Expr, bits: u32, ctx: &Interrupt) -> Result<Option<CBall>, Abort> {
    // Refinement may use up to sixteen times MAX_BITS, with finite work in every call.
    if bits == 0 || bits > 16 * (MAX_BITS + 32) {
        return Ok(None);
    }
    let mut frames = vec![Frame::Visit(e)];
    let mut values = Vec::new();
    while let Some(frame) = frames.pop() {
        ctx.tick()?;
        match frame {
            Frame::Visit(e) => match e.kind() {
                ExprKind::Number(n) => values.push(number_ball(n, bits)),
                ExprKind::Symbol(s) => {
                    let z = match *s {
                        B::PI => elementary::real(Ball::pi(bits)),
                        B::E => elementary::real(elementary::integer(1, bits).re.exp()),
                        B::I => CBall::exact(&Rational::ZERO, &Rational::ONE, bits),
                        _ => return Ok(None),
                    };
                    values.push(z);
                }
                ExprKind::Normal(_) => {
                    if !e
                        .head_symbol()
                        .is_some_and(|h| elementary::accepts(h, e.args().len()))
                    {
                        return Ok(None);
                    }
                    frames.push(Frame::Apply(e));
                    frames.extend(e.args().iter().rev().map(Frame::Visit));
                }
                _ => return Ok(None),
            },
            Frame::Apply(e) => {
                let start = values.len() - e.args().len();
                let args = values.split_off(start);
                let Some(z) = elementary::apply(e, &args, bits, ctx)? else {
                    return Ok(None);
                };
                if !finite(&z) {
                    return Ok(None);
                }
                values.push(z);
            }
        }
    }
    Ok(values.pop())
}

fn scalar(n: &Number, bits: u32) -> Ball {
    match n {
        Number::Integer(n) => Ball::exact(&n.clone().into(), bits),
        Number::Rational(q) => Ball::exact(q, bits),
        Number::Real(Real::Machine(x)) => Ball::exact(
            &Rational::try_from(*x).expect("invariant: Number machine values are finite"),
            bits,
        ),
        Number::Real(Real::Big(x)) => {
            let mid = x.clone().with_precision(bits as usize).value();
            let rad = if mid == *x {
                BigFloat::ZERO
            } else {
                let magnitude = x.repr().exponent()
                    + x.repr().significand().clone().into_parts().1.bit_len() as isize;
                // One ulp encloses the nearest-rounding error without expanding the exponent.
                BigFloat::from_parts(Integer::ONE, magnitude - bits as isize)
            };
            Ball {
                mid,
                rad,
                prec: bits,
            }
        }
        Number::Complex(_) => unreachable!("invariant: numeric components are scalar"),
    }
}
fn number_ball(n: &Number, bits: u32) -> CBall {
    if let Number::Complex(c) = n {
        CBall {
            re: scalar(&c.re, bits),
            im: scalar(&c.im, bits),
        }
    } else {
        elementary::real(scalar(n, bits))
    }
}
fn finite(z: &CBall) -> bool {
    [&z.re, &z.im]
        .iter()
        .all(|b| b.mid.repr().is_finite() && b.rad.repr().is_finite())
}
fn zero(b: &Ball) -> bool {
    b.mid == BigFloat::ZERO && b.rad == BigFloat::ZERO
}
fn component(x: BigFloat, p: Precision) -> Number {
    if p == Precision::Machine {
        let f = x.to_f64().value();
        if f.is_finite() && (f != 0.0 || x == BigFloat::ZERO) {
            return Number::Real(Real::Machine(f));
        }
    }
    let bits = if let Precision::Bits(bits) = p {
        bits
    } else {
        64
    };
    Number::Real(Real::Big(x.with_precision(bits as usize).value()))
}
fn compose(re: Number, im: Option<Number>) -> Number {
    if let Some(im) = im {
        // A machine overflow in either component promotes both to the 64-bit fallback.
        // Number's ordinary machine contagion would otherwise lower that to 53 bits.
        let bits = match (re.precision(), im.precision()) {
            (Precision::Bits(a), Precision::Bits(b)) => Some(a.min(b)),
            (Precision::Bits(a), _) | (_, Precision::Bits(a)) => Some(a),
            _ => None,
        };
        let (re, im) = if let Some(bits) = bits {
            (
                Number::Real(Real::Big(scalar(&re, bits).mid)),
                Number::Real(Real::Big(scalar(&im, bits).mid)),
            )
        } else {
            (re, im)
        };
        Number::Complex(Box::new(Complex { re, im })).normalize()
    } else {
        re
    }
}
fn midpoint(z: &CBall, p: Precision) -> Number {
    compose(
        component(z.re.mid.clone(), p),
        (!zero(&z.im)).then(|| component(z.im.mid.clone(), p)),
    )
}
fn rounded_component(b: &Ball, p: Precision) -> Option<Number> {
    if b.rad == BigFloat::ZERO {
        return Some(component(b.mid.clone(), p));
    }
    if b.mid == BigFloat::ZERO {
        let lo = component(-&b.rad, p);
        return (lo == component(b.rad.clone(), p)).then_some(lo);
    }
    // Align dyadic significands and round the exact endpoints once. Rational
    // conversion would expand huge exponents; nearest RBig conversion can also
    // double-round a p+1-bit quotient before reducing it to p bits.
    let exponent = b.mid.repr().exponent().min(b.rad.repr().exponent());
    let mid_shift = b.mid.repr().exponent().checked_sub(exponent)? as usize;
    let rad_shift = b.rad.repr().exponent().checked_sub(exponent)? as usize;
    if mid_shift.max(rad_shift) > 32 * (MAX_BITS as usize + 32) {
        return None;
    }
    let mid = b.mid.repr().significand() << mid_shift;
    let rad = b.rad.repr().significand() << rad_shift;
    let lo = component(BigFloat::from_parts(&mid - &rad, exponent), p);
    let hi = component(BigFloat::from_parts(mid + rad, exponent), p);
    (lo == hi).then_some(lo)
}
fn rounded(z: &CBall, p: Precision) -> Option<Number> {
    let re = rounded_component(&z.re, p)?;
    let im = if zero(&z.im) {
        None
    } else {
        Some(rounded_component(&z.im, p)?)
    };
    Some(compose(re, im))
}
