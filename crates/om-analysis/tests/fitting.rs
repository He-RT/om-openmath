//! Independent analytic parameters and normal-equation residuals verify QR/LM and honest failures.
use om_analysis::{
    Error,
    fitting::{Options, Termination, least_squares, nonlinear, rank},
};
use om_num::ctx::Interrupt;
#[test]
fn tall_qr_solves_more_than_sixty_four_rows_and_column_pivots_preserve_order() {
    let ctx = Interrupt::default();
    let (m, n) = (1001, 2);
    let mut a = vec![];
    let mut b = vec![];
    for i in 0..m {
        let x = i as f64 / 100.;
        a.extend([x, 1.]);
        b.push(2. * x + 3.);
    }
    let parameters = least_squares(m, n, &a, &b, &ctx).unwrap();
    assert!((parameters[0] - 2.).abs() < 1e-12 && (parameters[1] - 3.).abs() < 1e-12);
    assert_eq!(rank(m, n, &a, &ctx).unwrap(), 2);
    // Dataset [[0,1],[1,2],[2,2]] has independent least-squares line slope1/2, intercept7/6.
    let p = least_squares(3, 2, &[0., 1., 1., 1., 2., 1.], &[1., 2., 2.], &ctx).unwrap();
    assert!((p[0] - 0.5).abs() < 1e-14 && (p[1] - 7. / 6.).abs() < 1e-14);
    let residuals = [p[1] - 1., p[0] + p[1] - 2., 2. * p[0] + p[1] - 2.];
    assert!(residuals.iter().sum::<f64>().abs() < 1e-14);
    assert!((residuals[1] + 2. * residuals[2]).abs() < 1e-14);
}
#[test]
fn qr_equilibration_recovers_independent_small_columns_and_rejects_lost_data() {
    let ctx = Interrupt::default();
    let p = least_squares(
        4,
        2,
        &[1e-150, 0., 0., 1e150, -1e-150, 0., 0., -1e150],
        &[2e-150, 3e150, -2e-150, -3e150],
        &ctx,
    )
    .unwrap();
    assert!((p[0] - 2.).abs() < 1e-13 && (p[1] - 3.).abs() < 1e-13);
    assert!(least_squares(2, 2, &[1e-300, 0., 0., 1e300], &[1e-300, 1e300], &ctx).is_err());
}
#[test]
fn true_lm_fits_exponential_amplitude_and_rate_with_analytic_jacobian() {
    let ctx = Interrupt::default();
    let samples: Vec<_> = (0..21).map(|i| i as f64 / 10.).collect();
    let answer = nonlinear(
        |p, r, j, ctx| {
            for (i, x) in samples.iter().enumerate() {
                ctx.tick()?;
                let e = (p[1] * x).exp();
                r[i] = p[0] * e - 2. * (0.3 * x).exp();
                j[2 * i] = e;
                j[2 * i + 1] = p[0] * x * e;
            }
            Ok(())
        },
        vec![1., 0.],
        samples.len(),
        &Options::default(),
        &ctx,
    )
    .unwrap();
    assert!(
        (answer.parameters[0] - 2.).abs() < 1e-7 && (answer.parameters[1] - 0.3).abs() < 1e-7,
        "{answer:?}"
    );
    assert!(answer.residual_norm < 1e-7);
    assert!(answer.accepted_steps > 0);
    assert!(answer.evaluations > 1);
}
#[test]
fn noisy_lm_uses_actual_gradient_stopping_without_claiming_zero_residual() {
    let ctx = Interrupt::default();
    let a = nonlinear(
        |p, r, j, _| {
            for (i, (x, y)) in [(0., 1.), (1., 2.), (2., 2.)].into_iter().enumerate() {
                r[i] = p[0] * x + p[1] - y;
                j[2 * i] = x;
                j[2 * i + 1] = 1.;
            }
            Ok(())
        },
        vec![0., 0.],
        3,
        &Options::default(),
        &ctx,
    )
    .unwrap();
    assert_eq!(a.termination, Termination::GradientTolerance);
    assert!((a.parameters[0] - 0.5).abs() < 1e-7 && (a.parameters[1] - 7. / 6.).abs() < 1e-7);
    assert!((a.residual_norm - (1f64 / 6.).sqrt()).abs() < 1e-12);
}
#[test]
fn rank_dimensions_nonfinite_exhaustion_and_callback_cancellation_are_not_success() {
    let ctx = Interrupt::default();
    assert_eq!(rank(3, 2, &[1., 2., 2., 4., 3., 6.], &ctx).unwrap(), 1);
    assert!(least_squares(3, 2, &[1., 2., 2., 4., 3., 6.], &[1., 2., 3.], &ctx).is_err());
    assert!(least_squares(1, 2, &[1., 2.], &[1.], &ctx).is_err());
    assert!(least_squares(2, 1, &[1., f64::NAN], &[1., 2.], &ctx).is_err());
    assert!(nonlinear(|_, _, _, _| Ok(()), vec![1.], 1, &Options::default(), &ctx).is_err());
    assert!(
        nonlinear(
            |_, _, _, _| panic!("invalid shape must not allocate or call"),
            vec![1., 2.],
            usize::MAX,
            &Options::default(),
            &ctx
        )
        .is_err()
    );
    let options = Options {
        max_iterations: 1,
        abs_tol: 1e-14,
        rel_tol: 0.,
        ..Options::default()
    };
    let failure = nonlinear(
        |p, r, j, _| {
            r[0] = p[0] * p[0] - 4.;
            j[0] = 2. * p[0];
            Ok(())
        },
        vec![0.1],
        1,
        &options,
        &ctx,
    )
    .unwrap_err();
    assert!(matches!(failure.reason, Error::NoConvergence));
    assert!(failure.partial.is_some());
    let cancelled = Interrupt::default();
    let failure = nonlinear(
        |p, r, j, ctx| {
            r[0] = p[0] - 2.;
            j[0] = 1.;
            ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        },
        vec![0.],
        1,
        &Options::default(),
        &cancelled,
    )
    .unwrap_err();
    assert!(matches!(failure.reason, Error::Abort(_)));
}
