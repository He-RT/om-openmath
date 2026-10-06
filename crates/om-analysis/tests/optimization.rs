//! Analytic minimizers and independent residuals exercise real searches and failure states.
use om_analysis::{
    Error,
    optimization::{Method, Options, bounded, minimize},
};
use om_num::ctx::Interrupt;
#[test]
fn brent_checks_actual_interior_endpoint_values_and_reduces_bracket() {
    let ctx = Interrupt::default();
    let options = Options::default();
    let mut calls = 0;
    let answer = bounded(
        |x, _| {
            calls += 1;
            Ok((x - 2.).powi(2) + 3.)
        },
        -1.,
        5.,
        &options,
        &ctx,
    )
    .unwrap();
    assert_eq!(answer.method, Method::Brent);
    assert_eq!(answer.evaluations, calls);
    assert!((answer.point[0] - 2.).abs() < 1e-7);
    assert!((answer.value - 3.).abs() < 1e-13);
    assert!(answer.bracket_width.unwrap() < 1e-6);
    let answer = bounded(|x, _| Ok((x - 4.).powi(2)), -2., 1., &options, &ctx).unwrap();
    assert_eq!(answer.point, vec![1.]);
    assert_eq!(answer.value, 9.);
    let answer = bounded(|x, _| Ok((x + 0.25).powi(4)), -1., 1., &options, &ctx).unwrap();
    assert!((answer.point[0] + 0.25).abs() < 1e-7);
}
#[test]
fn bfgs_matches_quadratic_and_rosenbrock_stationary_solutions() {
    let ctx = Interrupt::default();
    let options = Options::default();
    let mut count = 0;
    let answer = minimize(
        |x, g, _| {
            count += 1;
            g[0] = 2. * (x[0] - 2.);
            g[1] = 8. * (x[1] + 3.);
            Ok((x[0] - 2.).powi(2) + 4. * (x[1] + 3.).powi(2))
        },
        vec![0., 0.],
        None,
        &options,
        &ctx,
    )
    .unwrap();
    assert_eq!(answer.method, Method::Bfgs);
    assert_eq!(answer.evaluations, count);
    assert!((answer.point[0] - 2.).abs() < 1e-7 && (answer.point[1] + 3.).abs() < 1e-7);
    assert!(answer.value < 1e-13);
    assert!(answer.gradient_norm.unwrap() <= options.gradient_tol);
    let answer = minimize(
        |x, g, _| {
            let a = x[1] - x[0] * x[0];
            let b = 1. - x[0];
            g[0] = -400. * x[0] * a - 2. * b;
            g[1] = 200. * a;
            Ok(100. * a * a + b * b)
        },
        vec![-1.2, 1.],
        None,
        &options,
        &ctx,
    )
    .unwrap();
    assert!(
        (answer.point[0] - 1.).abs() < 1e-7 && (answer.point[1] - 1.).abs() < 1e-7,
        "{answer:?}"
    );
    assert!(answer.value < 1e-14);
}
#[test]
fn box_projection_uses_correct_active_gradient_and_fixed_dimensions() {
    let ctx = Interrupt::default();
    let options = Options::default();
    let answer = minimize(
        |x, g, _| {
            g[0] = 2. * (x[0] - 2.);
            g[1] = 2. * (x[1] + 3.);
            Ok((x[0] - 2.).powi(2) + (x[1] + 3.).powi(2))
        },
        vec![0., 0.],
        Some(&[(-1., 1.), (-2., 2.)]),
        &options,
        &ctx,
    )
    .unwrap();
    assert_eq!(answer.point, vec![1., -2.]);
    assert_eq!(answer.value, 2.);
    assert_eq!(answer.gradient_norm, Some(0.));
    let answer = minimize(
        |x, g, _| {
            g[0] = 2. * x[0];
            g[1] = 2. * (x[1] - 4.);
            Ok(x[0] * x[0] + (x[1] - 4.).powi(2))
        },
        vec![2., 0.],
        Some(&[(2., 2.), (-10., 10.)]),
        &options,
        &ctx,
    )
    .unwrap();
    assert_eq!(answer.point[0], 2.);
    assert!((answer.point[1] - 4.).abs() < 1e-8);
    assert!((answer.value - 4.).abs() < 1e-12);
}
#[test]
fn dimensional_nonfinite_limits_and_cancellation_never_claim_success() {
    let ctx = Interrupt::default();
    let options = Options::default();
    assert!(bounded(|_, _| Ok(1.), 1., 1., &options, &ctx).is_err());
    assert!(
        bounded(|_, _| Ok(f64::NAN), 0., 1., &options, &ctx)
            .unwrap_err()
            .partial
            .is_none()
    );
    assert!(minimize(|_, _, _| Ok(1.), vec![], None, &options, &ctx).is_err());
    assert!(minimize(|_, _, _| Ok(1.), vec![0.], None, &options, &ctx).is_err());
    assert!(
        minimize(
            |_, g, _| {
                g[0] = 0.;
                Ok(1.)
            },
            vec![2.],
            Some(&[(0., 1.)]),
            &options,
            &ctx
        )
        .is_err()
    );
    let tiny = Options {
        max_iterations: 1,
        ..Options::default()
    };
    let failure = bounded(|x, _| Ok((x - 0.2).powi(2)), 0., 1., &tiny, &ctx).unwrap_err();
    assert!(matches!(failure.reason, Error::NoConvergence));
    assert!(failure.partial.unwrap().evaluations >= 4);
    let failure = minimize(
        |x, g, _| {
            g[0] = -1.;
            Ok(-x[0])
        },
        vec![0.],
        None,
        &tiny,
        &ctx,
    )
    .unwrap_err();
    assert!(matches!(failure.reason, Error::NoConvergence));
    assert_eq!(failure.partial.unwrap().point, vec![1.]);
    let cancelled = Interrupt::default();
    let mut calls = 0;
    let error = minimize(
        |x, g, ctx| {
            calls += 1;
            g[0] = 2. * (x[0] - 3.);
            if calls == 2 {
                ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Ok((x[0] - 3.).powi(2))
        },
        vec![0.],
        None,
        &options,
        &cancelled,
    )
    .unwrap_err();
    assert!(matches!(error.reason, Error::Abort(_)));
    assert!(error.partial.is_some());
}
#[test]
fn a_large_objective_offset_does_not_make_gradient_convergence_vacuous() {
    let ctx = Interrupt::default();
    let answer = minimize(
        |x, g, _| {
            g[0] = 2. * (x[0] - 2.);
            Ok(1e20 + (x[0] - 2.).powi(2))
        },
        vec![0.],
        None,
        &Options::default(),
        &ctx,
    )
    .unwrap();
    assert!((answer.point[0] - 2.).abs() < 1e-9);
    assert!(answer.gradient_norm.unwrap() < 1e-8);
}
#[test]
fn deterministic_coupled_positive_quadratics_have_independent_known_minimizers() {
    let ctx = Interrupt::default();
    for case in 0..24 {
        let n = case % 8 + 1;
        let target: Vec<_> = (0..n)
            .map(|i| ((i + case) % 7) as f64 * 0.1 - 0.3)
            .collect();
        let v: Vec<_> = (0..n).map(|i| ((i * 3 + case) % 5) as f64 - 2.).collect();
        let bounds = vec![(-1., 1.); n];
        let answer = minimize(
            |x, g, _| {
                let d: Vec<_> = x.iter().zip(&target).map(|(x, t)| x - t).collect();
                let product: f64 = d.iter().zip(&v).map(|(d, v)| d * v).sum();
                for i in 0..n {
                    g[i] = d[i] + v[i] * product;
                }
                Ok(0.5 * (d.iter().map(|d| d * d).sum::<f64>() + product * product))
            },
            vec![0.8; n],
            Some(&bounds),
            &Options::default(),
            &ctx,
        )
        .unwrap();
        for (x, t) in answer.point.iter().zip(&target) {
            assert!((x - t).abs() < 1e-7, "case={case}: {answer:?}");
        }
        assert!(answer.value < 1e-13);
    }
}
