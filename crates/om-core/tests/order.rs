//! Ordering examples and reproducible order-law tests.

use om_core::{BUILTIN as B, Expr, Symbol, canonical_cmp};
use om_num::{BigFloat, Complex, Integer, Number, Real};
use proptest::prelude::*;
use std::cmp::Ordering::{Equal, Less};

fn power(b: Expr, e: Expr) -> Expr {
    Expr::call(B::POWER, [b, e])
}
fn times(args: impl IntoIterator<Item = Expr>) -> Expr {
    Expr::call(B::TIMES, args)
}
fn big(n: i64, bits: usize) -> Expr {
    Expr::number(Number::Real(Real::Big(
        BigFloat::from(n).with_precision(bits).value(),
    )))
}
fn complex(re: f64, im: f64) -> Expr {
    Expr::number(Number::Complex(Box::new(Complex {
        re: Number::Real(Real::Machine(re)),
        im: Number::Real(Real::Machine(im)),
    })))
}
fn sorted(mut es: Vec<Expr>) -> Vec<Expr> {
    es.sort_by(canonical_cmp);
    es
}

#[test]
fn symbols_sort_by_folded_then_original_name_not_ids() {
    let input: Vec<_> = ["z", "a", "B", "b", "A", "α"]
        .into_iter()
        .map(Expr::symbol)
        .collect();
    let expected: Vec<_> = ["A", "a", "B", "b", "z", "α"]
        .into_iter()
        .map(Expr::symbol)
        .collect();
    assert_eq!(sorted(input), expected);
}

#[test]
fn numbers_sort_by_exact_components_then_precision() {
    assert_eq!(canonical_cmp(&Expr::int(-9), &Expr::rational(-17, 2)), Less);
    assert_eq!(canonical_cmp(&Expr::int(1), &Expr::real(1.0)), Less);
    assert_eq!(canonical_cmp(&big(1, 32), &Expr::real(1.0)), Less);
    assert_eq!(canonical_cmp(&Expr::real(1.0), &big(1, 128)), Less);
    assert_eq!(canonical_cmp(&complex(1.0, -1.0), &Expr::int(1)), Less);
    assert_eq!(
        canonical_cmp(&complex(1.0, 1.0), &complex(2.0, -99.0)),
        Less
    );
    // Values beyond binary64's integer resolution must remain distinguishable.
    assert_eq!(
        canonical_cmp(
            &Expr::integer(Integer::from(1u64 << 53)),
            &Expr::integer(Integer::from((1u64 << 53) + 1))
        ),
        Less
    );
    for e in [
        Expr::symbol("x"),
        Expr::string("s"),
        Expr::call(B::SIN, [Expr::int(0)]),
    ] {
        assert_eq!(canonical_cmp(&complex(2.0, 3.0), &e), Less);
    }
}

#[test]
fn monomials_compare_from_last_factor_then_exponent_then_coefficient() {
    let (x, y, z) = (Expr::symbol("x"), Expr::symbol("y"), Expr::symbol("z"));
    assert_eq!(canonical_cmp(&x, &power(x.clone(), Expr::int(2))), Less);
    assert_eq!(
        canonical_cmp(
            &times([z.clone(), x.clone()]),
            &times([x.clone(), y.clone()])
        ),
        Less
    );
    assert_eq!(canonical_cmp(&y, &times([x.clone(), y.clone()])), Less);
    assert_eq!(
        canonical_cmp(
            &times([Expr::int(2), x.clone()]),
            &times([Expr::int(3), x.clone()])
        ),
        Less
    );
    assert_eq!(canonical_cmp(&x, &times([Expr::int(2), x.clone()])), Less);
}

#[test]
fn atoms_compound_heads_and_arguments_are_lexicographic() {
    assert_eq!(
        canonical_cmp(&Expr::symbol("zz"), &Expr::string("aa")),
        Less
    );
    assert_eq!(
        canonical_cmp(&Expr::string("z"), &Expr::call(B::SIN, [])),
        Less
    );
    assert_eq!(
        canonical_cmp(&Expr::call(B::COS, []), &Expr::call(B::SIN, [])),
        Less
    );
    assert_eq!(
        canonical_cmp(
            &Expr::call(B::SIN, [Expr::int(1)]),
            &Expr::call(B::SIN, [Expr::int(1), Expr::int(2)])
        ),
        Less
    );
    assert_eq!(
        canonical_cmp(
            &Expr::normal(Expr::call(B::COS, []), []),
            &Expr::normal(Expr::call(B::SIN, []), [])
        ),
        Less
    );
}

#[test]
fn raw_degenerate_forms_and_same_precision_variants_have_total_ties() {
    let x = Expr::symbol("x");
    let forms = [
        x.clone(),
        power(x.clone(), Expr::int(1)),
        times([x.clone()]),
        times([Expr::int(1), x]),
        Expr::real(1.0),
        big(1, 53),
        complex(1.0, 0.0),
    ];
    for a in &forms {
        for b in &forms {
            assert_eq!(canonical_cmp(a, b) == Equal, a == b, "{a:?} vs {b:?}");
            assert_eq!(canonical_cmp(a, b), canonical_cmp(b, a).reverse());
        }
    }
}

fn expressions() -> impl Strategy<Value = Expr> {
    let atoms = prop_oneof![
        (-3i64..=3).prop_map(Expr::int),
        (-3i64..=3).prop_map(|n| Expr::rational(n, 2)),
        (-3i64..=3).prop_map(|n| Expr::real(n as f64)),
        (-3i64..=3, 2usize..=64).prop_map(|(n, p)| big(n, p)),
        (-3i64..=3, -3i64..=3).prop_map(|(r, i)| complex(r as f64, i as f64)),
        prop::sample::select(vec!["A", "a", "x", "y", "z", "α"]).prop_map(Expr::symbol),
        prop::sample::select(vec!["", "a", "z"]).prop_map(Expr::string)
    ];
    atoms.prop_recursive(4, 48, 4, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(b, e)| power(b, e)),
            prop::collection::vec(inner.clone(), 0..4).prop_map(times),
            prop::collection::vec(inner.clone(), 0..4).prop_map(|args| Expr::call(B::PLUS, args)),
            (
                prop::sample::select(vec![B::SIN, B::COS, Symbol::intern("f")]),
                prop::collection::vec(inner.clone(), 0..4)
            )
                .prop_map(|(h, args)| Expr::call(h, args)),
            (inner.clone(), prop::collection::vec(inner, 0..3))
                .prop_map(|(h, args)| Expr::normal(h, args))
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases:1024, rng_seed:proptest::test_runner::RngSeed::Fixed(0x434d50), ..ProptestConfig::default() })]
    #[test]
    fn ordering_is_reflexive_antisymmetric_transitive_and_structural(a in expressions(), b in expressions(), c in expressions()) {
        prop_assert_eq!(canonical_cmp(&a,&a),Equal);
        let ab=canonical_cmp(&a,&b);
        let bc=canonical_cmp(&b,&c);
        prop_assert_eq!(ab,canonical_cmp(&b,&a).reverse());
        prop_assert_eq!(ab==Equal,a==b);
        if ab.is_le() && bc.is_le() { prop_assert!(canonical_cmp(&a,&c).is_le(), "{a:?} <= {b:?} <= {c:?}"); }
        if ab.is_ge() && bc.is_ge() { prop_assert!(canonical_cmp(&a,&c).is_ge(), "{a:?} >= {b:?} >= {c:?}"); }
        prop_assert!(!ab.is_lt() || !canonical_cmp(&b,&a).is_lt());
    }
}
