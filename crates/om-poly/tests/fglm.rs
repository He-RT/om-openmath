//! FGLM must agree exactly with direct lex Buchberger, including nonradical ideals.
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{MPoly, MonoOrder, Monomial, fglm, groebner, normal_form};
use proptest::prelude::*;
type Q = MPoly<Rational>;
fn q(n: usize, terms: &[(Vec<u32>, i64, i64)]) -> Q {
    Q::new(
        n,
        terms
            .iter()
            .map(|(e, a, b)| {
                (
                    Monomial::new(e.clone()).unwrap(),
                    Rational::from(*a) / Rational::from(*b),
                )
            })
            .collect(),
        MonoOrder::GrevLex,
        &Interrupt::default(),
    )
    .unwrap()
}
fn verify(input: &[Q], ctx: &Interrupt) -> Vec<Q> {
    let gr = groebner(input, MonoOrder::GrevLex, ctx).unwrap().unwrap();
    let lex = groebner(input, MonoOrder::Lex, ctx).unwrap().unwrap();
    let converted = fglm(&gr, ctx).unwrap().unwrap();
    assert_eq!(converted, lex);
    for f in &converted {
        let source_order = Q::new(f.nvars, f.terms.clone(), MonoOrder::GrevLex, ctx).unwrap();
        assert!(
            normal_form(&source_order, &gr, ctx)
                .unwrap()
                .unwrap()
                .is_zero()
        );
    }
    converted
}
#[test]
fn authority_systems_convert_to_exact_direct_lex_bases() {
    let ctx = Interrupt::default();
    for input in [
        vec![
            q(
                2,
                &[(vec![2, 0], 1, 1), (vec![0, 2], 1, 1), (vec![0, 0], -1, 1)],
            ),
            q(2, &[(vec![1, 0], 1, 1), (vec![0, 1], -1, 1)]),
        ],
        vec![
            q(2, &[(vec![1, 1], 1, 1), (vec![0, 0], -1, 1)]),
            q(2, &[(vec![2, 0], 1, 1), (vec![0, 1], -1, 1)]),
        ],
        vec![
            q(
                3,
                &[
                    (vec![1, 0, 0], 1, 1),
                    (vec![0, 1, 0], 1, 1),
                    (vec![0, 0, 1], 1, 1),
                ],
            ),
            q(
                3,
                &[
                    (vec![1, 1, 0], 1, 1),
                    (vec![1, 0, 1], 1, 1),
                    (vec![0, 1, 1], 1, 1),
                ],
            ),
            q(3, &[(vec![1, 1, 1], 1, 1), (vec![0, 0, 0], -1, 1)]),
        ],
    ] {
        verify(&input, &ctx);
    }
}
#[test]
fn nonradical_multiplicities_and_rational_coefficients_survive_conversion() {
    let ctx = Interrupt::default();
    for input in [
        vec![q(2, &[(vec![2, 0], 1, 1)]), q(2, &[(vec![0, 3], 1, 1)])],
        vec![
            q(2, &[(vec![2, 0], 1, 1), (vec![0, 1], -1, 1)]),
            q(2, &[(vec![0, 2], 1, 1)]),
        ],
        vec![
            q(2, &[(vec![2, 0], 1, 2), (vec![0, 1], -1, 3)]),
            q(2, &[(vec![0, 2], 1, 5), (vec![1, 0], -1, 7)]),
        ],
    ] {
        let basis = verify(&input, &ctx);
        assert_eq!(fglm(&basis, &ctx).unwrap().unwrap(), basis);
    }
}
#[test]
fn positive_dimension_nonbasis_contexts_and_unit_ideal_are_handled() {
    let ctx = Interrupt::default();
    assert!(fglm(&[], &ctx).unwrap().is_none());
    assert!(
        fglm(&[q(2, &[(vec![1, 1], 1, 1)])], &ctx)
            .unwrap()
            .is_none()
    );
    assert!(
        fglm(&[Q::zero_in(2, MonoOrder::GrevLex)], &ctx)
            .unwrap()
            .is_none()
    );
    assert!(
        fglm(
            &[
                q(2, &[(vec![2, 0], 1, 1), (vec![0, 1], -1, 1)]),
                q(2, &[(vec![0, 2], 1, 1), (vec![1, 0], -1, 1)]),
                q(2, &[(vec![1, 1], 1, 1), (vec![0, 0], -2, 1)])
            ],
            &ctx
        )
        .unwrap()
        .is_none()
    );
    assert!(
        fglm(
            &[q(2, &[(vec![2, 0], 1, 1)]), q(3, &[(vec![0, 0, 2], 1, 1)])],
            &ctx
        )
        .unwrap()
        .is_none()
    );
    let unit = groebner(&[q(2, &[(vec![0, 0], -3, 2)])], MonoOrder::GrevLex, &ctx)
        .unwrap()
        .unwrap();
    let lex = fglm(&unit, &ctx).unwrap().unwrap();
    assert_eq!(lex.len(), 1);
    assert!(lex[0].is_one());
    assert_eq!(lex[0].order, MonoOrder::Lex);
    assert_eq!(lex[0].nvars, 2);
}
#[test]
fn interrupt_stops_validation_standard_basis_enumeration_and_row_reduction() {
    let ctx = Interrupt::default();
    let basis = vec![q(2, &[(vec![4, 0], 1, 1)]), q(2, &[(vec![0, 4], 1, 1)])];
    for budget in [0, 20, 200, 1500] {
        ctx.steps_left.set(budget);
        assert!(matches!(fglm(&basis, &ctx), Err(Abort::Budget)));
    }
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(fglm(&[], &ctx), Err(Abort::Interrupted)));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:64,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d372e32),..ProptestConfig::default()})]
 #[test]
 fn triangular_zero_dimensional_systems_equal_direct_lex(a in -3i64..=3,b in -3i64..=3,c in -3i64..=3,d in -3i64..=3){
  let ctx=Interrupt::default();let input=vec![q(2,&[(vec![1,0],1,1),(vec![0,2],a,1),(vec![0,1],b,1),(vec![0,0],c,1)]),q(2,&[(vec![0,3],1,1),(vec![0,1],d,1),(vec![0,0],a,1)])];verify(&input,&ctx);
 }
 #[test]
 fn coupled_zero_dimensional_quadratics_equal_direct_lex(a in -2i64..=2,b in -2i64..=2,c in -2i64..=2,d in -2i64..=2){
  let ctx=Interrupt::default();let input=vec![q(2,&[(vec![2,0],1,1),(vec![1,0],a,1),(vec![0,1],b,1),(vec![0,0],c,1)]),q(2,&[(vec![0,2],1,1),(vec![0,1],d,1),(vec![1,0],c,1),(vec![0,0],a,1)])];verify(&input,&ctx);
 }
}
