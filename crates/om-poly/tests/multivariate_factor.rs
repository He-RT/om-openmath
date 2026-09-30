//! Kronecker reconstruction preserves contexts, content, multiplicities and bounded status.
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
use om_poly::{FactorStatus, MPoly, MonoOrder, Monomial, MultivariateFactorization};
use proptest::prelude::*;
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
fn multiply(a: &MPoly<Integer>, b: &MPoly<Integer>, ctx: &Interrupt) -> MPoly<Integer> {
    a.mul(b, ctx).unwrap().unwrap()
}
fn reconstruct(
    f: &MultivariateFactorization,
    n: usize,
    order: MonoOrder,
    ctx: &Interrupt,
) -> MPoly<Integer> {
    let mut out = m(
        n,
        &[(vec![0; n], i64::try_from(&f.content).unwrap())],
        order,
    );
    for (g, m) in &f.factors {
        for _ in 0..*m {
            out = multiply(&out, g, ctx);
        }
    }
    out
}
#[test]
fn authority_multivariate_factor_vectors_in_both_orders() {
    let ctx = Interrupt::default();
    for order in [MonoOrder::Lex, MonoOrder::GrevLex] {
        let x = m(2, &[(vec![1, 0], 1)], order);
        let y = m(2, &[(vec![0, 1], 1)], order);
        let minus = m(2, &[(vec![1, 0], 1), (vec![0, 1], -1)], order);
        let plus = m(2, &[(vec![1, 0], 1), (vec![0, 1], 1)], order);
        let quadratic = m(
            2,
            &[(vec![2, 0], 1), (vec![1, 1], 1), (vec![0, 2], 1)],
            order,
        );
        for (f, want) in [
            (
                m(2, &[(vec![2, 0], 1), (vec![0, 2], -1)], order),
                vec![(minus.clone(), 1), (plus.clone(), 1)],
            ),
            (
                m(2, &[(vec![3, 0], 1), (vec![0, 3], -1)], order),
                vec![(minus.clone(), 1), (quadratic, 1)],
            ),
            (
                m(2, &[(vec![2, 1], 1), (vec![1, 2], 1)], order),
                vec![(x, 1), (y, 1), (plus, 1)],
            ),
        ] {
            let actual = f.factor_z(&ctx).unwrap().unwrap();
            assert_eq!(actual.status, FactorStatus::Complete);
            assert_eq!(actual.content, Integer::ONE);
            assert_eq!(actual.factors.len(), want.len());
            for factor in want {
                assert!(actual.factors.contains(&factor), "{actual:?}");
            }
            assert_eq!(reconstruct(&actual, 2, order, &ctx), f);
        }
    }
}
#[test]
fn signed_content_repeated_factors_monomials_constants_and_zero() {
    let ctx = Interrupt::default();
    let order = MonoOrder::Lex;
    let a = m(2, &[(vec![1, 0], 1), (vec![0, 1], -1)], order);
    let b = m(2, &[(vec![1, 0], 2), (vec![0, 1], 3)], order);
    let mut f = m(2, &[(vec![2, 1], -12)], order);
    for _ in 0..2 {
        f = multiply(&f, &a, &ctx);
    }
    for _ in 0..3 {
        f = multiply(&f, &b, &ctx);
    }
    let result = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(result.content, Integer::from(-12));
    assert_eq!(result.status, FactorStatus::Complete);
    assert!(result.factors.contains(&(a, 2)));
    assert!(result.factors.contains(&(b, 3)));
    assert_eq!(reconstruct(&result, 2, order, &ctx), f);
    assert_eq!(MPoly::<Integer>::zero().factor_z(&ctx).unwrap(), None);
    let f = m(3, &[(vec![0, 0, 0], -7)], order);
    let result = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(result.content, Integer::from(-7));
    assert!(result.factors.is_empty());
    assert_eq!(result.status, FactorStatus::Complete);
}
#[test]
fn irreducible_inverse_candidates_are_verified_and_inactive_variables_are_retained() {
    let ctx = Interrupt::default();
    let order = MonoOrder::Lex;
    // Its Kronecker image has spurious univariate factors.
    let f = m(2, &[(vec![1, 0], 1), (vec![0, 1], 1)], order);
    assert_eq!(
        f.factor_z(&ctx).unwrap().unwrap().factors,
        vec![(f.clone(), 1)]
    );
    let f = m(
        2,
        &[(vec![2, 0], 1), (vec![0, 2], 1), (vec![0, 0], 1)],
        order,
    );
    let result = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(result.factors, vec![(f.clone(), 1)]);
    assert_eq!(result.status, FactorStatus::Complete);
    let mut a = vec![0; 20];
    a[0] = 1;
    let mut b = vec![0; 20];
    b[19] = 1;
    let f = m(20, &[(a, 1), (b, 1)], order);
    let result = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(result.factors, vec![(f.clone(), 1)]);
    assert_eq!(reconstruct(&result, 20, order, &ctx), f);
}
#[test]
fn oversized_mapping_preserves_exact_remainder_and_marks_possible_reducibility() {
    let ctx = Interrupt::default();
    let terms = (0..20)
        .map(|i| {
            let mut e = vec![0; 20];
            e[i] = 1;
            (e, 1)
        })
        .collect::<Vec<_>>();
    let f = m(20, &terms, MonoOrder::Lex);
    let result = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(result.status, FactorStatus::PossiblyReducible);
    assert_eq!(result.factors, vec![(f.clone(), 1)]);
    assert_eq!(reconstruct(&result, 20, MonoOrder::Lex, &ctx), f);
}
#[test]
fn multivariate_factor_budget_and_external_abort_propagate() {
    let ctx = Interrupt::default();
    let f = m(2, &[(vec![3, 0], 1), (vec![0, 3], -1)], MonoOrder::Lex);
    for steps in [0, 3, 50] {
        ctx.steps_left.set(steps);
        assert_eq!(f.factor_z(&ctx), Err(Abort::Budget));
    }
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        MPoly::<Integer>::zero().factor_z(&ctx),
        Err(Abort::Interrupted)
    );
}
proptest! {
    #![proptest_config(ProptestConfig{cases:64,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d362e34),..ProptestConfig::default()})]
    #[test]
    fn planted_multivariate_linear_factors_reconstruct_and_remain_exact_divisors(
        constants in prop::collection::vec(-3i64..=3,2..5),
        scalar in -6i64..=6, grevlex in any::<bool>()
    ) {
        let ctx=Interrupt::default();
        let order=if grevlex {MonoOrder::GrevLex} else {MonoOrder::Lex};
        let mut f=m(2,&[(vec![0,0],scalar)],order);
        for c in constants {f=multiply(&f,&m(2,&[(vec![1,0],1),(vec![0,1],1),(vec![0,0],c)],order),&ctx);}
        match f.factor_z(&ctx).unwrap() {
            None=>prop_assert!(f.is_zero()),
            Some(actual)=> {
                prop_assert_eq!(reconstruct(&actual,2,order,&ctx),f.clone());
                prop_assert_eq!(actual.status,FactorStatus::Complete);
                for (g,m) in &actual.factors {prop_assert!(*m>0);prop_assert!(f.exact_div(g,&ctx).unwrap().is_some());}
            }
        }
    }
}

#[test]
fn nonlinear_and_three_variable_planted_factors_are_all_recovered() {
    let ctx = Interrupt::default();
    let order = MonoOrder::Lex;
    // Primitive and irreducible because the polynomial is linear in y with unit coefficient.
    let a = m(
        2,
        &[(vec![2, 0], 2), (vec![0, 1], 1), (vec![0, 0], 1)],
        order,
    );
    let b = m(
        2,
        &[(vec![1, 0], 1), (vec![0, 1], -1), (vec![0, 0], 3)],
        order,
    );
    let f = multiply(&multiply(&a, &a, &ctx), &b, &ctx);
    let result = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(result.status, FactorStatus::Complete);
    assert_eq!(result.factors.len(), 2);
    assert!(result.factors.contains(&(a, 2)));
    assert!(result.factors.contains(&(b, 1)));
    assert_eq!(reconstruct(&result, 2, order, &ctx), f);
    let a = m(
        3,
        &[(vec![1, 0, 0], 1), (vec![0, 1, 0], 1), (vec![0, 0, 0], 1)],
        order,
    );
    let b = m(
        3,
        &[(vec![1, 0, 0], 1), (vec![0, 0, 1], 1), (vec![0, 0, 0], 2)],
        order,
    );
    let f = multiply(&a, &b, &ctx);
    let result = f.factor_z(&ctx).unwrap().unwrap();
    assert_eq!(result.status, FactorStatus::Complete);
    assert_eq!(result.factors.len(), 2);
    assert!(result.factors.contains(&(a, 1)));
    assert!(result.factors.contains(&(b, 1)));
    assert_eq!(reconstruct(&result, 3, order, &ctx), f);
}
