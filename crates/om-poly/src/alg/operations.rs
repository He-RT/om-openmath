//! RootReduce selects a factor/root only when interval exclusion leaves a unique candidate.
use super::{
    Algebraic, DEGREE_LIMIT, Poly,
    bounds::{BoxBounds, ball_bounds, product, real_bounds, refine_iv},
    factors, ordering, real_intersects, real_roots, resultant,
};
use om_num::{
    BitTest, CBall, Integer, Rational,
    ctx::{Abort, Interrupt},
};
#[derive(Clone, Copy)]
enum Op {
    Add,
    Mul,
    Neg,
    Recip,
}
impl Algebraic {
    /// Exact algebraic sum using a parameter resultant and certified root selection.
    pub fn add(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        binary(self, other, Op::Add, ctx)
    }
    /// Exact algebraic difference; None represents an unsupported result.
    pub fn sub(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        let Some(negative) = other.neg(ctx)? else {
            return Ok(None);
        };
        self.add(&negative, ctx)
    }
    /// Exact algebraic product using a parameter resultant and certified root selection.
    pub fn mul(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        binary(self, other, Op::Mul, ctx)
    }
    /// Exact algebraic quotient; zero division and unsupported results return None.
    pub fn div(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        let Some(inverse) = other.recip(ctx)? else {
            return Ok(None);
        };
        self.mul(&inverse, ctx)
    }
    /// Exact negation, with transformed annihilator and certified branch selection.
    pub fn neg(&self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        unary(self, Op::Neg, ctx)
    }
    /// Exact reciprocal, using the reversed annihilator; zero returns None.
    pub fn recip(&self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        unary(self, Op::Recip, ctx)
    }
    /// Signed integer power by bounded exact multiplication and root reduction.
    pub fn pow_int(&self, exponent: &Integer, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        if exponent.is_zero() {
            return Ok((!self.is_zero()).then(|| Self::Rational(Rational::ONE)));
        }
        let mut base = if exponent < &Integer::ZERO {
            let Some(a) = self.recip(ctx)? else {
                return Ok(None);
            };
            a
        } else {
            self.clone()
        };
        let exp = exponent.clone().into_parts().1;
        let mut result = Self::Rational(Rational::ONE);
        for bit in 0..exp.bit_len() {
            ctx.tick()?;
            if exp.bit(bit) {
                let Some(next) = result.mul(&base, ctx)? else {
                    return Ok(None);
                };
                result = next;
            }
            if bit + 1 < exp.bit_len() {
                let Some(next) = base.mul(&base, ctx)? else {
                    return Ok(None);
                };
                base = next;
            }
        }
        Ok(Some(result))
    }
}
fn binary(
    a: &Algebraic,
    b: &Algebraic,
    op: Op,
    ctx: &Interrupt,
) -> Result<Option<Algebraic>, Abort> {
    ctx.tick()?;
    if let (Algebraic::Rational(a), Algebraic::Rational(b)) = (a, b) {
        return Ok(Some(Algebraic::Rational(match op {
            Op::Add => a + b,
            Op::Mul => a * b,
            _ => unreachable!("invariant: binary operation"),
        })));
    }
    if matches!(op, Op::Mul) && (a.is_zero() || b.is_zero()) {
        return Ok(Some(Algebraic::Rational(Rational::ZERO)));
    }
    let (p, q) = (a.minimal_polynomial(ctx)?, b.minimal_polynomial(ctx)?);
    let (Some(n), Some(m)) = (p.degree(), q.degree()) else {
        return Ok(None);
    };
    if n.checked_mul(m).is_none_or(|d| d > DEGREE_LIMIT) {
        return Ok(None);
    }
    let f = resultant::annihilator(&p, &q, matches!(op, Op::Mul), ctx)?;
    select(&f, a, Some(b), op, ctx)
}
fn unary(a: &Algebraic, op: Op, ctx: &Interrupt) -> Result<Option<Algebraic>, Abort> {
    ctx.tick()?;
    if let Algebraic::Rational(q) = a {
        return Ok(match op {
            Op::Neg => Some(Algebraic::Rational(-q)),
            Op::Recip if q != &Rational::ZERO => Some(Algebraic::Rational(Rational::ONE / q)),
            _ => None,
        });
    }
    let mut p = a.minimal_polynomial(ctx)?;
    if p.degree().is_none_or(|n| n > DEGREE_LIMIT) {
        return Ok(None);
    }
    match op {
        Op::Neg => {
            for (i, c) in p.coeffs.iter_mut().enumerate() {
                ctx.tick()?;
                if i % 2 == 1 {
                    *c = -&*c;
                }
            }
        }
        Op::Recip => p.coeffs.reverse(),
        _ => unreachable!("invariant: unary operation"),
    }
    select(&p, a, None, op, ctx)
}
fn candidates(
    factors: &[Poly],
    real: bool,
    ctx: &Interrupt,
) -> Result<Option<Vec<Algebraic>>, Abort> {
    let mut out = vec![];
    for f in factors {
        ctx.tick()?;
        let Some(roots) = (if real {
            real_roots(f, ctx)?
        } else {
            ordering::roots(f, ctx)?
        }) else {
            return Ok(None);
        };
        out.extend(roots);
    }
    Ok(Some(out))
}
pub(super) fn select_real(
    factors: &[Poly],
    iv: (Rational, Rational),
    ctx: &Interrupt,
) -> Result<Option<Algebraic>, Abort> {
    let Some(mut roots) = candidates(factors, true, ctx)? else {
        return Ok(None);
    };
    let mut bits = 32;
    let mut target = iv;
    let mut f = Poly::one();
    for p in factors {
        ctx.tick()?;
        f = f.mul(p, ctx)?;
    }
    loop {
        ctx.tick()?;
        let mut found = None;
        let mut ambiguous = false;
        for root in &roots {
            ctx.tick()?;
            if real_intersects(root, &target) {
                if found.is_some() {
                    ambiguous = true;
                    break;
                }
                found = Some(root.clone());
            }
        }
        if !ambiguous {
            return Ok(found);
        }
        let Some(iv) = refine_iv(&f, &target, bits, ctx)? else {
            return Ok(None);
        };
        target = iv;
        for root in &mut roots {
            ctx.tick()?;
            let Some(next) = root.refined(bits, ctx)? else {
                return Ok(None);
            };
            *root = next;
        }
        let Some(next) = super::next_precision(bits, false) else {
            return Ok(None);
        };
        bits = next;
    }
}
fn select(
    f: &Poly,
    a: &Algebraic,
    b: Option<&Algebraic>,
    op: Op,
    ctx: &Interrupt,
) -> Result<Option<Algebraic>, Abort> {
    let real = real_bounds(a).is_some() && b.is_none_or(|b| real_bounds(b).is_some());
    let Some(factors) = factors(f, ctx)? else {
        return Ok(None);
    };
    let Some(mut roots) = candidates(&factors, real, ctx)? else {
        return Ok(None);
    };
    let mut a = a.clone();
    let mut b = b.cloned();
    let mut bits = 32;
    loop {
        ctx.tick()?;
        let Some(next) = a.refined(bits, ctx)? else {
            return Ok(None);
        };
        a = next;
        if let Some(current) = &mut b {
            let Some(next) = current.refined(bits, ctx)? else {
                return Ok(None);
            };
            *current = next;
        }
        let target = operation_bounds(&a, b.as_ref(), op, bits);
        if let Some(target) = target {
            let mut found = None;
            let mut ambiguous = false;
            for root in &mut roots {
                ctx.tick()?;
                let Some(next) = root.refined(bits, ctx)? else {
                    return Ok(None);
                };
                *root = next;
                let rb = if let Some(iv) = real_bounds(root) {
                    Some(BoxBounds {
                        re: iv,
                        im: (Rational::ZERO, Rational::ZERO),
                    })
                } else {
                    ball_bounds(&root.current_ball(bits))
                };
                if rb.is_some_and(|bounds| target.intersects(&bounds)) {
                    if found.is_some() {
                        ambiguous = true;
                        break;
                    }
                    found = Some(root.clone());
                }
            }
            if !ambiguous && found.is_some() {
                return Ok(found);
            }
        }
        let Some(next) = super::next_precision(bits, !real) else {
            return Ok(None);
        };
        bits = next;
    }
}
fn operation_bounds(a: &Algebraic, b: Option<&Algebraic>, op: Op, bits: u32) -> Option<BoxBounds> {
    if let Some(av) = real_bounds(a)
        && b.is_none_or(|b| real_bounds(b).is_some())
    {
        let bv = b.and_then(real_bounds);
        let re = match op {
            Op::Add => {
                let b = bv.expect("invariant: addition operand");
                (&av.0 + &b.0, &av.1 + &b.1)
            }
            Op::Mul => product(&av, &bv.expect("invariant: multiplication operand")),
            Op::Neg => (-av.1, -av.0),
            Op::Recip => {
                if av.0 <= Rational::ZERO && av.1 >= Rational::ZERO {
                    return None;
                }
                (Rational::ONE / av.1, Rational::ONE / av.0)
            }
        };
        return Some(BoxBounds {
            re,
            im: (Rational::ZERO, Rational::ZERO),
        });
    }
    let ab = a.current_ball(bits);
    let bb = b.map(|b| b.current_ball(bits));
    let zero = CBall::exact(&Rational::ZERO, &Rational::ZERO, bits);
    let one = CBall::exact(&Rational::ONE, &Rational::ZERO, bits);
    let value = match op {
        Op::Add => ab.add(&bb.expect("invariant: addition operand")),
        Op::Mul => ab.mul(&bb.expect("invariant: multiplication operand")),
        Op::Neg => zero.sub(&ab),
        Op::Recip => one.div(&ab),
    };
    ball_bounds(&value)
}
