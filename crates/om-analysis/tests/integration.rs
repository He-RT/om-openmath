//! Analytic references and singular failures verify the actual GK15/7 algorithm independently.
use om_analysis::integration::{FailureKind, Options, integrate};
use om_num::ctx::Interrupt;
#[test]
fn finite_reversed_and_improper_integrals_match_closed_references() {
    let ctx = Interrupt::default();
    let opts = Options::default();
    for (a, b, f, want) in [
        (
            0.,
            1.,
            (f64::exp as fn(f64) -> f64),
            std::f64::consts::E - 1.,
        ),
        (-1., 1., (|x: f64| x.powi(8)) as fn(f64) -> f64, 2. / 9.),
        (std::f64::consts::PI, 0., f64::sin as fn(f64) -> f64, -2.),
        (
            0.,
            f64::INFINITY,
            (|x: f64| (-x).exp()) as fn(f64) -> f64,
            1.,
        ),
        (
            f64::NEG_INFINITY,
            f64::INFINITY,
            (|x: f64| 1. / (1. + x * x)) as fn(f64) -> f64,
            std::f64::consts::PI,
        ),
    ] {
        let result = integrate(|x, _| Ok(f(x)), a, b, &opts, &ctx).unwrap();
        assert!((result.value - want).abs() < 1e-8, "{a} {b}: {:?}", result);
        assert!(result.error_estimate >= 0.);
        assert!(result.evaluations > 0);
    }
}
#[test]
fn explicit_breakpoints_cover_interior_and_endpoint_singularities() {
    let ctx = Interrupt::default();
    let opts = Options {
        breakpoints: vec![0.],
        ..Options::default()
    };
    let result = integrate(|x, _| Ok(1. / x.abs().sqrt()), -1., 1., &opts, &ctx).unwrap();
    assert!((result.value - 4.).abs() < 1e-7);
    let result = integrate(|x, _| Ok(1. / x.sqrt()), 0., 1., &Options::default(), &ctx).unwrap();
    assert!((result.value - 2.).abs() < 1e-7);
}
#[test]
fn poles_divergence_and_exhaustion_are_not_successful_approximations() {
    let ctx = Interrupt::default();
    let err = integrate(|x, _| Ok(1. / x), -1., 1., &Options::default(), &ctx).unwrap_err();
    assert!(matches!(err.kind, FailureKind::NonFiniteSample));
    let opts = Options {
        max_intervals: 32,
        ..Options::default()
    };
    assert!(integrate(|x, _| Ok(1. / x), 1., f64::INFINITY, &opts, &ctx).is_err());
    assert!(
        integrate(
            |x, _| Ok(1. / x),
            f64::NEG_INFINITY,
            f64::INFINITY,
            &opts,
            &ctx
        )
        .is_err()
    );
    let opts = Options {
        max_intervals: 1,
        ..Options::default()
    };
    let err = integrate(|x, _| Ok(x.sqrt()), 0., 1., &opts, &ctx).unwrap_err();
    assert!(matches!(err.kind, FailureKind::IntervalLimit));
    assert!(err.partial.is_some());
}
#[test]
fn scale_roundoff_invalid_options_and_actual_cancellation_are_distinct() {
    let ctx = Interrupt::default();
    let r = integrate(|_, _| Ok(1e-308), -1e308, 1e308, &Options::default(), &ctx).unwrap();
    assert!((r.value - 2.).abs() < 1e-13);
    assert!(
        integrate(
            |_, _| Ok(1.),
            0.,
            1.,
            &Options {
                abs_tol: -1.,
                ..Options::default()
            },
            &ctx
        )
        .is_err()
    );
    assert!(
        integrate(
            |_, _| Ok(1.),
            0.,
            1.,
            &Options {
                breakpoints: vec![2.],
                ..Options::default()
            },
            &ctx
        )
        .is_err()
    );
    let cancelled = Interrupt::default();
    cancelled
        .flag
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        integrate(|_, _| Ok(1.), 0., 1., &Options::default(), &cancelled)
            .unwrap_err()
            .kind,
        FailureKind::Abort(_)
    ));
}
