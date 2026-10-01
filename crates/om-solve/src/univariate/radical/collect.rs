//! Common-base axes are discovered inside out; principal integer powers are preserved.
use super::super::extract;
use crate::SolveError;
use om_core::{BUILTIN as B, Expr, ExprKind, func, pow};
use om_num::{Integer, Number, Rational, ctx::Interrupt, gcd};
use om_simplify::convert::to_rational_function_with;
#[derive(Clone)]
pub(super) struct Radical {
    pub base: Expr,
    pub degree: u32,
}
impl Radical {
    pub fn expression(&self) -> Expr {
        pow(self.base.clone(), Expr::rational(1, i64::from(self.degree)))
    }
}
pub(super) fn discover(
    e: &Expr,
    x: &Expr,
    merge: bool,
    ctx: &Interrupt,
) -> Result<Vec<Radical>, SolveError> {
    let mut stack = vec![(e, false)];
    let mut found: Vec<Radical> = vec![];
    while let Some((e, visited)) = stack.pop() {
        ctx.tick()?;
        if !visited {
            stack.push((e, true));
            if let ExprKind::Normal(n) = e.kind() {
                stack.push((&n.head, false));
            }
            stack.extend(e.args().iter().rev().map(|e| (e, false)));
            continue;
        }
        if !e.is_head(B::POWER) || e.args().len() != 2 {
            continue;
        }
        let Some(q) = extract::exact(&e.args()[1]) else {
            continue;
        };
        if q.denominator() == &1_u8.into() || !extract::depends(&e.args()[0], x, ctx)? {
            continue;
        }
        let degree = u32::try_from(q.denominator())
            .map_err(|_| SolveError::Unsupported("radical denominator exceeds u32".into()))?;
        if let Some(r) = found
            .iter_mut()
            .find(|r| r.base == e.args()[0] && (merge || r.degree == degree))
        {
            let common = gcd(&Integer::from(r.degree), &Integer::from(degree));
            r.degree = u32::try_from(Integer::from(r.degree) / common * Integer::from(degree))
                .map_err(|_| {
                    SolveError::Unsupported("radical denominator lcm exceeds u32".into())
                })?;
        } else {
            found.push(Radical {
                base: e.args()[0].clone(),
                degree,
            });
        }
    }
    Ok(found)
}
pub(super) fn rational(e: &Expr, axes: &[Expr], ctx: &Interrupt) -> Result<bool, SolveError> {
    let Some(view) = to_rational_function_with(e, axes, ctx)? else {
        return Ok(false);
    };
    for g in &view.gens {
        ctx.tick()?;
        if axes.contains(g) {
            continue;
        }
        for axis in axes {
            if extract::depends(g, axis, ctx)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
pub(super) fn rewrite(
    e: &Expr,
    groups: &[Radical],
    axes: &[Expr],
    ctx: &Interrupt,
) -> Result<Expr, SolveError> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut stack = vec![Frame::Enter(e)];
    let mut values = vec![];
    while let Some(frame) = stack.pop() {
        ctx.tick()?;
        match frame {
            Frame::Enter(e) => {
                let replacement = if e.is_head(B::POWER) && e.args().len() == 2 {
                    if let Some(q) = extract::exact(&e.args()[1]) {
                        groups.iter().zip(axes).find_map(|(r, y)| {
                            if r.base != e.args()[0] {
                                return None;
                            }
                            let n = &q * Rational::from(r.degree);
                            (n.denominator() == &1_u8.into())
                                .then(|| pow(y.clone(), Expr::number(Number::Rational(n))))
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(replacement) = replacement {
                    values.push(replacement);
                    continue;
                }
                if let ExprKind::Normal(n) = e.kind() {
                    stack.push(Frame::Build(n));
                    stack.extend(n.args.iter().rev().map(Frame::Enter));
                    stack.push(Frame::Enter(&n.head));
                } else {
                    values.push(e.clone());
                }
            }
            Frame::Build(n) => {
                let args = values.split_off(values.len() - n.args.len());
                let head = values.pop().expect("invariant: visited expression head");
                values.push(if let Some(s) = head.as_symbol() {
                    func(s, args)
                } else {
                    Expr::normal(head, args)
                });
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: one rewritten radical expression"))
}
