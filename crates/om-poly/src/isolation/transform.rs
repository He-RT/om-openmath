//! Integer affine transforms and exact rational signs; no floating-point root decisions.
use super::Poly;
use crate::UPoly;
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};

pub(super) fn sign_at(f: &Poly, x: &Rational, ctx: &Interrupt) -> Result<i8, Abort> {
    ctx.tick()?;
    let Some(n) = f.degree() else {
        return Ok(0);
    };
    let (num, den) = (x.numerator(), Integer::from(x.denominator().clone()));
    let mut value = f.coeffs[n].clone();
    let mut denominator = Integer::ONE;
    for c in f.coeffs[..n].iter().rev() {
        ctx.tick()?;
        denominator *= &den;
        value = value * num + c * &denominator;
    }
    Ok(if value < Integer::ZERO {
        -1
    } else if value > Integer::ZERO {
        1
    } else {
        0
    })
}
pub(super) fn scale_variable(f: &Poly, scale: &Integer, ctx: &Interrupt) -> Result<Poly, Abort> {
    let mut power = Integer::ONE;
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for c in &f.coeffs {
        ctx.tick()?;
        coefficients.push(c * &power);
        power *= scale;
    }
    Ok(Poly::new(coefficients))
}
fn shift_one(f: &Poly, ctx: &Interrupt) -> Result<Poly, Abort> {
    ctx.tick()?;
    let mut coefficients = f.coeffs.clone();
    let n = coefficients.len().saturating_sub(1);
    for start in (0..n).rev() {
        for j in start..n {
            ctx.tick()?;
            let next = coefficients[j + 1].clone();
            coefficients[j] += next;
        }
    }
    Ok(Poly::new(coefficients))
}
pub(super) fn unit_variations(f: &Poly, ctx: &Interrupt) -> Result<usize, Abort> {
    let mut coefficients = Vec::with_capacity(f.coeffs.len());
    for c in f.coeffs.iter().rev() {
        ctx.tick()?;
        coefficients.push(c.clone());
    }
    let transformed = shift_one(&Poly::new(coefficients), ctx)?;
    let mut previous = 0;
    let mut count = 0;
    for c in &transformed.coeffs {
        ctx.tick()?;
        let sign = if c < &Integer::ZERO {
            -1
        } else if c > &Integer::ZERO {
            1
        } else {
            continue;
        };
        if previous != 0 && previous != sign {
            count += 1;
        }
        previous = sign;
    }
    Ok(count)
}
pub(super) fn bisect_polynomial(f: &Poly, ctx: &Interrupt) -> Result<(Poly, Poly), Abort> {
    let n = f.degree().expect("invariant: nonzero VCA node");
    let mut coefficients = Vec::with_capacity(n + 1);
    for (i, c) in f.coeffs.iter().enumerate().take(n + 1) {
        ctx.tick()?;
        coefficients.push(c << (n - i));
    }
    let left = Poly::new(coefficients);
    let right = shift_one(&left, ctx)?.primitive_part(ctx)?;
    Ok((left.primitive_part(ctx)?, right))
}
pub(super) fn affine(
    f: &Poly,
    lo: &Rational,
    hi: &Rational,
    ctx: &Interrupt,
) -> Result<Poly, Abort> {
    ctx.tick()?;
    let width = hi - lo;
    // A common denominator suffices; a product avoids an additional integer GCD.
    let a = Integer::from(lo.denominator().clone());
    let b = Integer::from(width.denominator().clone());
    let denominator = &a * &b;
    let offset = lo.numerator() * &b;
    let scale = width.numerator() * &a;
    let linear = Poly::new(vec![offset, scale]);
    let n = f.degree().expect("invariant: nonzero affine input");
    let mut polynomial = UPoly::new(vec![f.coeffs[n].clone()]);
    let mut power = Integer::ONE;
    for c in f.coeffs[..n].iter().rev() {
        ctx.tick()?;
        polynomial = polynomial.mul(&linear, ctx)?;
        power *= &denominator;
        if polynomial.coeffs.is_empty() {
            polynomial.coeffs.push(Integer::ZERO);
        }
        polynomial.coeffs[0] += c * &power;
    }
    Poly::new(polynomial.coeffs).primitive_part(ctx)
}
pub(super) fn count_up_to_two(f: Poly, ctx: &Interrupt) -> Result<usize, Abort> {
    let mut stack = vec![f];
    let mut count = 0;
    while let Some(p) = stack.pop() {
        ctx.tick()?;
        match unit_variations(&p, ctx)? {
            0 => {}
            1 => count += 1,
            _ => {
                let (left, mut right) = bisect_polynomial(&p, ctx)?;
                if right.coeffs[0].is_zero() {
                    count += 1;
                    right = Poly::new(right.coeffs[1..].to_vec());
                }
                stack.push(right);
                stack.push(left);
            }
        }
        if count >= 2 {
            return Ok(2);
        }
    }
    Ok(count)
}
