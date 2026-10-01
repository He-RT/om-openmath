//! Arithmetic distribution keeps denominators and function arguments as atomic kernels.
use crate::convert::{integer_power, normalize};
use om_core::{BUILTIN as B, Expr, add, mul};
use om_num::ctx::{Abort, Interrupt};
pub(super) fn expand(e: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    let e = normalize(e, ctx)?;
    enum Visit<'a> {
        Enter(&'a Expr),
        Build(&'a Expr),
    }
    let mut stack = vec![Visit::Enter(&e)];
    let mut values = vec![];
    while let Some(visit) = stack.pop() {
        ctx.tick()?;
        match visit {
            Visit::Enter(e) => {
                if e.is_head(B::PLUS) || e.is_head(B::TIMES) {
                    stack.push(Visit::Build(e));
                    stack.extend(e.args().iter().rev().map(Visit::Enter));
                } else if integer_power(e).is_some_and(|n| n > 0.into()) {
                    stack.push(Visit::Build(e));
                    stack.push(Visit::Enter(&e.args()[0]));
                } else {
                    values.push(e.clone());
                }
            }
            Visit::Build(e) => {
                if let Some(n) = integer_power(e) {
                    let Some(mut n) = u32::try_from(&n).ok() else {
                        return Ok(None);
                    };
                    let mut base = values
                        .pop()
                        .expect("invariant: expanded positive-power base");
                    let mut result = Expr::int(1);
                    while n > 0 {
                        ctx.tick()?;
                        if n & 1 != 0 {
                            result = product(&result, &base, ctx)?;
                        }
                        n >>= 1;
                        if n > 0 {
                            base = product(&base, &base, ctx)?;
                        }
                    }
                    values.push(result);
                } else {
                    let args = values.split_off(values.len() - e.args().len());
                    if e.is_head(B::PLUS) {
                        values.push(add(args));
                    } else {
                        let mut value = Expr::int(1);
                        for arg in args {
                            ctx.tick()?;
                            value = product(&value, &arg, ctx)?;
                        }
                        values.push(value);
                    }
                }
            }
        }
    }
    Ok(Some(values.pop().expect("invariant: one expanded root")))
}
fn product(a: &Expr, b: &Expr, ctx: &Interrupt) -> Result<Expr, Abort> {
    let left = if a.is_head(B::PLUS) {
        a.args()
    } else {
        std::slice::from_ref(a)
    };
    let right = if b.is_head(B::PLUS) {
        b.args()
    } else {
        std::slice::from_ref(b)
    };
    let mut terms = vec![];
    for a in left {
        for b in right {
            ctx.tick()?;
            terms.push(mul([a.clone(), b.clone()]));
        }
    }
    Ok(add(terms))
}
