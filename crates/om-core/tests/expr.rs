//! Structural expression contracts and traversal semantics.

use om_core::{BUILTIN as B, Expr, ExprKind, Symbol};
use om_num::{BigFloat, Complex, Integer, Number, Real};
use rustc_hash::FxHasher;
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
};

fn hash(e: &Expr) -> u64 {
    let mut h = FxHasher::default();
    e.hash(&mut h);
    h.finish()
}

#[test]
fn numeric_structure_normalizes_and_preserves_variants_and_precision() {
    assert_eq!(Expr::int(1), Expr::int(1));
    assert_ne!(Expr::real(1.0), Expr::int(1));
    assert_eq!(Expr::rational(2, 4), Expr::rational(-1, -2));
    assert_eq!(Expr::rational(6, 3), Expr::int(2));
    assert_eq!(Expr::integer(Integer::from(9)), Expr::int(9));
    assert_eq!(Expr::real(0.0), Expr::real(-0.0));
    assert_eq!(hash(&Expr::real(0.0)), hash(&Expr::real(-0.0)));
    let big = |bits| {
        Expr::number(Number::Real(Real::Big(
            BigFloat::ONE.with_precision(bits).value(),
        )))
    };
    assert_eq!(big(64), big(64));
    assert_eq!(hash(&big(64)), hash(&big(64)));
    assert_ne!(big(64), big(128));
    assert_ne!(big(53), Expr::real(1.0));
    assert!(Expr::rational(0, 2).is_zero());
    assert!(Expr::real(1.0).is_one());
    assert!(!Expr::symbol("zero").is_zero());
    assert!(!Expr::symbol("one").is_one());
}

#[test]
#[should_panic]
fn nonfinite_real_is_rejected() {
    Expr::real(f64::NAN);
}

#[test]
#[should_panic]
fn zero_denominator_is_rejected() {
    Expr::rational(1, 0);
}

#[test]
fn atom_accessors_report_type_heads_and_empty_arguments() {
    for (e, head) in [
        (Expr::int(2), B::INTEGER),
        (Expr::rational(1, 2), B::RATIONAL),
        (Expr::real(2.0), B::REAL),
        (
            Expr::number(Number::Complex(Box::new(Complex {
                re: Number::Integer(Integer::ONE),
                im: Number::Integer(Integer::ONE),
            }))),
            B::COMPLEX,
        ),
        (Expr::symbol("expr_x"), B::SYMBOL),
        (Expr::string("hello"), B::STRING),
    ] {
        assert_eq!(e.head(), Expr::sym(head));
        assert_eq!(e.head_symbol(), None);
        assert!(!e.is_head(head));
        assert!(e.args().is_empty());
    }
    assert!(matches!(Expr::int(2).kind(), ExprKind::Number(_)));
    assert_eq!(
        Expr::int(2).as_number(),
        Some(&Number::Integer(Integer::from(2)))
    );
    assert_eq!(
        Expr::symbol("expr_x").as_symbol(),
        Some(Symbol::intern("expr_x"))
    );
    assert_eq!(Expr::string("expr_x").as_symbol(), None);
    assert_eq!(Expr::string("expr_x").as_number(), None);
}

#[test]
fn raw_normal_construction_keeps_order_and_nesting() {
    let inner = Expr::call(B::PLUS, [Expr::int(0), Expr::symbol("x")]);
    let e = Expr::call(B::PLUS, [inner.clone(), Expr::int(1)]);
    assert_eq!(e.head_symbol(), Some(B::PLUS));
    assert!(e.is_head(B::PLUS));
    assert_eq!(e.args(), &[inner.clone(), Expr::int(1)]);
    let same = Expr::normal(Expr::sym(B::PLUS), [inner, Expr::int(1)]);
    assert_eq!(e, same);
    assert_eq!(hash(&e), hash(&same));
    assert_ne!(
        e,
        Expr::call(B::PLUS, [Expr::int(1), same.args()[0].clone()])
    );
    let compound_head = Expr::normal(same.clone(), []);
    assert_eq!(compound_head.head(), same);
    assert_eq!(compound_head.head_symbol(), None);
}

#[test]
fn free_of_visits_whole_subtrees_and_explicit_heads() {
    let x = Expr::symbol("x");
    let fx = Expr::call(Symbol::intern("f"), [x.clone()]);
    let e = Expr::normal(fx.clone(), [Expr::int(7)]);
    assert!(!e.free_of(&fx));
    assert!(!e.free_of(&x));
    assert!(!e.free_of(&e));
    assert!(!e.free_of(&Expr::symbol("f")));
    assert!(e.free_of(&Expr::symbol("y")));
    assert!(Expr::int(3).free_of(&x));
}

#[test]
fn free_symbols_exclude_constants_and_function_heads() {
    let x = Expr::symbol("free_x");
    let y = Expr::symbol("free_y");
    let h = Expr::call(Symbol::intern("f"), [x.clone()]);
    let e = Expr::normal(
        h,
        [
            y.clone(),
            x,
            Expr::sym(B::PI),
            Expr::sym(B::E),
            Expr::sym(B::I),
            Expr::sym(B::INFINITY),
            Expr::sym(B::COMPLEX_INFINITY),
            Expr::sym(B::INDETERMINATE),
            Expr::sym(B::TRUE),
            Expr::sym(B::NULL),
        ],
    );
    assert_eq!(
        e.free_symbols(),
        BTreeSet::from([Symbol::intern("free_x"), Symbol::intern("free_y")])
    );
    assert_eq!(y.free_symbols(), BTreeSet::from([Symbol::intern("free_y")]));
    assert!(Expr::string("free_x").free_symbols().is_empty());
}

#[test]
fn replacements_are_simultaneous_first_match_including_heads() {
    let (x, y) = (Expr::symbol("x"), Expr::symbol("y"));
    let e = Expr::call(B::PLUS, [x.clone(), y.clone()]);
    assert_eq!(
        e.replace_all(&[(x.clone(), y.clone()), (y.clone(), Expr::int(1))]),
        Expr::call(B::PLUS, [y.clone(), Expr::int(1)])
    );
    assert_eq!(
        x.replace_all(&[(x.clone(), Expr::int(1)), (x.clone(), Expr::int(2))]),
        Expr::int(1)
    );
    assert_eq!(
        e.replace_all(&[(Expr::sym(B::PLUS), Expr::sym(B::TIMES))]),
        Expr::call(B::TIMES, [x.clone(), y.clone()])
    );
    assert_eq!(
        e.replace_all(&[(e.clone(), y.clone()), (x, Expr::int(1))]),
        y
    );
    assert_eq!(e.replace_all(&[]), e);
}

#[test]
fn map_args_maps_immediate_arguments_once_in_order() {
    let e = Expr::call(B::LIST, [Expr::int(2), Expr::int(4)]);
    let mut seen = Vec::new();
    let mapped = e.map_args(|arg| {
        seen.push(arg.clone());
        Expr::call(B::HOLD, [arg.clone()])
    });
    assert_eq!(seen, e.args());
    assert_eq!(
        mapped,
        Expr::call(
            B::LIST,
            [
                Expr::call(B::HOLD, [Expr::int(2)]),
                Expr::call(B::HOLD, [Expr::int(4)])
            ]
        )
    );
    let atom = Expr::int(1);
    assert_eq!(atom.map_args(|_| panic!("atom has no arguments")), atom);
}

#[test]
fn leaf_count_counts_heads_and_atoms() {
    assert_eq!(Expr::int(1).leaf_count(), 1);
    assert_eq!(Expr::rational(1, 2).leaf_count(), 1);
    assert_eq!(Expr::string("a").leaf_count(), 1);
    assert_eq!(Expr::call(B::LIST, []).leaf_count(), 1);
    let e = Expr::call(
        B::PLUS,
        [
            Expr::int(1),
            Expr::call(B::TIMES, [Expr::int(2), Expr::symbol("x")]),
        ],
    );
    assert_eq!(e.leaf_count(), 5);
    assert_eq!(Expr::normal(e, [Expr::int(1)]).leaf_count(), 6);
}

#[test]
fn debug_prints_full_form() {
    let e = Expr::call(
        B::PLUS,
        [
            Expr::rational(1, 2),
            Expr::call(B::TIMES, [Expr::real(2.0), Expr::symbol("x")]),
        ],
    );
    assert_eq!(format!("{e:?}"), "Plus[Rational[1, 2], Times[2., x]]");
    assert_eq!(
        format!("{:?}", Expr::string("line\n\"quote")),
        "\"line\\n\\\"quote\""
    );
}
