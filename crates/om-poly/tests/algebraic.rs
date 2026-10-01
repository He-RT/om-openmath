//! Exact algebraic identities, certified branch selection and Root numbering.
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{Algebraic, ComplexAlg, RealAlg, UPoly, algebraic_root, real_alg};
use proptest::prelude::*;
use std::cmp::Ordering;
fn z(c: &[i64]) -> UPoly<Integer> {
    UPoly::new(c.iter().map(|x| Integer::from(*x)).collect())
}
fn real(f: &UPoly<Integer>, lo: i64, hi: i64, ctx: &Interrupt) -> Algebraic {
    real_alg(f, (Rational::from(lo), Rational::from(hi)), ctx)
        .unwrap()
        .unwrap()
}
fn rational(a: &Algebraic) -> Rational {
    match a {
        Algebraic::Rational(q) => q.clone(),
        _ => panic!("expected rational: {a:?}"),
    }
}
#[test]
fn authority_sum_and_product_have_irreducible_minpolys_and_correct_real_roots() {
    let ctx = Interrupt::default();
    let a = real(&z(&[-2, 0, 1]), 1, 2, &ctx);
    let b = real(&z(&[-3, 0, 1]), 1, 2, &ctx);
    let sum = a.add(&b, &ctx).unwrap().unwrap();
    assert_eq!(sum.minimal_polynomial(&ctx).unwrap(), z(&[1, 0, -10, 0, 1]));
    assert_eq!(
        sum.cmp_real(&Algebraic::Rational(3.into()), &ctx).unwrap(),
        Some(Ordering::Greater)
    );
    assert_eq!(
        sum.cmp_real(&Algebraic::Rational(4.into()), &ctx).unwrap(),
        Some(Ordering::Less)
    );
    let product = a.mul(&b, &ctx).unwrap().unwrap();
    assert_eq!(product.minimal_polynomial(&ctx).unwrap(), z(&[-6, 0, 1]));
    assert_eq!(
        product
            .cmp_real(&Algebraic::Rational(2.into()), &ctx)
            .unwrap(),
        Some(Ordering::Greater)
    );
    assert_eq!(
        product
            .cmp_real(&Algebraic::Rational(3.into()), &ctx)
            .unwrap(),
        Some(Ordering::Less)
    );
    let restored = sum.sub(&b, &ctx).unwrap().unwrap();
    assert_eq!(restored.equals(&a, &ctx).unwrap(), Some(true));
}
#[test]
fn authority_cubic_power_reduces_exactly_to_zero() {
    let ctx = Interrupt::default();
    let a = algebraic_root(&z(&[-2, 0, 0, 1]), 1, &ctx)
        .unwrap()
        .unwrap();
    let cube = a.pow_int(&3.into(), &ctx).unwrap().unwrap();
    let zero = cube
        .sub(&Algebraic::Rational(2.into()), &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(rational(&zero), Rational::ZERO);
    assert!(zero.is_zero());
}
#[test]
fn rational_demotion_and_reducible_input_select_the_certified_factor() {
    let ctx = Interrupt::default();
    assert_eq!(
        rational(&real(&z(&[-1, 3]), 0, 1, &ctx)),
        Rational::ONE / Rational::from(3)
    );
    let f = z(&[-2, 0, 1])
        .mul(&z(&[-3, 1]), &ctx)
        .unwrap()
        .scale(&(-12).into(), &ctx)
        .unwrap();
    assert_eq!(
        real(&f, 1, 2, &ctx).minimal_polynomial(&ctx).unwrap(),
        z(&[-2, 0, 1])
    );
    assert_eq!(
        rational(&real_alg(&f, (3.into(), 3.into()), &ctx).unwrap().unwrap()),
        Rational::from(3)
    );
    assert!(
        real_alg(&f, ((-4).into(), 4.into()), &ctx)
            .unwrap()
            .is_none()
    );
    assert!(real_alg(&f, (2.into(), 1.into()), &ctx).unwrap().is_none());
    assert!(algebraic_root(&f, 0, &ctx).unwrap().is_none());
    assert!(algebraic_root(&f, 4, &ctx).unwrap().is_none());
    let repeated = f.mul(&f, &ctx).unwrap();
    assert_eq!(
        rational(&algebraic_root(&repeated, 3, &ctx).unwrap().unwrap()),
        Rational::from(3)
    );
}
#[test]
fn same_minpoly_rank_different_minpoly_comparison_and_reciprocals_are_exact() {
    let ctx = Interrupt::default();
    let a = real(&z(&[-2, 0, 1]), 0, 10, &ctx);
    let same = real(&z(&[14, 0, -7]), 1, 2, &ctx);
    assert_eq!(a.cmp_real(&same, &ctx).unwrap(), Some(Ordering::Equal));
    let negative = a.neg(&ctx).unwrap().unwrap();
    assert_eq!(negative.real_sign(&ctx).unwrap(), Some(-1));
    assert_eq!(a.real_sign(&ctx).unwrap(), Some(1));
    assert_eq!(negative.cmp_real(&a, &ctx).unwrap(), Some(Ordering::Less));
    let inverse = a.recip(&ctx).unwrap().unwrap();
    assert_eq!(inverse.minimal_polynomial(&ctx).unwrap(), z(&[-1, 0, 2]));
    assert_eq!(
        rational(&a.mul(&inverse, &ctx).unwrap().unwrap()),
        Rational::ONE
    );
    assert!(
        Algebraic::Rational(Rational::ZERO)
            .recip(&ctx)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        a.cmp_real(&real(&z(&[-3, 0, 1]), 1, 2, &ctx), &ctx)
            .unwrap(),
        Some(Ordering::Less)
    );
}
fn approx(a: &Algebraic, ctx: &Interrupt) -> (f64, f64) {
    let b = a.enclosure(80, ctx).unwrap().unwrap();
    (b.re.to_f64(), b.im.to_f64())
}
#[test]
fn authority_root_numbering_puts_reals_first_and_negative_conjugate_first() {
    let ctx = Interrupt::default();
    for (f, expected) in [
        (z(&[1, 0, 1]), vec![(0., -1.), (0., 1.)]),
        (
            z(&[-2, 0, 0, 1]),
            vec![
                (1.2599210498948732, 0.),
                (-0.6299605249474366, -1.0911236359717214),
                (-0.6299605249474366, 1.0911236359717214),
            ],
        ),
        (
            z(&[1, -1, 0, 0, 0, 1]),
            vec![
                (-1.1673039782614187, 0.),
                (-0.1812324444698754, -1.0839541013177107),
                (-0.1812324444698754, 1.0839541013177107),
                (0.7648844336005847, -0.3524715460317262),
                (0.7648844336005847, 0.3524715460317262),
            ],
        ),
    ] {
        for (k, (re, im)) in expected.iter().enumerate() {
            let a = algebraic_root(&f, k + 1, &ctx).unwrap().unwrap();
            let got = approx(&a, &ctx);
            assert!(
                (got.0 - re).abs() < 1e-12 && (got.1 - im).abs() < 1e-12,
                "{k}: {got:?}"
            );
        }
    }
    // Equal real keys across irreducible factors are broken by |Im|.
    let f = z(&[1, 0, 1]).mul(&z(&[2, 0, 1]), &ctx).unwrap();
    for (k, im) in [-1., 1., -2_f64.sqrt(), 2_f64.sqrt()].iter().enumerate() {
        assert!(
            (approx(&algebraic_root(&f, k + 1, &ctx).unwrap().unwrap(), &ctx).1 - im).abs() < 1e-12
        );
    }
}
#[test]
fn complex_rootreduce_and_refinement_preserve_branch_and_index() {
    let ctx = Interrupt::default();
    let i = algebraic_root(&z(&[1, 0, 1]), 2, &ctx).unwrap().unwrap();
    match &i {
        Algebraic::Complex(ComplexAlg { minpoly, index, .. }) => {
            assert_eq!(minpoly, &z(&[1, 0, 1]));
            assert_eq!(*index, 2);
        }
        _ => panic!(),
    }
    assert_eq!(i.real_sign(&ctx).unwrap(), None);
    assert_eq!(i.cmp_real(&i, &ctx).unwrap(), None);
    assert_eq!(
        rational(&i.mul(&i, &ctx).unwrap().unwrap()),
        Rational::from(-1)
    );
    let minus_i = i.recip(&ctx).unwrap().unwrap();
    assert_eq!(
        rational(&i.add(&minus_i, &ctx).unwrap().unwrap()),
        Rational::ZERO
    );
    let one_plus_i = i
        .add(&Algebraic::Rational(Rational::ONE), &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(one_plus_i.minimal_polynomial(&ctx).unwrap(), z(&[2, -2, 1]));
    let ball = one_plus_i.enclosure(150, &ctx).unwrap().unwrap();
    assert!(
        ball.re
            .sub(&om_num::Ball::exact(&Rational::ONE, 150))
            .contains_zero()
    );
    assert!(
        ball.im
            .sub(&om_num::Ball::exact(&Rational::ONE, 150))
            .contains_zero()
    );
    assert_eq!(
        one_plus_i
            .sub(&Algebraic::Rational(Rational::ONE), &ctx)
            .unwrap()
            .unwrap()
            .equals(&i, &ctx)
            .unwrap(),
        Some(true)
    );
}
#[test]
fn realalg_contract_keeps_primitive_positive_leading_poly_and_exact_interval() {
    let ctx = Interrupt::default();
    let a = real(&z(&[6, 0, -3]), 1, 2, &ctx);
    match &a {
        Algebraic::Real(RealAlg { minpoly, iv }) => {
            assert_eq!(minpoly, &z(&[-2, 0, 1]));
            assert!(iv.0 >= Rational::ONE && iv.1 <= Rational::from(2));
        }
        _ => panic!(),
    }
}
#[test]
fn real_sign_and_comparison_refine_beyond_the_numeric_complex_precision_cap() {
    let ctx = Interrupt::default();
    let n = Integer::ONE << 17_000;
    let unit = Rational::ONE / Rational::from(n.clone());
    let a = Algebraic::Real(RealAlg {
        minpoly: UPoly::new(vec![Integer::from(-2), Integer::ZERO, &n * &n]),
        iv: (-&unit, &unit * Rational::from(2)),
    });
    assert_eq!(a.real_sign(&ctx).unwrap(), Some(1));
    assert_eq!(
        a.cmp_real(&Algebraic::Rational(Rational::ZERO), &ctx)
            .unwrap(),
        Some(Ordering::Greater)
    );
}
#[test]
fn nonmonic_rational_shifts_and_mixed_complex_arithmetic_select_correct_branches() {
    let ctx = Interrupt::default();
    let a = real(&z(&[-2, 0, 9]), 0, 1, &ctx);
    let half = Algebraic::Rational(Rational::ONE / Rational::from(2));
    let shifted = a.add(&half, &ctx).unwrap().unwrap();
    assert_eq!(shifted.minimal_polynomial(&ctx).unwrap(), z(&[1, -36, 36]));
    assert_eq!(
        shifted
            .sub(&half, &ctx)
            .unwrap()
            .unwrap()
            .equals(&a, &ctx)
            .unwrap(),
        Some(true)
    );
    assert_eq!(
        rational(&shifted.div(&shifted, &ctx).unwrap().unwrap()),
        Rational::ONE
    );
    let sqrt2 = real(&z(&[-2, 0, 1]), 1, 2, &ctx);
    let i = algebraic_root(&z(&[1, 0, 1]), 2, &ctx).unwrap().unwrap();
    let mixed = sqrt2.add(&i, &ctx).unwrap().unwrap();
    assert_eq!(
        mixed.minimal_polynomial(&ctx).unwrap(),
        z(&[9, 0, -2, 0, 1])
    );
    let (re, im) = approx(&mixed, &ctx);
    assert!((re - 2_f64.sqrt()).abs() < 1e-12 && (im - 1.).abs() < 1e-12);
    assert_eq!(
        mixed
            .sub(&i, &ctx)
            .unwrap()
            .unwrap()
            .equals(&sqrt2, &ctx)
            .unwrap(),
        Some(true)
    );
    assert_eq!(
        rational(&i.pow_int(&Integer::from(-2), &ctx).unwrap().unwrap()),
        Rational::from(-1)
    );
}
#[test]
fn degree_cap_unknown_and_interrupts_are_honest() {
    let ctx = Interrupt::default();
    let mut c = vec![0; 10];
    c[0] = -2;
    c[9] = 1;
    let a = real(&z(&c), 1, 2, &ctx);
    assert!(a.mul(&a, &ctx).unwrap().is_none());
    ctx.steps_left.set(0);
    assert!(matches!(a.add(&a, &ctx), Err(Abort::Budget)));
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        algebraic_root(&z(&[]), 0, &ctx),
        Err(Abort::Interrupted)
    ));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:24,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d362e37),..ProptestConfig::default()})]
 #[test]
 fn square_and_opposite_cancel_for_planted_quadratics(d in prop::sample::select(vec![2i64,3,5,6,7]),scale in 1i64..=7) {
  let ctx=Interrupt::default();let a=real(&z(&[-d*scale,0,scale]),1,3,&ctx);
  prop_assert_eq!(rational(&a.mul(&a,&ctx).unwrap().unwrap()),Rational::from(d));
  prop_assert_eq!(rational(&a.add(&a.neg(&ctx).unwrap().unwrap(),&ctx).unwrap().unwrap()),Rational::ZERO);
 }
}
