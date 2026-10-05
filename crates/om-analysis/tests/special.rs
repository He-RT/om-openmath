//! Independent reference values and identities for the machine special-function path.
use om_analysis::special::{beta, erf, erfc, gamma, log_gamma, normal_quantile};
use om_num::ctx::Interrupt;

fn near(value: f64, expected: f64, tolerance: f64) {
    assert!(
        (value - expected).abs() <= tolerance * expected.abs().max(1e-300),
        "{value} != {expected}"
    );
}
#[test]
fn independent_gamma_beta_and_error_function_references() {
    let ctx = Interrupt::default();
    for (x, g) in [
        (0.5, 1.772453850905516),
        (0.1, 9.513507698668732),
        (1.5, 0.886226925452758),
        (8.25, 8376.512350919926),
    ] {
        near(gamma(x, &ctx).unwrap(), g, 8e-14);
    }
    near(log_gamma(1000., &ctx).unwrap(), 5905.220423209181, 3e-15);
    near(beta(0.5, 0.5, &ctx).unwrap(), std::f64::consts::PI, 8e-14);
    near(beta(2., 3., &ctx).unwrap(), 1. / 12., 8e-14);
    near(beta(1., 1e100, &ctx).unwrap(), 1e-100, 8e-14);
    for (x, e, c) in [
        (0.1, 0.1124629160182849, 0.887_537_083_981_715),
        (1., 0.8427007929497149, 0.15729920705028513),
        (2., 0.9953222650189527, 0.004677734981047266),
        (8., 1., 1.1224297172982926e-29),
        (26., 1., 5.663192408856143e-296),
    ] {
        near(erf(x, &ctx).unwrap(), e, 1e-14);
        near(erfc(x, &ctx).unwrap(), c, 8e-14);
        near(erf(-x, &ctx).unwrap(), -e, 1e-14);
    }
}
#[test]
fn probability_tails_and_failure_contracts() {
    let ctx = Interrupt::default();
    for (p, z) in [
        (0.5, 0.),
        (0.025, -1.959963984540054),
        (1e-20, -9.262340089798409),
        (1e-100, -21.273453560965326),
    ] {
        assert!((normal_quantile(p, &ctx).unwrap() - z).abs() < 5e-13);
    }
    for x in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(gamma(x, &ctx).is_err());
        assert!(log_gamma(x, &ctx).is_err());
    }
    assert!(gamma(172., &ctx).is_err());
    assert!(normal_quantile(-0.1, &ctx).is_err());
    assert!(normal_quantile(0., &ctx).unwrap().is_infinite());
    let cancelled = Interrupt::default();
    cancelled
        .flag
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        erfc(8., &cancelled),
        Err(om_analysis::Error::Abort(_))
    ));
}
#[test]
fn independent_libm_grid_covers_switches_and_subnormal_tails() {
    let ctx = Interrupt::default();
    for row in include_str!("special.tsv")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        let fields: Vec<_> = row.split('\t').collect();
        let x: f64 = fields[1].parse().unwrap();
        let reference: f64 = fields[2].parse().unwrap();
        let value = match fields[0] {
            "gamma" => gamma(x, &ctx),
            "log_gamma" => log_gamma(x, &ctx),
            "erf" => erf(x, &ctx),
            "erfc" => erfc(x, &ctx),
            _ => panic!("{row}"),
        }
        .unwrap();
        assert!(
            (value - reference).abs()
                <= 3e-13
                    * reference.abs().max(1.)
                    * if fields[0] == "erfc" {
                        reference.abs().max(1e-308)
                    } else {
                        1.
                    }
                    + 1e-320,
            "{row}: {value}"
        );
    }
}
#[test]
fn log_gamma_preserves_small_values_near_its_two_zeros() {
    let ctx = Interrupt::default();
    for delta in [1e-8, -1e-8, 1e-12, -1e-12] {
        let x = 1. + delta;
        // Independent local coefficients: derivative at 1 is -EulerGamma, at 2 is 1-EulerGamma.
        let z = x - 1.;
        let expected = -0.5772156649015329 * z + std::f64::consts::PI.powi(2) / 12. * z * z;
        assert!((log_gamma(x, &ctx).unwrap() - expected).abs() < 2e-15 * expected.abs());
        let x = 2. + delta;
        let z = x - 2.;
        let expected =
            (1. - 0.5772156649015329) * z + (std::f64::consts::PI.powi(2) / 12. - 0.5) * z * z;
        assert!((log_gamma(x, &ctx).unwrap() - expected).abs() < 2e-15 * expected.abs());
    }
}
