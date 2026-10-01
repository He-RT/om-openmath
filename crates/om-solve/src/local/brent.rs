//! Safeguarded inverse quadratic interpolation retains an opposite-sign real bracket.
use super::arithmetic as n;
use crate::{FindRootOptions, SolveError, StepSink};
use om_core::Expr;
use om_num::{Number, ctx::Interrupt};
fn half(v: &Number) -> Number {
    v.mul(&Number::Rational(
        om_num::Rational::from(1) / om_num::Rational::from(2),
    ))
}
pub(super) fn iterate(
    e: &Expr,
    x: &Expr,
    ends: &(Number, Number),
    opts: &FindRootOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Number>, SolveError> {
    let target = crate::numeric::bits(opts.precision)?;
    let bits = target + 32;
    let mut a = crate::numeric::rounded(&ends.0, om_num::Precision::Bits(bits), ctx)?;
    let mut b = crate::numeric::rounded(&ends.1, om_num::Precision::Bits(bits), ctx)?;
    if !n::less(&a, &b) {
        return Err(SolveError::Invalid(
            "Brent requires ordered real endpoints".into(),
        ));
    }
    let eval = |v: &Number| {
        let r = n::eval(
            e,
            std::slice::from_ref(x),
            std::slice::from_ref(v),
            bits,
            ctx,
        )?;
        if r.cmp_real(&Number::Integer(0.into())).is_none() {
            return Err(SolveError::Unsupported(
                "Brent requires real residuals".into(),
            ));
        }
        Ok(r)
    };
    let (mut fa, mut fb) = (eval(&a)?, eval(&b)?);
    if fa.is_zero() {
        return Ok(vec![a]);
    }
    if fb.is_zero() {
        return Ok(vec![b]);
    }
    if fa.is_negative() == fb.is_negative() {
        return Err(SolveError::Unsupported(
            "Brent endpoints do not bracket a root".into(),
        ));
    }
    if n::less(&n::norm(&fa), &n::norm(&fb)) {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut fa, &mut fb);
    }
    let (mut c, mut fc) = (a.clone(), fa.clone());
    let mut d = c.clone();
    let mut bisected = true;
    let tolerance = n::norm(&n::power(-i64::from(target + 8), bits));
    for _ in 0..opts.max_iterations {
        ctx.tick()?;
        if fb.is_zero() || n::less(&n::norm(&n::sub(&b, &a)), &tolerance) {
            return Ok(vec![b]);
        }
        let mut s = if fa != fc && fb != fc {
            let aa = n::div(
                &a.mul(&fb).mul(&fc),
                &n::sub(&fa, &fb).mul(&n::sub(&fa, &fc)),
            )?;
            let bb = n::div(
                &b.mul(&fa).mul(&fc),
                &n::sub(&fb, &fa).mul(&n::sub(&fb, &fc)),
            )?;
            let cc = n::div(
                &c.mul(&fa).mul(&fb),
                &n::sub(&fc, &fa).mul(&n::sub(&fc, &fb)),
            )?;
            aa.add(&bb).add(&cc)
        } else {
            n::sub(&b, &n::div(&fb.mul(&n::sub(&b, &a)), &n::sub(&fb, &fa))?)
        };
        let edge = half(&half(&a.mul(&Number::Integer(3.into())).add(&b)));
        let inside = if n::less(&edge, &b) {
            n::less(&edge, &s) && n::less(&s, &b)
        } else {
            n::less(&b, &s) && n::less(&s, &edge)
        };
        let previous = if bisected {
            n::sub(&b, &c)
        } else {
            n::sub(&c, &d)
        };
        let too_large = !n::less(&n::norm(&n::sub(&s, &b)), &n::norm(&half(&previous)));
        if !inside || too_large || n::less(&n::norm(&previous), &tolerance) {
            s = half(&a.add(&b));
            bisected = true;
        } else {
            bisected = false;
        }
        let fs = eval(&s)?;
        super::record(
            std::slice::from_ref(e),
            std::slice::from_ref(x),
            std::slice::from_ref(&b),
            std::slice::from_ref(&s),
            std::slice::from_ref(&fs),
            sink,
        );
        if fs.is_zero() {
            return Ok(vec![s]);
        }
        d = c;
        c = b.clone();
        fc = fb.clone();
        if fa.is_negative() != fs.is_negative() {
            b = s;
            fb = fs;
        } else {
            a = s;
            fa = fs;
        }
        if n::less(&n::norm(&fa), &n::norm(&fb)) {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut fa, &mut fb);
        }
    }
    Err(SolveError::Unsupported(
        "Brent iteration limit reached".into(),
    ))
}
