//! Analytic Jacobians cover branch-local elementary functions; unknown kernels fall back.
use crate::SolveError;
use om_core::{BUILTIN as B, Expr, add, div, func, mul, pow};
use om_num::ctx::Interrupt;
pub(super) fn derivative(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, SolveError> {
    fn walk(e: &Expr, x: &Expr, ctx: &Interrupt, depth: usize) -> Result<Option<Expr>, SolveError> {
        ctx.tick()?;
        if depth > 128 {
            return Ok(None);
        }
        if e == x {
            return Ok(Some(Expr::int(1)));
        }
        if e.free_of(x) {
            return Ok(Some(Expr::int(0)));
        }
        let mut d = vec![];
        for a in e.args() {
            let Some(value) = walk(a, x, ctx, depth + 1)? else {
                return Ok(None);
            };
            d.push(value);
        }
        Ok(Some(match e.head_symbol() {
            Some(B::PLUS) => add(d),
            Some(B::TIMES) => add(d.into_iter().enumerate().map(|(i, d)| {
                mul(std::iter::once(d).chain(
                    e.args()
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| i != *j)
                        .map(|(_, a)| a.clone()),
                ))
            })),
            Some(B::POWER) if e.args().len() == 2 => {
                let (u, v) = (e.args()[0].clone(), e.args()[1].clone());
                if d[1].is_zero() {
                    mul([
                        v.clone(),
                        pow(u, om_core::sub(v, Expr::int(1))),
                        d[0].clone(),
                    ])
                } else {
                    mul([
                        e.clone(),
                        add([
                            mul([d[1].clone(), func(B::LOG, vec![u.clone()])]),
                            mul([v, div(d[0].clone(), u)]),
                        ]),
                    ])
                }
            }
            Some(h) if e.args().len() == 1 => {
                let u = e.args()[0].clone();
                let outer = match h {
                    B::SIN => func(B::COS, vec![u]),
                    B::COS => om_core::neg(func(B::SIN, vec![u])),
                    B::TAN => pow(func(B::SEC, vec![u]), Expr::int(2)),
                    B::SINH => func(B::COSH, vec![u]),
                    B::COSH => func(B::SINH, vec![u]),
                    B::TANH => pow(func(B::COSH, vec![u]), Expr::int(-2)),
                    B::LOG => div(Expr::int(1), u),
                    B::EXP => func(B::EXP, vec![u]),
                    B::SQRT => div(Expr::int(1), mul([Expr::int(2), om_core::sqrt(u)])),
                    B::ARCSIN => div(
                        Expr::int(1),
                        om_core::sqrt(om_core::sub(Expr::int(1), pow(u, Expr::int(2)))),
                    ),
                    B::ARCCOS => om_core::neg(div(
                        Expr::int(1),
                        om_core::sqrt(om_core::sub(Expr::int(1), pow(u, Expr::int(2)))),
                    )),
                    B::ARCTAN => div(Expr::int(1), add([Expr::int(1), pow(u, Expr::int(2))])),
                    _ => return Ok(None),
                };
                mul([outer, d[0].clone()])
            }
            _ => return Ok(None),
        }))
    }
    walk(e, x, ctx, 0)
}
