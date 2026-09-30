//! Square-free factors reconstruct inputs, including inseparable positive characteristics.
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
    rng::SplitMix64,
};
use om_poly::{FpElem, Ring, SquareFree, UPoly};

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
fn pow<R: Ring>(f: &UPoly<R>, n: u32, ctx: &Interrupt) -> UPoly<R> {
    (0..n).fold(UPoly::one(), |a, _| a.mul(f, ctx).unwrap())
}
fn reconstruct<R: Ring>(sf: &SquareFree<R>, ctx: &Interrupt) -> UPoly<R> {
    sf.factors
        .iter()
        .fold(UPoly::new(vec![sf.content.clone()]), |a, (f, m)| {
            a.mul(&pow(f, *m, ctx), ctx).unwrap()
        })
}
#[test]
fn authority_yun_and_frobenius_vectors() {
    let ctx = Interrupt::default();
    let factors = vec![(z(&[-1, 1]), 1), (z(&[-2, 1]), 2), (z(&[-3, 1]), 3)];
    let want = SquareFree {
        content: Integer::ONE,
        factors,
    };
    let f = reconstruct(&want, &ctx);
    assert_eq!(f.square_free(&ctx).unwrap(), Some(want));
    assert_eq!(
        z(&[1, 2, 1]).square_free(&ctx).unwrap(),
        Some(SquareFree {
            content: Integer::ONE,
            factors: vec![(z(&[1, 1]), 2)]
        })
    );
    let f = fp(&[1, 0, 0, 1], 3);
    assert_eq!(
        f.square_free(&ctx).unwrap(),
        Some(SquareFree {
            content: FpElem::new(1, 3).unwrap(),
            factors: vec![(fp(&[1, 1], 3), 3)]
        })
    );
}
#[test]
fn integer_content_nonmonic_factors_zero_and_constants() {
    let ctx = Interrupt::default();
    let want = SquareFree {
        content: (-12).into(),
        factors: vec![(z(&[-1, 2]), 2), (z(&[2, 3]), 5)],
    };
    let f = reconstruct(&want, &ctx);
    assert_eq!(f.square_free(&ctx).unwrap(), Some(want));
    assert_eq!(z(&[]).square_free(&ctx).unwrap(), None);
    assert_eq!(
        z(&[-7]).square_free(&ctx).unwrap(),
        Some(SquareFree {
            content: (-7).into(),
            factors: vec![]
        })
    );
    assert_eq!(
        z(&[1]).square_free(&ctx).unwrap(),
        Some(SquareFree {
            content: 1.into(),
            factors: vec![]
        })
    );
    let want = SquareFree {
        content: 1.into(),
        factors: vec![(z(&[1, 0, 1]).mul(&z(&[2, 0, 1]), &ctx).unwrap(), 2)],
    };
    assert_eq!(
        reconstruct(&want, &ctx).square_free(&ctx).unwrap(),
        Some(want)
    );
}
#[test]
fn rational_yun_clears_denominators_and_retains_primitive_factors() {
    let ctx = Interrupt::default();
    let want = SquareFree {
        content: Rational::from(-5) / Rational::from(18),
        factors: vec![
            (UPoly::new(vec![(-1).into(), 2.into()]), 2),
            (UPoly::new(vec![2.into(), 3.into()]), 3),
        ],
    };
    let f = reconstruct(&want, &ctx);
    assert_eq!(f.square_free(&ctx).unwrap(), Some(want));
    assert_eq!(UPoly::<Rational>::zero().square_free(&ctx).unwrap(), None);
    let constant = Rational::from(7) / Rational::from(13);
    assert_eq!(
        UPoly::new(vec![constant.clone()])
            .square_free(&ctx)
            .unwrap(),
        Some(SquareFree {
            content: constant,
            factors: vec![]
        })
    );
}
#[test]
fn finite_fields_mixed_inseparability_content_and_nested_pth_roots() {
    let ctx = Interrupt::default();
    for p in [2, 3, 5, 7] {
        let h = fp(&[0, 1], p);
        let g = fp(&[1, 1], p);
        let (a, b) = (p as u32, p as u32 + 1);
        let want = SquareFree {
            content: FpElem::new(p - 1, p).unwrap(),
            factors: vec![(h, a), (g, b)],
        };
        let f = reconstruct(&want, &ctx);
        let actual = f.square_free(&ctx).unwrap().unwrap();
        assert_eq!(actual, want);
        assert_eq!(reconstruct(&actual, &ctx), f);
        let f = pow(&fp(&[1, 1], p), (p * p) as u32, &ctx);
        assert_eq!(
            f.square_free(&ctx).unwrap().unwrap().factors,
            vec![(fp(&[1, 1], p), (p * p) as u32)]
        );
        assert_eq!(fp(&[0], p).square_free(&ctx).unwrap(), None);
        assert_eq!(
            fp(&[2], p).square_free(&ctx).unwrap(),
            if p == 2 {
                None
            } else {
                Some(SquareFree {
                    content: FpElem::new(2, p).unwrap(),
                    factors: vec![],
                })
            }
        );
    }
    // A modulus must be supplied for a nonconstant polynomial's Frobenius semantics.
    assert_eq!(
        UPoly::new(vec![FpElem::one(), FpElem::one()])
            .square_free(&ctx)
            .unwrap(),
        None
    );
    let f = fp(&[1, 2, 1], 18446744073709551557);
    assert_eq!(
        f.square_free(&ctx).unwrap().unwrap().factors,
        vec![(fp(&[1, 1], 18446744073709551557), 2)]
    );
}
#[test]
fn derivatives_and_monic_field_gcd_obey_each_characteristic() {
    let ctx = Interrupt::default();
    assert_eq!(z(&[8, -3, 0, 4]).derivative(&ctx).unwrap(), z(&[-3, 0, 12]));
    assert_eq!(z(&[]).derivative(&ctx).unwrap(), z(&[]));
    assert_eq!(
        fp(&[1, 0, 0, 1], 3).derivative(&ctx).unwrap(),
        UPoly::zero()
    );
    // Runtime context can come from a lower coefficient while the leading one is a factory.
    let mixed = UPoly::new(vec![
        FpElem::new(2, 3).unwrap(),
        FpElem::one(),
        FpElem::zero(),
        FpElem::one(),
    ]);
    assert_eq!(mixed.derivative(&ctx).unwrap(), fp(&[1], 3));
    assert_eq!(mixed.monic(&ctx).unwrap().unwrap(), fp(&[2, 1, 0, 1], 3));
    let f = fp(&[2, 4, 2], 5);
    let g = fp(&[4, 4], 5);
    assert_eq!(f.monic_gcd(&g, &ctx).unwrap().unwrap(), fp(&[1, 1], 5));
    assert_eq!(
        fp(&[], 5).monic_gcd(&fp(&[], 5), &ctx).unwrap(),
        Some(UPoly::zero())
    );
    let f = UPoly::new(vec![Rational::from(-2), Rational::from(4)]);
    assert_eq!(
        f.monic(&ctx).unwrap().unwrap(),
        UPoly::new(vec![Rational::from(-1) / Rational::from(2), Rational::ONE])
    );
}
#[test]
fn fixed_seed_squarefree_factor_products_and_arbitrary_inputs_reconstruct() {
    let ctx = Interrupt::default();
    let mut rng = SplitMix64::new(0x4d352e34);
    for _ in 0..128 {
        let factors = vec![
            (z(&[-1, 2]), 1 + rng.next_range(0, 5) as u32),
            (z(&[2, 3]), 1 + rng.next_range(0, 5) as u32),
        ];
        let sf = SquareFree {
            content: Integer::from(rng.next_range(1, 11) as i64 - 6),
            factors,
        };
        let f = reconstruct(&sf, &ctx);
        if let Some(sf) = f.square_free(&ctx).unwrap() {
            assert_eq!(reconstruct(&sf, &ctx), f);
            for (h, _) in &sf.factors {
                assert!(
                    h.gcdheu(&h.derivative(&ctx).unwrap(), &ctx)
                        .unwrap()
                        .is_one()
                );
            }
            for pair in sf.factors.windows(2) {
                assert!(pair[0].0.gcdheu(&pair[1].0, &ctx).unwrap().is_one());
            }
        } else {
            assert!(f.is_zero());
        }
        for p in [2, 3, 5, 7] {
            let f = UPoly::new(
                (0..10)
                    .map(|_| FpElem::new(rng.next_range(0, p), p).unwrap())
                    .collect(),
            );
            if let Some(sf) = f.square_free(&ctx).unwrap() {
                assert_eq!(reconstruct(&sf, &ctx), f);
                for (h, _) in &sf.factors {
                    assert!(
                        h.monic_gcd(&h.derivative(&ctx).unwrap(), &ctx)
                            .unwrap()
                            .unwrap()
                            .is_one()
                    );
                }
                for pair in sf.factors.windows(2) {
                    assert!(pair[0].1 < pair[1].1);
                    assert!(
                        pair[0]
                            .0
                            .monic_gcd(&pair[1].0, &ctx)
                            .unwrap()
                            .unwrap()
                            .is_one()
                    );
                }
            } else {
                assert!(f.is_zero());
            }
        }
    }
}
#[test]
fn squarefree_derivative_and_field_gcd_propagate_interrupts() {
    let ctx = Interrupt::default();
    for budget in [0, 3, 20] {
        ctx.steps_left.set(budget);
        assert_eq!(z(&[1; 30]).square_free(&ctx), Err(Abort::Budget));
        ctx.steps_left.set(budget);
        assert_eq!(fp(&[1; 30], 3).square_free(&ctx), Err(Abort::Budget));
    }
    ctx.steps_left.set(0);
    assert_eq!(z(&[]).derivative(&ctx), Err(Abort::Budget));
    assert_eq!(fp(&[1], 3).monic(&ctx), Err(Abort::Budget));
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        UPoly::<Rational>::zero().square_free(&ctx),
        Err(Abort::Interrupted)
    );
}
