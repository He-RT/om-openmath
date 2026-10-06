//! Exact linear and cubic data definitions validate portable interpolation independently of ODE.
use om_analysis::interpolation::{Interpolation, Method};
use om_num::ctx::Interrupt;
#[test]
fn linear_hermite_and_reversed_nodes_match_known_polynomials() {
    let ctx = Interrupt::default();
    let linear = Interpolation::from_points(
        vec![0., 1., 2.],
        vec![vec![0.], vec![2.], vec![4.]],
        None,
        Method::Linear,
        &ctx,
    )
    .unwrap();
    assert_eq!(linear.evaluate(0.25, &ctx).unwrap(), vec![0.5]);
    let cubic = Interpolation::from_points(
        vec![0., 1.],
        vec![vec![0.], vec![1.]],
        Some(vec![vec![0.], vec![3.]]),
        Method::Hermite,
        &ctx,
    )
    .unwrap();
    assert!((cubic.evaluate(0.37, &ctx).unwrap()[0] - 0.37f64.powi(3)).abs() < 1e-15);
    let reversed = Interpolation::from_points(
        vec![2., 1., 0.],
        vec![vec![4.], vec![2.], vec![0.]],
        None,
        Method::Linear,
        &ctx,
    )
    .unwrap();
    assert_eq!(reversed.evaluate(0.25, &ctx).unwrap(), vec![0.5]);
    assert!(linear.evaluate(2.1, &ctx).is_err());
    assert!(linear.evaluate(-0.1, &ctx).is_err());
}
#[test]
fn duplicate_nonmonotone_malformed_and_cancelled_data_are_rejected() {
    let ctx = Interrupt::default();
    for times in [vec![0., 0.], vec![0., 2., 1.]] {
        let values = times.iter().map(|_| vec![1.]).collect();
        assert!(Interpolation::from_points(times, values, None, Method::Linear, &ctx).is_err());
    }
    assert!(
        Interpolation::from_points(
            vec![0., 1.],
            vec![vec![1.], vec![1., 2.]],
            None,
            Method::Linear,
            &ctx
        )
        .is_err()
    );
    assert!(
        Interpolation::from_points(
            vec![0., 1.],
            vec![vec![1.], vec![2.]],
            Some(vec![vec![1.]]),
            Method::Hermite,
            &ctx
        )
        .is_err()
    );
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(
        Interpolation::from_points(
            vec![0., 1.],
            vec![vec![1.], vec![2.]],
            None,
            Method::Linear,
            &ctx
        )
        .is_err()
    );
}
#[test]
fn tiny_linear_intervals_do_not_require_overflowing_slopes_and_forged_degree_fails() {
    let ctx = Interrupt::default();
    let value = Interpolation::from_points(
        vec![0., 1e-310, 2e-310],
        vec![vec![1.], vec![2.], vec![3.]],
        None,
        Method::Linear,
        &ctx,
    )
    .unwrap();
    assert!((value.evaluate(5e-311, &ctx).unwrap()[0] - 1.5).abs() < 1e-12);
    assert!(
        Interpolation::new(
            vec![0., 1.],
            vec![vec![0.], vec![1.]],
            vec![vec![[0., 1., 0., 0.]]],
            Method::Linear,
            &ctx
        )
        .is_err()
    );
    assert!(
        Interpolation::from_points(
            vec![0., 1.],
            vec![vec![0.], vec![1.]],
            Some(vec![vec![0.], vec![1.]]),
            Method::Linear,
            &ctx
        )
        .is_err()
    );
}
