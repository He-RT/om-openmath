use super::*;
use proptest::prelude::*;

#[test]
fn gcd_and_bezout_cover_signs_and_zero() {
    assert_eq!(gcd(&48.into(), &18.into()), 6.into());
    for (a, b) in [
        (48, 18),
        (-48, 18),
        (48, -18),
        (-48, -18),
        (0, 18),
        (18, 0),
        (0, 0),
    ] {
        let (a, b) = (Integer::from(a), Integer::from(b));
        let (g, s, t) = ext_gcd(&a, &b);
        assert_eq!(&s * &a + &t * &b, g);
        assert_eq!(g, gcd(&a, &b));
        assert!(g >= Integer::ZERO);
    }
}

#[test]
fn square_root_is_exact_at_large_boundaries() {
    assert_eq!(
        isqrt(&Integer::from(1_000_000_000_000_000_000_u64)),
        1_000_000_000.into()
    );
    let root = (Integer::ONE << 200) + 123;
    let square = &root * &root;
    assert_eq!(isqrt(&square), root);
    assert_eq!(isqrt(&(&square - 1)), &root - 1);
    assert_eq!(isqrt(&(&square + 1)), root);
    assert_eq!(isqrt(&Integer::ZERO), Integer::ZERO);
}

#[test]
fn roots_handle_sign_degree_and_units() {
    assert_eq!(exact_root(&64.into(), 3), Some(4.into()));
    assert_eq!(exact_root(&(-64).into(), 3), Some((-4).into()));
    assert_eq!(exact_root(&(-64).into(), 2), None);
    assert_eq!(exact_root(&65.into(), 3), None);
    assert_eq!(exact_root(&10.into(), 0), None);
    assert_eq!(exact_root(&(-10).into(), 1), Some((-10).into()));
    assert_eq!(exact_root(&Integer::ZERO, u32::MAX), Some(Integer::ZERO));
    assert_eq!(exact_root(&Integer::ONE, u32::MAX), Some(Integer::ONE));
    assert_eq!(exact_root(&2.into(), u32::MAX), None);
    let root: Integer = (Integer::ONE << 128) + 7;
    assert_eq!(exact_root(&root.pow(7), 7), Some(root));
}

#[test]
fn perfect_powers_select_maximal_exponent() {
    for (n, expected) in [
        (12, None),
        (64, Some((2, 6))),
        (81, Some((3, 4))),
        (-64, Some((-4, 3))),
        (-8, Some((-2, 3))),
        (0, None),
        (1, None),
        (-1, None),
    ] {
        assert_eq!(
            perfect_power(&n.into()),
            expected.map(|(b, k)| (b.into(), k))
        );
    }
    assert_eq!(perfect_power(&(Integer::ONE << 256)), Some((2.into(), 256)));
}

#[test]
fn primes_match_independent_sieve_through_the_first_thousand_primes() {
    let mut composite = vec![false; 8000];
    composite[0] = true;
    composite[1] = true;
    for p in 2..8000 {
        if !composite[p] {
            for n in (p * 2..8000).step_by(p) {
                composite[n] = true;
            }
        }
    }
    assert!(composite.iter().filter(|&&x| !x).count() >= 1000);
    assert!(!is_probable_prime(&(-7).into()));
    for (n, &composite) in composite.iter().enumerate() {
        assert_eq!(is_probable_prime(&n.into()), !composite, "n={n}");
    }
}

#[test]
fn primality_rejects_carmichael_and_strong_pseudoprimes() {
    for n in [
        561_u64,
        1105,
        1729,
        2047,
        1_373_653,
        3_215_031_751,
        341_550_071_728_321,
        3_825_123_056_546_413_051,
        u64::MAX,
    ] {
        assert!(!is_probable_prime(&n.into()), "{n}");
    }
    assert!(is_probable_prime(&18_446_744_073_709_551_557_u64.into()));
}

#[test]
fn bpsw_covers_large_primes_squares_and_miller_rabin_pseudoprimes() {
    for exponent in [89, 107, 127] {
        assert!(is_probable_prime(&((Integer::ONE << exponent) - 1)));
    }
    let prime: Integer = (Integer::ONE << 127) - 1;
    assert!(!is_probable_prime(&prime.pow(2)));
    assert!(!is_probable_prime(&(&prime * 41)));
    let pseudoprime = Integer::from(399_165_290_221_u64) * 798_330_580_441_u64;
    assert!(!is_probable_prime(&pseudoprime));
}

fn product(factors: &[(Integer, u32)]) -> Integer {
    factors
        .iter()
        .fold(Integer::ONE, |n, (p, k)| n * p.pow(*k as usize))
}

#[test]
fn fermat_64_factorization_matches_known_factors() {
    let n = (Integer::ONE << 64) + 1;
    let mut budget = 100_000;
    let factors = factor_integer(&n, &mut budget);
    assert_eq!(
        factors,
        vec![
            (274177.into(), 1),
            (Integer::from(67_280_421_310_721_u64), 1)
        ]
    );
    assert_eq!(product(&factors), n);
    assert!(budget > 0);
}

#[test]
fn partial_factorizations_preserve_unresolved_composites() {
    let n = Integer::from(1_000_003) * 1_000_033;
    let mut budget = 0;
    assert_eq!(factor_integer(&n, &mut budget), vec![(n.clone(), 1)]);
    assert!(!is_probable_prime(&n));
    let n = n * 8;
    let mut budget = 2;
    let factors = factor_integer(&n, &mut budget);
    assert_eq!(budget, 0);
    assert_eq!(product(&factors), n);
}

#[test]
fn factoring_handles_zero_units_signs_and_multiplicities() {
    for (n, expected) in [
        (0, vec![(0, 1)]),
        (1, vec![]),
        (-1, vec![(-1, 1)]),
        (-72, vec![(-1, 1), (2, 3), (3, 2)]),
        (81, vec![(3, 4)]),
    ] {
        assert_eq!(
            factor_integer(&n.into(), &mut 10_000),
            expected
                .into_iter()
                .map(|(p, k)| (p.into(), k))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn pollard_factorizations_are_reproducible() {
    let n = Integer::from(1_000_003) * 1_000_033;
    let (mut a, mut b) = (100_000, 100_000);
    let factors = factor_integer(&n, &mut a);
    assert_eq!(factors, vec![(1_000_003.into(), 1), (1_000_033.into(), 1)]);
    assert_eq!(factors, factor_integer(&n, &mut b));
    assert_eq!(a, b);
}

#[test]
fn radical_factors_cover_rational_constructor_numerators() {
    for (n, k, expected) in [
        (12, 2, (2, 3)),
        (72, 2, (6, 2)),
        (64, 3, (4, 1)),
        (432, 3, (6, 2)),
        (-128, 3, (-4, 2)),
        (0, 2, (0, 1)),
        (1, 2, (1, 1)),
        (-12, 1, (-12, 1)),
    ] {
        assert_eq!(
            extract_root_factor(&n.into(), k),
            (expected.0.into(), expected.1.into())
        );
    }
    let large = Integer::from(1_000_003).pow(3);
    assert_eq!(
        extract_root_factor(&large, 3),
        (1_000_003.into(), Integer::ONE)
    );
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: proptest::test_runner::RngSeed::Fixed(0x4e54), ..ProptestConfig::default() })]
    #[test]
    fn factor_product_is_original_for_any_budget(n in -1_000_000_i64..1_000_000, budget in 0_u64..10_000) {
        let mut remaining=budget;
        let n=Integer::from(n);
        let factors=factor_integer(&n,&mut remaining);
        prop_assert_eq!(product(&factors),n);
        prop_assert!(remaining<=budget);
    }
    #[test]
    fn bezout_identity(a in any::<i64>(), b in any::<i64>()) {
        let (a,b)=(Integer::from(a),Integer::from(b));
        let (g,s,t)=ext_gcd(&a,&b);
        prop_assert_eq!(&s*&a+&t*&b,g.clone());
        prop_assert_eq!(g,gcd(&a,&b));
    }
    #[test]
    fn roots_and_radical_extraction_preserve_integers(n in 0_u64..1_000_000, k in 2_u32..7) {
        let n=Integer::from(n);
        let root=isqrt(&n);
        prop_assert!(&root*&root<=n && (&root+1_u8).pow(2)>n);
        let (a,b)=extract_root_factor(&n,k);
        prop_assert_eq!(a.pow(k as usize)*b,n);
    }
}
