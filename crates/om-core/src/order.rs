//! Structural total order; symbol IDs and hash values never determine ordering.

use crate::{BUILTIN as B, Expr, ExprKind};
use om_num::{Number, Precision, Real};
use std::cmp::Ordering;

/// Compare expressions in the canonical internal order from §8.1.1.
/// Numbers precede terms; terms compare factors from right to left.
pub fn canonical_cmp(a: &Expr, b: &Expr) -> Ordering {
    match (a.as_number(), b.as_number()) {
        (Some(a), Some(b)) => return number_cmp(a, b),
        (Some(_), None) => return Ordering::Less,
        (None, Some(_)) => return Ordering::Greater,
        _ => {}
    }
    let (af, ac) = factors(a);
    let (bf, bc) = factors(b);
    for (p, q) in af.iter().rev().zip(bf.iter().rev()) {
        let cmp = cmp_factor(p, q);
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    let one = Number::Integer(1.into());
    af.len()
        .cmp(&bf.len())
        .then_with(|| number_cmp(ac.unwrap_or(&one), bc.unwrap_or(&one)))
        .then_with(|| structural_cmp(a, b))
}

fn factors(e: &Expr) -> (&[Expr], Option<&Number>) {
    if e.is_head(B::TIMES) {
        let args = e.args();
        if let Some(n) = args.first().and_then(Expr::as_number) {
            (&args[1..], Some(n))
        } else {
            (args, None)
        }
    } else {
        (std::slice::from_ref(e), None)
    }
}

fn base_exp(e: &Expr) -> (&Expr, Option<&Expr>) {
    if e.is_head(B::POWER) && e.args().len() == 2 {
        (&e.args()[0], Some(&e.args()[1]))
    } else {
        (e, None)
    }
}

fn cmp_factor(a: &Expr, b: &Expr) -> Ordering {
    let (ab, ae) = base_exp(a);
    let (bb, be) = base_exp(b);
    cmp_atom(ab, bb).then_with(|| {
        let one = Expr::int(1);
        canonical_cmp(ae.unwrap_or(&one), be.unwrap_or(&one))
    })
}

fn kind_rank(e: &Expr) -> u8 {
    match e.kind() {
        ExprKind::Number(_) => 0,
        ExprKind::Symbol(_) => 1,
        ExprKind::String(_) => 2,
        ExprKind::Normal(_) => 3,
    }
}

fn cmp_atom(a: &Expr, b: &Expr) -> Ordering {
    kind_rank(a)
        .cmp(&kind_rank(b))
        .then_with(|| match (a.kind(), b.kind()) {
            (ExprKind::Number(a), ExprKind::Number(b)) => number_cmp(a, b),
            (ExprKind::Symbol(a), ExprKind::Symbol(b)) => name_cmp(a.name(), b.name()),
            (ExprKind::String(a), ExprKind::String(b)) => a.cmp(b),
            (ExprKind::Normal(a), ExprKind::Normal(b)) => canonical_cmp(&a.head, &b.head)
                .then_with(|| sequence_cmp(&a.args, &b.args, canonical_cmp)),
            _ => Ordering::Equal,
        })
}

fn name_cmp(a: &str, b: &str) -> Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a.cmp(b))
}

fn sequence_cmp(a: &[Expr], b: &[Expr], cmp: fn(&Expr, &Expr) -> Ordering) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let order = cmp(x, y);
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

fn structural_cmp(a: &Expr, b: &Expr) -> Ordering {
    kind_rank(a)
        .cmp(&kind_rank(b))
        .then_with(|| match (a.kind(), b.kind()) {
            (ExprKind::Number(a), ExprKind::Number(b)) => number_cmp(a, b),
            (ExprKind::Symbol(a), ExprKind::Symbol(b)) => name_cmp(a.name(), b.name()),
            (ExprKind::String(a), ExprKind::String(b)) => a.cmp(b),
            (ExprKind::Normal(a), ExprKind::Normal(b)) => structural_cmp(&a.head, &b.head)
                .then_with(|| sequence_cmp(&a.args, &b.args, structural_cmp)),
            _ => Ordering::Equal,
        })
}

fn components<'a>(n: &'a Number, zero: &'a Number) -> (&'a Number, &'a Number) {
    if let Number::Complex(c) = n {
        (&c.re, &c.im)
    } else {
        (n, zero)
    }
}

fn precision_key(n: &Number) -> (u8, u32) {
    match n.precision() {
        Precision::Exact => (0, 0),
        Precision::Machine => (1, 53),
        Precision::Bits(bits) => (1, bits),
    }
}

fn number_cmp(a: &Number, b: &Number) -> Ordering {
    let zero = Number::Integer(0.into());
    let (ar, ai) = components(a, &zero);
    let (br, bi) = components(b, &zero);
    // Normalized complex components are scalar and finite, as is the default zero.
    ar.cmp_real(br)
        .expect("invariant: complex real components are scalar")
        .then_with(|| {
            ai.cmp_real(bi)
                .expect("invariant: complex imaginary components are scalar")
        })
        .then_with(|| precision_key(a).cmp(&precision_key(b)))
        .then_with(|| number_repr_cmp(a, b))
}

fn number_rank(n: &Number) -> u8 {
    match n {
        Number::Integer(_) => 0,
        Number::Rational(_) => 1,
        Number::Real(Real::Machine(_)) => 2,
        Number::Real(Real::Big(_)) => 3,
        Number::Complex(_) => 4,
    }
}

fn number_repr_cmp(a: &Number, b: &Number) -> Ordering {
    number_rank(a)
        .cmp(&number_rank(b))
        .then_with(|| match (a, b) {
            (Number::Real(Real::Big(a)), Number::Real(Real::Big(b))) => a
                .repr()
                .significand()
                .cmp(b.repr().significand())
                .then_with(|| a.repr().exponent().cmp(&b.repr().exponent())),
            (Number::Complex(a), Number::Complex(b)) => {
                number_repr_cmp(&a.re, &b.re).then_with(|| number_repr_cmp(&a.im, &b.im))
            }
            _ => Ordering::Equal,
        })
}
