//! Branch predicates and exact inverse identities are shared by P3 verification.
use super::super::super::radical::verify;
use crate::SolveError;
use om_core::{BUILTIN as B, Expr, ExprKind, func, sub};
use om_num::{BigFloat, ctx::Interrupt};
use om_simplify::{numeval::enclose, zero::Tri};
pub(crate) fn residual(e: &Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut stack = vec![Frame::Enter(e)];
    let mut values = vec![];
    while let Some(f) = stack.pop() {
        ctx.tick()?;
        match f {
            Frame::Enter(e) => {
                if let ExprKind::Normal(n) = e.kind() {
                    stack.push(Frame::Build(n));
                    stack.extend(n.args.iter().rev().map(Frame::Enter));
                    stack.push(Frame::Enter(&n.head))
                } else {
                    values.push(e.clone())
                }
            }
            Frame::Build(n) => {
                let args = values.split_off(values.len() - n.args.len());
                let head = values
                    .pop()
                    .expect("invariant: visited Lambert expression head");
                let mut e = if let Some(s) = head.as_symbol() {
                    func(s, args)
                } else {
                    Expr::normal(head, args)
                };
                if e.is_head(B::POWER)
                    && e.args().len() == 2
                    && e.args()[0].as_symbol() == Some(B::E)
                    && e.args()[1].is_head(B::PLUS)
                {
                    e = om_core::mul(
                        e.args()[1]
                            .args()
                            .iter()
                            .map(|t| om_core::pow(Expr::sym(B::E), t.clone())),
                    );
                }
                if e.is_head(B::TIMES) {
                    let mut factors = e.args().to_vec();
                    let mut replacement = None;
                    for (i, w) in factors.iter().enumerate() {
                        if w.is_head(B::PRODUCT_LOG)
                            && matches!(w.args().len(), 1 | 2)
                            && let Some(j) = factors.iter().position(|p| {
                                p.is_head(B::POWER)
                                    && p.args().len() == 2
                                    && p.args()[0].as_symbol() == Some(B::E)
                                    && p.args()[1] == *w
                            })
                        {
                            replacement = Some((
                                i,
                                j,
                                w.args()
                                    .last()
                                    .expect("invariant: ProductLog argument")
                                    .clone(),
                            ));
                            break;
                        }
                    }
                    if let Some((i, j, v)) = replacement {
                        factors.remove(i.max(j));
                        factors.remove(i.min(j));
                        factors.push(v);
                        e = om_core::mul(factors)
                    }
                }
                values.push(om_simplify::special::eval(&e).unwrap_or(e));
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: one verified Lambert expression"))
}
pub(crate) fn nonzero(e: &Expr, ctx: &Interrupt) -> Result<bool, SolveError> {
    match verify::zero(e, ctx)? {
        Tri::Zero => Ok(false),
        Tri::NonZero => Ok(true),
        _ => {
            Ok(enclose(e, 512, ctx)?.is_some_and(|z| z.re.excludes_zero() || z.im.excludes_zero()))
        }
    }
}
pub(crate) fn allows(e: &Expr, ctx: &Interrupt) -> Result<Option<bool>, SolveError> {
    ctx.tick()?;
    if e.as_symbol() == Some(B::TRUE) {
        return Ok(Some(true));
    }
    if e.as_symbol() == Some(B::FALSE) {
        return Ok(Some(false));
    }
    if e.is_head(B::AND) {
        let mut unknown = false;
        for c in e.args() {
            match allows(c, ctx)? {
                Some(false) => return Ok(Some(false)),
                None => unknown = true,
                _ => {}
            }
        }
        return Ok((!unknown).then_some(true));
    }
    let [a, b] = e.args() else { return Ok(None) };
    if e.is_head(B::EQUAL) {
        let difference = residual(&sub(a.clone(), b.clone()), ctx)?;
        return Ok(match verify::zero(&difference, ctx)? {
            Tri::Zero => Some(true),
            Tri::NonZero => difference.free_symbols().is_empty().then_some(false),
            Tri::Unknown(_) => {
                if difference.free_symbols().is_empty() {
                    if verify::numeric(&difference, ctx)? {
                        Some(true)
                    } else {
                        enclose(&difference, 512, ctx)?.and_then(|z| {
                            (z.re.excludes_zero() || z.im.excludes_zero()).then_some(false)
                        })
                    }
                } else {
                    None
                }
            }
        });
    }
    if e.is_head(B::UNEQUAL) {
        let value = sub(a.clone(), b.clone());
        return Ok(match verify::zero(&value, ctx)? {
            Tri::Zero => Some(false),
            Tri::NonZero => value.free_symbols().is_empty().then_some(true),
            Tri::Unknown(_) => enclose(&value, 512, ctx)?
                .and_then(|z| (z.re.excludes_zero() || z.im.excludes_zero()).then_some(true)),
        });
    }
    if e.is_head(B::ELEMENT) && b.as_symbol() == Some(B::REALS) {
        return Ok(enclose(a, 256, ctx)?.and_then(|z| {
            if z.im.excludes_zero() {
                Some(false)
            } else if z.im.mid == BigFloat::ZERO && z.im.rad == BigFloat::ZERO {
                Some(true)
            } else {
                None
            }
        }));
    }
    if e.is_head(B::ELEMENT) && matches!(b.as_symbol(), Some(B::INTEGERS | B::RATIONALS)) {
        return Ok(match om_simplify::root_reduce::to_algebraic(a, ctx)? {
            Some(om_poly::Algebraic::Rational(q)) => {
                Some(b.as_symbol() == Some(B::RATIONALS) || q.denominator() == &1_u8.into())
            }
            Some(_) => Some(false),
            None => None,
        });
    }
    let Some(z) = enclose(&sub(a.clone(), b.clone()), 256, ctx)? else {
        return Ok(None);
    };
    if z.im.mid != BigFloat::ZERO || z.im.rad != BigFloat::ZERO {
        return Ok(None);
    }
    let positive = z.re.mid > z.re.rad;
    let negative = z.re.mid < -&z.re.rad;
    let zero = z.re.mid == BigFloat::ZERO && z.re.rad == BigFloat::ZERO;
    Ok(match e.head_symbol() {
        Some(B::LESS) => {
            if negative {
                Some(true)
            } else if positive || zero {
                Some(false)
            } else {
                None
            }
        }
        Some(B::LESS_EQUAL) => {
            if negative || zero {
                Some(true)
            } else if positive {
                Some(false)
            } else {
                None
            }
        }
        Some(B::GREATER) => {
            if positive {
                Some(true)
            } else if negative || zero {
                Some(false)
            } else {
                None
            }
        }
        Some(B::GREATER_EQUAL) => {
            if positive || zero {
                Some(true)
            } else if negative {
                Some(false)
            } else {
                None
            }
        }
        _ => None,
    })
}
