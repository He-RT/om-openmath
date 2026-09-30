//! Aberth centers require rigorous root-disk certificates, not just small residuals.
use om_num::{
    Ball, BigFloat, CBall, Integer, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{RootDisk, UPoly, complex_roots};
use proptest::prelude::*;
fn z(c: &[i64]) -> UPoly<Integer> {
    UPoly::new(c.iter().map(|c| (*c).into()).collect())
}
fn dyadic(x: &BigFloat) -> Rational {
    let rep = x.repr();
    let q = Rational::from(rep.significand().clone());
    if rep.exponent() >= 0 {
        q * Rational::from(Integer::ONE << rep.exponent() as usize)
    } else {
        q / Rational::from(Integer::ONE << rep.exponent().unsigned_abs())
    }
}
type Complex = (Rational, Rational);
fn plus(a: &Complex, b: &Complex) -> Complex {
    (&a.0 + &b.0, &a.1 + &b.1)
}
fn times(a: &Complex, b: &Complex) -> Complex {
    (&a.0 * &b.0 - &a.1 * &b.1, &a.0 * &b.1 + &a.1 * &b.0)
}
fn l1(a: &Complex) -> Rational {
    a.0.clone().max(-&a.0) + a.1.clone().max(-&a.1)
}
fn rouche_one_root(f: &UPoly<Integer>, disk: &RootDisk) {
    // Independently expand f(center+t) over Q[i], then use the linear term
    // as a Rouche witness on the circle; this proves exactly one enclosed root.
    let center = (dyadic(&disk.re), dyadic(&disk.im));
    let zero = (Rational::ZERO, Rational::ZERO);
    let mut t = vec![zero.clone()];
    for c in f.coeffs.iter().rev() {
        let mut next = vec![zero.clone(); t.len() + 1];
        for (k, a) in t.iter().enumerate() {
            next[k] = plus(&next[k], &times(a, &center));
            next[k + 1] = plus(&next[k + 1], a);
        }
        next[0].0 += Rational::from(c.clone());
        t = next;
    }
    let r = dyadic(&disk.radius);
    let derivative_squared = &t[1].0 * &t[1].0 + &t[1].1 * &t[1].1;
    if r == Rational::ZERO {
        assert_eq!(t[0], zero);
        assert!(derivative_squared > Rational::ZERO);
        return;
    }
    let mut tail = l1(&t[0]);
    let mut power = &r * &r;
    for a in t.iter().skip(2) {
        tail += l1(a) * &power;
        power *= &r;
    }
    assert!(derivative_squared * &r * &r > &tail * &tail);
}
fn certify(f: &UPoly<Integer>, disks: &[RootDisk], bits: u32, ctx: &Interrupt) {
    assert_eq!(disks.len(), f.degree().unwrap());
    let epsilon = BigFloat::from_parts(Integer::ONE, -(bits as isize));
    for (i, disk) in disks.iter().enumerate() {
        assert!(
            disk.radius >= BigFloat::ZERO && disk.radius <= epsilon,
            "{disk:?}"
        );
        for other in &disks[..i] {
            assert!(disk.disjoint(other, ctx).unwrap());
        }
        let box_ = disk.to_cball();
        let mut value = CBall::exact(&Rational::ZERO, &Rational::ZERO, box_.re.prec);
        for c in f.coeffs.iter().rev() {
            value = value.mul(&box_).add(&CBall::exact(
                &Rational::from(c.clone()),
                &Rational::ZERO,
                box_.re.prec,
            ));
        }
        assert!(
            value.re.contains_zero() && value.im.contains_zero(),
            "{value:?}"
        );
        rouche_one_root(f, disk);
    }
}
#[test]
fn exact_gaussian_and_real_roots_lie_in_certified_disjoint_disks() {
    let ctx = Interrupt::default();
    for (f, roots) in [
        (z(&[1, 0, 1]), vec![(0, 1), (0, -1)]),
        (z(&[-1, 0, 1]), vec![(-1, 0), (1, 0)]),
        (z(&[5, -4, 1]), vec![(2, 1), (2, -1)]),
        (z(&[0, -1, 0, 1]), vec![(-1, 0), (0, 0), (1, 0)]),
    ] {
        let disks = complex_roots(&f, 100, &ctx).unwrap().unwrap();
        certify(&f, &disks, 100, &ctx);
        for (re, im) in roots {
            assert_eq!(
                disks
                    .iter()
                    .filter(|d| d
                        .contains(&Rational::from(re), &Rational::from(im), &ctx)
                        .unwrap())
                    .count(),
                1
            );
        }
    }
}
#[test]
fn authority_irreducible_root_vectors_match_independent_approximations() {
    let ctx = Interrupt::default();
    for (f, roots) in [
        (
            z(&[-2, 0, 1]),
            vec![(-2_f64.sqrt(), 0.0), (2_f64.sqrt(), 0.0)],
        ),
        (
            z(&[0, -2, 0, 1]),
            vec![(-2_f64.sqrt(), 0.0), (0.0, 0.0), (2_f64.sqrt(), 0.0)],
        ),
        (
            z(&[1, 0, -10, 0, 1]),
            vec![
                (-3.1462643699419726, 0.0),
                (-0.317837245195782, 0.0),
                (0.317837245195782, 0.0),
                (3.1462643699419726, 0.0),
            ],
        ),
        (
            z(&[-2, 0, 0, 1]),
            vec![
                (1.2599210498948732, 0.0),
                (-0.6299605249474366, -1.0911236359717214),
                (-0.6299605249474366, 1.0911236359717214),
            ],
        ),
    ] {
        let disks = complex_roots(&f, 80, &ctx).unwrap().unwrap();
        certify(&f, &disks, 80, &ctx);
        for (re, im) in roots {
            assert_eq!(
                disks
                    .iter()
                    .filter(|d| (d.re.to_f64().value() - re).abs() < 1e-12
                        && (d.im.to_f64().value() - im).abs() < 1e-12)
                    .count(),
                1
            );
        }
    }
}
#[test]
fn precision_doubles_for_near_collisions_and_tiny_imaginary_pairs() {
    let ctx = Interrupt::default();
    let n = Integer::ONE << 60;
    let f = UPoly::new(vec![&n * &n, Integer::from(-2) * &n * &n, &n * &n + 1_u8]);
    let disks = complex_roots(&f, 110, &ctx).unwrap().unwrap();
    certify(&f, &disks, 110, &ctx);
    assert_eq!(disks.len(), 2);
    assert!(disks.iter().any(|d| d.im > BigFloat::ZERO));
    assert!(disks.iter().any(|d| d.im < BigFloat::ZERO));
    let f = UPoly::new(vec![Integer::ONE, Integer::ZERO, &n * &n]);
    let disks = complex_roots(&f, 100, &ctx).unwrap().unwrap();
    certify(&f, &disks, 100, &ctx);
    let root = Rational::ONE / Rational::from(n);
    assert!(
        disks
            .iter()
            .any(|d| d.contains(&Rational::ZERO, &root, &ctx).unwrap())
    );
    assert!(
        disks
            .iter()
            .any(|d| d.contains(&Rational::ZERO, &(-root.clone()), &ctx).unwrap())
    );
}
#[test]
fn zeros_constants_repetitions_and_invalid_precisions_are_checked() {
    let ctx = Interrupt::default();
    assert_eq!(complex_roots(&z(&[]), 80, &ctx).unwrap(), None);
    assert_eq!(complex_roots(&z(&[7]), 80, &ctx).unwrap(), Some(vec![]));
    assert_eq!(complex_roots(&z(&[1, 2, 1]), 80, &ctx).unwrap(), None);
    assert_eq!(complex_roots(&z(&[1, 0, 1]), 0, &ctx).unwrap(), None);
    assert_eq!(complex_roots(&z(&[1, 0, 1]), u32::MAX, &ctx).unwrap(), None);
    let f = z(&[1, 0, 1]);
    assert_eq!(
        complex_roots(&f, 64, &ctx).unwrap(),
        complex_roots(&f.scale(&(-12).into(), &ctx).unwrap(), 64, &ctx).unwrap()
    );
}
#[test]
fn linear_exact_root_disks_support_rational_nonmonic_inputs() {
    let ctx = Interrupt::default();
    let f = z(&[-1, 3]);
    let disks = complex_roots(&f, 200, &ctx).unwrap().unwrap();
    certify(&f, &disks, 200, &ctx);
    assert!(
        disks[0]
            .contains(&(Rational::ONE / Rational::from(3)), &Rational::ZERO, &ctx)
            .unwrap()
    );
}
#[test]
fn external_cancel_and_budget_stop_iterations_and_certification() {
    let ctx = Interrupt::default();
    let f = z(&[1, 0, -10, 0, 1]);
    for budget in [0, 5, 100] {
        ctx.steps_left.set(budget);
        assert_eq!(complex_roots(&f, 100, &ctx), Err(Abort::Budget));
    }
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(complex_roots(&z(&[]), 0, &ctx), Err(Abort::Interrupted));
    // Constructors expose balls without losing their radius or midpoint precision.
    let center = Ball::exact(&Rational::ONE, 100).mid;
    let disk = RootDisk {
        re: center.clone(),
        im: BigFloat::ZERO,
        radius: BigFloat::from_parts(Integer::ONE, -80),
    };
    assert_eq!(disk.to_cball().re.mid, center);
}
proptest! {
    #![proptest_config(ProptestConfig{cases:32,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d362e36),..ProptestConfig::default()})]
    #[test]
    fn planted_gaussian_roots_are_all_enclosed_in_unique_disks(a in 1i64..=5,b in -5i64..=5,r in -5i64..=5) {
        let ctx=Interrupt::default();
        let f=z(&[a*a+b*b,-2*b,1]).mul(&z(&[-r,1]),&ctx).unwrap();
        let disks=complex_roots(&f,80,&ctx).unwrap().unwrap();
        certify(&f,&disks,80,&ctx);
        for (re,im) in [(b,a),(b,-a),(r,0)] {prop_assert_eq!(disks.iter().filter(|d|d.contains(&Rational::from(re),&Rational::from(im),&ctx).unwrap()).count(),1);}
    }
}
