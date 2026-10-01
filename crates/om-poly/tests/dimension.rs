//! Leading-ideal dimensions checked against an independent exhaustive support oracle.
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{
    IdealDimension, MPoly, MonoOrder, Monomial, groebner, ideal_dimension, is_zero_dimensional,
};
use proptest::prelude::*;
type Q = MPoly<Rational>;
fn q(n: usize, terms: &[(Vec<u32>, i64)]) -> Q {
    Q::new(
        n,
        terms
            .iter()
            .map(|(e, c)| (Monomial::new(e.clone()).unwrap(), Rational::from(*c)))
            .collect(),
        MonoOrder::GrevLex,
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn authority_xy_is_positive_dimensional_and_standard_systems_are_zero_dimensional() {
    let ctx = Interrupt::default();
    let xy = q(2, &[(vec![1, 1], 1)]);
    assert_eq!(
        ideal_dimension(std::slice::from_ref(&xy), 2, &ctx).unwrap(),
        Some(IdealDimension::Dimension(1))
    );
    assert_eq!(is_zero_dimensional(&[xy], 2, &ctx).unwrap(), Some(false));
    for input in [
        vec![
            q(2, &[(vec![2, 0], 1), (vec![0, 2], 1), (vec![0, 0], -1)]),
            q(2, &[(vec![1, 0], 1), (vec![0, 1], -1)]),
        ],
        vec![
            q(2, &[(vec![1, 1], 1), (vec![0, 0], -1)]),
            q(2, &[(vec![2, 0], 1), (vec![0, 1], -1)]),
        ],
    ] {
        let gr = groebner(&input, MonoOrder::GrevLex, &ctx).unwrap().unwrap();
        let lex = groebner(&input, MonoOrder::Lex, &ctx).unwrap().unwrap();
        for basis in [gr, lex] {
            assert_eq!(
                ideal_dimension(&basis, 2, &ctx).unwrap(),
                Some(IdealDimension::Dimension(0))
            );
            assert_eq!(is_zero_dimensional(&basis, 2, &ctx).unwrap(), Some(true));
        }
    }
}
#[test]
fn unit_zero_parameter_variables_and_supports_have_explicit_dimensions() {
    let ctx = Interrupt::default();
    assert_eq!(
        ideal_dimension(&[], 4, &ctx).unwrap(),
        Some(IdealDimension::Dimension(4))
    );
    assert_eq!(
        ideal_dimension(&[Q::zero_in(3, MonoOrder::Lex)], 3, &ctx).unwrap(),
        Some(IdealDimension::Dimension(3))
    );
    assert_eq!(
        ideal_dimension(&[Q::one()], 3, &ctx).unwrap(),
        Some(IdealDimension::Empty)
    );
    assert_eq!(
        is_zero_dimensional(&[Q::one()], 3, &ctx).unwrap(),
        Some(true)
    );
    assert_eq!(
        ideal_dimension(&[], 0, &ctx).unwrap(),
        Some(IdealDimension::Dimension(0))
    );
    let basis = vec![
        q(3, &[(vec![2, 1, 0], 1)]),
        q(3, &[(vec![0, 2, 1], 1)]),
        q(3, &[(vec![1, 0, 3], 1)]),
    ];
    assert_eq!(
        ideal_dimension(&basis, 3, &ctx).unwrap(),
        Some(IdealDimension::Dimension(1))
    );
    let basis = vec![
        q(4, &[(vec![2, 0, 0, 0], 1)]),
        q(4, &[(vec![0, 1, 1, 0], 1)]),
    ];
    assert_eq!(
        ideal_dimension(&basis, 4, &ctx).unwrap(),
        Some(IdealDimension::Dimension(2))
    );
}
#[test]
fn invalid_basis_contexts_and_all_traversals_propagate_interrupt() {
    let ctx = Interrupt::default();
    let not_basis = vec![
        q(2, &[(vec![2, 0], 1), (vec![0, 1], -1)]),
        q(2, &[(vec![0, 2], 1), (vec![1, 0], -1)]),
        q(2, &[(vec![1, 1], 1), (vec![0, 0], -2)]),
    ];
    assert!(ideal_dimension(&not_basis, 2, &ctx).unwrap().is_none());
    assert!(
        ideal_dimension(&[q(3, &[(vec![1, 0, 0], 1)])], 2, &ctx)
            .unwrap()
            .is_none()
    );
    let basis: Vec<_> = (0..12)
        .map(|j| {
            let mut e = vec![0; 12];
            e[j] = 1;
            q(12, &[(e, 1)])
        })
        .collect();
    for budget in [0, 20, 200, 2000] {
        ctx.steps_left.set(budget);
        assert!(matches!(
            ideal_dimension(&basis, 12, &ctx),
            Err(Abort::Budget)
        ));
    }
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        is_zero_dimensional(&[], 0, &ctx),
        Err(Abort::Interrupted)
    ));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d372e33),..ProptestConfig::default()})]
 #[test]
 fn dimension_matches_support_bitmask_oracle(n in 1usize..=6,supports in prop::collection::vec(0u32..64,0..8)){
  let ctx=Interrupt::default();let mask=(1u32<<n)-1;let supports:Vec<_>=supports.into_iter().map(|m|m&mask).collect();
  let basis:Vec<_>=supports.iter().map(|m|q(n,&[((0..n).map(|j|if m&(1<<j)!=0{2}else{0}).collect(),1)])).collect();
  let expected=if supports.contains(&0){IdealDimension::Empty}else{IdealDimension::Dimension((0..=mask).filter(|u|supports.iter().all(|m|m&u!=*m)).map(|u|u.count_ones() as usize).max().unwrap_or(0))};
  prop_assert_eq!(ideal_dimension(&basis,n,&ctx).unwrap(),Some(expected));
  prop_assert_eq!(is_zero_dimensional(&basis,n,&ctx).unwrap(),Some(matches!(expected,IdealDimension::Empty|IdealDimension::Dimension(0))));
 }
}
