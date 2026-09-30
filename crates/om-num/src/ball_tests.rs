use super::*;
use proptest::prelude::*;

fn q(n: i64, d: u64) -> Rational {
    Rational::from_parts(n.into(), d.into())
}
fn bounds(b: &Ball) -> (Rational, Rational) {
    let mid = Rational::try_from(b.mid.clone()).unwrap();
    let rad = Rational::try_from(b.rad.clone()).unwrap();
    (&mid - &rad, mid + rad)
}
fn contains(b: &Ball, value: &Rational) {
    let (lo, hi) = bounds(b);
    assert!(lo <= *value && *value <= hi, "{value:?} not in {b:?}");
}
fn interval(mid: i64, rad: i64, prec: u32) -> Ball {
    Ball {
        mid: BigFloat::from(mid).with_precision(prec as usize).value(),
        rad: BigFloat::from(rad).with_precision(prec as usize).value(),
        prec,
    }
}

#[test]
fn rational_conversion_encloses_rounding_including_negative_values() {
    for (n, d) in [(1, 3), (-1, 3), (17, 19), (-1234567, 101), (0, 1), (1, 1)] {
        for prec in [1, 2, 8, 53, 64, 120] {
            contains(&Ball::exact(&q(n, d), prec), &q(n, d));
        }
    }
    assert_eq!(Ball::exact(&q(1, 2), 64).rad, BigFloat::ZERO);
    assert!(Ball::exact(&q(1, 3), 64).rad > BigFloat::ZERO);
}

#[test]
fn sqrt_two_is_certified_by_exact_squared_endpoints() {
    for prec in [8, 53, 64, 120, 256] {
        let b = Ball::exact(&2.into(), prec).sqrt();
        let (lo, hi) = bounds(&b);
        assert!(lo >= Rational::ZERO);
        assert!(&lo * &lo <= 2.into() && &hi * &hi >= 2.into());
        assert!(b.rad > BigFloat::ZERO);
        assert!((b.to_f64() - 2_f64.sqrt()).abs() < 0.01);
    }
    assert_eq!(Ball::exact(&4.into(), 64).sqrt().rad, BigFloat::ZERO);
    assert_eq!(Ball::exact(&0.into(), 64).sqrt().to_f64(), 0.0);
}

#[test]
fn signs_uncertainty_and_division_domains() {
    let a = interval(2, 1, 64);
    let b = interval(-3, 1, 64);
    for x in [1, 3] {
        for y in [-4, -2] {
            contains(&a.add(&b), &(x + y).into());
            contains(&a.sub(&b), &(x - y).into());
            contains(&a.mul(&b), &(x * y).into());
            contains(&a.div(&b), &(-q(x, (-y) as u64)));
        }
    }
    let zero = interval(0, 1, 64);
    assert!(zero.contains_zero());
    assert!(!zero.excludes_zero());
    assert!(a.excludes_zero());
    assert!(a.div(&zero).rad.repr().is_infinite());
    assert!(interval(-1, 2, 64).sqrt().rad.repr().is_infinite());
    assert!(!a.div(&zero).excludes_zero());
}

#[test]
fn whole_line_propagates_without_infinity_arithmetic() {
    let whole = Ball::whole(64);
    let a = Ball::exact(&3.into(), 64);
    assert!(whole.contains_zero());
    assert!(!whole.excludes_zero());
    for b in [
        whole.add(&a),
        whole.sub(&a),
        whole.mul(&a),
        whole.div(&a),
        a.div(&whole),
        whole.sqrt(),
    ] {
        assert!(b.rad.repr().is_infinite());
        assert!(b.mid.repr().is_finite());
    }
}

#[test]
fn complex_arithmetic_encloses_exact_answers() {
    let a = CBall::exact(&1.into(), &2.into(), 64);
    let b = CBall::exact(&3.into(), &(-4).into(), 64);
    let product = a.mul(&b);
    contains(&product.re, &11.into());
    contains(&product.im, &2.into());
    let inverse = CBall::exact(&1.into(), &0.into(), 64).div(&a);
    contains(&inverse.re, &q(1, 5));
    contains(&inverse.im, &q(-2, 5));
    let i = CBall::exact(&0.into(), &1.into(), 64);
    let squared = i.pow_int(&2.into());
    contains(&squared.re, &(-1).into());
    contains(&squared.im, &0.into());
    let reciprocal = a.pow_int(&(-1).into());
    contains(&reciprocal.re, &q(1, 5));
    contains(&reciprocal.im, &q(-2, 5));
}

#[test]
fn cancellation_and_precision_reduction_keep_enclosures() {
    let a = Ball::exact(&q(1, 3), 120);
    let b = Ball::exact(&q(-1, 3), 8);
    let sum = a.add(&b);
    assert_eq!(sum.prec, 8);
    assert!(sum.contains_zero());
    let tiny = Rational::from_parts(Integer::ONE, dashu::integer::UBig::ONE << 2000);
    let ball = Ball::exact(&tiny, 64);
    assert!(ball.excludes_zero());
    contains(&ball, &tiny);
}

#[test]
fn complex_division_certifies_nonzero_imaginary_intervals() {
    let a = CBall {
        re: interval(2, 1, 64),
        im: interval(-1, 1, 64),
    };
    let b = CBall {
        re: interval(0, 1, 64),
        im: interval(3, 1, 64),
    };
    let quotient = a.div(&b);
    assert!(quotient.re.rad.repr().is_finite() && quotient.im.rad.repr().is_finite());
    for ar in [1, 3] {
        for ai in [-2, 0] {
            for br in [-1, 1] {
                for bi in [2, 4] {
                    let norm = (br * br + bi * bi) as u64;
                    contains(&quotient.re, &q(ar * br + ai * bi, norm));
                    contains(&quotient.im, &q(ai * br - ar * bi, norm));
                }
            }
        }
    }
    let zero = CBall::exact(&0.into(), &0.into(), 64);
    let quotient = a.div(&zero);
    assert!(quotient.re.rad.repr().is_infinite() && quotient.im.rad.repr().is_infinite());
    assert!(zero.pow_int(&0.into()).re.rad.repr().is_infinite());
}

#[test]
fn large_exponents_remain_finite_and_zero_boundary_is_inclusive() {
    let huge = Rational::from(Integer::ONE << 2000);
    let ball = Ball::exact(&huge, 120);
    contains(&ball.mul(&ball), &(&huge * &huge));
    contains(&ball.div(&ball), &Rational::ONE);
    assert!(ball.mul(&ball).mid.repr().is_finite());
    assert!(interval(1, 1, 64).contains_zero());
    assert!(interval(-1, 1, 64).contains_zero());
}

proptest! {
    #![proptest_config(ProptestConfig {cases:256,rng_seed:proptest::test_runner::RngSeed::Fixed(0x42414c4c),..ProptestConfig::default()})]
    #[test]
    fn arithmetic_encloses_exact_rational_results(a in -10000_i64..10000,b in -10000_i64..10000,
                                                da in 1_u64..1000,db in 1_u64..1000,prec in 2_u32..128) {
        let (a,b)=(q(a,da),q(b,db)); let (ba,bb)=(Ball::exact(&a,prec),Ball::exact(&b,prec));
        contains(&ba.add(&bb),&(&a+&b)); contains(&ba.sub(&bb),&(&a-&b)); contains(&ba.mul(&bb),&(&a*&b));
        if !bb.contains_zero() { contains(&ba.div(&bb),&(&a/&b)); }
        prop_assert!(ba.mul(&bb).sub(&bb.mul(&ba)).contains_zero());
    }
    #[test]
    fn arbitrary_finite_ball_products_enclose_all_corners(a in -1000_i64..1000,b in -1000_i64..1000,
                                                        ra in 0_i64..100,rb in 0_i64..100,prec in 12_u32..128) {
        let (a,b)=(interval(a,ra,prec),interval(b,rb,prec));
        let ((alo,ahi),(blo,bhi))=(bounds(&a),bounds(&b));
        let result=a.mul(&b);
        for x in [&alo,&ahi] { for y in [&blo,&bhi] { contains(&result,&(x*y)); } }
        prop_assert!(result.sub(&b.mul(&a)).contains_zero());
    }
}
