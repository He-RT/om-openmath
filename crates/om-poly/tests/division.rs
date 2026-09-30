//! Exact/pseudo division and signed content, including the plan's fixed vectors.
use om_num::{Integer, Rational, ctx::Interrupt, rng::SplitMix64};
use om_poly::{FpElem, MPoly, MonoOrder, Monomial, Ring, UPoly};
fn z(v: &[i64]) -> UPoly<Integer> {
    UPoly::new(v.iter().map(|v| (*v).into()).collect())
}
fn q(v: &[i64]) -> UPoly<Rational> {
    UPoly::new(v.iter().map(|v| (*v).into()).collect())
}
fn fp(v: &[u64], p: u64) -> UPoly<FpElem> {
    UPoly::new(v.iter().map(|v| FpElem::new(*v, p).unwrap()).collect())
}
#[test]
fn authority_division_prem_and_signed_content_vectors() {
    let ctx = Interrupt::default();
    assert_eq!(
        q(&[-4, 0, -2, 1])
            .divrem(&q(&[-3, 1]), &ctx)
            .unwrap()
            .unwrap(),
        (q(&[3, 1, 1]), q(&[5]))
    );
    assert_eq!(
        z(&[1, 0, 1]).prem(&z(&[1, 2]), &ctx).unwrap().unwrap(),
        z(&[5])
    );
    assert_eq!(
        z(&[2, 4, 6]).content_pp(&ctx).unwrap(),
        (2.into(), z(&[1, 2, 3]))
    );
    assert_eq!(
        z(&[3, -6]).content_pp(&ctx).unwrap(),
        ((-3).into(), z(&[-1, 2]))
    );
    assert_eq!(z(&[0]).content_pp(&ctx).unwrap(), (0.into(), UPoly::zero()));
    assert_eq!(
        z(&[-7]).content_pp(&ctx).unwrap(),
        ((-7).into(), UPoly::one())
    );
}
#[test]
fn exact_polynomial_and_scalar_division_reject_nondivisibility() {
    let ctx = Interrupt::default();
    let a = z(&[-2, 1]);
    let b = z(&[3, 2]);
    let f = a.mul(&b, &ctx).unwrap();
    assert_eq!(f.exact_div(&a, &ctx).unwrap(), Some(b));
    assert_eq!(z(&[1, 1]).exact_div(&z(&[2]), &ctx).unwrap(), None);
    assert_eq!(z(&[1, 0, 1]).exact_div(&z(&[-1, 1]), &ctx).unwrap(), None);
    assert_eq!(f.exact_div(&UPoly::zero(), &ctx).unwrap(), None);
    assert_eq!(
        z(&[2, 4, 6])
            .divide_scalar(&Integer::from(2), &ctx)
            .unwrap(),
        Some(z(&[1, 2, 3]))
    );
    assert_eq!(
        z(&[1, 2]).divide_scalar(&Integer::from(2), &ctx).unwrap(),
        None
    );
    let half = Rational::from(1) / Rational::from(2);
    assert_eq!(
        q(&[1, 1]).exact_div(&q(&[2]), &ctx).unwrap(),
        Some(q(&[1, 1]).scale(&half, &ctx).unwrap())
    );
    for p in [2, 3, 5, 7, 11] {
        let a = fp(&[1, 0, 1], p);
        let b = fp(&[1, 1], p);
        assert_eq!(
            a.mul(&b, &ctx).unwrap().exact_div(&b, &ctx).unwrap(),
            Some(a)
        );
    }
}
#[test]
fn pseudo_division_retains_missing_leading_powers_when_degrees_drop_early() {
    let ctx = Interrupt::default();
    for (f, g, scale, want_q, want_r) in [
        (z(&[1, 0, 1]), z(&[1, 2]), 4, z(&[-1, 2]), z(&[5])),
        (
            z(&[1, 0, 0, 0, 1]),
            z(&[0, 0, 2]),
            8,
            z(&[0, 0, 4]),
            z(&[8]),
        ),
        (
            z(&[1, 0, 0, 0, 1]),
            z(&[0, 0, -2]),
            -8,
            z(&[0, 0, 4]),
            z(&[-8]),
        ),
        (z(&[3]), z(&[1, 2]), 1, UPoly::zero(), z(&[3])),
    ] {
        let (s, a, b) = f.pseudo_divrem(&g, &ctx).unwrap().unwrap();
        assert_eq!(
            (s.clone(), a.clone(), b.clone()),
            (Integer::from(scale), want_q, want_r)
        );
        assert_eq!(
            f.scale(&s, &ctx).unwrap(),
            a.mul(&g, &ctx).unwrap().add(&b, &ctx).unwrap()
        );
    }
    assert!(z(&[1]).prem(&UPoly::zero(), &ctx).unwrap().is_none());
    assert_eq!(
        UPoly::<Integer>::zero().prem(&z(&[1, 2]), &ctx).unwrap(),
        Some(UPoly::zero())
    );
}
#[test]
fn pseudo_remainders_work_over_integer_parameter_polynomials() {
    let ctx = Interrupt::default();
    let a = MPoly::new(
        1,
        vec![(Monomial::new([1]).unwrap(), Integer::ONE)],
        MonoOrder::Lex,
        &ctx,
    )
    .unwrap();
    let f = UPoly::new(vec![a.clone(), MPoly::zero(), MPoly::one()]);
    let g = UPoly::new(vec![MPoly::one(), a.clone()]);
    let (scale, quotient, remainder) = f.pseudo_divrem(&g, &ctx).unwrap().unwrap();
    assert_eq!(scale, Ring::mul(&a, &a));
    assert_eq!(
        quotient,
        UPoly::new(vec![Ring::neg(&MPoly::one()), a.clone()])
    );
    assert_eq!(
        remainder,
        UPoly::new(vec![Ring::add(
            &Ring::mul(&Ring::mul(&a, &a), &a),
            &MPoly::one()
        )])
    );
    assert_eq!(
        f.scale(&scale, &ctx).unwrap(),
        quotient
            .mul(&g, &ctx)
            .unwrap()
            .add(&remainder, &ctx)
            .unwrap()
    );
}
#[test]
fn fixed_seed_pseudo_division_exact_division_and_content_reconstruct() {
    let ctx = Interrupt::default();
    let mut rng = SplitMix64::new(0x4d352e32);
    for _ in 0..512 {
        let mut poly = || {
            UPoly::new(
                (0..7)
                    .map(|_| Integer::from(rng.next_range(0, 11) as i64 - 5))
                    .collect(),
            )
        };
        let (f, g) = (poly(), poly());
        let (content, primitive) = f.content_pp(&ctx).unwrap();
        assert_eq!(primitive.scale(&content, &ctx).unwrap(), f);
        assert!(primitive.is_zero() || primitive.lc().unwrap() > &Integer::ZERO);
        assert_eq!(f.content(&ctx).unwrap(), content);
        assert_eq!(f.primitive_part(&ctx).unwrap(), primitive);
        if !g.is_zero() {
            let (scale, a, b) = f.pseudo_divrem(&g, &ctx).unwrap().unwrap();
            assert_eq!(
                f.scale(&scale, &ctx).unwrap(),
                a.mul(&g, &ctx).unwrap().add(&b, &ctx).unwrap()
            );
            assert!(b.degree().is_none_or(|d| d < g.degree().unwrap()));
            if let Some(df) = f.degree().filter(|d| *d >= g.degree().unwrap()) {
                assert_eq!(scale, g.lc().unwrap().pow(df - g.degree().unwrap() + 1));
            }
            assert_eq!(
                f.mul(&g, &ctx).unwrap().exact_div(&g, &ctx).unwrap(),
                Some(f)
            );
        }
    }
}
#[test]
fn division_content_and_pseudo_remainder_budget_failures_return_abort() {
    for kind in 0..4 {
        let ctx = Interrupt::default();
        ctx.steps_left.set(3);
        let f = z(&[1; 40]);
        let g = z(&[1, 1]); // Monic divisor ensures exact division reaches its work loop.
        let failed = match kind {
            0 => f.prem(&g, &ctx).is_err(),
            1 => f.exact_div(&g, &ctx).is_err(),
            2 => f.content(&ctx).is_err(),
            _ => f.divide_scalar(&Integer::ONE, &ctx).is_err(),
        };
        assert!(failed, "operation {kind}");
    }
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(UPoly::<Integer>::zero().content(&ctx).is_err());
}
