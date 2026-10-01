//! Monotone real W0 inversion on the nonnegative axis, DLMF 4.13.
use super::{elementary, zero};
use om_core::{Abort, Interrupt};
use om_num::{Ball, BigFloat, CBall, Integer, Rational};

fn bounds(b: &Ball) -> Option<(Rational, Rational)> {
    // Exact endpoint conversion must not expand compact huge-exponent inputs.
    if [&b.mid, &b.rad]
        .iter()
        .any(|x| !x.repr().is_finite() || x.repr().exponent().unsigned_abs() > 32_768)
    {
        return None;
    }
    let m = Rational::try_from(b.mid.clone()).ok()?;
    let r = Rational::try_from(b.rad.clone()).ok()?;
    Some((&m - &r, m + r))
}
fn interval(lo: &Rational, hi: &Rational, bits: u32) -> CBall {
    let center = Ball::exact(&((lo + hi) / Rational::from(2)), bits);
    let radius = Ball::exact(&((hi - lo) / Rational::from(2)), bits);
    // A power-of-two enlargement avoids downward rounding of mid+rad.
    let radius = radius.mid.max(radius.rad) * 2_u8;
    elementary::real(center.add(&Ball {
        mid: BigFloat::ZERO,
        rad: radius,
        prec: bits,
    }))
}
pub(super) fn principal(z: &CBall, bits: u32, ctx: &Interrupt) -> Result<Option<CBall>, Abort> {
    if !zero(&z.im) {
        return Ok(None);
    }
    let Some((xlo, xhi)) = bounds(&z.re) else {
        return Ok(None);
    };
    if xlo < Rational::ZERO {
        return Ok(None);
    }
    if xhi == Rational::ZERO {
        return Ok(Some(elementary::integer(0, bits)));
    }
    let work = bits.saturating_add(32);
    let x = z.re.clone();
    let mut lo = Rational::ZERO;
    // W(x)<=x, and W(x)<=ln(1+x), since (1+x)ln(1+x)>=x.
    let mut hi = if xhi <= Rational::ONE {
        xhi
    } else {
        let b = Ball::exact(&(Rational::ONE + xhi), work).ln();
        let Some((_, upper)) = bounds(&b) else {
            return Ok(None);
        };
        upper
    };
    let tolerance = hi.clone().min(Rational::ONE)
        / Rational::from(Integer::ONE << bits.saturating_add(4) as usize);
    for _ in 0..bits.saturating_add(128) {
        ctx.tick()?;
        if &hi - &lo <= tolerance {
            return Ok(Some(interval(&lo, &hi, bits)));
        }
        let mid = (&lo + &hi) / Rational::from(2);
        let w = Ball::exact(&mid, work);
        let f = w.mul(&w.exp()).sub(&x);
        let Some((flo, fhi)) = bounds(&f) else {
            return Ok(None);
        };
        if fhi < Rational::ZERO {
            lo = mid;
        } else if flo > Rational::ZERO {
            hi = mid;
        } else {
            // f'(w)=exp(w)(1+w) is increasing and >=f'(lo).
            // The mean value theorem encloses every inverse of the input ball.
            let lower = Ball::exact(&lo, work);
            let derivative = lower
                .exp()
                .mul(&lower.add(&Ball::exact(&Rational::ONE, work)));
            let Some((elo, ehi)) = bounds(&f.div(&derivative)) else {
                return Ok(None);
            };
            let error = (-elo).max(ehi);
            lo = lo.max(&mid - &error);
            hi = hi.min(mid + error);
            return Ok(Some(interval(&lo, &hi, bits)));
        }
    }
    Ok(None)
}
