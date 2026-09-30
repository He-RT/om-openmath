//! Validated exact sign bisection of one simple root.
use super::{
    Poly, RootInterval, point, prepare,
    transform::{affine, count_up_to_two, sign_at},
};
use om_num::{
    BitTest, Integer, Rational,
    ctx::{Abort, Interrupt},
};

/// Refine a certified single simple real root until width <=2^-bits, or an exact root is found.
/// None rejects zero/repeated inputs, malformed endpoints, endpoint roots, incorrect exact
/// claims or a nonexact interval containing anything other than one root. All validation
/// and refinement is exact; a huge precision request does not allocate 2^bits upfront.
pub fn refine(
    f: &Poly,
    interval: &RootInterval,
    bits: u32,
    ctx: &Interrupt,
) -> Result<Option<RootInterval>, Abort> {
    let Some(f) = prepare(f, ctx)? else {
        return Ok(None);
    };
    if interval.exact {
        return Ok(
            (interval.lo == interval.hi && sign_at(&f, &interval.lo, ctx)? == 0)
                .then(|| interval.clone()),
        );
    }
    if interval.lo >= interval.hi
        || sign_at(&f, &interval.lo, ctx)? == 0
        || sign_at(&f, &interval.hi, ctx)? == 0
    {
        return Ok(None);
    }
    if count_up_to_two(affine(&f, &interval.lo, &interval.hi, ctx)?, ctx)? != 1 {
        return Ok(None);
    }
    let mut out = interval.clone();
    while !out.exact && too_wide(&(&out.hi - &out.lo), bits) {
        ctx.tick()?;
        out = bisect_known(&f, &out, ctx)?;
    }
    Ok(Some(out))
}
fn too_wide(width: &Rational, bits: u32) -> bool {
    let numerator = width.numerator();
    let denominator = Integer::from(width.denominator().clone());
    let Some(left_bits) = numerator.bit_len().checked_add(bits as usize) else {
        return true;
    };
    match left_bits.cmp(&denominator.bit_len()) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => (numerator << (bits as usize)) > denominator,
    }
}
pub(super) fn bisect_known(
    f: &Poly,
    interval: &RootInterval,
    ctx: &Interrupt,
) -> Result<RootInterval, Abort> {
    ctx.tick()?;
    let mid = (&interval.lo + &interval.hi) / Rational::from(2);
    let sign = sign_at(f, &mid, ctx)?;
    if sign == 0 {
        return Ok(point(mid));
    }
    let mut out = interval.clone();
    if sign == sign_at(f, &interval.lo, ctx)? {
        out.lo = mid;
    } else {
        out.hi = mid;
    }
    Ok(out)
}
