//! Polynomial representations and coefficient-ring contracts before algorithms.
use om_num::{Integer, Rational, ctx::Interrupt, rng::SplitMix64};
use om_poly::{EuclideanRing, Field, FpElem, MPoly, MonoOrder, Monomial, Ring, UPoly};

fn z(xs: &[i64]) -> UPoly<Integer> {
    UPoly::new(xs.iter().map(|x| (*x).into()).collect())
}
fn q(xs: &[i64]) -> UPoly<Rational> {
    UPoly::new(xs.iter().map(|x| (*x).into()).collect())
}
fn fp(xs: &[u64], p: u64) -> UPoly<FpElem> {
    UPoly::new(xs.iter().map(|x| FpElem::new(*x, p).unwrap()).collect())
}
fn m(nvars: usize, ts: &[(&[u32], i64)], order: MonoOrder) -> MPoly<Integer> {
    MPoly::new(
        nvars,
        ts.iter()
            .map(|(e, c)| (Monomial::new(e.iter().copied()).unwrap(), (*c).into()))
            .collect(),
        order,
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn integer_and_rational_traits_have_exact_arithmetic_and_euclidean_remainders() {
    for (a, b, q, r) in [(7, 3, 2, 1), (-7, 3, -3, 2), (7, -3, -2, 1), (-7, -3, 3, 2)] {
        let (gotq, gotr) = EuclideanRing::divrem(&Integer::from(a), &Integer::from(b));
        assert_eq!((gotq, gotr), (Integer::from(q), Integer::from(r)));
    }
    assert_eq!(
        EuclideanRing::exact_div(&Integer::from(6), &Integer::from(3)),
        Some(2.into())
    );
    assert_eq!(
        EuclideanRing::exact_div(&Integer::from(7), &Integer::from(3)),
        None
    );
    assert_eq!(
        EuclideanRing::exact_div(&Integer::from(0), &Integer::from(0)),
        None
    );
    assert_eq!(
        Field::inv(&(Rational::from(2) / Rational::from(3))),
        Some(Rational::from(3) / Rational::from(2))
    );
    assert_eq!(Field::inv(&Rational::ZERO), None);
}
#[test]
fn runtime_prime_field_identities_adopt_the_modulus_and_large_residues_stay_safe() {
    assert_eq!(FpElem::new(1, 15), None);
    assert_eq!(FpElem::new(1, 0), None);
    for p in [2, 3, 5, 7, 11, 18_446_744_073_709_551_557] {
        let x = FpElem::new(p - 1, p).unwrap();
        assert_eq!(x.add(&x.neg()), FpElem::zero());
        assert_eq!(x.mul(&Field::inv(&x).unwrap()), FpElem::one());
        assert_eq!(Ring::add(&x, &FpElem::zero()), x);
        assert_eq!(Ring::mul(&x, &FpElem::one()), x);
        assert_eq!(Ring::mul(&x, &x), FpElem::new(1, p).unwrap());
        assert_eq!(
            Ring::mul(&x, &Field::inv(&x).unwrap()),
            FpElem::new(1, p).unwrap()
        );
        assert_eq!(
            Ring::neg(&FpElem::one()).mul(&FpElem::new(1, p).unwrap()),
            x
        );
    }
    assert_eq!(Field::inv(&FpElem::new(0, 7).unwrap()), None);
    let a = FpElem::new(3, 7).unwrap();
    let b = FpElem::new(5, 7).unwrap();
    assert_eq!(a.add(&b), FpElem::new(1, 7).unwrap());
    assert_eq!(a.sub(&b), FpElem::new(5, 7).unwrap());
    assert_eq!(a.mul(&b), FpElem::new(1, 7).unwrap());
}
#[test]
#[should_panic(expected = "different prime fields")]
fn different_bound_moduli_are_rejected_instead_of_silently_mixed() {
    let _ = FpElem::new(1, 5).unwrap().add(&FpElem::new(1, 7).unwrap());
}
#[test]
fn univariate_integer_arithmetic_trims_zeros_and_preserves_ring_identities() {
    let ctx = Interrupt::default();
    let a = z(&[1, 2, 1, 0, 0]);
    let b = z(&[-1, 1]);
    assert_eq!(a.coeffs.len(), 3);
    assert_eq!(a.degree(), Some(2));
    assert_eq!(a.lc(), Some(&Integer::ONE));
    assert_eq!(a.add(&b, &ctx).unwrap(), z(&[0, 3, 1]));
    assert_eq!(a.sub(&b, &ctx).unwrap(), z(&[2, 1, 1]));
    assert_eq!(a.mul(&b, &ctx).unwrap(), z(&[-1, -1, 1, 1]));
    assert_eq!(a.sub(&a, &ctx).unwrap(), UPoly::zero());
    assert_eq!(a.mul(&UPoly::one(), &ctx).unwrap(), a);
    assert!(UPoly::<Integer>::zero().coeffs.is_empty());
    assert_eq!(UPoly::<Integer>::zero().degree(), None);
    assert!(UPoly::<Integer>::one().is_one());
}
#[test]
fn rational_and_finite_field_polynomial_operations_use_the_real_coefficient_field() {
    let ctx = Interrupt::default();
    let a = q(&[1, 1]);
    let b = q(&[1, -1]);
    assert_eq!(a.mul(&b, &ctx).unwrap(), q(&[1, 0, -1]));
    let scaled = a
        .scale(&(Rational::from(1) / Rational::from(2)), &ctx)
        .unwrap();
    assert_eq!(scaled.add(&scaled, &ctx).unwrap(), a);
    for p in [2, 3, 5, 7, 11] {
        let a = fp(&[1, 1], p);
        let b = fp(&[p - 1, 1], p);
        assert_eq!(a.mul(&b, &ctx).unwrap(), fp(&[p - 1, 0, 1], p));
        assert_eq!(a.sub(&a, &ctx).unwrap(), UPoly::zero());
        assert_eq!(a.mul(&UPoly::one(), &ctx).unwrap(), a);
    }
}
#[test]
fn field_long_division_has_a_reconstruction_and_a_smaller_remainder() {
    let ctx = Interrupt::default();
    let (quotient, remainder) = q(&[-4, 0, -2, 1])
        .divrem(&q(&[-3, 1]), &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(quotient, q(&[3, 1, 1]));
    assert_eq!(remainder, q(&[5]));
    let half = Rational::from(1) / Rational::from(2);
    assert_eq!(
        q(&[1, 1]).divrem(&q(&[2]), &ctx).unwrap().unwrap().0,
        q(&[1, 1]).scale(&half, &ctx).unwrap()
    );
    for p in [2, 3, 5, 7, 11] {
        let f = fp(&[1, 2, 3, 4], p);
        let g = fp(&[1, 1], p);
        let (a, b) = f.divrem(&g, &ctx).unwrap().unwrap();
        assert_eq!(a.mul(&g, &ctx).unwrap().add(&b, &ctx).unwrap(), f);
        assert!(b.degree().is_none_or(|d| d < g.degree().unwrap()));
    }
    assert!(q(&[1]).divrem(&UPoly::zero(), &ctx).unwrap().is_none());
    assert_eq!(
        q(&[1]).divrem(&q(&[1, 1]), &ctx).unwrap().unwrap(),
        (UPoly::zero(), q(&[1]))
    );
}
#[test]
fn monomial_orders_and_cached_degrees_follow_lex_and_grevlex() {
    let a = Monomial::new([2, 0, 1]).unwrap();
    let b = Monomial::new([1, 2, 0]).unwrap();
    assert_eq!(a.deg, 3);
    assert_eq!(a.cmp(&b, MonoOrder::Lex), std::cmp::Ordering::Greater);
    assert_eq!(a.cmp(&b, MonoOrder::GrevLex), std::cmp::Ordering::Less);
    assert!(Monomial::new([u32::MAX, 1]).is_none());
    let spilled = Monomial::new([1, 0, 2, 0, 3]).unwrap();
    assert_eq!(spilled.deg, 6);
    assert_eq!(spilled.exps.len(), 5);
}
#[test]
fn sparse_constructor_merges_duplicates_and_sorts_each_order_deterministically() {
    let terms = [
        (&[2, 0, 1][..], 2),
        (&[1, 2, 0][..], 3),
        (&[2, 0, 1][..], -1),
        (&[0, 0, 0][..], 0),
    ];
    let lex = m(3, &terms, MonoOrder::Lex);
    let rev = m(3, &terms, MonoOrder::GrevLex);
    assert_eq!(lex.terms.len(), 2);
    assert_eq!(lex.terms[0].0.exps.as_slice(), &[2, 0, 1]);
    assert_eq!(rev.terms[0].0.exps.as_slice(), &[1, 2, 0]);
    let backwards: Vec<_> = terms.into_iter().rev().collect();
    assert_eq!(m(3, &backwards, MonoOrder::Lex), lex);
}
#[test]
fn sparse_ring_arithmetic_also_works_as_a_univariate_coefficient_ring() {
    let ctx = Interrupt::default();
    let x = m(2, &[(&[1, 0], 1)], MonoOrder::GrevLex);
    let y = m(2, &[(&[0, 1], 1)], MonoOrder::GrevLex);
    let a = x.add(&y, &ctx).unwrap();
    let b = x.sub(&y, &ctx).unwrap();
    assert_eq!(
        a.mul(&b, &ctx).unwrap().unwrap(),
        m(2, &[(&[2, 0], 1), (&[0, 2], -1)], MonoOrder::GrevLex)
    );
    assert_eq!(Ring::add(&a, &MPoly::zero()), a);
    assert_eq!(Ring::mul(&a, &MPoly::one()), a);
    assert_eq!(Ring::sub(&a, &a), MPoly::zero());
    let p = UPoly::new(vec![x.clone(), MPoly::one()]);
    let q = UPoly::new(vec![y.clone(), MPoly::one()]);
    assert_eq!(
        p.mul(&q, &ctx).unwrap(),
        UPoly::new(vec![Ring::mul(&x, &y), a, MPoly::one()])
    );
    let large = m(1, &[(&[u32::MAX], 1)], MonoOrder::Lex);
    assert!(
        large
            .mul(&m(1, &[(&[1], 1)], MonoOrder::Lex), &ctx)
            .unwrap()
            .is_none()
    );
}
#[test]
fn fixed_seed_polynomial_ring_laws_and_field_division_reconstruct() {
    let mut rng = SplitMix64::new(0x4d352e31);
    let ctx = Interrupt::default();
    for _ in 0..256 {
        let mut poly = || {
            UPoly::new(
                (0..5)
                    .map(|_| Rational::from(rng.next_range(0, 11) as i64 - 5))
                    .collect(),
            )
        };
        let (a, b, c) = (poly(), poly(), poly());
        assert_eq!(
            a.mul(&b.add(&c, &ctx).unwrap(), &ctx).unwrap(),
            a.mul(&b, &ctx)
                .unwrap()
                .add(&a.mul(&c, &ctx).unwrap(), &ctx)
                .unwrap()
        );
        assert_eq!(a.add(&b, &ctx).unwrap().sub(&b, &ctx).unwrap(), a);
        if !b.is_zero() {
            let (q, r) = a.divrem(&b, &ctx).unwrap().unwrap();
            assert_eq!(q.mul(&b, &ctx).unwrap().add(&r, &ctx).unwrap(), a);
            assert!(r.degree().is_none_or(|d| d < b.degree().unwrap()));
        }
    }
}

#[test]
fn sparse_distributivity_and_negation_hold_in_both_monomial_orders() {
    let ctx = Interrupt::default();
    let mut rng = SplitMix64::new(0x4d504f4c59);
    for order in [MonoOrder::Lex, MonoOrder::GrevLex] {
        for _ in 0..128 {
            let mut poly = || {
                MPoly::new(
                    3,
                    (0..6)
                        .map(|_| {
                            let mono =
                                Monomial::new((0..3).map(|_| rng.next_range(0, 3) as u32)).unwrap();
                            (mono, Integer::from(rng.next_range(0, 11) as i64 - 5))
                        })
                        .collect(),
                    order,
                    &ctx,
                )
                .unwrap()
            };
            let (a, b, c) = (poly(), poly(), poly());
            let lhs = a.mul(&b.add(&c, &ctx).unwrap(), &ctx).unwrap().unwrap();
            let rhs = a
                .mul(&b, &ctx)
                .unwrap()
                .unwrap()
                .add(&a.mul(&c, &ctx).unwrap().unwrap(), &ctx)
                .unwrap();
            assert_eq!(lhs, rhs);
            assert_eq!(a.add(&a.neg(&ctx).unwrap(), &ctx).unwrap(), MPoly::zero());
        }
    }
}
#[test]
fn polynomial_loops_and_sparse_normalization_obey_interrupts() {
    let ctx = Interrupt::default();
    ctx.steps_left.set(3);
    assert!(z(&[1; 20]).mul(&z(&[1; 20]), &ctx).is_err());
    let ctx = Interrupt::default();
    ctx.steps_left.set(2);
    assert!(q(&[1; 20]).divrem(&q(&[1, 1]), &ctx).is_err());
    let ctx = Interrupt::default();
    ctx.steps_left.set(3);
    let terms = (0..100)
        .map(|i| (Monomial::new([i]).unwrap(), Integer::ONE))
        .collect();
    assert!(MPoly::new(1, terms, MonoOrder::Lex, &ctx).is_err());
    let ctx = Interrupt::default();
    ctx.steps_left.set(211);
    let terms = (0..100)
        .rev()
        .map(|i| (Monomial::new([i]).unwrap(), Integer::ONE))
        .collect();
    assert!(MPoly::new(1, terms, MonoOrder::Lex, &ctx).is_err());
}

#[test]
fn finite_field_zero_holes_are_canonical_in_products_and_exact_division() {
    let ctx = Interrupt::default();
    for p in [2, 3, 5, 7, 11] {
        let x = fp(&[0, 1], p);
        let square = fp(&[0, 0, 1], p);
        assert_eq!(x.mul(&x, &ctx).unwrap(), square);
        let (q, r) = square.divrem(&x, &ctx).unwrap().unwrap();
        assert_eq!(q, x);
        assert!(r.is_zero());
        assert_eq!(q.mul(&x, &ctx).unwrap(), square);
    }
    let mut rng = SplitMix64::new(0x46502d5a45524f);
    for _ in 0..128 {
        let p = 7;
        let mut poly = || {
            UPoly::new(
                (0..5)
                    .map(|_| FpElem::new(rng.next_range(0, p), p).unwrap())
                    .collect(),
            )
        };
        let (a, b, c) = (poly(), poly(), poly());
        assert_eq!(
            a.mul(&b.add(&c, &ctx).unwrap(), &ctx).unwrap(),
            a.mul(&b, &ctx)
                .unwrap()
                .add(&a.mul(&c, &ctx).unwrap(), &ctx)
                .unwrap()
        );
        if !b.is_zero() {
            let (q, r) = a.divrem(&b, &ctx).unwrap().unwrap();
            assert_eq!(q.mul(&b, &ctx).unwrap().add(&r, &ctx).unwrap(), a);
        }
    }
}
