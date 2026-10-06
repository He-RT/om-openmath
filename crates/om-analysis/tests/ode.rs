//! Independent analytic solutions verify adaptive DP5(4), dense output, reversal, events and failures.
use om_analysis::ode::{Options, Termination, solve};
use om_num::ctx::Interrupt;
#[test]
fn exponential_and_harmonic_solutions_are_correct_between_adaptive_nodes() {
    let ctx = Interrupt::default();
    let options = Options::default();
    let answer = solve(
        |_, y, out, _| {
            out[0] = y[0];
            Ok(())
        },
        None,
        0.,
        1.,
        vec![1.],
        &options,
        &ctx,
    )
    .unwrap();
    for t in [0.13, 0.37, 0.91, 1.] {
        let value = answer.solution.evaluate(t, &ctx).unwrap();
        assert!((value[0] - t.exp()).abs() < 5e-8);
    }
    assert!(matches!(answer.termination, Termination::End));
    assert!(answer.accepted_steps > 0);
    assert!(answer.evaluations > 0);
    let answer = solve(
        |_, y, out, _| {
            out[0] = y[1];
            out[1] = -y[0];
            Ok(())
        },
        None,
        0.,
        6.,
        vec![1., 0.],
        &options,
        &ctx,
    )
    .unwrap();
    for t in [0.13, 1.37, 3.91, 6.] {
        let value = answer.solution.evaluate(t, &ctx).unwrap();
        assert!((value[0] - t.cos()).abs() < 2e-7);
        assert!((value[1] + t.sin()).abs() < 2e-7);
    }
}
#[test]
fn backward_time_and_simple_event_use_actual_dense_interpolation() {
    let ctx = Interrupt::default();
    let answer = solve(
        |_, y, out, _| {
            out[0] = y[0];
            Ok(())
        },
        None,
        1.,
        0.,
        vec![std::f64::consts::E],
        &Options::default(),
        &ctx,
    )
    .unwrap();
    assert!((answer.solution.evaluate(0.37, &ctx).unwrap()[0] - 0.37f64.exp()).abs() < 5e-8);
    let mut event = |_: f64, y: &[f64], _: &Interrupt| Ok(y[0] - 2.);
    let answer = solve(
        |_, y, out, _| {
            out[0] = y[0];
            Ok(())
        },
        Some(&mut event),
        0.,
        2.,
        vec![1.],
        &Options::default(),
        &ctx,
    )
    .unwrap();
    assert!(matches!(answer.termination, Termination::Event));
    let t = *answer.solution.times().last().unwrap();
    assert!((t - 2f64.ln()).abs() < 1e-7);
    assert!((answer.solution.evaluate(t, &ctx).unwrap()[0] - 2.).abs() < 1e-9);
    assert!(answer.solution.evaluate(t + 0.1, &ctx).is_err());
}
#[test]
fn invalid_dimensions_nonfinite_rhs_limits_and_cancel_never_return_success() {
    let ctx = Interrupt::default();
    assert!(
        solve(
            |_, _, _, _| Ok(()),
            None,
            0.,
            1.,
            vec![],
            &Options::default(),
            &ctx
        )
        .is_err()
    );
    assert!(
        solve(
            |_, _, out, _| {
                out[0] = f64::NAN;
                Ok(())
            },
            None,
            0.,
            1.,
            vec![1.],
            &Options::default(),
            &ctx
        )
        .is_err()
    );
    let options = Options {
        max_steps: 1,
        ..Options::default()
    };
    let error = solve(
        |_, y, out, _| {
            out[0] = y[0];
            Ok(())
        },
        None,
        0.,
        10.,
        vec![1.],
        &options,
        &ctx,
    )
    .unwrap_err();
    assert!(error.partial.is_some());
    let ctx = Interrupt::default();
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(
        solve(
            |_, _, out, _| {
                out[0] = 0.;
                Ok(())
            },
            None,
            0.,
            1.,
            vec![1.],
            &Options::default(),
            &ctx
        )
        .is_err()
    );
}
#[test]
fn halving_fixed_steps_exhibits_fifth_order_endpoint_convergence() {
    let ctx = Interrupt::default();
    let mut errors = vec![];
    for step in [0.25, 0.125, 0.0625] {
        let options = Options {
            initial_step: Some(step),
            max_step: step,
            abs_tol: 1.,
            rel_tol: 0.,
            ..Options::default()
        };
        let result = solve(
            |_, y, out, _| {
                out[0] = y[0];
                Ok(())
            },
            None,
            0.,
            1.,
            vec![1.],
            &options,
            &ctx,
        )
        .unwrap();
        assert_eq!(result.rejected_steps, 0);
        errors.push((result.solution.evaluate(1., &ctx).unwrap()[0] - std::f64::consts::E).abs());
    }
    for e in errors.windows(2) {
        assert!(e[0] / e[1] > 20. && e[0] / e[1] < 50., "{errors:?}");
    }
}
#[test]
fn cancelling_inside_callback_preserves_only_completed_nodes() {
    let ctx = Interrupt::default();
    let mut calls = 0;
    let failure = solve(
        |_, y, out, ctx| {
            calls += 1;
            out[0] = y[0];
            if calls == 20 {
                ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Ok(())
        },
        None,
        0.,
        10.,
        vec![1.],
        &Options::default(),
        &ctx,
    )
    .unwrap_err();
    assert!(matches!(failure.reason, om_analysis::Error::Abort(_)));
    let partial = failure.partial.unwrap();
    assert!(*partial.solution.times().last().unwrap() < 10.);
    assert!(partial.accepted_steps > 0);
    let clean = Interrupt::default();
    partial.solution.validate(&clean).unwrap();
}
#[test]
fn initial_event_zero_and_zero_span_have_truthful_work_and_domains() {
    let ctx = Interrupt::default();
    let mut event = |_: f64, y: &[f64], _: &Interrupt| Ok(y[0]);
    let answer = solve(
        |_, _, out, _| {
            out[0] = 1.;
            Ok(())
        },
        Some(&mut event),
        2.,
        5.,
        vec![0.],
        &Options::default(),
        &ctx,
    )
    .unwrap();
    assert_eq!(answer.termination, Termination::Event);
    assert_eq!(answer.evaluations, 0);
    assert_eq!(answer.solution.evaluate(2., &ctx).unwrap(), vec![0.]);
    assert!(answer.solution.evaluate(2.01, &ctx).is_err());
    let answer = solve(
        |_, _, _, _| panic!("empty span does not execute RHS"),
        None,
        2.,
        2.,
        vec![3.],
        &Options::default(),
        &ctx,
    )
    .unwrap();
    assert_eq!(answer.evaluations, 0);
    assert_eq!(answer.accepted_steps, 0);
    assert_eq!(answer.termination, Termination::End);
}
