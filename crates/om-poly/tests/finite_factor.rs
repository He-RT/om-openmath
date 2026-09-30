//! Distinct/equal-degree finite-field factorization with reconstruction and irreducibility.
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
    rng::SplitMix64,
};
use om_poly::{FpElem, Ring, UPoly};
fn f(c: &[i64], p: u64) -> UPoly<FpElem> {
    UPoly::new(
        c.iter()
            .map(|c| FpElem::new((*c as i128).rem_euclid(p as i128) as u64, p).unwrap())
            .collect(),
    )
}
fn product(factors: &[UPoly<FpElem>], ctx: &Interrupt) -> UPoly<FpElem> {
    factors
        .iter()
        .fold(UPoly::one(), |a, b| a.mul(b, ctx).unwrap())
}
fn irreducible_by_trial_divisors(f: &UPoly<FpElem>, p: u64, ctx: &Interrupt) -> bool {
    let degree = f.degree().unwrap();
    for d in 1..=degree / 2 {
        for mut value in 0..p.pow(d as u32) {
            let mut c = vec![];
            for _ in 0..d {
                c.push(FpElem::new(value % p, p).unwrap());
                value /= p;
            }
            c.push(FpElem::new(1, p).unwrap());
            if f.divrem(&UPoly::new(c), ctx).unwrap().unwrap().1.is_zero() {
                return false;
            }
        }
    }
    true
}
#[test]
fn authority_ddf_and_edf_prime_field_vectors() {
    let ctx = Interrupt::default();
    let polynomial = f(&[-1, 0, 0, 0, 1], 5);
    assert_eq!(
        polynomial.ddf(&ctx).unwrap(),
        Some(vec![(polynomial.clone(), 1)])
    );
    let factors = polynomial.edf(1, 0x4d362e31, &ctx).unwrap().unwrap();
    assert_eq!(
        factors,
        vec![f(&[1, 1], 5), f(&[2, 1], 5), f(&[3, 1], 5), f(&[4, 1], 5)]
    );
    let polynomial = f(&[1, 0, 0, 0, 1], 3);
    assert_eq!(
        polynomial.ddf(&ctx).unwrap(),
        Some(vec![(polynomial.clone(), 2)])
    );
    assert_eq!(
        polynomial.edf(2, 0x4d362e31, &ctx).unwrap(),
        Some(vec![f(&[2, 1, 1], 3), f(&[2, 2, 1], 3)])
    );
}
#[test]
fn ddf_separates_linear_quadratic_and_cubic_factors() {
    let ctx = Interrupt::default();
    let a = f(&[1, 1], 3);
    let b = f(&[1, 0, 1], 3);
    let c = f(&[1, 2, 0, 1], 3); // no roots in F3, hence irreducible cubic
    assert!(irreducible_by_trial_divisors(&c, 3, &ctx));
    let polynomial = product(&[a.clone(), b.clone(), c.clone()], &ctx)
        .scale(&FpElem::new(2, 3).unwrap(), &ctx)
        .unwrap();
    assert_eq!(
        polynomial.ddf(&ctx).unwrap(),
        Some(vec![(a.clone(), 1), (b.clone(), 2), (c.clone(), 3)])
    );
    for (group, d) in polynomial.ddf(&ctx).unwrap().unwrap() {
        let factors = group.edf(d, 8, &ctx).unwrap().unwrap();
        assert_eq!(factors.len(), 1);
        assert_eq!(product(&factors, &ctx), group);
    }
}
#[test]
fn finite_factorization_rejects_non_squarefree_and_invalid_equal_degrees() {
    let ctx = Interrupt::default();
    for polynomial in [
        f(&[], 3),
        f(&[1, 2, 1], 3),
        f(&[1, 0, 0, 1], 3),
        f(&[1, 1], 2),
        UPoly::new(vec![FpElem::one(), FpElem::one()]),
    ] {
        assert_eq!(polynomial.ddf(&ctx).unwrap(), None);
        assert_eq!(polynomial.edf(1, 0, &ctx).unwrap(), None);
    }
    assert_eq!(f(&[2], 3).ddf(&ctx).unwrap(), Some(vec![]));
    assert_eq!(f(&[2], 3).edf(1, 0, &ctx).unwrap(), Some(vec![]));
    assert_eq!(f(&[1, 1], 3).edf(0, 0, &ctx).unwrap(), None);
    let mixed = product(&[f(&[1, 1], 3), f(&[1, 0, 1], 3)], &ctx);
    assert_eq!(mixed.edf(1, 0, &ctx).unwrap(), None);
    assert_eq!(mixed.edf(3, 0, &ctx).unwrap(), None);
    assert_eq!(f(&[1, 0, 0, 0, 1], 3).edf(1, 0, &ctx).unwrap(), None);
}
#[test]
fn modular_power_large_exponents_and_constant_moduli() {
    let ctx = Interrupt::default();
    let x = f(&[0, 1], 5);
    let modulus = f(&[1, 0, 1], 5);
    assert_eq!(
        x.pow_mod(&Integer::from(4), &modulus, &ctx).unwrap(),
        Some(f(&[1], 5))
    );
    assert_eq!(
        x.pow_mod(&(Integer::ONE << 130), &modulus, &ctx).unwrap(),
        Some(f(&[1], 5))
    );
    assert_eq!(
        x.pow_mod(&Integer::ZERO, &modulus, &ctx).unwrap(),
        Some(UPoly::one())
    );
    assert_eq!(
        x.pow_mod(&Integer::ZERO, &f(&[2], 5), &ctx).unwrap(),
        Some(UPoly::zero())
    );
    assert_eq!(x.pow_mod(&Integer::from(-1), &modulus, &ctx).unwrap(), None);
    assert_eq!(
        x.pow_mod(&Integer::ONE, &UPoly::zero(), &ctx).unwrap(),
        None
    );
}
#[test]
fn edf_handles_cubic_groups_large_characteristics_and_repeated_seeds() {
    let ctx = Interrupt::default();
    let p = 3;
    let a = f(&[1, 2, 0, 1], p);
    let b = f(&[2, 2, 0, 1], p);
    assert!(irreducible_by_trial_divisors(&b, p, &ctx));
    let polynomial = product(&[a, b], &ctx);
    let factors = polynomial.edf(3, 12345, &ctx).unwrap().unwrap();
    assert_eq!(product(&factors, &ctx), polynomial);
    assert_eq!(polynomial.edf(3, 12345, &ctx).unwrap().unwrap(), factors);
    let p = 18446744073709551557;
    let a = f(&[1, 1], p);
    let b = f(&[2, 1], p);
    let polynomial = product(&[a.clone(), b.clone()], &ctx);
    assert_eq!(polynomial.edf(1, 5, &ctx).unwrap(), Some(vec![a, b]));
}
#[test]
fn fixed_seed_ddf_edf_factor_products_and_independent_irreducibility() {
    let ctx = Interrupt::default();
    let mut rng = SplitMix64::new(0x46414354);
    for p in [3, 5, 7] {
        for _ in 0..128 {
            let mut coefficients = (0..8)
                .map(|_| FpElem::new(rng.next_range(0, p), p).unwrap())
                .collect::<Vec<_>>();
            coefficients.push(FpElem::new(1, p).unwrap());
            let polynomial = UPoly::new(coefficients);
            // DDF/EDF consume square-free parts; multiplicity belongs to M5.4.
            for (polynomial, _) in polynomial.square_free(&ctx).unwrap().unwrap().factors {
                let mut all = vec![];
                for (group, d) in polynomial.ddf(&ctx).unwrap().unwrap() {
                    let seed = rng.next_u64();
                    let factors = group.edf(d, seed, &ctx).unwrap().unwrap();
                    assert_eq!(product(&factors, &ctx), group);
                    assert!(factors.iter().all(|factor| factor.degree() == Some(d)));
                    for factor in &factors {
                        assert!(irreducible_by_trial_divisors(factor, p, &ctx));
                    }
                    assert_eq!(group.edf(d, seed, &ctx).unwrap().unwrap(), factors);
                    all.extend(factors);
                }
                assert_eq!(product(&all, &ctx), polynomial);
            }
        }
    }
}
#[test]
fn finite_factorization_and_power_charge_steps_and_propagate_cancellation() {
    let ctx = Interrupt::default();
    for steps in [0, 5, 30] {
        let polynomial = f(&[1, 0, 0, 0, 1], 3);
        ctx.steps_left.set(steps);
        assert_eq!(polynomial.ddf(&ctx), Err(Abort::Budget));
        ctx.steps_left.set(steps);
        assert_eq!(polynomial.edf(2, 8, &ctx), Err(Abort::Budget));
    }
    ctx.steps_left.set(20);
    assert_eq!(
        f(&[0, 1], 5).pow_mod(&(Integer::ONE << 100), &f(&[1, 0, 1], 5), &ctx),
        Err(Abort::Budget)
    );
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(f(&[2], 3).ddf(&ctx), Err(Abort::Interrupted));
}
