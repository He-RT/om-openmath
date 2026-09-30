//! Exact Descartes isolation checked by independent rational Sturm counts.
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{RootInterval, UPoly, isolate, refine};
use proptest::prelude::*;
use std::collections::BTreeSet;
fn z(c: &[i64]) -> UPoly<Integer> {
    UPoly::new(c.iter().map(|c| (*c).into()).collect())
}
fn q(a: i64, b: i64) -> Rational {
    Rational::from(a) / Rational::from(b)
}
fn evaluate(f: &UPoly<Rational>, x: &Rational) -> Rational {
    f.coeffs.iter().rev().fold(Rational::ZERO, |a, c| a * x + c)
}
fn sturm(f: &UPoly<Integer>) -> Vec<UPoly<Rational>> {
    let ctx = Interrupt::default();
    let f = UPoly::new(f.coeffs.iter().cloned().map(Rational::from).collect());
    let derivative = f.derivative(&ctx).unwrap();
    let mut out = vec![f, derivative];
    while !out.last().unwrap().is_zero() {
        let n = out.len();
        let r = out[n - 2]
            .divrem(&out[n - 1], &ctx)
            .unwrap()
            .unwrap()
            .1
            .neg(&ctx)
            .unwrap();
        if r.is_zero() {
            break;
        }
        out.push(r);
    }
    out.retain(|f| !f.is_zero());
    out
}
fn variations(signs: impl IntoIterator<Item = i8>) -> usize {
    let mut previous = 0;
    let mut count = 0;
    for sign in signs {
        if sign == 0 {
            continue;
        }
        if previous != 0 && previous != sign {
            count += 1;
        }
        previous = sign;
    }
    count
}
fn sign(x: &Rational) -> i8 {
    if x < &Rational::ZERO {
        -1
    } else if x > &Rational::ZERO {
        1
    } else {
        0
    }
}
fn count_between(chain: &[UPoly<Rational>], a: &Rational, b: &Rational) -> usize {
    variations(chain.iter().map(|f| sign(&evaluate(f, a))))
        - variations(chain.iter().map(|f| sign(&evaluate(f, b))))
}
fn total_real_roots(chain: &[UPoly<Rational>]) -> usize {
    let minus = variations(chain.iter().map(|f| {
        let s = sign(f.lc().unwrap());
        if f.degree().unwrap() % 2 == 1 { -s } else { s }
    }));
    let plus = variations(chain.iter().map(|f| sign(f.lc().unwrap())));
    minus - plus
}
fn certify(f: &UPoly<Integer>, intervals: &[RootInterval]) {
    let chain = sturm(f);
    assert_eq!(intervals.len(), total_real_roots(&chain), "{f:?}");
    for pair in intervals.windows(2) {
        assert!(pair[0].hi < pair[1].lo);
    }
    for i in intervals {
        if i.exact {
            assert_eq!(i.lo, i.hi);
            assert_eq!(evaluate(&chain[0], &i.lo), Rational::ZERO);
        } else {
            assert!(i.lo < i.hi);
            assert!(!evaluate(&chain[0], &i.lo).is_zero());
            assert!(!evaluate(&chain[0], &i.hi).is_zero());
            assert_eq!(count_between(&chain, &i.lo, &i.hi), 1);
        }
    }
}
#[test]
fn authority_real_root_vectors_are_sorted_and_certified() {
    let ctx = Interrupt::default();
    for (f, roots) in [
        (z(&[-2, 0, 1]), vec![q(-14142, 10000), q(14142, 10000)]),
        (
            z(&[0, -2, 0, 1]),
            vec![q(-14142, 10000), Rational::ZERO, q(14142, 10000)],
        ),
        (
            z(&[1, 0, -10, 0, 1]),
            vec![
                q(-314626, 100000),
                q(-31784, 100000),
                q(31784, 100000),
                q(314626, 100000),
            ],
        ),
        (z(&[1, 0, 1]), vec![]),
    ] {
        let intervals = isolate(&f, &ctx).unwrap().unwrap();
        certify(&f, &intervals);
        assert_eq!(intervals.len(), roots.len());
        for (i, r) in intervals.iter().zip(&roots) {
            assert!(&i.lo <= r && r <= &i.hi);
        }
        if f.coeffs[0].is_zero() {
            assert!(intervals.iter().any(|i| i.exact && i.lo.is_zero()));
        }
    }
}
#[test]
fn refinement_uses_exact_signs_and_reaches_requested_bits() {
    let ctx = Interrupt::default();
    let f = z(&[-2, 0, 1]);
    let roots = isolate(&f, &ctx).unwrap().unwrap();
    let epsilon = Rational::ONE / Rational::from(Integer::ONE << 120);
    let mut refined = vec![];
    for i in roots {
        let r = refine(&f, &i, 120, &ctx).unwrap().unwrap();
        assert!(r.lo >= i.lo && r.hi <= i.hi);
        assert!(&r.hi - &r.lo <= epsilon);
        assert!(r.lo.clone() * &r.lo != Rational::from(2));
        refined.push(r);
    }
    certify(&f, &refined);
    let positive = &refined[1];
    assert!(&positive.lo * &positive.lo < Rational::from(2));
    assert!(&positive.hi * &positive.hi > Rational::from(2));
}
#[test]
fn dyadic_split_roots_are_emitted_once_and_neighbour_intervals_exclude_them() {
    let ctx = Interrupt::default();
    let f = z(&[0, -6, 11, -6, 1]); // 0,1,2,3
    let intervals = isolate(&f, &ctx).unwrap().unwrap();
    certify(&f, &intervals);
    assert_eq!(intervals.len(), 4);
    for (i, value) in intervals.iter().zip(0..4) {
        let r = refine(&f, i, 40, &ctx).unwrap().unwrap();
        assert!(r.exact);
        assert_eq!(r.lo, Rational::from(value));
    }
    let f = z(&[0, 2, -1, -2, 1]); // 0,1,-1,2
    let intervals = isolate(&f, &ctx).unwrap().unwrap();
    certify(&f, &intervals);
    assert_eq!(intervals.len(), 4);
}
#[test]
fn negative_content_constants_zero_and_repeated_roots_have_checked_semantics() {
    let ctx = Interrupt::default();
    assert_eq!(isolate(&UPoly::zero(), &ctx).unwrap(), None);
    assert_eq!(isolate(&z(&[-7]), &ctx).unwrap(), Some(vec![]));
    assert_eq!(isolate(&z(&[1, 2, 1]), &ctx).unwrap(), None);
    let f = z(&[-2, 0, 1]);
    assert_eq!(
        isolate(&f, &ctx).unwrap(),
        isolate(&f.scale(&(-12).into(), &ctx).unwrap(), &ctx).unwrap()
    );
    let f = z(&[0, 1]);
    assert_eq!(
        isolate(&f, &ctx).unwrap(),
        Some(vec![RootInterval {
            lo: Rational::ZERO,
            hi: Rational::ZERO,
            exact: true
        }])
    );
}
#[test]
fn very_close_rational_roots_and_large_cauchy_bounds_remain_exact() {
    let ctx = Interrupt::default();
    let denominator = Integer::ONE << 80;
    let a = z(&[-1, 1]);
    let b = UPoly::new(vec![-(&denominator + 1_u8), denominator.clone()]);
    let f = a.mul(&b, &ctx).unwrap().mul(&z(&[1, 0, 1]), &ctx).unwrap();
    let intervals = isolate(&f, &ctx).unwrap().unwrap();
    certify(&f, &intervals);
    assert_eq!(intervals.len(), 2);
    let roots = [
        Rational::ONE,
        Rational::ONE + Rational::ONE / Rational::from(denominator),
    ];
    for (i, r) in intervals.iter().zip(roots) {
        let refined = refine(&f, i, 100, &ctx).unwrap().unwrap();
        assert!(refined.lo <= r && r <= refined.hi);
    }
    let large = Integer::ONE << 150;
    let f = UPoly::new(vec![-large.clone(), Integer::ONE]);
    let intervals = isolate(&f, &ctx).unwrap().unwrap();
    certify(&f, &intervals);
    let root = refine(&f, &intervals[0], 8, &ctx).unwrap().unwrap();
    assert!(root.exact);
    assert_eq!(root.lo, Rational::from(large));
}
#[test]
fn refinement_rejects_incorrect_interval_certificates() {
    let ctx = Interrupt::default();
    let f = z(&[0, -1, 0, 1]);
    let bad = [
        RootInterval {
            lo: 2.into(),
            hi: 1.into(),
            exact: false,
        },
        RootInterval {
            lo: (-2).into(),
            hi: 2.into(),
            exact: false,
        }, // three roots
        RootInterval {
            lo: 1.into(),
            hi: 2.into(),
            exact: false,
        }, // endpoint root
        RootInterval {
            lo: 2.into(),
            hi: 3.into(),
            exact: false,
        }, // no root
        RootInterval {
            lo: 2.into(),
            hi: 2.into(),
            exact: true,
        },
        RootInterval {
            lo: 0.into(),
            hi: 1.into(),
            exact: true,
        },
    ];
    for i in bad {
        assert_eq!(refine(&f, &i, 30, &ctx).unwrap(), None);
    }
    let exact = RootInterval {
        lo: Rational::ZERO,
        hi: Rational::ZERO,
        exact: true,
    };
    assert_eq!(refine(&f, &exact, 30, &ctx).unwrap(), Some(exact));
    let i = RootInterval {
        lo: 0.into(),
        hi: 2.into(),
        exact: false,
    };
    assert_eq!(refine(&z(&[1, -2, 1]), &i, 10, &ctx).unwrap(), None);
}
#[test]
fn isolation_and_refinement_propagate_budget_deadline_and_external_cancel() {
    let ctx = Interrupt::default();
    let f = z(&[1, 0, -10, 0, 1]);
    for budget in [0, 3, 30] {
        ctx.steps_left.set(budget);
        assert_eq!(isolate(&f, &ctx), Err(Abort::Budget));
    }
    ctx.steps_left.set(500_000_000);
    let i = isolate(&f, &ctx).unwrap().unwrap().remove(0);
    let probe = Interrupt::default();
    refine(&f, &i, 0, &probe).unwrap().unwrap();
    let validation_cost = 500_000_000 - probe.steps_left.get();
    ctx.steps_left.set(validation_cost + 100);
    assert_eq!(refine(&f, &i, u32::MAX, &ctx), Err(Abort::Budget));
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(isolate(&z(&[]), &ctx), Err(Abort::Interrupted));
    struct FixedClock;
    impl om_num::ctx::Clock for FixedClock {
        fn now_ms(&self) -> f64 {
            2.0
        }
    }
    let expired = Interrupt {
        deadline_ms: Some(1.0),
        clock: Some(std::sync::Arc::new(FixedClock)),
        ..Interrupt::default()
    };
    assert_eq!(isolate(&f, &expired), Err(Abort::Timeout));
    expired.steps_left.set(500_000_000);
    assert_eq!(refine(&f, &i, 10, &expired), Err(Abort::Timeout));
}
proptest! {
    #![proptest_config(ProptestConfig{cases:64,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d362e35),..ProptestConfig::default()})]
    #[test]
    fn planted_real_roots_with_complex_pair_agree_with_sturm(
        roots in prop::collection::vec(-8i64..=8,0..7), a in 1i64..=5
    ) {
        let ctx=Interrupt::default();
        let unique=roots.iter().copied().collect::<BTreeSet<_>>();
        prop_assume!(unique.len()==roots.len());
        let mut f=z(&[a*a,0,1]);
        for r in &unique {f=f.mul(&z(&[-r,1]),&ctx).unwrap();}
        let intervals=isolate(&f,&ctx).unwrap().unwrap();
        certify(&f,&intervals);
        prop_assert_eq!(intervals.len(),roots.len());
        for (i,r) in intervals.iter().zip(unique) {
            let r=Rational::from(r);
            prop_assert!(i.lo<=r && r<=i.hi);
            let refined=refine(&f,i,80,&ctx).unwrap().unwrap();
            prop_assert!(refined.lo<=r && r<=refined.hi);
            prop_assert!(&refined.hi-&refined.lo<=Rational::ONE/Rational::from(Integer::ONE<<80));
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig{cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x53545552),..ProptestConfig::default()})]
    #[test]
    fn arbitrary_squarefree_integer_polynomials_have_all_sturm_roots(
        coefficients in prop::collection::vec(-6i64..=6,2..10)
    ) {
        let ctx=Interrupt::default();
        let f=z(&coefficients);
        prop_assume!(f.degree().is_some_and(|d|d>0));
        prop_assume!(f.primitive_part(&ctx).unwrap().gcdheu(&f.derivative(&ctx).unwrap(),&ctx).unwrap().degree()==Some(0));
        let intervals=isolate(&f,&ctx).unwrap().unwrap();
        certify(&f,&intervals);
        let refined=intervals.iter().map(|i|refine(&f,i,50,&ctx).unwrap().unwrap()).collect::<Vec<_>>();
        certify(&f,&refined);
        for (old,new) in intervals.iter().zip(&refined) {
            prop_assert!(new.lo>=old.lo && new.hi<=old.hi);
            prop_assert!(&new.hi-&new.lo<=Rational::ONE/Rational::from(Integer::ONE<<50));
        }
    }
}
