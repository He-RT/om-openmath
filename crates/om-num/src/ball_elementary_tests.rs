use super::*;
use proptest::prelude::*;

fn q(n: i64, d: u64) -> Rational {
    Rational::from_parts(n.into(), d.into())
}
fn bounds(b: &Ball) -> (Rational, Rational) {
    let m = Rational::try_from(b.mid.clone()).unwrap();
    let r = Rational::try_from(b.rad.clone()).unwrap();
    (&m - &r, m + r)
}
fn contains(b: &Ball, q: &Rational) {
    let (lo, hi) = bounds(b);
    assert!(lo <= *q && *q <= hi, "{q} outside {b:?}");
}
fn overlaps(a: &Ball, b: &Ball) {
    let (alo, ahi) = bounds(a);
    let (blo, bhi) = bounds(b);
    assert!(alo <= bhi && blo <= ahi, "{a:?} does not overlap {b:?}");
}
fn apply(b: &Ball, func: &str) -> Ball {
    match func {
        "exp" => b.exp(),
        "ln" => b.ln(),
        "sin" => b.sin(),
        "cos" => b.cos(),
        "atan" => b.atan(),
        _ => panic!("fixture function"),
    }
}

#[test]
fn real_functions_match_binary64_with_the_planned_tolerance() {
    for x in [-3.0_f64, -1.0, -0.01, 0.0, 0.25, 1.0, 3.0] {
        let b = Ball::exact(&Rational::try_from(x).unwrap(), 80);
        for (name, expected) in [
            ("exp", x.exp()),
            ("sin", x.sin()),
            ("cos", x.cos()),
            ("atan", x.atan()),
        ] {
            assert!(
                (apply(&b, name).to_f64() - expected).abs() < 1e-15,
                "{name}({x})"
            );
        }
        if x > 0.0 {
            assert!((b.ln().to_f64() - x.ln()).abs() < 1e-15);
        }
    }
}

#[test]
fn pi_has_the_first_thousand_decimal_digits_with_certified_endpoints() {
    let b = Ball::pi(3400);
    let digits = include_str!("../tests/pi_1000.txt").trim().replace('.', "");
    let expected: Integer = digits.parse().unwrap();
    let scale = Integer::from(10).pow(1000);
    let (lo, hi) = bounds(&b);
    for endpoint in [lo, hi] {
        let scaled = endpoint * Rational::from(scale.clone());
        assert_eq!(scaled.numerator() / scaled.denominator(), expected);
    }
    let cached = Ball::pi(3400);
    assert_eq!(b.mid, cached.mid);
    assert_eq!(b.rad, cached.rad);
}

#[test]
fn high_precision_values_match_independent_decimal_fixtures() {
    for line in include_str!("../tests/elementary.tsv")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        let input = Rational::from_str_radix(fields[1], 10).unwrap();
        let floor: Integer = fields[2].parse().unwrap();
        let scale = dashu::integer::UBig::from(10_u8).pow(100);
        let lo = Rational::from_parts(floor.clone(), scale.clone());
        let hi = Rational::from_parts(floor + 1, scale);
        let result = apply(&Ball::exact(&input, 256), fields[0]);
        // A 100-digit independently computed enclosure is much narrower than the 256-bit ball.
        contains(&result, &lo);
        contains(&result, &hi);
    }
}

#[test]
fn whole_and_nonpositive_domains_are_conservative() {
    let whole = Ball::whole(80);
    for b in [whole.sin(), whole.cos()] {
        contains(&b, &(-1).into());
        contains(&b, &1.into());
    }
    let atan = whole.atan();
    overlaps(&atan, &Ball::pi(80).div(&Ball::exact(&2.into(), 80)));
    assert!(whole.exp().rad.repr().is_infinite());
    assert!(whole.ln().rad.repr().is_infinite());
    for q in [q(-1, 1), q(0, 1)] {
        assert!(Ball::exact(&q, 80).ln().rad.repr().is_infinite());
    }
    contains(&Ball::exact(&0.into(), 80).sin(), &0.into());
    contains(&Ball::exact(&0.into(), 80).cos(), &1.into());
    contains(&Ball::exact(&0.into(), 80).exp(), &1.into());
    contains(&Ball::exact(&1.into(), 80).ln(), &0.into());
}

#[test]
fn uncertain_inputs_include_interior_trigonometric_extrema() {
    let x = Ball {
        mid: BigFloat::from(2).with_precision(80).value(),
        rad: BigFloat::from(2).with_precision(80).value(),
        prec: 80,
    };
    contains(&x.sin(), &1.into());
    contains(&x.cos(), &1.into());
    contains(&x.cos(), &(-1).into());
    let p = Ball::pi(128);
    contains(&p.sin(), &0.into());
    contains(&p.cos(), &(-1).into());
    overlaps(
        &Ball::exact(&q(1, 3), 128)
            .atan()
            .add(&Ball::exact(&3.into(), 128).atan()),
        &p.div(&Ball::exact(&2.into(), 128)),
    );
}

#[test]
fn complex_principal_values_and_elementary_identities() {
    let i = CBall::exact(&0.into(), &1.into(), 128);
    let minus_one = CBall::exact(&(-1).into(), &0.into(), 128);
    let sqrt = minus_one.sqrt();
    contains(&sqrt.re, &0.into());
    contains(&sqrt.im, &1.into());
    let log = minus_one.ln();
    contains(&log.re, &0.into());
    overlaps(&log.im, &Ball::pi(128));
    let sqrt_i = i.sqrt();
    overlaps(&sqrt_i.re, &Ball::exact(&q(1, 2), 128).sqrt());
    overlaps(&sqrt_i.im, &sqrt_i.re);
    for (re, im) in [(1, 2), (-3, -4), (-3, 4), (0, -1), (0, 1), (0, 0)] {
        let z = CBall::exact(&re.into(), &im.into(), 128);
        let squared = z.sqrt().mul(&z.sqrt());
        contains(&squared.re, &re.into());
        contains(&squared.im, &im.into());
        let trig = z.sin().mul(&z.sin()).add(&z.cos().mul(&z.cos()));
        contains(&trig.re, &1.into());
        contains(&trig.im, &0.into());
        if re != 0 || im != 0 {
            let roundtrip = z.ln().exp();
            contains(&roundtrip.re, &re.into());
            contains(&roundtrip.im, &im.into());
        }
    }
}

#[test]
fn logarithm_cut_and_sqrt_sign_uncertainty_are_enclosed() {
    let z = CBall {
        re: Ball::exact(&(-1).into(), 128),
        im: Ball {
            mid: BigFloat::ZERO.with_precision(128).value(),
            rad: BigFloat::from_parts(Integer::ONE, -10)
                .with_precision(128)
                .value(),
            prec: 128,
        },
    };
    let log = z.ln();
    let (_, upper) = bounds(&Ball::pi(128));
    contains(&log.im, &upper);
    contains(&log.im, &(-upper));
    let sqrt = z.sqrt();
    contains(&sqrt.im, &1.into());
    contains(&sqrt.im, &(-1).into());
    let zero = CBall::exact(&0.into(), &0.into(), 128).ln();
    assert!(zero.re.rad.repr().is_infinite() && zero.im.rad.repr().is_infinite());
}

#[test]
fn complex_functions_match_independent_decimal_fixtures() {
    for line in include_str!("../tests/complex_elementary.tsv")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        let f: Vec<_> = line.split('\t').collect();
        let re: Integer = f[1].parse().unwrap();
        let im: Integer = f[2].parse().unwrap();
        let input = CBall::exact(&re.into(), &im.into(), 256);
        let result = match f[0] {
            "exp" => input.exp(),
            "ln" => input.ln(),
            "sqrt" => input.sqrt(),
            "sin" => input.sin(),
            "cos" => input.cos(),
            _ => panic!("fixture function"),
        };
        let b = if f[3] == "re" { result.re } else { result.im };
        let value: Integer = f[4].parse().unwrap();
        let offset: u8 = f[5].parse().unwrap();
        let scale = dashu::integer::UBig::from(10_u8).pow(100);
        contains(&b, &Rational::from_parts(value.clone(), scale.clone()));
        contains(&b, &Rational::from_parts(value + offset, scale));
    }
}

proptest! {
    #![proptest_config(ProptestConfig {cases:96,rng_seed:proptest::test_runner::RngSeed::Fixed(0x454c454d),..ProptestConfig::default()})]
    #[test]
    fn varied_precision_enclosures_contain_directed_library_oracles(n in -2048_i64..2048,prec in 8_u32..160) {
        let input=q(n,256);
        let b=Ball::exact(&input,prec);
        let lo:Lower=input.to_float(256).value(); let hi:Upper=input.to_float(256).value();
        for (name,lower,upper) in [("exp",lo.exp(),hi.exp()),("sin",lo.sin(),hi.sin()),("cos",lo.cos(),hi.cos()),("atan",lo.atan(),hi.atan())] {
            let result=apply(&b,name);
            contains(&result,&Rational::try_from(lower).unwrap());
            contains(&result,&Rational::try_from(upper).unwrap());
        }
        if n>0 {
            contains(&b.ln(),&Rational::try_from(lo.ln()).unwrap());
            contains(&b.ln(),&Rational::try_from(hi.ln()).unwrap());
        }
    }
}
