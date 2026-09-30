//! Product canonicalization and exact substitution properties.

use om_core::{BUILTIN as B, Expr, add, canonical_cmp, mul};
use om_num::{Complex, Integer, Number, Rational};
use proptest::prelude::*;

fn x() -> Expr {
    Expr::symbol("x")
}
fn y() -> Expr {
    Expr::symbol("y")
}
fn p(b: Expr, e: Expr) -> Expr {
    Expr::call(B::POWER, [b, e])
}
fn di(z: Expr) -> Expr {
    Expr::call(B::DIRECTED_INFINITY, [z])
}
fn i() -> Expr {
    Expr::number(Number::Complex(Box::new(Complex {
        re: Number::Integer(Integer::ZERO),
        im: Number::Integer(Integer::ONE),
    })))
}

#[test]
fn product_vectors_cover_powers_roots_infinities_and_distribution() {
    let cases = [
        (vec![x(), x()], "Power[x, 2]"),
        (vec![p(x(), Expr::int(2)), p(x(), Expr::int(-2))], "1"),
        (vec![Expr::int(0), x()], "0"),
        (
            vec![
                p(Expr::int(2), Expr::rational(1, 2)),
                p(Expr::int(3), Expr::rational(1, 2)),
            ],
            "Power[6, Rational[1, 2]]",
        ),
        (
            vec![
                p(Expr::int(2), Expr::rational(1, 2)),
                p(Expr::int(2), Expr::rational(1, 2)),
            ],
            "2",
        ),
        (
            vec![
                p(Expr::int(2), Expr::rational(1, 2)),
                p(Expr::int(3), Expr::rational(-1, 2)),
            ],
            "Power[Rational[2, 3], Rational[1, 2]]",
        ),
        (vec![i(), i()], "-1"),
        (
            vec![
                add([Expr::int(1), i()]),
                add([Expr::int(1), Expr::number(i().as_number().unwrap().neg())]),
            ],
            "2",
        ),
        (
            vec![Expr::int(-2), di(Expr::int(1))],
            "DirectedInfinity[-1]",
        ),
        (
            vec![i(), di(Expr::int(1))],
            "DirectedInfinity[Complex[0, 1]]",
        ),
        (
            vec![Expr::int(-1), add([Expr::int(1), x()])],
            "Plus[-1, Times[-1, x]]",
        ),
        (
            vec![Expr::int(-2), add([Expr::int(1), x()])],
            "Times[-2, Plus[1, x]]",
        ),
    ];
    for (args, expected) in cases {
        assert_eq!(format!("{:?}", mul(args)), expected);
    }
}

#[test]
fn flattening_empty_singleton_and_numeric_contagion() {
    assert_eq!(mul([]), Expr::int(1));
    assert_eq!(mul([x()]), x());
    assert_eq!(
        mul([Expr::call(
            B::TIMES,
            [Expr::int(2), Expr::call(B::TIMES, [x(), Expr::int(3)])]
        )]),
        Expr::call(B::TIMES, [Expr::int(6), x()])
    );
    assert_eq!(mul([Expr::real(0.0), x()]), Expr::real(0.0));
    assert_eq!(
        mul([Expr::real(1.0), x()]),
        Expr::call(B::TIMES, [Expr::real(1.0), x()])
    );
    assert_eq!(mul([Expr::rational(2, 3), Expr::int(3)]), Expr::int(2));
}

#[test]
fn zero_does_not_hide_indeterminate_or_infinity() {
    for zero in [Expr::int(0), Expr::real(0.0)] {
        for other in [
            Expr::sym(B::INDETERMINATE),
            di(Expr::int(1)),
            Expr::call(B::DIRECTED_INFINITY, []),
        ] {
            assert_eq!(
                mul([zero.clone(), other.clone()]),
                Expr::sym(B::INDETERMINATE)
            );
            assert_eq!(mul([other, zero.clone()]), Expr::sym(B::INDETERMINATE));
        }
    }
    assert_eq!(
        mul([Expr::sym(B::COMPLEX_INFINITY), x()]),
        Expr::call(B::DIRECTED_INFINITY, [])
    );
    assert_eq!(
        mul([di(Expr::int(-1)), di(Expr::int(-1))]),
        di(Expr::int(1))
    );
    assert_eq!(
        mul([di(Expr::int(1)), x()]),
        Expr::call(B::TIMES, [x(), di(Expr::int(1))])
    );
}

#[test]
fn grouping_reaches_a_fixed_point_after_numeric_rebuilds() {
    assert_eq!(
        mul([
            p(Expr::int(2), Expr::rational(1, 2)),
            p(Expr::int(2), Expr::rational(3, 2)),
            Expr::int(3)
        ]),
        Expr::int(12)
    );
    assert_eq!(mul([x(), p(x(), y()), p(x(), Expr::int(-1))]), p(x(), y()));
    let result = mul([
        p(Expr::int(2), Expr::rational(1, 2)),
        p(Expr::int(8), Expr::rational(1, 2)),
        x(),
    ]);
    assert_eq!(result, Expr::call(B::TIMES, [Expr::int(4), x()]));
    assert_eq!(mul([result.clone()]), result);
}

#[test]
fn merged_radicals_include_negative_exponents_and_sorted_factors() {
    assert_eq!(
        mul([
            p(Expr::int(2), Expr::rational(-1, 2)),
            p(Expr::int(3), Expr::rational(1, 2))
        ]),
        p(Expr::rational(3, 2), Expr::rational(1, 2))
    );
    assert_eq!(
        mul([
            p(Expr::int(2), Expr::rational(-1, 2)),
            p(Expr::int(8), Expr::rational(-1, 2))
        ]),
        Expr::rational(1, 4)
    );
    let product = mul([y(), x(), Expr::int(2)]);
    assert_eq!(product, Expr::call(B::TIMES, [Expr::int(2), x(), y()]));
    assert!(
        product
            .args()
            .windows(2)
            .all(|w| canonical_cmp(&w[0], &w[1]).is_lt())
    );
}

#[test]
fn plus_uses_complete_product_rebuild_including_minus_one_distribution() {
    assert_eq!(
        add([
            Expr::call(B::TIMES, [Expr::int(-1), add([Expr::int(1), x()])]),
            x()
        ]),
        Expr::int(-1)
    );
}

fn value(e: &Expr, xv: i64, yv: i64) -> Number {
    if let Some(n) = e.as_number() {
        n.clone()
    } else if *e == x() {
        Number::Integer(xv.into())
    } else if *e == y() {
        Number::Integer(yv.into())
    } else if e.is_head(B::POWER) {
        let Number::Integer(exp) = e.args()[1].as_number().unwrap() else {
            panic!()
        };
        value(&e.args()[0], xv, yv).pow_int(exp).unwrap()
    } else {
        e.args()
            .iter()
            .map(|e| value(e, xv, yv))
            .fold(Number::Integer(1.into()), |a, b| a.mul(&b))
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases:256, rng_seed:proptest::test_runner::RngSeed::Fixed(0x54494d4553), ..ProptestConfig::default() })]
    #[test]
    fn exact_products_are_permutation_invariant_idempotent_and_equal_in_value(
        terms in prop::collection::vec((0u8..3,-5i64..=5,1i64..=5,0i64..=4),0..30),xv in 1i64..=5,yv in 1i64..=5) {
        let args:Vec<_>=terms.iter().map(|&(kind,n,d,ex)| match kind {0=>p(x(),Expr::int(ex)),1=>p(y(),Expr::int(ex)),_=>Expr::rational(n,d)}).collect();
        let e=mul(args.clone());
        let expected=args.iter().fold(Number::Rational(Rational::ONE),|a,b|a.mul(&value(b,xv,yv)));
        prop_assert_eq!(&e,&mul(args.into_iter().rev()));
        prop_assert_eq!(&e,&mul([e.clone()]));
        prop_assert_eq!(value(&e,xv,yv),expected.normalize());
    }
}
