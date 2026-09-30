//! Resultant/discriminant vectors with independent Sylvester determinant checks.
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
    rng::SplitMix64,
};
use om_poly::{MPoly, MonoOrder, Monomial, Ring, UPoly};
fn z(c: &[i64]) -> UPoly<Integer> {
    UPoly::new(c.iter().map(|c| (*c).into()).collect())
}
fn m(n: usize, terms: &[(Vec<u32>, i64)]) -> MPoly<Integer> {
    MPoly::new(
        n,
        terms
            .iter()
            .map(|(e, c)| (Monomial::new(e.clone()).unwrap(), (*c).into()))
            .collect(),
        MonoOrder::Lex,
        &Interrupt::default(),
    )
    .unwrap()
}
fn sylvester(f: &UPoly<Integer>, g: &UPoly<Integer>) -> Integer {
    if f.is_zero() || g.is_zero() {
        return Integer::ZERO;
    }
    let (a, b) = (f.degree().unwrap(), g.degree().unwrap());
    let n = a + b;
    let mut matrix = vec![vec![Rational::ZERO; n]; n];
    for (i, row) in matrix.iter_mut().take(b).enumerate() {
        for j in 0..=a {
            row[i + j] = Rational::from(f.coeffs[a - j].clone());
        }
    }
    for (i, row) in matrix.iter_mut().skip(b).enumerate() {
        for j in 0..=b {
            row[i + j] = Rational::from(g.coeffs[b - j].clone());
        }
    }
    let mut determinant = Rational::ONE;
    for col in 0..n {
        let Some(pivot) = (col..n).find(|r| !matrix[*r][col].is_zero()) else {
            return Integer::ZERO;
        };
        if pivot != col {
            matrix.swap(pivot, col);
            determinant = -determinant;
        }
        let pivot = matrix[col][col].clone();
        determinant *= &pivot;
        let pivot_row = matrix[col].clone();
        for row in matrix.iter_mut().skip(col + 1) {
            let multiplier = &row[col] / &pivot;
            for (value, pivot_value) in row[col..].iter_mut().zip(&pivot_row[col..]) {
                *value -= &multiplier * pivot_value;
            }
        }
    }
    assert!(determinant.denominator().is_one());
    determinant.numerator().clone()
}
#[test]
fn authority_resultants_and_parameter_discriminants() {
    let ctx = Interrupt::default();
    assert_eq!(
        z(&[1, 0, 1]).resultant(&z(&[-1, 1]), &ctx).unwrap(),
        Integer::from(2)
    );
    let x = m(1, &[(vec![1], 1)]);
    let x2 = Ring::mul(&x, &x);
    let one = MPoly::one();
    let two = Ring::add(&one, &one);
    let f = UPoly::new(vec![Ring::sub(&x2, &one), MPoly::zero(), one.clone()]);
    let g = UPoly::new(vec![Ring::neg(&x), one.clone()]);
    assert_eq!(
        f.resultant(&g, &ctx).unwrap(),
        Ring::sub(&Ring::mul(&two, &x2), &one)
    );
    let b = m(2, &[(vec![1, 0], 1)]);
    let c = m(2, &[(vec![0, 1], 1)]);
    let f = UPoly::new(vec![c.clone(), b.clone(), one.clone()]);
    assert_eq!(
        f.discriminant(&ctx).unwrap(),
        m(2, &[(vec![2, 0], 1), (vec![0, 1], -4)])
    );
    let f = UPoly::new(vec![c, b, MPoly::zero(), one.clone()]);
    assert_eq!(
        f.discriminant(&ctx).unwrap(),
        m(2, &[(vec![3, 0], -4), (vec![0, 2], -27)])
    );
    let f = UPoly::new(vec![Ring::neg(&two), MPoly::zero(), one.clone()]);
    let g = UPoly::new(vec![
        Ring::sub(&x2, &Ring::add(&two, &one)),
        Ring::neg(&Ring::mul(&two, &x)),
        one,
    ]);
    assert_eq!(
        f.resultant(&g, &ctx).unwrap(),
        m(1, &[(vec![4], 1), (vec![2], -10), (vec![0], 1)])
    );
}
#[test]
fn constant_zero_content_and_resultant_sign_conventions() {
    let ctx = Interrupt::default();
    for (f, g, result) in [
        (z(&[]), z(&[2]), 0),
        (z(&[2]), z(&[3]), 1),
        (z(&[-2]), z(&[1, 0, 0, 3]), -8),
        (z(&[1, 0, 0, 3]), z(&[-2]), -8),
        (z(&[-1, 1]), z(&[-2, 1]), -1),
        (z(&[-2, 1]), z(&[-1, 1]), 1),
        (z(&[2, 0, 2]), z(&[-3, 3]), 36),
        (z(&[-2, 0, -2]), z(&[-3, 3]), -36),
        (z(&[-1, 0, 1]), z(&[-1, 1]), 0),
        (z(&[1, 0, 0, 0, 1]), z(&[0, 0, 2]), 16),
    ] {
        assert_eq!(f.resultant(&g, &ctx).unwrap(), Integer::from(result));
        assert_eq!(sylvester(&f, &g), Integer::from(result));
    }
    for f in [z(&[]), z(&[7])] {
        assert_eq!(f.discriminant(&ctx).unwrap(), Integer::ZERO);
    }
    assert_eq!(z(&[7, -3]).discriminant(&ctx).unwrap(), Integer::ONE);
    assert_eq!(z(&[1, -2, 1]).discriminant(&ctx).unwrap(), Integer::ZERO);
    assert_eq!(
        z(&[3, 2, 2]).discriminant(&ctx).unwrap(),
        Integer::from(-20)
    );
}
#[test]
fn parameter_contents_and_nonmonic_prs_retain_exact_scaling() {
    let ctx = Interrupt::default();
    let x = m(1, &[(vec![1], 1)]);
    let x1 = m(1, &[(vec![1], 1), (vec![0], 1)]);
    let x2 = m(1, &[(vec![1], 1), (vec![0], 2)]);
    let f = UPoly::new(vec![Ring::neg(&x), MPoly::one()])
        .scale(&x1, &ctx)
        .unwrap();
    let g = UPoly::new(vec![MPoly::one(), x.clone()])
        .scale(&x2, &ctx)
        .unwrap();
    let base = m(1, &[(vec![2], 1), (vec![0], 1)]);
    let want = Ring::mul(&Ring::mul(&x1, &x2), &base);
    assert_eq!(f.resultant(&g, &ctx).unwrap(), want);
    let common = f.mul(&g, &ctx).unwrap();
    assert!(f.resultant(&common, &ctx).unwrap().is_zero());
    assert_eq!(f.discriminant(&ctx).unwrap(), MPoly::one());
}
#[test]
fn fixed_seed_resultants_match_sylvester_determinants_and_discriminant_identities() {
    let ctx = Interrupt::default();
    let mut rng = SplitMix64::new(0x4d352e35);
    for _ in 0..256 {
        let mut poly = || {
            let len = rng.next_range(0, 7);
            UPoly::new(
                (0..len)
                    .map(|_| Integer::from(rng.next_range(0, 11) as i64 - 5))
                    .collect(),
            )
        };
        let (f, g) = (poly(), poly());
        let res = f.resultant(&g, &ctx).unwrap();
        assert_eq!(res, sylvester(&f, &g), "f={f:?},g={g:?}");
        let reverse = g.resultant(&f, &ctx).unwrap();
        let odd = f.degree().unwrap_or(0) % 2 == 1 && g.degree().unwrap_or(0) % 2 == 1;
        assert_eq!(reverse, if odd { -res.clone() } else { res.clone() });
        let df = f.derivative(&ctx).unwrap();
        let disc = f.discriminant(&ctx).unwrap();
        if let Some(n) = f.degree().filter(|d| *d > 0) {
            let resultant = f.resultant(&df, &ctx).unwrap();
            let sign = if n % 4 >= 2 {
                -Integer::ONE
            } else {
                Integer::ONE
            };
            assert_eq!(disc * f.lc().unwrap(), sign * resultant);
        } else {
            assert!(disc.is_zero());
        }
        if !f.is_zero() && !g.is_zero() {
            assert_eq!(
                f.scale(&(-2).into(), &ctx)
                    .unwrap()
                    .resultant(&g.scale(&3.into(), &ctx).unwrap(), &ctx)
                    .unwrap(),
                Integer::from(-2).pow(g.degree().unwrap())
                    * Integer::from(3).pow(f.degree().unwrap())
                    * res
            );
        }
    }
}
#[test]
fn parameter_resultants_commute_with_degree_preserving_specializations() {
    let ctx = Interrupt::default();
    let mut rng = SplitMix64::new(0x50415241);
    for _ in 0..32 {
        let mut poly = || {
            let mut coeffs = vec![];
            for _ in 0..3 {
                coeffs.push(m(
                    1,
                    &[
                        (vec![1], rng.next_range(0, 7) as i64 - 3),
                        (vec![0], rng.next_range(0, 7) as i64 - 3),
                    ],
                ));
            }
            coeffs.push(MPoly::one());
            UPoly::new(coeffs)
        };
        let (f, g) = (poly(), poly());
        let res = f.resultant(&g, &ctx).unwrap();
        let evaluate = |c: &MPoly<Integer>, value: i64| {
            c.terms.iter().fold(Integer::ZERO, |sum, (m, c)| {
                sum + c * Integer::from(value).pow(m.exps.first().copied().unwrap_or(0) as usize)
            })
        };
        for value in [-3, -1, 0, 2, 5] {
            let specialize = |p: &UPoly<MPoly<Integer>>| {
                UPoly::new(p.coeffs.iter().map(|c| evaluate(c, value)).collect())
            };
            assert_eq!(
                evaluate(&res, value),
                sylvester(&specialize(&f), &specialize(&g))
            );
        }
    }
}
#[test]
fn resultant_and_discriminant_propagate_interrupts_before_boundary_returns() {
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert_eq!(z(&[]).resultant(&z(&[]), &ctx), Err(Abort::Budget));
    assert_eq!(z(&[7]).discriminant(&ctx), Err(Abort::Budget));
    ctx.steps_left.set(10);
    assert_eq!(
        z(&[1; 30]).resultant(&z(&[1; 20]), &ctx),
        Err(Abort::Budget)
    );
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        UPoly::<MPoly<Integer>>::zero().discriminant(&ctx),
        Err(Abort::Interrupted)
    );
}
