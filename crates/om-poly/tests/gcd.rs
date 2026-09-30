//! Integer GCD authority vectors and deterministic agreement of independent paths.
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{MPoly, MonoOrder, Monomial, UPoly};
use proptest::prelude::*;

fn z(c: &[i64]) -> UPoly<Integer> {
    UPoly::new(c.iter().copied().map(Integer::from).collect())
}
fn m(n: usize, terms: &[(Vec<u32>, i64)], order: MonoOrder) -> MPoly<Integer> {
    MPoly::new(
        n,
        terms
            .iter()
            .map(|(e, c)| (Monomial::new(e.clone()).unwrap(), (*c).into()))
            .collect(),
        order,
        &Interrupt::default(),
    )
    .unwrap()
}
fn mul(a: &MPoly<Integer>, b: &MPoly<Integer>) -> MPoly<Integer> {
    a.mul(b, &Interrupt::default()).unwrap().unwrap()
}

#[test]
fn authority_univariate_gcd_vectors_and_zero_content_edges() {
    let ctx = Interrupt::default();
    for (f, g, h) in [
        (z(&[-1, 0, 1]), z(&[2, -3, 1]), z(&[-1, 1])),
        (z(&[2, 4, 2]), z(&[4, 4]), z(&[2, 2])),
        (
            z(&[-1, 0, 0, 0, 1]),
            z(&[-1, 0, 0, 0, 0, 0, 1]),
            z(&[-1, 0, 1]),
        ),
        (z(&[0, 1]), z(&[]), z(&[0, 1])),
        (z(&[]), z(&[]), z(&[])),
        (z(&[-18]), z(&[24]), z(&[6])),
        (z(&[-12]), z(&[6, 18]), z(&[6])),
        (z(&[]), z(&[3, -6]), z(&[-3, 6])),
        (z(&[-2, -4, -2]), z(&[-4, -4]), z(&[2, 2])),
        (z(&[1, 0, 1]), z(&[1, 1]), z(&[1])),
    ] {
        for actual in [
            f.gcdheu(&g, &ctx).unwrap(),
            f.subresultant_gcd(&g, &ctx).unwrap(),
            g.gcdheu(&f, &ctx).unwrap(),
        ] {
            assert_eq!(actual, h);
            if !h.is_zero() {
                assert!(f.exact_div(&h, &ctx).unwrap().is_some());
                assert!(g.exact_div(&h, &ctx).unwrap().is_some());
            }
        }
    }
}

#[test]
fn recursive_multivariate_gcd_preserves_coefficient_contents_and_orders() {
    let ctx = Interrupt::default();
    for order in [MonoOrder::Lex, MonoOrder::GrevLex] {
        let f = m(2, &[(vec![2, 0], 1), (vec![0, 2], -1)], order);
        let g = m(
            2,
            &[(vec![2, 0], 1), (vec![1, 1], 2), (vec![0, 2], 1)],
            order,
        );
        let h = m(2, &[(vec![1, 0], 1), (vec![0, 1], 1)], order);
        assert_eq!(f.gcdheu(&g, &ctx).unwrap(), h);
        assert_eq!(f.subresultant_gcd(&g, &ctx).unwrap(), h);
        assert!(f.exact_div(&h, &ctx).unwrap().is_some());
        assert!(g.exact_div(&h, &ctx).unwrap().is_some());
        // A factor independent of the last variable belongs to recursive content.
        let h = m(3, &[(vec![1, 0, 0], 6), (vec![0, 1, 0], 6)], order);
        let a = m(
            3,
            &[(vec![0, 0, 2], 2), (vec![1, 0, 0], 1), (vec![0, 0, 0], 1)],
            order,
        );
        let b = m(3, &[(vec![0, 0, 1], 3), (vec![0, 0, 0], 1)], order);
        let (f, g) = (mul(&h, &a), mul(&h, &b));
        assert_eq!(f.gcdheu(&g, &ctx).unwrap(), h);
        assert_eq!(f.subresultant_gcd(&g, &ctx).unwrap(), h);
        assert_eq!(f.exact_div(&h, &ctx).unwrap(), Some(a));
        assert_eq!(g.exact_div(&h, &ctx).unwrap(), Some(b));
        assert_eq!(h.gcdheu(&MPoly::zero(), &ctx).unwrap(), h);
    }
}

#[test]
fn dense_parameter_coefficients_share_the_recursive_integer_gcd() {
    let ctx = Interrupt::default();
    let x = m(1, &[(vec![1], 1)], MonoOrder::Lex);
    let x2 = mul(&x, &x);
    let f = UPoly::new(vec![
        x2.clone(),
        MPoly::zero(),
        MPoly::one().neg(&ctx).unwrap(),
    ]);
    let g = UPoly::new(vec![x2, x.add(&x, &ctx).unwrap(), MPoly::one()]);
    let h = UPoly::new(vec![x, MPoly::one()]);
    assert_eq!(f.gcdheu(&g, &ctx).unwrap(), h);
    assert_eq!(f.subresultant_gcd(&g, &ctx).unwrap(), h);
}

#[test]
fn subresultant_degree_gaps_nonmonic_and_large_integer_coefficients() {
    let ctx = Interrupt::default();
    let h = z(&[3, -2, 5]);
    let a = z(&[-5, 2, 0, 8, -3, 0, 1, 0, 1]);
    let b = z(&[21, -9, -4, 0, 5, 0, 3]);
    let (f, g) = (h.mul(&a, &ctx).unwrap(), h.mul(&b, &ctx).unwrap());
    assert_eq!(f.subresultant_gcd(&g, &ctx).unwrap(), h);
    assert_eq!(f.gcdheu(&g, &ctx).unwrap(), h);
    let big = Integer::ONE << 180;
    let f = z(&[1, 1]).scale(&(&big * 6), &ctx).unwrap();
    let g = z(&[1, 1]).scale(&(&big * 15), &ctx).unwrap();
    let h = z(&[1, 1]).scale(&(&big * 3), &ctx).unwrap();
    assert_eq!(f.gcdheu(&g, &ctx).unwrap(), h);
    assert_eq!(f.subresultant_gcd(&g, &ctx).unwrap(), h);
}

#[test]
fn sparse_exact_division_rejects_nondivisibility_and_zero_divisors() {
    let ctx = Interrupt::default();
    let f = m(2, &[(vec![2, 0], 1), (vec![0, 2], -1)], MonoOrder::Lex);
    let g = m(2, &[(vec![1, 0], 2), (vec![0, 1], 2)], MonoOrder::Lex);
    assert_eq!(f.exact_div(&g, &ctx).unwrap(), None);
    assert_eq!(f.exact_div(&MPoly::zero(), &ctx).unwrap(), None);
    assert_eq!(
        MPoly::zero().exact_div(&f, &ctx).unwrap(),
        Some(MPoly::zero())
    );
}

#[test]
fn gcd_paths_and_sparse_division_propagate_budget_and_cancellation() {
    for steps in [0, 1, 7, 30] {
        let ctx = Interrupt::default();
        ctx.steps_left.set(steps);
        assert_eq!(z(&[1; 20]).gcdheu(&z(&[1; 15]), &ctx), Err(Abort::Budget));
        ctx.steps_left.set(steps);
        assert_eq!(
            z(&[1; 20]).subresultant_gcd(&z(&[1; 15]), &ctx),
            Err(Abort::Budget)
        );
    }
    let ctx = Interrupt::default();
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(z(&[]).gcdheu(&z(&[]), &ctx), Err(Abort::Interrupted));
    let f = m(2, &[(vec![1, 1], 1)], MonoOrder::Lex);
    assert_eq!(f.exact_div(&f, &ctx), Err(Abort::Interrupted));
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: proptest::test_runner::RngSeed::Fixed(0x4d352e33), ..ProptestConfig::default() })]
    #[test]
    fn univariate_paths_agree_and_scaling_preserves_gcd(
        f in prop::collection::vec(-8i64..=8,0..8),
        g in prop::collection::vec(-8i64..=8,0..8),
        h in prop::collection::vec(-4i64..=4,1..5)
    ) {
        let ctx=Interrupt::default();
        let (f,g,h)=(z(&f),z(&g),z(&h));
        let (f,g)=(f.mul(&h,&ctx).unwrap(),g.mul(&h,&ctx).unwrap());
        let gcd=f.gcdheu(&g,&ctx).unwrap();
        prop_assert_eq!(&gcd,&f.subresultant_gcd(&g,&ctx).unwrap());
        if !gcd.is_zero() {
            prop_assert!(f.exact_div(&gcd,&ctx).unwrap().is_some());
            prop_assert!(g.exact_div(&gcd,&ctx).unwrap().is_some());
            prop_assert!(gcd.lc().unwrap()>&Integer::ZERO);
        }
        prop_assert_eq!(f.neg(&ctx).unwrap().gcdheu(&g,&ctx).unwrap(),gcd.clone());
        let scaled_content=om_num::gcd(&(f.content(&ctx).unwrap()*6),&(g.content(&ctx).unwrap()*9));
        prop_assert_eq!(f.scale(&(-6).into(),&ctx).unwrap().gcdheu(&g.scale(&9.into(),&ctx).unwrap(),&ctx).unwrap(),gcd.primitive_part(&ctx).unwrap().scale(&scaled_content,&ctx).unwrap());
        if !gcd.is_zero() {
            // A separate field Euclidean algorithm detects proper common divisors.
            let lift=|p:&UPoly<Integer>| UPoly::new(p.coeffs.iter().map(|c|Rational::from(c.clone())).collect());
            let (mut a,mut b)=(lift(&f),lift(&g));
            while !b.is_zero() {let r=a.divrem(&b,&ctx).unwrap().unwrap().1;a=b;b=r;}
            let reference=a.scale(&(Rational::ONE/a.lc().unwrap()),&ctx).unwrap();
            let actual=lift(&gcd);
            prop_assert_eq!(actual.scale(&(Rational::ONE/actual.lc().unwrap()),&ctx).unwrap(),reference);
        }
    }
    #[test]
    fn sparse_paths_agree_with_planted_multivariate_factors(
        fa in prop::collection::vec((0u32..3,0u32..3,-3i64..=3),0..7),
        ga in prop::collection::vec((0u32..3,0u32..3,-3i64..=3),0..7),
        hc in -3i64..=3, grevlex in any::<bool>()
    ) {
        let ctx=Interrupt::default();
        let order=if grevlex {MonoOrder::GrevLex} else {MonoOrder::Lex};
        let convert=|v: Vec<(u32,u32,i64)>| m(2,&v.into_iter().map(|(x,y,c)|(vec![x,y],c)).collect::<Vec<_>>(),order);
        let h=m(2,&[(vec![1,0],1),(vec![0,1],1),(vec![0,0],hc)],order);
        let (f,g)=(mul(&convert(fa),&h),mul(&convert(ga),&h));
        let gcd=f.gcdheu(&g,&ctx).unwrap();
        prop_assert_eq!(&gcd,&f.subresultant_gcd(&g,&ctx).unwrap());
        if !gcd.is_zero() {
            prop_assert!(f.exact_div(&gcd,&ctx).unwrap().is_some());
            prop_assert!(g.exact_div(&gcd,&ctx).unwrap().is_some());
        }
    }
}

#[test]
fn planted_coprime_cofactors_verify_multivariate_gcd_maximality() {
    let ctx = Interrupt::default();
    for c in -4..=4 {
        for d in -4..=4 {
            let h = m(
                3,
                &[(vec![1, 1, 1], 2), (vec![0, 1, 0], -3), (vec![0, 0, 0], 5)],
                MonoOrder::Lex,
            );
            let a = m(
                3,
                &[(vec![1, 0, 0], 6), (vec![0, 0, 0], 6 * c)],
                MonoOrder::Lex,
            );
            let b = m(
                3,
                &[(vec![0, 0, 1], -9), (vec![0, 0, 0], -9 * d)],
                MonoOrder::Lex,
            );
            let (f, g) = (mul(&h, &a), mul(&h, &b));
            let want = mul(&h, &m(3, &[(vec![0, 0, 0], 3)], MonoOrder::Lex));
            assert_eq!(f.gcdheu(&g, &ctx).unwrap(), want);
            assert_eq!(f.subresultant_gcd(&g, &ctx).unwrap(), want);
        }
    }
}
