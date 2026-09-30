//! Quadratic Hensel certificates, balanced factor-tree lifting and bad-prime rejection.
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
use om_poly::{FpElem, HenselPair, UPoly, hensel_lift, hensel_step};
use proptest::prelude::*;
fn z(c: &[i64]) -> UPoly<Integer> {
    UPoly::new(c.iter().map(|a| (*a).into()).collect())
}
fn fp(c: &[i64], p: u64) -> UPoly<FpElem> {
    UPoly::new(
        c.iter()
            .map(|a| FpElem::new((*a as i128).rem_euclid(p as i128) as u64, p).unwrap())
            .collect(),
    )
}
fn integer(f: &UPoly<FpElem>) -> UPoly<Integer> {
    UPoly::new(f.coeffs.iter().map(|a| Integer::from(a.v)).collect())
}
fn modulo(f: &UPoly<Integer>, m: &Integer) -> UPoly<Integer> {
    UPoly::new(
        f.coeffs
            .iter()
            .map(|c| {
                let r = c % m;
                if r < Integer::ZERO { r + m } else { r }
            })
            .collect(),
    )
}
fn product(factors: &[UPoly<Integer>], ctx: &Interrupt) -> UPoly<Integer> {
    factors
        .iter()
        .fold(UPoly::one(), |a, b| a.mul(b, ctx).unwrap())
}
fn certificate(target: &UPoly<Integer>, pair: &HenselPair, m: &Integer, ctx: &Interrupt) {
    assert!(
        modulo(
            &target.sub(&pair.g.mul(&pair.h, ctx).unwrap(), ctx).unwrap(),
            m
        )
        .is_zero()
    );
    let bezout = pair
        .s
        .mul(&pair.g, ctx)
        .unwrap()
        .add(&pair.t.mul(&pair.h, ctx).unwrap(), ctx)
        .unwrap();
    assert_eq!(modulo(&bezout, m), UPoly::one());
    assert_eq!(pair.g.lc(), target.lc());
    assert_eq!(pair.h.lc(), Some(&Integer::ONE));
}
#[test]
fn quadratic_hensel_steps_preserve_product_and_bezout_at_squared_moduli() {
    let ctx = Interrupt::default();
    let f = z(&[-1, 0, 0, 0, 1]);
    let mut pair = HenselPair {
        g: z(&[-1, 0, 1]),
        h: z(&[1, 0, 1]),
        s: z(&[2]),
        t: z(&[3]),
    };
    let mut m = Integer::from(5);
    certificate(&f, &pair, &m, &ctx);
    for _ in 0..4 {
        let next = hensel_step(&m, &f, &pair.g, &pair.h, &pair.s, &pair.t, &ctx)
            .unwrap()
            .unwrap();
        let squared = &m * &m;
        certificate(&f, &next, &squared, &ctx);
        assert_eq!(modulo(&next.g, &m), modulo(&pair.g, &m));
        assert_eq!(modulo(&next.h, &m), modulo(&pair.h, &m));
        pair = next;
        m = squared;
    }
}
#[test]
fn nonmonic_integer_lift_retains_original_finite_factors() {
    let ctx = Interrupt::default();
    let f = z(&[-2, 1, 6]);
    let factors = vec![fp(&[2, 1], 5), fp(&[4, 1], 5)];
    let lifted = hensel_lift(5, &f, &factors, 4, &ctx).unwrap().unwrap();
    assert_eq!(lifted, vec![z(&[312, 1]), z(&[209, 1])]);
    let modulus = Integer::from(625);
    assert_eq!(
        modulo(
            &product(&lifted, &ctx).scale(f.lc().unwrap(), &ctx).unwrap(),
            &modulus
        ),
        modulo(&f, &modulus)
    );
    let neg = f.neg(&ctx).unwrap();
    assert_eq!(
        hensel_lift(5, &neg, &factors, 4, &ctx).unwrap().unwrap(),
        lifted
    );
}
#[test]
fn balanced_multifactor_tree_single_factor_and_non_power_of_two_exponents() {
    let ctx = Interrupt::default();
    let factors = (0..5).map(|i| fp(&[-i, 1], 7)).collect::<Vec<_>>();
    let integer_factors = (0..5).map(|i| z(&[-i, 1])).collect::<Vec<_>>();
    let f = product(&integer_factors, &ctx)
        .scale(&Integer::from(6), &ctx)
        .unwrap();
    for exponent in [1, 2, 3, 5, 12] {
        let m = Integer::from(7).pow(exponent as usize);
        let lifted = hensel_lift(7, &f, &factors, exponent, &ctx)
            .unwrap()
            .unwrap();
        assert_eq!(lifted.len(), 5);
        assert_eq!(
            modulo(
                &product(&lifted, &ctx).scale(f.lc().unwrap(), &ctx).unwrap(),
                &m
            ),
            modulo(&f, &m)
        );
        for (a, b) in lifted.iter().zip(&integer_factors) {
            assert_eq!(*a, modulo(b, &m));
        }
    }
    let f = z(&[3, 0, 2]);
    let factors = vec![fp(&[4, 0, 1], 5)];
    let lifted = hensel_lift(5, &f, &factors, 3, &ctx).unwrap().unwrap();
    assert_eq!(lifted, vec![z(&[64, 0, 1])]);
    assert_eq!(
        hensel_lift(5, &z(&[2]), &[], 3, &ctx).unwrap(),
        Some(vec![])
    );
    let f = z(&[0, 1, 1]);
    let factors = vec![fp(&[0, 1], 2), fp(&[1, 1], 2)];
    assert!(hensel_lift(2, &f, &factors, 10, &ctx).unwrap().is_some());
}
#[test]
fn field_extended_gcd_produces_monic_bezout_certificate() {
    let ctx = Interrupt::default();
    let f = fp(&[-1, 0, 1], 5);
    let g = fp(&[1, 0, 1], 5);
    let (h, s, t) = f.extended_gcd(&g, &ctx).unwrap().unwrap();
    assert!(h.is_one());
    assert_eq!(
        s.mul(&f, &ctx)
            .unwrap()
            .add(&t.mul(&g, &ctx).unwrap(), &ctx)
            .unwrap(),
        h
    );
    let f = fp(&[2, 4, 2], 5);
    let g = fp(&[4, 4], 5);
    let (h, s, t) = f.extended_gcd(&g, &ctx).unwrap().unwrap();
    assert_eq!(h, fp(&[1, 1], 5));
    assert_eq!(
        s.mul(&f, &ctx)
            .unwrap()
            .add(&t.mul(&g, &ctx).unwrap(), &ctx)
            .unwrap(),
        h
    );
    assert_eq!(
        UPoly::<FpElem>::zero()
            .extended_gcd(&UPoly::zero(), &ctx)
            .unwrap(),
        Some((UPoly::zero(), UPoly::zero(), UPoly::zero()))
    );
}
#[test]
fn invalid_hensel_inputs_reject_singular_primes_or_broken_certificates() {
    let ctx = Interrupt::default();
    let f = z(&[-2, 1, 6]);
    let factors = vec![fp(&[2, 1], 5), fp(&[4, 1], 5)];
    assert_eq!(hensel_lift(5, &f, &factors, 0, &ctx).unwrap(), None);
    assert_eq!(hensel_lift(4, &f, &[], 2, &ctx).unwrap(), None);
    assert_eq!(
        hensel_lift(3, &f, &[fp(&[1, 1], 3)], 2, &ctx).unwrap(),
        None
    );
    assert_eq!(
        hensel_lift(5, &f, &[fp(&[1, 1], 5), fp(&[4, 1], 5)], 2, &ctx).unwrap(),
        None
    );
    assert_eq!(
        hensel_lift(
            5,
            &z(&[1, 2, 1]),
            &[fp(&[1, 1], 5), fp(&[1, 1], 5)],
            2,
            &ctx
        )
        .unwrap(),
        None
    );
    assert_eq!(
        hensel_lift(5, &f, &[fp(&[2, 1], 7), fp(&[4, 1], 7)], 2, &ctx).unwrap(),
        None
    );
    assert_eq!(hensel_lift(5, &UPoly::zero(), &[], 2, &ctx).unwrap(), None);
    assert_eq!(hensel_lift(5, &f, &[], 2, &ctx).unwrap(), None);
    assert_eq!(
        hensel_step(
            &5.into(),
            &z(&[-1, 0, 0, 0, 1]),
            &z(&[-1, 0, 1]),
            &z(&[1, 0, 1]),
            &z(&[0]),
            &z(&[0]),
            &ctx
        )
        .unwrap(),
        None
    );
    assert_eq!(
        hensel_step(
            &Integer::ONE,
            &z(&[-1, 0, 0, 0, 1]),
            &z(&[-1, 0, 1]),
            &z(&[1, 0, 1]),
            &z(&[2]),
            &z(&[3]),
            &ctx
        )
        .unwrap(),
        None
    );
}
#[test]
fn big_modulus_lifts_and_cancellation_are_portable_and_bounded() {
    let ctx = Interrupt::default();
    let f = z(&[-2, 1, 6]);
    let factors = vec![fp(&[2, 1], 5), fp(&[4, 1], 5)];
    let m = Integer::from(5).pow(50);
    let lifted = hensel_lift(5, &f, &factors, 50, &ctx).unwrap().unwrap();
    assert_eq!(
        modulo(
            &product(&lifted, &ctx).scale(f.lc().unwrap(), &ctx).unwrap(),
            &m
        ),
        modulo(&f, &m)
    );
    for steps in [0, 5, 40] {
        ctx.steps_left.set(steps);
        assert_eq!(hensel_lift(5, &f, &factors, 5, &ctx), Err(Abort::Budget));
    }
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        hensel_step(
            &5.into(),
            &f,
            &z(&[1, 6]),
            &z(&[1, 1]),
            &z(&[0]),
            &z(&[1]),
            &ctx
        ),
        Err(Abort::Interrupted)
    );
}
proptest! {
    #![proptest_config(ProptestConfig {cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d362e32),..ProptestConfig::default()})]
    #[test]
    fn random_prime_hensel_products_reconstruct_and_preserve_residues(
        prime_index in 0usize..4, exponent in 1u32..12,
        a in -20i64..=20, b in -20i64..=20, c in -20i64..=20,
        lc in 1i64..12
    ) {
        let ctx=Interrupt::default();
        let p=[3,5,7,11][prime_index];
        let ps=p as i64;
        prop_assume!(lc%ps!=0);
        prop_assume!(a.rem_euclid(ps)!=b.rem_euclid(ps) && a.rem_euclid(ps)!=c.rem_euclid(ps) && b.rem_euclid(ps)!=c.rem_euclid(ps));
        let integer_factors=vec![z(&[-a,1]),z(&[-b,1]),z(&[-c,1])];
        let factors=vec![fp(&[-a,1],p),fp(&[-b,1],p),fp(&[-c,1],p)];
        let f=product(&integer_factors,&ctx).scale(&lc.into(),&ctx).unwrap();
        let lifted=hensel_lift(p,&f,&factors,exponent,&ctx).unwrap().unwrap();
        let modulus=Integer::from(p).pow(exponent as usize);
        prop_assert_eq!(modulo(&product(&lifted,&ctx).scale(f.lc().unwrap(),&ctx).unwrap(),&modulus),modulo(&f,&modulus));
        for (a,b) in lifted.iter().zip(&factors) {prop_assert_eq!(modulo(a,&p.into()),integer(b));}
    }
}

#[test]
fn hensel_step_retains_a_leading_coefficient_that_vanishes_in_the_residue() {
    let ctx = Interrupt::default();
    let g = z(&[1, 25]);
    let h = z(&[1, 1]);
    let f = g.mul(&h, &ctx).unwrap();
    let pair = hensel_step(&5.into(), &f, &g, &h, &z(&[1]), &z(&[]), &ctx)
        .unwrap()
        .unwrap();
    certificate(&f, &pair, &25.into(), &ctx);
    assert_eq!(pair.g, g);
}

proptest! {
    #![proptest_config(ProptestConfig {cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x48454e53),..ProptestConfig::default()})]
    #[test]
    fn arbitrary_integer_inputs_lift_their_actual_modular_factorization(
        coefficients in prop::collection::vec(-12i64..=12,2..8),
        prime_index in 0usize..4, exponent in 1u32..10
    ) {
        let ctx=Interrupt::default();
        let p=[3,5,7,11][prime_index];
        let polynomial=z(&coefficients);
        let ff=fp(&coefficients,p);
        prop_assume!(polynomial.degree().is_some_and(|d|d>0) && ff.degree()==polynomial.degree());
        prop_assume!(ff.monic_gcd(&ff.derivative(&ctx).unwrap(),&ctx).unwrap().unwrap().is_one());
        let mut factors=vec![];
        for (group,d) in ff.ddf(&ctx).unwrap().unwrap() {factors.extend(group.edf(d,0x48454e53,&ctx).unwrap().unwrap());}
        let lifted=hensel_lift(p,&polynomial,&factors,exponent,&ctx).unwrap().unwrap();
        let modulus=Integer::from(p).pow(exponent as usize);
        prop_assert_eq!(modulo(&product(&lifted,&ctx).scale(polynomial.lc().unwrap(),&ctx).unwrap(),&modulus),modulo(&polynomial,&modulus));
        for (lifted,original) in lifted.iter().zip(&factors) {
            prop_assert_eq!(modulo(lifted,&p.into()),integer(original));
            prop_assert_eq!(lifted.lc(),Some(&Integer::ONE));
            prop_assert_eq!(lifted.degree(),original.degree());
        }
    }
}
