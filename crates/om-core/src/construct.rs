//! Derived constructors and iterative structural normalization.

use super::{
    add, mul,
    mul::phase,
    pow,
    special::{infinity_direction, literal_alias},
};
use crate::{BUILTIN as B, Expr, ExprKind, Normal, Symbol};

/// Negate with the canonical product constructor.
pub fn neg(x: Expr) -> Expr {
    mul([Expr::int(-1), x])
}
/// Subtract with the canonical sum and product constructors.
pub fn sub(a: Expr, b: Expr) -> Expr {
    add([a, neg(b)])
}
/// Divide using a principal inverse power.
pub fn div(a: Expr, b: Expr) -> Expr {
    mul([a, pow(b, Expr::int(-1))])
}
/// Construct a principal square root.
pub fn sqrt(x: Expr) -> Expr {
    pow(x, Expr::rational(1, 2))
}
/// Construct the exponential, including E^Log[z] cancellation.
pub fn exp(x: Expr) -> Expr {
    pow(Expr::sym(B::E), x)
}

/// Build from canonical arguments using minimal arithmetic and alias rewrites.
/// Exact elementary-function special values are evaluated by om-simplify.
pub fn func(head: Symbol, mut args: Vec<Expr>) -> Expr {
    match head {
        B::PLUS => add(args),
        B::TIMES => mul(args),
        B::POWER if args.len() == 2 => {
            let exponent = args.remove(1);
            pow(args.remove(0), exponent)
        }
        B::SQRT if args.len() == 1 => sqrt(args.remove(0)),
        B::EXP if args.len() == 1 => exp(args.remove(0)),
        B::DIRECTED_INFINITY if args.len() == 1 => {
            let direction = literal_alias(args.remove(0));
            if direction.is_zero() || direction.as_symbol() == Some(B::INDETERMINATE) {
                Expr::call(B::DIRECTED_INFINITY, [])
            } else if let Some(nested) = infinity_direction(&direction) {
                Expr::call(B::DIRECTED_INFINITY, nested)
            } else {
                Expr::call(B::DIRECTED_INFINITY, [phase(direction)])
            }
        }
        _ => Expr::call(head, args),
    }
}

enum Visit<'a> {
    Enter(&'a Expr),
    Build(&'a Normal),
}
/// Normalize every explicit head and argument from leaves to root.
/// The traversal uses an explicit stack rather than recursive calls.
pub fn canonicalize(e: &Expr) -> Expr {
    let mut stack = vec![Visit::Enter(e)];
    let mut values = Vec::new();
    while let Some(visit) = stack.pop() {
        match visit {
            Visit::Enter(e) => match e.kind() {
                ExprKind::Normal(n) => {
                    stack.push(Visit::Build(n));
                    stack.extend(n.args.iter().rev().map(Visit::Enter));
                    stack.push(Visit::Enter(&n.head));
                }
                ExprKind::Symbol(_) => values.push(literal_alias(e.clone())),
                _ => values.push(e.clone()),
            },
            Visit::Build(n) => {
                // Each visit leaves one value; a Build follows its head and all arguments.
                let start = values
                    .len()
                    .checked_sub(n.args.len())
                    .expect("invariant: visited arguments have values");
                let args = values.split_off(start);
                let head = values.pop().expect("invariant: visited head has a value");
                values.push(if let Some(symbol) = head.as_symbol() {
                    func(symbol, args)
                } else {
                    Expr::normal(head, args)
                });
            }
        }
    }
    values
        .pop()
        .expect("invariant: the root visit leaves one result")
}
