//! N's precision parsing and iterative approximation of symbolic subtrees.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel};
use om_num::{Number, Precision};
use std::collections::BTreeMap;

pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    specs.insert(
        "N",
        BuiltinSpec {
            symbol: B::N,
            f: n,
            attrs: A::PROTECTED,
            arity: Arity::Range(1, 2),
            doc: DocEntry {
                name: "N",
                modern: "N(expr, digits)",
                wolfram: "N[expr, digits]",
                summary_zh: "按机器精度或指定十进制位数数值求值。",
                summary_en: "Approximate at machine precision or a requested decimal precision.",
                examples: &["N[1/3]", "N[Pi,50]"],
                category: "Mathematics",
            },
        },
    );
}
fn n(ev: &mut Evaluator, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let precision = if args.len() == 1 {
        Precision::Machine
    } else {
        let digits = if let Some(Number::Integer(d)) = args[1].as_number() {
            u32::try_from(d).ok()
        } else {
            None
        };
        let bits = digits
            .filter(|d| *d > 0 && *d <= 4932)
            .map(|d| (d as f64 * std::f64::consts::LOG2_10).ceil() as u32);
        let Some(bits) = bits.filter(|b| *b <= 16_384) else {
            ev.message(
                "N",
                "precbd",
                "Requested precision must be a positive integer of at most 4932 decimal digits."
                    .into(),
                MsgLevel::Warning,
            );
            return Ok(None);
        };
        Precision::Bits(bits)
    };
    enum Frame<'a> {
        Visit(&'a Expr),
        Rebuild(&'a Expr),
        Keep(&'a Expr),
    }
    let mut frames = vec![Frame::Visit(&args[0])];
    let mut values = Vec::new();
    while let Some(frame) = frames.pop() {
        ctx.tick()?;
        match frame {
            Frame::Visit(e) => {
                if let Some(n) = om_simplify::numeval::approximate(e, precision, ctx)? {
                    values.push(Expr::number(n));
                } else if matches!(e.kind(), ExprKind::Normal(_))
                    && !e
                        .head_symbol()
                        .is_some_and(|h| ev.attributes(h).contains(A::HOLD_ALL))
                {
                    frames.push(Frame::Rebuild(e));
                    frames.extend(e.args().iter().rev().enumerate().map(|(reverse, arg)| {
                        // The engine's holding contract also applies inside a symbolic N tree.
                        let index = e.args().len() - reverse - 1;
                        let attrs = e
                            .head_symbol()
                            .map(|h| ev.attributes(h))
                            .unwrap_or_default();
                        if (index == 0 && attrs.contains(A::HOLD_FIRST))
                            || (index > 0 && attrs.contains(A::HOLD_REST))
                        {
                            Frame::Keep(arg)
                        } else {
                            Frame::Visit(arg)
                        }
                    }));
                } else {
                    values.push(e.clone());
                }
            }
            Frame::Rebuild(e) => {
                let start = values.len() - e.args().len();
                let args = values.split_off(start);
                values.push(if let Some(h) = e.head_symbol() {
                    om_core::func(h, args)
                } else {
                    Expr::normal(e.head(), args)
                });
            }
            Frame::Keep(e) => values.push(e.clone()),
        }
    }
    Ok(values.pop())
}
