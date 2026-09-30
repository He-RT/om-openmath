use super::*;
use proptest::prelude::*;
use std::cmp::Ordering;

fn int(n: i64) -> Number {
    Number::Integer(n.into())
}
fn rat(n: i64, d: u64) -> Number {
    Number::Rational(Rational::from_parts(n.into(), d.into()))
}
fn machine(n: f64) -> Number {
    Number::Real(Real::Machine(n))
}
fn big(n: f64, p: usize) -> Number {
    Number::Real(Real::Big(
        BigFloat::try_from(n).unwrap().with_precision(p).value(),
    ))
}
fn complex(re: Number, im: Number) -> Number {
    Number::Complex(Box::new(Complex { re, im }))
}
fn assert_machine(n: Number, value: f64) {
    assert!(
        matches!(n, Number::Real(Real::Machine(x)) if x == value),
        "{n:?}"
    );
}
fn assert_bits(n: &Number, bits: u32) {
    assert_eq!(n.precision(), Precision::Bits(bits));
}

#[test]
fn rational_with_unit_denominator_normalizes_to_integer() {
    assert_eq!(rat(6, 3).normalize(), int(2));
}
#[test]
fn rational_zero_normalizes_to_integer() {
    assert_eq!(rat(0, 5).normalize(), int(0));
}
#[test]
fn exact_complex_zero_imaginary_collapses() {
    assert_eq!(complex(rat(2, 3), int(0)).normalize(), rat(2, 3));
}
#[test]
fn approximate_complex_zero_imaginary_is_retained() {
    assert!(matches!(
        complex(int(2), machine(0.0)).normalize(),
        Number::Complex(_)
    ));
}
#[test]
fn complex_components_share_machine_precision() {
    let Number::Complex(c) = complex(int(1), machine(2.0)).normalize() else {
        panic!()
    };
    assert_machine(c.re, 1.0);
    assert_machine(c.im, 2.0);
}
#[test]
fn exact_addition_reduces_rationals() {
    assert_eq!(rat(1, 3).add(&rat(1, 6)), rat(1, 2));
}
#[test]
fn exact_addition_can_become_integer() {
    assert_eq!(rat(1, 3).add(&rat(2, 3)), int(1));
}
#[test]
fn integers_do_not_overflow_machine_words() {
    let huge = Integer::from(1) << 200;
    assert_eq!(
        Number::Integer(huge.clone()).add(&int(1)),
        Number::Integer(huge + 1)
    );
}
#[test]
fn exact_multiplication_reduces_rationals() {
    assert_eq!(rat(2, 3).mul(&rat(9, 4)), rat(3, 2));
}
#[test]
fn negative_rational_is_negated_exactly() {
    assert_eq!(rat(-2, 3).neg(), rat(2, 3));
}
#[test]
fn integer_reciprocal_is_exact_rational() {
    assert_eq!(int(3).recip().unwrap(), rat(1, 3));
}
#[test]
fn negative_rational_reciprocal_normalizes() {
    assert_eq!(rat(-1, 4).recip().unwrap(), int(-4));
}
#[test]
fn every_representation_of_zero_has_no_reciprocal() {
    for zero in [
        int(0),
        rat(0, 7),
        machine(0.0),
        machine(-0.0),
        big(0.0, 80),
        complex(machine(0.0), machine(0.0)),
    ] {
        assert_eq!(zero.recip(), Err(NumError::DivByZero));
    }
}
#[test]
fn negative_integer_power_is_exact() {
    assert_eq!(int(2).pow_int(&(-3).into()).unwrap(), rat(1, 8));
}
#[test]
fn rational_power_with_positive_exponent() {
    assert_eq!(rat(2, 3).pow_int(&3.into()).unwrap(), rat(8, 27));
}
#[test]
fn zero_exponent_yields_one_for_nonzero_base() {
    assert_eq!(int(7).pow_int(&0.into()).unwrap(), int(1));
}
#[test]
fn zero_to_zero_is_indeterminate() {
    assert_eq!(int(0).pow_int(&0.into()), Err(NumError::Indeterminate));
}
#[test]
fn exact_and_machine_arithmetic_remains_machine() {
    assert_machine(rat(1, 2).add(&machine(0.25)), 0.75);
}
#[test]
fn exact_and_big_arithmetic_preserves_big_precision() {
    let n = rat(1, 2).mul(&big(3.0, 80));
    assert_bits(&n, 80);
    assert_eq!(n.to_f64(), Some(1.5));
}
#[test]
fn machine_and_big_selects_machine() {
    assert_machine(machine(2.0).add(&big(3.0, 120)), 5.0);
}
#[test]
fn two_big_operands_select_minimum_precision() {
    let n = big(2.0, 120).mul(&big(3.0, 80));
    assert_bits(&n, 80);
    assert_eq!(n.to_f64(), Some(6.0));
}
#[test]
fn machine_multiplication_overflow_promotes_to_finite_big() {
    let n = machine(f64::MAX).mul(&machine(2.0));
    assert_bits(&n, 53);
    let Number::Real(Real::Big(value)) = n else {
        panic!()
    };
    assert!(value.repr().is_finite());
    assert_eq!(
        value,
        BigFloat::try_from(f64::MAX).unwrap() * BigFloat::from(2)
    );
}
#[test]
fn machine_addition_overflow_promotes_to_finite_big() {
    let n = machine(f64::MAX).add(&machine(f64::MAX));
    assert_bits(&n, 53);
    assert!(matches!(n, Number::Real(Real::Big(ref x)) if x.repr().is_finite()));
}
#[test]
fn machine_reciprocal_overflow_promotes_to_big() {
    let n = machine(f64::from_bits(1)).recip().unwrap();
    assert_bits(&n, 53);
    assert!(matches!(n, Number::Real(Real::Big(ref x)) if x.repr().is_finite()));
}
#[test]
fn huge_exact_operand_does_not_create_machine_infinity() {
    let n = Number::Integer(Integer::from(1) << 2000).add(&machine(1.0));
    assert_bits(&n, 53);
    assert_eq!(n.to_f64(), None);
}
#[test]
fn complex_multiplication_uses_exact_cross_terms() {
    assert_eq!(
        complex(int(1), int(2)).mul(&complex(int(3), int(-4))),
        complex(int(11), int(2))
    );
}
#[test]
fn imaginary_unit_squared_is_minus_one() {
    let i = complex(int(0), int(1));
    assert_eq!(i.mul(&i), int(-1));
}
#[test]
fn complex_reciprocal_uses_exact_squared_magnitude() {
    assert_eq!(
        complex(int(1), int(2)).recip().unwrap(),
        complex(rat(1, 5), rat(-2, 5))
    );
}
#[test]
fn complex_reciprocal_avoids_machine_squared_magnitude_overflow() {
    let n = complex(machine(1e200), machine(1e200)).recip().unwrap();
    let (re, im) = n.to_complex_f64();
    assert!(re.is_finite() && im.is_finite());
    assert!((re / 5e-201 - 1.0).abs() < 1e-14);
    assert!((im / -5e-201 - 1.0).abs() < 1e-14);
}
#[test]
fn exact_comparison_does_not_round_large_integers_to_f64() {
    let a = Number::Integer((Integer::from(1) << 100) + 1);
    let b = Number::Integer(Integer::from(1) << 100);
    assert_eq!(a.cmp_real(&b), Some(Ordering::Greater));
}
#[test]
fn exact_machine_comparison_is_mathematical_not_coerced() {
    let a = Number::Integer(Integer::from(9_007_199_254_740_993_u64));
    assert_eq!(
        a.cmp_real(&machine(9_007_199_254_740_992.0)),
        Some(Ordering::Greater)
    );
}
#[test]
fn complex_numbers_are_not_ordered_as_reals() {
    assert_eq!(complex(int(1), int(1)).cmp_real(&int(2)), None);
    assert_eq!(complex(int(1), machine(0.0)).normalize().to_f64(), None);
}
#[test]
fn sign_and_identity_predicates_cover_scalar_categories() {
    assert!(rat(-1, 2).is_negative());
    assert!(big(-1.0, 80).is_negative());
    assert!(!complex(int(-1), int(2)).is_negative());
    assert!(machine(-0.0).is_zero());
    assert!(big(1.0, 80).is_one());
    assert!(int(1).is_exact());
    assert!(!machine(1.0).is_exact());
}
#[test]
fn arithmetic_roundtrips_through_serde_without_losing_precision() {
    let n = complex(big(1.25, 80), big(-2.5, 80));
    let encoded = serde_json::to_string(&n).unwrap();
    let decoded: Number = serde_json::from_str(&encoded).unwrap();
    assert_eq!(n, decoded);
    assert_bits(&decoded, 80);
}

#[test]
fn mixed_complex_precision_is_selected_before_arithmetic() {
    let n = complex(big(1.0, 120), big(2.0, 120)).add(&machine(3.0));
    let Number::Complex(c) = n else { panic!() };
    assert_machine(c.re, 4.0);
    assert_machine(c.im, 2.0);
}

#[test]
fn complex_normalization_promotes_both_components_if_machine_cannot_hold_one() {
    let n = complex(Number::Integer(Integer::ONE << 2000), machine(1.0)).normalize();
    let Number::Complex(c) = n else { panic!() };
    assert_bits(&c.re, 53);
    assert_bits(&c.im, 53);
}

#[test]
fn nested_complex_components_are_flattened() {
    assert_eq!(
        complex(complex(int(1), int(2)), complex(int(3), int(4))).normalize(),
        complex(int(-3), int(5))
    );
}

#[test]
fn complex_reciprocal_avoids_squared_magnitude_underflow() {
    let n = complex(machine(1e-200), machine(1e-200)).recip().unwrap();
    let (re, im) = n.to_complex_f64();
    assert!((re / 5e199 - 1.0).abs() < 1e-14);
    assert!((im / -5e199 - 1.0).abs() < 1e-14);
}

#[test]
fn huge_signed_exponents_work_for_unit_bases() {
    let exponent = (Integer::ONE << 100) + 1;
    assert_eq!(int(-1).pow_int(&exponent).unwrap(), int(-1));
    assert_eq!(int(1).pow_int(&(-exponent)).unwrap(), int(1));
}

#[test]
fn big_comparison_does_not_round_through_machine_precision() {
    let a = BigFloat::from_parts((Integer::ONE << 100) + 1, -100)
        .with_precision(120)
        .value();
    assert_eq!(
        Number::Real(Real::Big(a)).cmp_real(&int(1)),
        Some(Ordering::Greater)
    );
    assert_eq!(rat(1, 10).cmp_real(&machine(0.1)), Some(Ordering::Less));
}

#[test]
fn big_serde_preserves_nontrivial_significand_exponent_and_precision() {
    for (mantissa, exponent, bits) in [
        (123456789_i64, -300, 120),
        (-123456789, 300, 80),
        (0, 0, 53),
    ] {
        let n = Number::Real(Real::Big(
            BigFloat::from_parts(mantissa.into(), exponent)
                .with_precision(bits)
                .value(),
        ));
        let decoded: Number = serde_json::from_str(&serde_json::to_string(&n).unwrap()).unwrap();
        assert_eq!(n, decoded);
        assert_eq!(n.precision(), decoded.precision());
    }
}

#[test]
fn finite_big_can_cancel_after_machine_overflow() {
    let n = machine(f64::MAX).add(&machine(f64::MAX));
    assert!(n.add(&n.neg()).is_zero());
}

#[test]
fn arithmetic_vectors_from_canonical_spec() {
    assert_eq!(rat(1, 2).add(&rat(1, 3)), rat(5, 6));
    assert_machine(int(1).add(&machine(2.5)), 3.5);
    assert_eq!(
        complex(int(1), int(1)).mul(&complex(int(1), int(-1))),
        int(2)
    );
}

#[test]
fn huge_exact_power_stops_at_the_bit_limit() {
    assert_eq!(
        int(2).pow_int(&(Integer::ONE << 100)),
        Err(NumError::ExactOverflow)
    );
    assert_eq!(
        rat(1, 2).pow_int(&(Integer::ONE << 100)),
        Err(NumError::ExactOverflow)
    );
}

#[test]
fn negative_powers_of_zero_are_rejected() {
    for zero in [int(0), machine(0.0), big(0.0, 80)] {
        assert_eq!(zero.pow_int(&(-1).into()), Err(NumError::DivByZero));
    }
}

#[test]
fn precision_contagion_matrix_is_symmetric() {
    let values = [int(2), rat(3, 2), machine(2.0), big(2.0, 80), big(2.0, 120)];
    let expected = [
        [
            Precision::Exact,
            Precision::Exact,
            Precision::Machine,
            Precision::Bits(80),
            Precision::Bits(120),
        ],
        [
            Precision::Exact,
            Precision::Exact,
            Precision::Machine,
            Precision::Bits(80),
            Precision::Bits(120),
        ],
        [Precision::Machine; 5],
        [
            Precision::Bits(80),
            Precision::Bits(80),
            Precision::Machine,
            Precision::Bits(80),
            Precision::Bits(80),
        ],
        [
            Precision::Bits(120),
            Precision::Bits(120),
            Precision::Machine,
            Precision::Bits(80),
            Precision::Bits(120),
        ],
    ];
    for (i, a) in values.iter().enumerate() {
        for (j, b) in values.iter().enumerate() {
            assert_eq!(a.add(b).precision(), expected[i][j]);
            assert_eq!(a.mul(b).precision(), expected[i][j]);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x4f4d),
        ..ProptestConfig::default()
    })]

    #[test]
    fn exact_rational_ring_laws(a in -1000_i64..1000, b in -1000_i64..1000, c in -1000_i64..1000,
                               da in 1_u64..1000, db in 1_u64..1000, dc in 1_u64..1000) {
        let (a, b, c) = (rat(a, da).normalize(), rat(b, db).normalize(), rat(c, dc).normalize());
        prop_assert_eq!(a.add(&b), b.add(&a));
        prop_assert_eq!(a.mul(&b), b.mul(&a));
        prop_assert_eq!(a.mul(&b.add(&c)), a.mul(&b).add(&a.mul(&c)));
        prop_assert!(a.add(&a.neg()).is_zero());
        if !a.is_zero() { prop_assert!(a.mul(&a.recip().unwrap()).is_one()); }
    }

    #[test]
    fn exact_complex_inverse_and_normalization(a in -100_i64..100, b in -100_i64..100,
                                              da in 1_u64..100, db in 1_u64..100) {
        let n = complex(rat(a, da), rat(b, db)).normalize();
        prop_assert_eq!(n.clone().normalize(), n.clone());
        prop_assert!(n.add(&n.neg()).is_zero());
        if !n.is_zero() { prop_assert_eq!(n.mul(&n.recip().unwrap()), int(1)); }
    }
}
