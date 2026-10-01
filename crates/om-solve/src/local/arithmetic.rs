//! Small complex linear systems and merit comparisons retain working precision.
use crate::SolveError;
use om_core::Expr;
use om_num::{BigFloat, Number, Real, ctx::Interrupt};
use std::cmp::Ordering;
pub(super) fn sub(a: &Number, b: &Number) -> Number {
    a.add(&b.neg())
}
pub(super) fn div(a: &Number, b: &Number) -> Result<Number, SolveError> {
    Ok(a.mul(
        &b.recip()
            .map_err(|_| SolveError::Unsupported("singular local numerical divisor".into()))?,
    ))
}
pub(super) fn norm(v: &Number) -> Number {
    let (re, im) = match v {
        Number::Complex(c) => (&c.re, c.im.clone()),
        v => (v, Number::Integer(0.into())),
    };
    re.mul(re).add(&im.mul(&im))
}
pub(super) fn merit(v: &[Number]) -> Number {
    v.iter()
        .fold(Number::Integer(0.into()), |a, v| a.add(&norm(v)))
}
pub(super) fn less(a: &Number, b: &Number) -> bool {
    a.cmp_real(b) == Some(Ordering::Less)
}
pub(super) fn power(exponent: i64, bits: u32) -> Number {
    Number::Real(Real::Big(
        BigFloat::from_parts(1.into(), exponent as isize)
            .with_precision(bits as usize)
            .value(),
    ))
}
pub(super) fn eval(
    e: &Expr,
    vars: &[Expr],
    values: &[Number],
    bits: u32,
    ctx: &Interrupt,
) -> Result<Number, SolveError> {
    let rules = vars
        .iter()
        .cloned()
        .zip(values.iter().cloned().map(Expr::number))
        .collect::<Vec<_>>();
    om_simplify::numeval::evaluate(&e.replace_all(&rules), bits, ctx)?
        .ok_or_else(|| SolveError::Unsupported("local numerical evaluation unavailable".into()))
}
pub(super) fn linear(
    mut a: Vec<Vec<Number>>,
    mut b: Vec<Number>,
    ctx: &Interrupt,
) -> Result<Vec<Number>, SolveError> {
    let n = b.len();
    for k in 0..n {
        ctx.tick()?;
        let mut pivot = k;
        for i in k + 1..n {
            if less(&norm(&a[pivot][k]), &norm(&a[i][k])) {
                pivot = i;
            }
        }
        if a[pivot][k].is_zero() {
            return Err(SolveError::Unsupported("singular local Jacobian".into()));
        }
        a.swap(k, pivot);
        b.swap(k, pivot);
        let source = a[k].clone();
        for i in k + 1..n {
            let factor = div(&a[i][k], &a[k][k])?;
            for (value, source) in a[i].iter_mut().zip(&source).skip(k + 1) {
                ctx.tick()?;
                *value = sub(value, &factor.mul(source));
            }
            a[i][k] = Number::Integer(0.into());
            b[i] = sub(&b[i], &factor.mul(&b[k]));
        }
    }
    let mut x = vec![Number::Integer(0.into()); n];
    for i in (0..n).rev() {
        let mut value = b[i].clone();
        for (j, x) in x.iter().enumerate().skip(i + 1) {
            ctx.tick()?;
            value = sub(&value, &a[i][j].mul(x));
        }
        x[i] = div(&value, &a[i][i])?;
    }
    Ok(x)
}
