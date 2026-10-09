//! Numeric persistence keeps actual machine bits and binary mantissa/exponent/precision.
use om_num::checkpoint::{NumberLimits, decode_number, encode_number};
use om_num::ctx::Interrupt;
use om_num::{BigFloat, Complex, Integer, Number, Rational, Real};
fn roundtrip(number: Number) {
    let limits = NumberLimits::default();
    let ctx = Interrupt::default();
    let bytes = encode_number(&number, limits, &ctx).unwrap();
    let restored = decode_number(&bytes, limits, &ctx).unwrap();
    assert_eq!(encode_number(&restored, limits, &ctx).unwrap(), bytes);
    assert_eq!(restored, number);
}
#[test]
fn exact_binary_floats_and_complex_numbers_roundtrip_without_decimal_projection() {
    for i in [0, -1, 127, 128, -128, -129] {
        roundtrip(Number::Integer(i.into()));
    }
    roundtrip(Number::Integer(Integer::ONE << 4096));
    roundtrip(Number::Rational(
        "12345678901234567890123456789/100000000000000000000000000003"
            .parse::<Rational>()
            .unwrap(),
    ));
    for bits in [
        0,
        1,
        0x8000000000000000,
        0x0010000000000000,
        0x7fefffffffffffff,
        0x3fb999999999999a,
    ] {
        let value = Number::Real(Real::Machine(f64::from_bits(bits)));
        let encoded =
            encode_number(&value, NumberLimits::default(), &Interrupt::default()).unwrap();
        let Number::Real(Real::Machine(restored)) =
            decode_number(&encoded, NumberLimits::default(), &Interrupt::default()).unwrap()
        else {
            panic!("machine category changed")
        };
        assert_eq!(restored.to_bits(), bits);
        roundtrip(value);
    }
    let big = BigFloat::from_parts((Integer::ONE << 190) + Integer::from(3), -12000)
        .with_precision(257)
        .value();
    roundtrip(Number::Real(Real::Big(big)));
    roundtrip(Number::Complex(Box::new(Complex {
        re: Number::Rational("1/3".parse().unwrap()),
        im: Number::Rational("2/7".parse().unwrap()),
    })));
}
#[test]
fn invalid_numbers_version_limits_truncation_and_abort_do_not_produce_partial_values() {
    let limits = NumberLimits::default();
    let ctx = Interrupt::default();
    assert!(encode_number(&Number::Real(Real::Machine(f64::NAN)), limits, &ctx).is_err());
    let bytes = encode_number(&Number::Real(Real::Machine(-0.0)), limits, &ctx).unwrap();
    for n in 0..bytes.len() {
        assert!(decode_number(&bytes[..n], limits, &ctx).is_err());
    }
    let mut future = bytes.clone();
    future[4] = 2;
    assert!(decode_number(&future, limits, &ctx).is_err());
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(decode_number(&tail, limits, &ctx).is_err());
    assert!(
        decode_number(
            &bytes,
            NumberLimits {
                max_bytes: 8,
                ..limits
            },
            &ctx
        )
        .is_err()
    );
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(decode_number(&bytes, limits, &ctx).is_err());
}

#[test]
fn high_precision_signed_zero_and_boundary_mantissas_are_not_reinterpreted() {
    for value in [
        BigFloat::ZERO.with_precision(137).value(),
        (-BigFloat::ZERO).with_precision(137).value(),
    ] {
        roundtrip(Number::Real(Real::Big(value)));
    }
    for exponent in [-1000000, -1, 0, 1, 1000000] {
        for sign in [-1, 1] {
            roundtrip(Number::Real(Real::Big(
                BigFloat::from_parts(Integer::from(sign), exponent)
                    .with_precision(4096)
                    .value(),
            )));
        }
    }
}

#[test]
fn raw_big_float_infinity_sentinels_and_noncanonical_mantissas_are_rejected() {
    let encoded = encode_number(
        &Number::Real(Real::Big(BigFloat::ZERO.with_precision(53).value())),
        NumberLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    for exponent in [i64::MAX, i64::MIN, 2, -2] {
        let mut malformed = encoded.clone();
        malformed[10..18].copy_from_slice(&exponent.to_le_bytes());
        assert!(decode_number(&malformed, NumberLimits::default(), &Interrupt::default()).is_err());
    }
    let one = encode_number(
        &Number::Real(Real::Big(BigFloat::ONE.with_precision(53).value())),
        NumberLimits::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let mut even = one.clone();
    even[10] = 2;
    assert!(decode_number(&even, NumberLimits::default(), &Interrupt::default()).is_err());
    for exponent in [i64::MAX, i64::MIN] {
        let mut malformed = one.clone();
        malformed[11..19].copy_from_slice(&exponent.to_le_bytes());
        assert!(decode_number(&malformed, NumberLimits::default(), &Interrupt::default()).is_err());
    }
}

#[test]
fn seeded_machine_bit_corpus_preserves_all_finite_payloads() {
    let mut random = om_num::rng::SplitMix64::new(91);
    for _ in 0..4096 {
        let bits = random.next_u64();
        let value = f64::from_bits(bits);
        if !value.is_finite() {
            continue;
        }
        let bytes = encode_number(
            &Number::Real(Real::Machine(value)),
            NumberLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
        let Number::Real(Real::Machine(actual)) =
            decode_number(&bytes, NumberLimits::default(), &Interrupt::default()).unwrap()
        else {
            panic!("not machine")
        };
        assert_eq!(actual.to_bits(), bits);
    }
}
