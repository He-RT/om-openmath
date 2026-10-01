//! Point multiplicity is the original primary quotient length per conjugate root.
use super::convert::{self, Poly};
use crate::{SolveError, StepSink};
use om_core::Expr;
use om_num::{Rational, ctx::Interrupt};
use om_poly::{MonoOrder, UPoly};

pub(super) fn power(
    p: &UPoly<Rational>,
    mut n: usize,
    ctx: &Interrupt,
) -> Result<UPoly<Rational>, SolveError> {
    let mut base = p.clone();
    let mut value = UPoly::one();
    while n != 0 {
        ctx.tick()?;
        if n & 1 != 0 {
            value = value.mul(&base, ctx)?
        }
        n >>= 1;
        if n != 0 {
            base = base.mul(&base, ctx)?
        }
    }
    Ok(value)
}
pub(super) fn length(
    basis: &[Poly],
    n: usize,
    ctx: &Interrupt,
) -> Result<Option<usize>, SolveError> {
    if om_poly::is_zero_dimensional(basis, n, ctx)? != Some(true) {
        return Ok(None);
    }
    if basis
        .iter()
        .any(|p| p.terms.first().is_some_and(|(m, _)| m.deg == 0))
    {
        return Ok(Some(0));
    }
    let mut bounds = vec![];
    for i in 0..n {
        let bound = basis
            .iter()
            .filter_map(|p| p.terms.first().map(|(m, _)| m))
            .filter(|m| m.exps[i] > 0 && m.exps.iter().enumerate().all(|(j, e)| j == i || *e == 0))
            .map(|m| m.exps[i])
            .min();
        let Some(bound) = bound else { return Ok(None) };
        bounds.push(bound)
    }
    let mut exps = vec![0; n];
    let mut count = 0usize;
    loop {
        ctx.tick()?;
        if !basis.iter().any(|p| {
            p.terms
                .first()
                .is_some_and(|(m, _)| m.exps.iter().zip(&exps).all(|(a, b)| a <= b))
        }) {
            count = count.checked_add(1).ok_or_else(|| {
                SolveError::Unsupported("primary quotient length overflow".into())
            })?;
            if count > 65536 {
                return Ok(None);
            }
        }
        let mut i = n;
        while i > 0 {
            i -= 1;
            exps[i] += 1;
            if exps[i] < bounds[i] {
                break;
            }
            exps[i] = 0
        }
        if n == 0 || i == 0 && exps[0] == 0 {
            break;
        }
    }
    Ok(Some(count))
}
pub(super) fn component(
    original: &[Poly],
    f: &UPoly<Rational>,
    axis: usize,
    exponent: usize,
    axes: &[Expr],
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<(Vec<Poly>, usize)>, SolveError> {
    let mut inputs = original.to_vec();
    inputs.push(convert::embed(
        &power(f, exponent, ctx)?,
        axis,
        axes.len(),
        ctx,
    )?);
    let Some(primary) = convert::basis(&inputs, axes, MonoOrder::Lex, ctx, sink)? else {
        return Ok(None);
    };
    let Some(length) = length(&primary, axes.len(), ctx)? else {
        return Ok(None);
    };
    Ok(Some((primary, length)))
}
pub(super) fn per_root(length: usize, degree: usize) -> Result<u32, SolveError> {
    if degree == 0 || length == 0 || !length.is_multiple_of(degree) {
        return Err(SolveError::Unsupported(
            "nonintegral primary multiplicity certificate".into(),
        ));
    }
    u32::try_from(length / degree)
        .map_err(|_| SolveError::Unsupported("point multiplicity exceeds u32".into()))
}
