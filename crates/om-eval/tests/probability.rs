//! Probability definitions and reproducible streams run through the real modern evaluator.
use om_core::{Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, source: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(source, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn f64_value(ev: &mut Evaluator, source: &str) -> f64 {
    eval(ev, source).as_number().unwrap().to_f64().unwrap()
}
#[test]
fn real_special_values_and_distributions() {
    let mut ev = Evaluator::new();
    assert_eq!(eval(&mut ev, "gamma(5)"), Expr::int(24));
    assert_eq!(eval(&mut ev, "erf(0)"), Expr::int(0));
    assert_eq!(eval(&mut ev, "erfc(0)"), Expr::int(1));
    assert!((f64_value(&mut ev, "numeric(erf(1))") - 0.8427007929497149).abs() < 1e-14);
    assert!((f64_value(&mut ev, "numeric(acoth(2))") - 0.5493061443340548).abs() < 1e-14);
    assert!((f64_value(&mut ev, "cdf(normal_distribution(),0)") - 0.5).abs() < 1e-15);
    assert!(
        (f64_value(&mut ev, "pdf(normal_distribution(mean:2,std:3),2)") - 0.1329807601338109).abs()
            < 1e-14
    );
    assert!(
        (f64_value(&mut ev, "quantile(normal_distribution(),0.025)") + 1.959963984540054).abs()
            < 1e-13
    );
    assert_eq!(
        eval(&mut ev, "quantile(uniform_distribution(bounds:[2,6]),1/4)"),
        Expr::int(3)
    );
    assert_eq!(
        eval(&mut ev, "cdf(uniform_distribution(bounds:[2,6]),3)"),
        Expr::rational(1, 4)
    );
}
#[test]
fn explicit_seed_readonly_and_failed_requests_do_not_advance_session_stream() {
    let (mut a, mut b) = (Evaluator::new(), Evaluator::new());
    eval(&mut a, "random_uniform(count:4,seed:42)");
    assert_eq!(
        eval(&mut a, "random_uniform()"),
        eval(&mut b, "random_uniform()")
    );
    let mut fork = a.fork_readonly();
    eval(&mut fork, "random_normal(count:9)");
    eval(&mut fork, "seed_random(9)");
    eval(&mut a, "random_uniform(count:3,bounds:[1,0])");
    assert_eq!(
        eval(&mut a, "random_uniform()"),
        eval(&mut b, "random_uniform()")
    );
    assert_eq!(
        eval(&mut a, "random_normal(count:12,seed:77)"),
        eval(&mut b, "random_normal(count:12,seed:77)")
    );
    eval(&mut a, "seed_random(0)");
    assert_eq!(
        f64_value(&mut a, "random_uniform()"),
        (0xe220a8397b1dcdafu64 >> 11) as f64 / (1u64 << 53) as f64
    );
    let selected = eval(&mut a, "random_choice([\"a\",\"b\"],count:30,seed:42)");
    assert_eq!(selected.args().len(), 30);
    assert!(
        selected
            .args()
            .iter()
            .all(|v| *v == Expr::string("a") || *v == Expr::string("b"))
    );
}
#[test]
fn unsupported_domains_precision_and_options_report_real_failure() {
    let mut ev = Evaluator::new();
    for source in [
        "gamma(-0.5)",
        "erf(decimal(\"1\",precision:50))",
        "normal_distribution(std:0)",
        "uniform_distribution(bounds:[1,1])",
        "quantile(normal_distribution(),2)",
        "random_choice([])",
        "random_uniform(seed:-1)",
        "random_uniform(bounds:[1,1+1/10^30])",
    ] {
        ev.messages.take();
        let parsed = om_parse::parse_expr(source, om_parse::Dialect::Modern);
        if parsed.is_err() {
            continue;
        } // Literal options may be rejected by the same executable metadata before evaluation.
        ev.evaluate(&parsed.unwrap(), &Interrupt::default())
            .unwrap();
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    let invalid = om_parse::parse_expr(
        "PDF[UniformDistribution[{1,0}],0.5]",
        om_parse::Dialect::Wolfram,
    )
    .unwrap();
    assert!(
        ev.evaluate(&invalid, &Interrupt::default())
            .unwrap()
            .as_number()
            .is_none()
    );
}
#[test]
fn compiled_special_functions_keep_domains_and_actual_interrupts() {
    let expr = om_parse::parse_expr("erfc(x) + beta(x,2)", om_parse::Dialect::Modern).unwrap();
    let compiled = om_eval::numeric::compile_f64(&expr, &[om_core::Symbol::intern("x")]).unwrap();
    assert!((compiled.eval(&[2.]) - (0.004677734981047266 + 1. / 6.)).abs() < 1e-14);
    assert!(compiled.eval(&[-1.]).is_nan());
    let ctx = Interrupt::default();
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(compiled.eval_with_ctx(&[2.], &mut vec![], &ctx).is_err());
    let mut ev = Evaluator::new();
    assert!(f64_value(&mut ev, "acsch(1e-310)").is_finite());
    ev.messages.take();
    let original = eval(&mut ev, "numeric(erf(1),precision:50)");
    assert!(original.is_head(om_core::Symbol::intern("Erf")));
    assert!(!ev.messages.take().is_empty());
}
#[test]
fn random_cancellation_is_atomic_and_known_distribution_moments_hold() {
    let (mut a, mut b) = (Evaluator::new(), Evaluator::new());
    let source =
        om_parse::parse_expr("random_normal(count:1000)", om_parse::Dialect::Modern).unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(80);
    assert!(a.evaluate(&source, &ctx).is_err());
    assert_eq!(
        eval(&mut a, "random_uniform()"),
        eval(&mut b, "random_uniform()")
    );
    let samples = eval(&mut a, "random_normal(count:20000,seed:1234)");
    let values: Vec<_> = samples
        .args()
        .iter()
        .map(|v| v.as_number().unwrap().to_f64().unwrap())
        .collect();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    assert!(mean.abs() < 0.03);
    assert!((variance - 1.).abs() < 0.04);
    let uniform = eval(
        &mut a,
        "random_uniform(count:20000,bounds:[-2,3],seed:1234)",
    );
    assert!(uniform.args().iter().all(|v| {
        let x = v.as_number().unwrap().to_f64().unwrap();
        (-2. ..3.).contains(&x)
    }));
    for source in [
        "quantile(normal_distribution(),1-1/10^50)",
        "quantile(normal_distribution(),1/10^500)",
    ] {
        a.messages.take();
        eval(&mut a, source);
        assert!(!a.messages.take().is_empty());
    }
}
