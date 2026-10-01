//! Exact algebra precedes interval tests; sampling establishes nonidentity, never identity.
pub(crate) mod radicals;
use crate::convert::{normalize, to_rational_function_with};
use om_core::{BUILTIN as B, Expr, ExprKind, Symbol};
use om_num::{
    BigFloat, Complex, Number, Rational,
    ctx::{Abort, Interrupt},
    rng::SplitMix64,
};
use std::collections::BTreeSet;

/// Reason an expression's exact zero status is not established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownReason {
    /// All prescribed numeric balls contain zero with sufficiently small radii.
    ProbablyZero,
    /// Unsupported, undefined or insufficiently narrow evaluations.
    NoInfo,
}
/// Exact zero/nonzero certificate or an explicitly inconclusive result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tri {
    /// An exact identity (or a numeric literal zero).
    Zero,
    /// A proven nonzero value or a function proven not identically zero.
    NonZero,
    /// No exact zero certificate is available.
    Unknown(UnknownReason),
}
/// Layered zero decision; interrupted/unsupported work yields Unknown(NoInfo).
pub fn is_zero(e: &Expr) -> Tri {
    is_zero_with(e, &Interrupt::default()).unwrap_or(Tri::Unknown(UnknownReason::NoInfo))
}
/// Interruptible L0 structural, L1 rational, L2 algebraic and L3 numeric decision.
/// Rational functions are interpreted formally; domain exclusions belong to solving.
pub fn is_zero_with(e: &Expr, ctx: &Interrupt) -> Result<Tri, Abort> {
    ctx.tick()?;
    let e = normalize(e, ctx)?;
    if let Some(n) = e.as_number() {
        return Ok(if n.is_zero() { Tri::Zero } else { Tri::NonZero });
    }
    if let Some(view) = to_rational_function_with(&e, &[], ctx)? {
        if view.num.is_zero() {
            return Ok(Tri::Zero);
        }
        let mut rational = true;
        for g in &view.gens {
            ctx.tick()?;
            rational &= g.as_symbol().is_some_and(|s| g.free_symbols().contains(&s));
        }
        if rational {
            return Ok(Tri::NonZero);
        }
    }
    let denested = radicals::denest(&e, ctx)?;
    if let Some(z) = radicals::relations(&denested, ctx)? {
        return Ok(z);
    }
    if let Some(value) = crate::root_reduce::evaluate(&denested, ctx)? {
        return Ok(if value.is_zero() {
            Tri::Zero
        } else {
            Tri::NonZero
        });
    }
    numerical(&denested, ctx)
}
pub(crate) fn numerical(e: &Expr, ctx: &Interrupt) -> Result<Tri, Abort> {
    numerical_assuming(e, &[], ctx)
}
pub(crate) fn numerical_assuming(
    e: &Expr,
    positive: &[Symbol],
    ctx: &Interrupt,
) -> Result<Tri, Abort> {
    let vars = free_symbols(e, ctx)?;
    if vars.is_empty() {
        return numeric_value(e, ctx);
    }
    let mut rng = SplitMix64::new(0x4f4d_5a45_524f);
    let mut probably = true;
    for _ in 0..3 {
        ctx.tick()?;
        let mut rules = vec![];
        for s in &vars {
            ctx.tick()?;
            let re = if positive.contains(s) {
                Rational::from(rng.next_range(1, 6))
            } else {
                Rational::from(rng.next_range(0, 6) as i64 - 3)
            } / Rational::from(rng.next_range(1, 5));
            let im = if positive.contains(s) {
                Rational::ZERO
            } else {
                Rational::from(rng.next_range(0, 6) as i64 - 3)
                    / Rational::from(rng.next_range(1, 5))
            };
            let z = Number::Complex(Box::new(Complex {
                re: Number::Rational(re),
                im: Number::Rational(im),
            }));
            rules.push((Expr::sym(*s), Expr::number(z)));
        }
        let sample = radicals::rewrite(e, ctx, |node| {
            rules
                .iter()
                .find(|(s, _)| s == node)
                .map_or_else(|| Ok(node.clone()), |(_, z)| Ok(z.clone()))
        })?;
        match numeric_value(&sample, ctx)? {
            Tri::NonZero => return Ok(Tri::NonZero),
            Tri::Unknown(UnknownReason::ProbablyZero) => {}
            _ => probably = false,
        }
    }
    Ok(Tri::Unknown(if probably {
        UnknownReason::ProbablyZero
    } else {
        UnknownReason::NoInfo
    }))
}
fn numeric_value(e: &Expr, ctx: &Interrupt) -> Result<Tri, Abort> {
    let mut probably = true;
    for bits in [64, 256, 1024] {
        ctx.tick()?;
        let Some(z) = crate::numeval::enclose(e, bits, ctx)? else {
            probably = false;
            continue;
        };
        if z.re.excludes_zero() || z.im.excludes_zero() {
            return Ok(Tri::NonZero);
        }
        let radius = BigFloat::from_parts(1.into(), 20 - bits as isize);
        probably &= z.re.rad < radius && z.im.rad < radius;
    }
    Ok(Tri::Unknown(if probably {
        UnknownReason::ProbablyZero
    } else {
        UnknownReason::NoInfo
    }))
}
fn free_symbols(e: &Expr, ctx: &Interrupt) -> Result<Vec<Symbol>, Abort> {
    let mut symbols = BTreeSet::new();
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        match e.kind() {
            ExprKind::Symbol(s) if e.free_symbols().contains(s) => {
                symbols.insert((s.name(), *s));
            }
            ExprKind::Normal(n) => {
                if e.is_head(B::ROOT) {
                    continue;
                }
                if matches!(n.head.kind(), ExprKind::Normal(_)) {
                    stack.push(&n.head);
                }
                stack.extend(n.args.iter());
            }
            _ => {}
        }
    }
    Ok(symbols.into_iter().map(|(_, s)| s).collect())
}
