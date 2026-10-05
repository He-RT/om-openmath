//! Classical recurrence/Stirling and incomplete-gamma series/continued fractions.
//! Formula references and precision boundaries: docs/design/special-probability.md.
use crate::Error;
use om_num::ctx::Interrupt;

fn positive(x: f64) -> Result<(), Error> {
    if !x.is_finite() || x <= 0.0 {
        Err(Error::Input("此函数首版只支持有限正实参数"))
    } else {
        Ok(())
    }
}
fn finite(x: f64) -> Result<f64, Error> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(Error::NonFinite)
    }
}
fn correction(inverse: f64) -> f64 {
    let square = inverse * inverse;
    inverse
        * (1. / 12.
            + square
                * (-1. / 360.
                    + square
                        * (1. / 1260.
                            + square
                                * (-1. / 1680.
                                    + square
                                        * (1. / 1188.
                                            + square * (-691. / 360360. + square * (1. / 156.)))))))
}
/// Log Gamma for finite positive machine reals, using recurrence up to 16.
pub fn log_gamma(mut x: f64, ctx: &Interrupt) -> Result<f64, Error> {
    ctx.tick()?;
    positive(x)?;
    if x == 1. || x == 2. {
        return Ok(0.);
    }
    // Around Gamma(1)=Gamma(2)=1, subtracting large Stirling/recurrence terms loses the small logarithm.
    if (0.75..=1.25).contains(&x) || (1.75..=2.25).contains(&x) {
        let second = x > 1.5;
        let z = if second { x - 2. } else { x - 1. };
        let zeta = [
            1.6449340668482264,
            1.2020569031595943,
            1.0823232337111381,
            1.03692775514337,
            1.0173430619844492,
            1.0083492773819228,
            1.0040773561979443,
            1.0020083928260822,
            1.000994575127818,
            1.0004941886041194,
            1.000246086553308,
            1.0001227133475785,
            1.0000612481350588,
            1.000030588236307,
        ];
        let mut value = -0.5772156649015329 * z;
        let mut power = z;
        for n in 2..=32 {
            ctx.tick()?;
            power *= -z;
            let coefficient = zeta.get(n - 2).copied().unwrap_or_else(|| {
                1. + (2..=16).map(|k| (k as f64).powi(-(n as i32))).sum::<f64>()
            });
            value -= coefficient * power / n as f64;
        }
        return finite(value + if second { z.ln_1p() } else { 0. });
    }
    let mut shift = 0.;
    while x < 16. {
        ctx.tick()?;
        shift -= x.ln();
        x += 1.;
    }
    finite((x - 0.5) * x.ln() - x + 0.9189385332046727 + correction(1. / x) + shift)
}
/// Gamma on the positive real axis. Overflow is a failure, not a successful infinity.
pub fn gamma(x: f64, ctx: &Interrupt) -> Result<f64, Error> {
    finite(log_gamma(x, ctx)?.exp())
}
/// Positive-real Beta, retaining small arguments beside very large ones.
pub fn beta(mut a: f64, mut b: f64, ctx: &Interrupt) -> Result<f64, Error> {
    ctx.tick()?;
    positive(a)?;
    positive(b)?;
    if a == 1. {
        return finite(1. / b);
    }
    if b == 1. {
        return finite(1. / a);
    }
    let mut shift = 0.;
    while a < 16. {
        ctx.tick()?;
        shift += b.ln() + (a / b).ln_1p() - a.ln();
        a += 1.;
    }
    while b < 16. {
        ctx.tick()?;
        shift += a.ln() + (b / a).ln_1p() - b.ln();
        b += 1.;
    }
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    let ratio = a / b;
    let log_sum = b.ln() + ratio.ln_1p();
    let exponent =
        (a - 0.5) * (ratio.ln() - ratio.ln_1p()) - (b - 0.5) * ratio.ln_1p() - 0.5 * log_sum
            + 0.9189385332046727
            + correction(1. / a)
            + correction(1. / b)
            - correction((1. / b) / (1. + ratio))
            + shift;
    finite(exponent.exp())
}
fn small_erf(x: f64, ctx: &Interrupt) -> Result<f64, Error> {
    let mut sum = 1.;
    let mut term = 1.;
    for n in 1..=256 {
        ctx.tick()?;
        term *= x * x / (n as f64 + 0.5);
        sum += term;
        if term.abs() <= sum.abs() * f64::EPSILON {
            return Ok(2. * x * (-x * x).exp() * sum / std::f64::consts::PI.sqrt());
        }
    }
    Err(Error::NoConvergence)
}
/// Complementary error function with a separately computed tail (no 1-erf cancellation).
pub fn erfc(x: f64, ctx: &Interrupt) -> Result<f64, Error> {
    ctx.tick()?;
    if !x.is_finite() {
        return Err(Error::NonFinite);
    }
    if x < 0. {
        return Ok(2. - erfc(-x, ctx)?);
    }
    if x < 1.5 {
        return Ok(1. - small_erf(x, ctx)?);
    }
    if x > 28. {
        return Ok(0.);
    } // Below the smallest representable subnormal.
    let t = x * x;
    let mut b = t + 0.5;
    let mut c = 1e300;
    let mut d = 1. / b;
    let mut h = d;
    for n in 1..=512 {
        ctx.tick()?;
        let a = -(n as f64) * (n as f64 - 0.5);
        b += 2.;
        d = b + a * d;
        c = b + a / c;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1. / d;
        let delta = c * d;
        h *= delta;
        if (delta - 1.).abs() <= 4. * f64::EPSILON {
            return finite(if t < 700. {
                (-t).exp() * (x / std::f64::consts::PI.sqrt()) * h
            } else {
                (-t + x.ln() - 0.5723649429247001 + h.ln()).exp()
            });
        }
    }
    Err(Error::NoConvergence)
}
/// Error function with exact odd symmetry and a noncancelling small-argument series.
pub fn erf(x: f64, ctx: &Interrupt) -> Result<f64, Error> {
    ctx.tick()?;
    if !x.is_finite() {
        return Err(Error::NonFinite);
    }
    if x.abs() < 1.5 {
        small_erf(x, ctx)
    } else {
        Ok(x.signum() * (1. - erfc(x.abs(), ctx)?))
    }
}
/// Standard normal quantile, including mathematical infinities at p=0/1.
pub fn normal_quantile(p: f64, ctx: &Interrupt) -> Result<f64, Error> {
    ctx.tick()?;
    if !(0. ..=1.).contains(&p) {
        return Err(Error::Input("概率须在0..1内"));
    }
    if p == 0. {
        return Ok(f64::NEG_INFINITY);
    }
    if p == 1. {
        return Ok(f64::INFINITY);
    }
    if p == 0.5 {
        return Ok(0.);
    }
    let tail = p.min(1. - p);
    let (mut lo, mut hi) = (0., 40.);
    for _ in 0..64 {
        ctx.tick()?;
        let middle = (lo + hi) * 0.5;
        if 0.5 * erfc(middle / std::f64::consts::SQRT_2, ctx)? > tail {
            lo = middle;
        } else {
            hi = middle;
        }
    }
    Ok(if p < 0.5 {
        -(lo + hi) * 0.5
    } else {
        (lo + hi) * 0.5
    })
}
