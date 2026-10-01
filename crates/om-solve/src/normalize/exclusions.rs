//! Scan raw syntax completely before normalizing a base or residual.
use crate::ExclReason;
use om_core::{BUILTIN as B, Expr};
use om_num::{
    Number,
    ctx::{Abort, Interrupt},
};
use om_simplify::convert::canonicalize_with;
pub(super) fn collect(e: &Expr, ctx: &Interrupt) -> Result<Vec<(Expr, ExclReason)>, Abort> {
    let mut found = vec![];
    let mut powers = vec![];
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if e.is_head(B::POWER) && e.args().len() == 2 {
            powers.push((e.args()[0].clone(), e.args()[1].clone(), found.len()));
        }
        if e.is_head(B::LOG) {
            for arg in e.args() {
                found.push((arg.clone(), ExclReason::UndefinedFunction));
            }
            if e.args().len() == 2 {
                found.push((
                    Expr::call(B::LOG, [e.args()[0].clone()]),
                    ExclReason::UndefinedFunction,
                ));
            }
        }
        if e.args().len() == 1 {
            let head = match e.head_symbol() {
                Some(B::TAN | B::SEC) => Some(B::COS),
                Some(B::COT | B::CSC) => Some(B::SIN),
                _ => None,
            };
            if let Some(h) = head {
                found.push((
                    Expr::call(h, [e.args()[0].clone()]),
                    ExclReason::UndefinedFunction,
                ));
            }
        }
        stack.extend(e.args().iter().rev());
    }
    // Exponents are interpreted only after the full original tree has been scanned.
    // Insert by source position so negative powers retain their original priority.
    let mut offset = 0;
    for (base, exp, position) in powers {
        ctx.tick()?;
        let exp = canonicalize_with(&exp, ctx)?;
        let negative = match exp.as_number() {
            Some(Number::Complex(z)) => z.re.is_negative(),
            Some(n) => n.is_negative(),
            None => {
                let mut negative = false;
                for bits in [64, 256, 1024] {
                    ctx.tick()?;
                    let Some(z) = om_simplify::numeval::enclose(&exp, bits, ctx)? else {
                        break;
                    };
                    if -&z.re.mid > z.re.rad {
                        negative = true;
                        break;
                    }
                    if z.re.mid > z.re.rad {
                        break;
                    }
                }
                negative
            }
        };
        if negative {
            found.insert(position + offset, (base, ExclReason::ZeroDenominator));
            offset += 1;
        }
    }
    Ok(found)
}
