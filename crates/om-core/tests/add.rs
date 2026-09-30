//! Plus canonicalization, precision contagion, special values and sorting.

use om_core::{BUILTIN as B, Expr, add, canonical_cmp};
use om_num::{BigFloat, Number, Rational, Real};
use proptest::prelude::*;

fn x() -> Expr {
    Expr::symbol("x")
}
fn scale(n: Expr, term: Expr) -> Expr {
    Expr::call(B::TIMES, [n, term])
}
fn di(n: i64) -> Expr {
    Expr::call(B::DIRECTED_INFINITY, [Expr::int(n)])
}

#[test]
fn plus_vectors_match_contract_full_forms() {
    let y = Expr::symbol("y");
    let cases = [
        (
            vec![Expr::symbol("a"), scale(Expr::int(-1), Expr::symbol("b"))],
            "Plus[a, Times[-1, b]]",
        ),
        (vec![x(), x()], "Times[2, x]"),
        (
            vec![scale(Expr::int(2), x()), scale(Expr::int(3), x())],
            "Times[5, x]",
        ),
        (vec![y, x(), Expr::int(2)], "Plus[2, x, y]"),
        (
            vec![Expr::call(B::POWER, [x(), Expr::int(2)]), x()],
            "Plus[x, Power[x, 2]]",
        ),
        (
            vec![Expr::rational(1, 2), Expr::rational(1, 3)],
            "Rational[5, 6]",
        ),
        (vec![Expr::int(1), Expr::real(2.5)], "3.5"),
        (vec![x(), scale(Expr::real(1.0), x())], "Times[2., x]"),
        (vec![di(1), di(-1)], "Indeterminate"),
    ];
    for (args, expected) in cases {
        assert_eq!(format!("{:?}", add(args)), expected);
    }
}

#[test]
fn plus_flattens_nesting_and_reduces_empty_and_singleton_forms() {
    assert_eq!(add([]), Expr::int(0));
    assert_eq!(add([x()]), x());
    assert_eq!(
        add([
            Expr::call(
                B::PLUS,
                [Expr::int(1), Expr::call(B::PLUS, [x(), Expr::int(2)])]
            ),
            Expr::int(3)
        ]),
        Expr::call(B::PLUS, [Expr::int(6), x()])
    );
    assert_eq!(add([Expr::call(B::PLUS, [])]), Expr::int(0));
}

#[test]
fn only_exact_zeros_are_dropped_and_approximate_coefficients_survive() {
    assert_eq!(add([Expr::int(0), x()]), x());
    assert_eq!(
        add([Expr::real(0.0), x()]),
        Expr::call(B::PLUS, [Expr::real(0.0), x()])
    );
    assert_eq!(add([x(), scale(Expr::int(-1), x())]), Expr::int(0));
    assert_eq!(add([x(), scale(Expr::real(-1.0), x())]), Expr::real(0.0));
    assert_eq!(
        add([scale(Expr::real(0.5), x()), scale(Expr::real(0.5), x())]),
        scale(Expr::real(1.0), x())
    );
    assert_eq!(add([Expr::real(0.0), Expr::int(1)]), Expr::real(1.0));
}

#[test]
fn plus_merges_multi_factor_rests_without_nested_times() {
    let y = Expr::symbol("y");
    assert_eq!(
        add([
            Expr::call(B::TIMES, [Expr::int(2), x(), y.clone()]),
            Expr::call(B::TIMES, [Expr::int(3), x(), y.clone()])
        ]),
        Expr::call(B::TIMES, [Expr::int(5), x(), y])
    );
    assert_eq!(
        add([
            scale(Expr::rational(1, 2), x()),
            scale(Expr::rational(1, 3), x())
        ]),
        scale(Expr::rational(5, 6), x())
    );
}

#[test]
fn big_coefficients_keep_precision_and_machine_contagion() {
    let big = Expr::number(Number::Real(Real::Big(
        BigFloat::ONE.with_precision(100).value(),
    )));
    let e = add([scale(big, x()), x()]);
    assert!(
        matches!(e.args()[0].as_number(),Some(Number::Real(Real::Big(n))) if n.precision()==100)
    );
    assert_eq!(
        add([e, scale(Expr::real(-1.0), x())]),
        scale(Expr::real(1.0), x())
    );
}

#[test]
fn indeterminate_and_infinity_rules_are_order_independent() {
    let ind = Expr::sym(B::INDETERMINATE);
    let undirected = Expr::call(B::DIRECTED_INFINITY, []);
    for inf in [di(1), undirected.clone()] {
        assert_eq!(add([inf.clone(), x(), Expr::int(1)]), inf);
        assert_eq!(add([x(), inf.clone(), ind.clone()]), ind);
        assert_eq!(add([inf.clone(), undirected.clone()]), ind);
        assert_eq!(add([undirected.clone(), inf]), ind);
    }
    assert_eq!(add([di(1), di(1)]), di(1));
    assert_eq!(add([Expr::sym(B::INFINITY), x()]), di(1));
    assert_eq!(add([Expr::sym(B::COMPLEX_INFINITY), x()]), undirected);
    assert_eq!(
        add([di(1), Expr::call(B::DIRECTED_INFINITY, [Expr::symbol("d")])]),
        ind
    );
}

#[test]
fn thousand_terms_are_sorted_and_merged_within_release_budget() {
    let terms: Vec<_> = (0..1000)
        .rev()
        .map(|n| Expr::symbol(&format!("x_{n:04}")))
        .collect();
    let start = std::time::Instant::now();
    let e = add(terms);
    let elapsed = start.elapsed();
    assert_eq!(e.args().len(), 1000);
    assert!(
        e.args()
            .windows(2)
            .all(|w| canonical_cmp(&w[0], &w[1]).is_lt())
    );
    if !cfg!(debug_assertions) {
        assert!(elapsed.as_millis() < 10, "{elapsed:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases:256, rng_seed:proptest::test_runner::RngSeed::Fixed(0x504c5553), ..ProptestConfig::default() })]
    #[test]
    fn exact_linear_sums_are_permutation_invariant_idempotent_and_equal_in_value(
        terms in prop::collection::vec((0u8..3,-20i64..=20,1i64..=9),0..80), xv in -9i64..=9, yv in -9i64..=9) {
        let xs=[x(),Expr::symbol("y")];
        let args:Vec<_>=terms.iter().map(|&(kind,n,d)| if kind==2 { Expr::rational(n,d) } else { scale(Expr::rational(n,d),xs[kind as usize].clone()) }).collect();
        let expected=terms.iter().fold(Rational::ZERO,|sum,&(kind,n,d)| sum+(Rational::from(n)/Rational::from(d))*Rational::from(match kind {0=>xv,1=>yv,_=>1}));
        let e=add(args.clone());
        prop_assert_eq!(&e,&add(args.into_iter().rev()));
        prop_assert_eq!(&e,&add([e.clone()]));
        let value=|term:&Expr| -> Number {
            if let Some(n)=term.as_number() { n.clone() }
            else {
                let (c,s)=if term.is_head(B::TIMES) { (term.args()[0].as_number().unwrap().clone(),&term.args()[1]) }
                    else { (Number::Integer(1.into()),term) };
                c.mul(&Number::Integer((if *s==xs[0] {xv} else {yv}).into()))
            }
        };
        let actual=if e.is_head(B::PLUS) {e.args().iter().map(value).fold(Number::Integer(0.into()),|a,b|a.add(&b))} else {value(&e)};
        prop_assert_eq!(actual,Number::Rational(expected).normalize());
    }
}
