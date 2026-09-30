//! Ball compositions, with explicit principal values on real branch cuts.

use super::zero;
use om_core::{Abort, BUILTIN as B, Expr, Interrupt, Symbol};
use om_num::{Ball, BigFloat, BitTest, CBall, Integer, Number, Rational};

pub(super) fn accepts(h: Symbol, n: usize) -> bool {
    match h.name() {
        "Plus" | "Times" => true,
        "Power" | "Subtract" | "Divide" => n == 2,
        "Log" | "ArcTan" => n == 1 || n == 2,
        "Minus" | "Sqrt" | "Exp" | "Sin" | "Cos" | "Tan" | "Cot" | "Sec" | "Csc" | "ArcSin"
        | "ArcCos" | "ArcCot" | "ArcSec" | "ArcCsc" | "Sinh" | "Cosh" | "Tanh" | "Coth"
        | "Sech" | "Csch" | "ArcSinh" | "ArcCosh" | "ArcTanh" | "Abs" | "Sign" | "Re" | "Im"
        | "Conjugate" | "Arg" => n == 1,
        _ => false,
    }
}
pub(super) fn integer(n: i64, bits: u32) -> CBall {
    CBall::exact(&n.into(), &Rational::ZERO, bits)
}
pub(super) fn real(re: Ball) -> CBall {
    let im = integer(0, re.prec).im;
    CBall { re, im }
}
fn neg(z: &CBall) -> CBall {
    CBall {
        re: Ball {
            mid: -&z.re.mid,
            ..z.re.clone()
        },
        im: Ball {
            mid: -&z.im.mid,
            ..z.im.clone()
        },
    }
}
fn i_mul(z: &CBall) -> CBall {
    CBall {
        re: neg(z).im,
        im: z.re.clone(),
    }
}
fn pi(bits: u32) -> CBall {
    real(Ball::pi(bits))
}
fn is_real(z: &CBall) -> bool {
    zero(&z.im)
}
fn positive(b: &Ball) -> bool {
    b.mid > b.rad
}
fn negative(b: &Ball) -> bool {
    -&b.mid > b.rad
}
fn exact(b: &Ball, n: i64) -> bool {
    b.mid == BigFloat::from(n) && b.rad == BigFloat::ZERO
}

pub(super) fn apply(
    e: &Expr,
    args: &[CBall],
    bits: u32,
    ctx: &Interrupt,
) -> Result<Option<CBall>, Abort> {
    ctx.tick()?;
    let head = e
        .head_symbol()
        .expect("invariant: numeric head checked before applying");
    let one = integer(1, bits);
    let two = integer(2, bits);
    if head == B::PLUS || head == B::TIMES {
        let mut value = integer(i64::from(head == B::TIMES), bits);
        for x in args {
            ctx.tick()?;
            value = if head == B::PLUS {
                value.add(x)
            } else {
                value.mul(x)
            };
        }
        return Ok(Some(value));
    }
    let z = &args[0];
    if matches!(head, B::SIN | B::COS | B::TAN | B::COT | B::SEC | B::CSC) && !exp_range(&z.im) {
        return Ok(None);
    }
    Ok(Some(match head.name() {
        "Subtract" => z.sub(&args[1]),
        "Divide" => z.div(&args[1]),
        "Minus" => neg(z),
        "Power" => return power(e, z, &args[1], bits, ctx),
        "Sqrt" => sqrt(z),
        "Exp" => {
            let Some(value) = exp(z) else {
                return Ok(None);
            };
            value
        }
        "Log" if args.len() == 2 => ln(&args[1]).div(&ln(z)),
        "Log" => ln(z),
        "Sin" => z.sin(),
        "Cos" => z.cos(),
        "Tan" => z.sin().div(&z.cos()),
        "Cot" => z.cos().div(&z.sin()),
        "Sec" => one.div(&z.cos()),
        "Csc" => one.div(&z.sin()),
        "ArcSin" => asin(z),
        "ArcCos" => acos(z),
        "ArcTan" if args.len() == 2 => {
            let Some(value) = atan2(z, &args[1]) else {
                return Ok(None);
            };
            value
        }
        "ArcTan" => atan(z),
        "ArcCot" if zero(&z.re) && zero(&z.im) => pi(bits).div(&two),
        "ArcCot" => atan(&one.div(z)),
        "ArcSec" => acos(&one.div(z)),
        "ArcCsc" => asin(&one.div(z)),
        "Sinh" | "Cosh" | "Tanh" | "Coth" | "Sech" | "Csch" => {
            let Some((sinh, cosh)) = hyperbolic(z) else {
                return Ok(None);
            };
            match head.name() {
                "Sinh" => sinh,
                "Cosh" => cosh,
                "Tanh" => sinh.div(&cosh),
                "Coth" => cosh.div(&sinh),
                "Sech" => one.div(&cosh),
                _ => one.div(&sinh),
            }
        }
        "ArcSinh" => asinh(z),
        "ArcCosh" => acosh(z),
        "ArcTanh" => ln(&one.add(z)).sub(&ln(&one.sub(z))).div(&two),
        "Re" => real(z.re.clone()),
        "Im" => real(z.im.clone()),
        "Conjugate" => CBall {
            re: z.re.clone(),
            im: neg(z).im,
        },
        "Abs" => magnitude(z),
        "Sign" if zero(&z.re) && zero(&z.im) => integer(0, bits),
        "Sign" => z.div(&magnitude(z)),
        "Arg" if zero(&z.re) && zero(&z.im) => integer(0, bits),
        "Arg" => real(ln(z).im),
        _ => return Ok(None),
    }))
}
fn sqrt(z: &CBall) -> CBall {
    if is_real(z) {
        if negative(&z.re) {
            return CBall {
                re: integer(0, z.re.prec).re,
                im: neg(z).re.sqrt(),
            };
        }
        if z.re.mid >= z.re.rad {
            return real(z.re.sqrt());
        }
    }
    z.sqrt()
}
fn ln(z: &CBall) -> CBall {
    if is_real(z) {
        if positive(&z.re) {
            return real(z.re.ln());
        }
        if negative(&z.re) {
            return CBall {
                re: neg(z).re.ln(),
                im: Ball::pi(z.re.prec),
            };
        }
    }
    z.ln()
}
fn exp(z: &CBall) -> Option<CBall> {
    if !exp_range(&z.re) {
        return None;
    }
    Some(if is_real(z) {
        real(z.re.exp())
    } else {
        z.exp()
    })
}
fn exp_range(b: &Ball) -> bool {
    let magnitude = (if b.mid < BigFloat::ZERO {
        -&b.mid
    } else {
        b.mid.clone()
    }) + &b.rad;
    let max_log = (isize::MAX - 16_384) as f64 * std::f64::consts::LN_2;
    let x = magnitude.to_f64().value();
    x.is_finite() && x <= max_log
}
fn magnitude(z: &CBall) -> CBall {
    if is_real(z) {
        if negative(&z.re) {
            return neg(z);
        }
        if z.re.mid >= z.re.rad {
            return z.clone();
        }
    }
    // Squaring via independent intervals can include a negative lower endpoint at zero.
    // CBall::sqrt knows the nonnegative norm and handles that dependency safely.
    real(real(z.re.mul(&z.re).add(&z.im.mul(&z.im))).sqrt().re)
}
fn asin(z: &CBall) -> CBall {
    let bits = z.re.prec;
    let (one, two) = (integer(1, bits), integer(2, bits));
    if is_real(z) {
        if exact(&z.re, 1) {
            return pi(bits).div(&two);
        }
        if exact(&z.re, -1) {
            return neg(&pi(bits).div(&two));
        }
        let square = one.sub(&z.mul(z));
        if positive(&square.re) {
            return real(z.re.div(&square.re.sqrt()).atan());
        }
        if positive(&z.sub(&one).re) {
            return CBall {
                re: pi(bits).div(&two).re,
                im: neg(&acosh(z)).re,
            };
        }
        if negative(&z.add(&one).re) {
            return neg(&asin(&neg(z)));
        }
    }
    neg(&i_mul(&ln(&i_mul(z).add(&sqrt(&one.sub(&z.mul(z)))))))
}
fn atan(z: &CBall) -> CBall {
    if is_real(z) {
        return real(z.re.atan());
    }
    let one = integer(1, z.re.prec);
    i_mul(&ln(&one.sub(&i_mul(z))).sub(&ln(&one.add(&i_mul(z))))).div(&integer(2, z.re.prec))
}
fn acos(z: &CBall) -> CBall {
    let bits = z.re.prec;
    let one = integer(1, bits);
    if is_real(z) {
        if exact(&z.re, 1) {
            return integer(0, bits);
        }
        if exact(&z.re, -1) {
            return pi(bits);
        }
        if positive(&z.sub(&one).re) {
            return i_mul(&acosh(z));
        }
        if negative(&z.add(&one).re) {
            return pi(bits).sub(&i_mul(&acosh(&neg(z))));
        }
    }
    pi(bits).div(&integer(2, bits)).sub(&asin(z))
}
fn atan2(x: &CBall, y: &CBall) -> Option<CBall> {
    if !is_real(x) || !is_real(y) {
        return None;
    }
    let bits = x.re.prec;
    if zero(&x.re) {
        return if positive(&y.re) {
            Some(pi(bits).div(&integer(2, bits)))
        } else if negative(&y.re) {
            Some(neg(&pi(bits).div(&integer(2, bits))))
        } else {
            None
        };
    }
    if positive(&x.re) {
        return Some(atan(&y.div(x)));
    }
    if negative(&x.re) {
        let a = atan(&y.div(x));
        if y.re.mid >= y.re.rad {
            return Some(a.add(&pi(bits)));
        }
        if negative(&y.re) {
            return Some(a.sub(&pi(bits)));
        }
    }
    None
}
fn hyperbolic(z: &CBall) -> Option<(CBall, CBall)> {
    let (a, b) = (exp(z)?, exp(&neg(z))?);
    let two = integer(2, z.re.prec);
    Some((a.sub(&b).div(&two), a.add(&b).div(&two)))
}
fn asinh(z: &CBall) -> CBall {
    if is_real(z) && negative(&z.re) {
        return neg(&asinh(&neg(z)));
    }
    ln(&z.add(&sqrt(&z.mul(z).add(&integer(1, z.re.prec)))))
}
fn acosh(z: &CBall) -> CBall {
    let bits = z.re.prec;
    let one = integer(1, bits);
    if is_real(z) {
        if negative(&z.add(&one).re) {
            return CBall {
                re: acosh(&neg(z)).re,
                im: Ball::pi(bits),
            };
        }
        if &z.re.mid + &z.re.rad <= BigFloat::ONE && &z.re.mid - &z.re.rad >= -BigFloat::ONE {
            return i_mul(&pi(bits).div(&integer(2, bits)).sub(&asin(z)));
        }
    }
    ln(&z.add(&sqrt(&z.sub(&one)).mul(&sqrt(&z.add(&one)))))
}
fn power(
    e: &Expr,
    base: &CBall,
    exponent: &CBall,
    bits: u32,
    ctx: &Interrupt,
) -> Result<Option<CBall>, Abort> {
    let one = integer(1, bits);
    if let Some(Number::Integer(n)) = e.args()[1].as_number() {
        if n.is_zero() && zero(&base.re) && zero(&base.im) {
            return Ok(None);
        }
        if zero(&base.re) && zero(&base.im) {
            return Ok((n > &Integer::ZERO).then(|| integer(0, bits)));
        }
        if !n.is_zero() && !exp_range(&ln(base).mul(exponent).re) {
            return Ok(None);
        }
        let mut base = if n < &Integer::ZERO {
            one.div(base)
        } else {
            base.clone()
        };
        let n = n.clone().into_parts().1;
        let mut result = one;
        for bit in 0..n.bit_len() {
            ctx.tick()?;
            if n.bit(bit) {
                result = result.mul(&base);
                if !super::finite(&result) {
                    return Ok(None);
                }
            }
            if bit + 1 < n.bit_len() {
                base = base.mul(&base);
            }
        }
        return Ok(Some(result));
    }
    if is_real(exponent) && exact(&exponent.re, 0) {
        return Ok((!zero(&base.re) || !zero(&base.im)).then_some(one));
    }
    if zero(&base.re) && zero(&base.im) {
        return Ok(positive(&exponent.re).then(|| integer(0, bits)));
    }
    if is_real(exponent)
        && exponent.re.rad == BigFloat::ZERO
        && exponent.re.mid == BigFloat::ONE / 2_u8
    {
        return Ok(Some(sqrt(base)));
    }
    Ok(exp(&ln(base).mul(exponent)))
}
