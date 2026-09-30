//! Relations retain unknown symbolic comparisons and never coerce exact numbers.

use om_core::{BUILTIN as B, Expr, Symbol};
use om_num::Number;
use std::cmp::Ordering;

pub(crate) fn eval(head: Symbol, args: &[Expr]) -> Option<Expr> {
    if head.name() == "SameQ" {
        return Some(crate::scalar::boolean(
            args.windows(2).all(|w| w[0] == w[1]),
        ));
    }
    if head == B::NOT {
        return match args[0].as_symbol() {
            Some(B::TRUE) => Some(Expr::sym(B::FALSE)),
            Some(B::FALSE) => Some(Expr::sym(B::TRUE)),
            _ => None,
        };
    }
    if matches!(head, B::AND | B::OR) {
        let decisive = if head == B::AND { B::FALSE } else { B::TRUE };
        let identity = if head == B::AND { B::TRUE } else { B::FALSE };
        if args.iter().any(|e| e.as_symbol() == Some(decisive)) {
            return Some(Expr::sym(decisive));
        }
        let rest: Vec<_> = args
            .iter()
            .filter(|e| e.as_symbol() != Some(identity))
            .cloned()
            .collect();
        return Some(match rest.len() {
            0 => Expr::sym(identity),
            1 => rest[0].clone(),
            _ => Expr::call(head, rest),
        });
    }
    if head == B::INEQUALITY {
        if args.len().is_multiple_of(2) {
            return None;
        }
        let mut comparisons = vec![];
        for i in (0..args.len() - 2).step_by(2) {
            let op = args[i + 1].as_symbol()?;
            if !matches!(
                op,
                B::EQUAL | B::UNEQUAL | B::LESS | B::LESS_EQUAL | B::GREATER | B::GREATER_EQUAL
            ) {
                return None;
            }
            comparisons.push(Expr::call(op, [args[i].clone(), args[i + 2].clone()]));
        }
        return Some(Expr::call(B::AND, comparisons));
    }
    let mut unknown = false;
    if head == B::UNEQUAL {
        for i in 0..args.len() {
            for j in i + 1..args.len() {
                match equal(&args[i], &args[j]) {
                    Some(true) => return Some(crate::scalar::boolean(false)),
                    Some(false) => {}
                    None => unknown = true,
                }
            }
        }
    } else {
        for pair in args.windows(2) {
            let value = if head == B::EQUAL {
                equal(&pair[0], &pair[1])
            } else {
                pair[0]
                    .as_number()
                    .zip(pair[1].as_number())
                    .and_then(|(a, b)| a.cmp_real(b))
                    .map(|ord| match head {
                        B::LESS => ord == Ordering::Less,
                        B::LESS_EQUAL => ord != Ordering::Greater,
                        B::GREATER => ord == Ordering::Greater,
                        _ => ord != Ordering::Less,
                    })
            };
            match value {
                Some(false) => return Some(crate::scalar::boolean(false)),
                Some(true) => {}
                None => unknown = true,
            }
        }
    }
    (!unknown).then(|| crate::scalar::boolean(true))
}
fn equal(a: &Expr, b: &Expr) -> Option<bool> {
    if a == b {
        return Some(true);
    }
    if let Some((a, b)) = a.as_number().zip(b.as_number()) {
        let zero = Number::Integer(0.into());
        let components = |n: &Number| match n {
            Number::Complex(c) => (c.re.clone(), c.im.clone()),
            _ => (n.clone(), zero.clone()),
        };
        let (ar, ai) = components(a);
        let (br, bi) = components(b);
        return Some(
            ar.cmp_real(&br) == Some(Ordering::Equal) && ai.cmp_real(&bi) == Some(Ordering::Equal),
        );
    }
    if let (om_core::ExprKind::String(a), om_core::ExprKind::String(b)) = (a.kind(), b.kind()) {
        return Some(a == b);
    }
    None
}
