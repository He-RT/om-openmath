//! Zassenhaus authority vectors and factor products including signed content/multiplicity.
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
use om_poly::{FactorStatus, IntegerFactorization, UPoly};
use proptest::prelude::*;
fn z(c: &[i64]) -> UPoly<Integer> {
    UPoly::new(c.iter().map(|a| (*a).into()).collect())
}
fn reconstruct(f: &IntegerFactorization, ctx: &Interrupt) -> UPoly<Integer> {
    let mut out = UPoly::new(vec![f.content.clone()]);
    for (g, m) in &f.factors {
        for _ in 0..*m {
            out = out.mul(g, ctx).unwrap();
        }
    }
    out
}
#[test]
fn authority_zassenhaus_integer_factor_vectors() {
    let ctx = Interrupt::default();
    for (polynomial, factors) in [
        (
            z(&[4, 0, 0, 0, 1]),
            vec![(z(&[2, -2, 1]), 1), (z(&[2, 2, 1]), 1)],
        ),
        (
            z(&[-1, 0, 0, 0, 0, 0, 1]),
            vec![
                (z(&[-1, 1]), 1),
                (z(&[1, 1]), 1),
                (z(&[1, -1, 1]), 1),
                (z(&[1, 1, 1]), 1),
            ],
        ),
        (z(&[-2, 1, 6]), vec![(z(&[-1, 2]), 1), (z(&[2, 3]), 1)]),
        (z(&[1, 0, 0, 0, 1]), vec![(z(&[1, 0, 0, 0, 1]), 1)]),
        (z(&[1, 0, -10, 0, 1]), vec![(z(&[1, 0, -10, 0, 1]), 1)]),
    ] {
        let actual = polynomial.factor_z(&ctx).unwrap().unwrap();
        assert_eq!(
            actual,
            IntegerFactorization {
                content: Integer::ONE,
                factors,
                status: FactorStatus::Complete
            }
        );
        assert_eq!(reconstruct(&actual, &ctx), polynomial);
    }
}
#[test]
fn content_zero_constants_and_extracted_monomial_multiplicities() {
    let ctx = Interrupt::default();
    assert_eq!(z(&[]).factor_z(&ctx).unwrap(), None);
    assert_eq!(
        z(&[-7]).factor_z(&ctx).unwrap(),
        Some(IntegerFactorization {
            content: (-7).into(),
            factors: vec![],
            status: FactorStatus::Complete
        })
    );
    assert_eq!(
        z(&[0, 0, 0, -6]).factor_z(&ctx).unwrap(),
        Some(IntegerFactorization {
            content: (-6).into(),
            factors: vec![(z(&[0, 1]), 3)],
            status: FactorStatus::Complete
        })
    );
    let want = IntegerFactorization {
        content: (-12).into(),
        factors: vec![(z(&[-1, 2]), 2), (z(&[0, 1]), 3), (z(&[2, 3]), 4)],
        status: FactorStatus::Complete,
    };
    let f = reconstruct(&want, &ctx);
    assert_eq!(f.factor_z(&ctx).unwrap(), Some(want));
}
#[test]
fn mignotte_bound_is_exact_and_sign_independent() {
    let ctx = Interrupt::default();
    assert_eq!(
        z(&[-2, 1, 6]).mignotte_bound(&ctx).unwrap(),
        Integer::from(288)
    );
    assert_eq!(
        z(&[2, -1, -6]).mignotte_bound(&ctx).unwrap(),
        Integer::from(288)
    );
    assert_eq!(z(&[]).mignotte_bound(&ctx).unwrap(), Integer::ZERO);
    assert_eq!(z(&[3]).mignotte_bound(&ctx).unwrap(), Integer::from(18));
}
#[test]
fn large_integer_coefficients_and_bad_small_primes_still_factor() {
    let ctx = Interrupt::default();
    let big = (Integer::ONE << 90) + Integer::from(123);
    let a = UPoly::new(vec![big.clone(), Integer::ONE]);
    let b = UPoly::new(vec![&big + 1, Integer::ONE]);
    let f = a.mul(&b, &ctx).unwrap();
    let actual = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(actual.factors, vec![(a, 1), (b, 1)]);
    assert_eq!(actual.status, FactorStatus::Complete);
    assert_eq!(reconstruct(&actual, &ctx), f);
    let a = z(&[1, 1155]); // lc divisible by 3,5,7,11
    let b = z(&[2, 1]);
    let f = a.mul(&b, &ctx).unwrap();
    let actual = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(actual.factors, vec![(a, 1), (b, 1)]);
}
#[test]
fn fixed_seeds_and_cancelled_searches_are_reproducible() {
    let ctx = Interrupt::default();
    let f = z(&[-1, 0, 0, 0, 0, 0, 1]);
    assert_eq!(f.factor_z(&ctx).unwrap(), f.factor_z(&ctx).unwrap());
    for steps in [0, 5, 100] {
        ctx.steps_left.set(steps);
        assert_eq!(f.factor_z(&ctx), Err(Abort::Budget));
    }
    ctx.steps_left.set(0);
    assert_eq!(z(&[1]).mignotte_bound(&ctx), Err(Abort::Budget));
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(z(&[]).factor_z(&ctx), Err(Abort::Interrupted));
}
proptest! {
    #![proptest_config(ProptestConfig {cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d362e33),..ProptestConfig::default()})]
    #[test]
    fn generated_polynomial_products_reconstruct_with_exact_divisors(
        polynomials in prop::collection::vec(prop::collection::vec(-5i64..=5,2..5),2..5),
        content in -12i64..=12
    ) {
        let ctx=Interrupt::default();
        let mut f=z(&[content]);
        for coefficients in polynomials {f=f.mul(&z(&coefficients),&ctx).unwrap();}
        match f.factor_z(&ctx).unwrap() {
            None=>prop_assert!(f.is_zero()),
            Some(actual)=> {
                prop_assert_eq!(reconstruct(&actual,&ctx),f.clone());
                prop_assert_eq!(actual.status,FactorStatus::Complete);
                for (g,m) in &actual.factors {
                    prop_assert!(*m>0);
                    prop_assert!(g.degree().is_some_and(|d|d>0));
                    prop_assert_eq!(g.content(&ctx).unwrap(),Integer::ONE);
                    prop_assert!(f.exact_div(g,&ctx).unwrap().is_some());
                    prop_assert!(g.gcdheu(&g.derivative(&ctx).unwrap(),&ctx).unwrap().is_one());
                }
            }
        }
    }
}
