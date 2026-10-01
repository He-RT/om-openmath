//! Search chooses shorter equivalent forms while preserving principal branches.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_simplify::{
    simplify::{SimplifyOptions, complexity, simplify, simplify_with},
    zero::{Tri, is_zero},
};
use proptest::prelude::*;
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
#[test]
fn rational_search_combines_cancellation_factoring_and_content() {
    for (input, expected) in [
        ("(x^2-1)/(x-1)", "x+1"),
        ("x*y+x*z", "x*(y+z)"),
        ("6*x+9*y", "6*x+9*y"),
        (
            "6000000000000000*x+9000000000000000*y",
            "3000000000000000*(2*x+3*y)",
        ),
        ("(x+1)^2-x^2-2*x-1", "0"),
        ("1/x+1/y-(x+y)/(x*y)", "0"),
        ("f[(x^2-1)/(x-1)]", "f[x+1]"),
    ] {
        assert_eq!(simplify(&e(input)), e(expected), "{input}");
    }
}
#[test]
fn trigonometric_pythagorean_and_both_double_angle_directions() {
    for (input, expected) in [
        ("Sin[x]^2+Cos[x]^2", "1"),
        ("3*Sin[x]^2+3*Cos[x]^2-3", "0"),
        ("2*Sin[x]*Cos[x]", "Sin[2*x]"),
        ("Cos[x]^2-Sin[x]^2", "Cos[2*x]"),
        ("Cos[2*x]+2*Sin[x]^2", "1"),
        ("Sin[2*x]-2*Sin[x]*Cos[x]", "0"),
        ("Sin[x]^2+Cos[y]^2", "Sin[x]^2+Cos[y]^2"),
    ] {
        assert_eq!(simplify(&e(input)), e(expected), "{input}");
    }
}
#[test]
fn exact_denesting_and_root_reduce_are_search_candidates() {
    assert_eq!(simplify(&e("Sqrt[3+2*Sqrt[2]]-1-Sqrt[2]")), Expr::int(0));
    assert_eq!(simplify(&e("Root[#^3-2&,1]^3-2")), Expr::int(0));
    let input = e("Sqrt[2]+Sqrt[3]");
    let got = simplify(&input);
    assert!(complexity(&got) < complexity(&input));
    assert_eq!(is_zero(&om_core::sub(input, got)), Tri::Zero);
}
#[test]
fn power_expansion_requires_explicit_positive_real_variables() {
    let ctx = Interrupt::default();
    let input = e("Sqrt[x*y]-Sqrt[x]*Sqrt[y]");
    assert_ne!(simplify(&input), Expr::int(0));
    let opts = SimplifyOptions {
        positive: vec![e("x").as_symbol().unwrap(), e("y").as_symbol().unwrap()],
    };
    assert_eq!(simplify_with(&input, &opts, &ctx).unwrap(), Expr::int(0));
    assert_eq!(simplify_with(&e("Sqrt[x^2]"), &opts, &ctx).unwrap(), e("x"));
    assert_ne!(simplify(&e("Sqrt[x^2]")), e("x"));
    assert_eq!(simplify(&e("Sqrt[(-2)^2]")), Expr::int(2));
}
#[test]
fn weighted_integer_and_root_costs_are_exact_quarter_units() {
    for (input, cost) in [
        ("x", 4),
        ("0", 4),
        ("1", 4),
        ("-1", 4),
        ("2", 5),
        ("10", 5),
        ("11", 6),
        ("1000", 7),
        ("1001", 8),
        ("Root[#^5-#-1&,1]", 12),
    ] {
        assert_eq!(complexity(&e(input)), cost, "{input}");
    }
}
#[test]
fn checked_search_propagates_interrupts_and_preserves_unsupported_atoms() {
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert_eq!(
        simplify_with(&e("x"), &SimplifyOptions::default(), &ctx),
        Err(Abort::Budget)
    );
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        simplify_with(&Expr::int(0), &SimplifyOptions::default(), &ctx),
        Err(Abort::Interrupted)
    );
    for input in ["f[x]", "\"text\"", "Indeterminate", "Sin[2*x]"] {
        assert_eq!(simplify(&e(input)), e(input));
    }
}
#[test]
fn rational_denominator_cycles_preserve_the_winner_with_finite_work() {
    let ctx = Interrupt::default();
    ctx.steps_left.set(10_000_000);
    let input = e("(x^2-1)/(x-1)");
    assert_eq!(
        simplify_with(&input, &SimplifyOptions::default(), &ctx).unwrap(),
        e("x+1")
    );
    assert!(ctx.steps_left.get() > 0);
    struct Expired;
    impl om_num::ctx::Clock for Expired {
        fn now_ms(&self) -> f64 {
            1.0
        }
    }
    let ctx = Interrupt {
        clock: Some(std::sync::Arc::new(Expired)),
        deadline_ms: Some(0.0),
        ..Interrupt::default()
    };
    assert_eq!(
        simplify_with(&input, &SimplifyOptions::default(), &ctx),
        Err(Abort::Timeout)
    );
    struct Advancing(std::sync::atomic::AtomicUsize);
    impl om_num::ctx::Clock for Advancing {
        fn now_ms(&self) -> f64 {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as f64
        }
    }
    let ctx = Interrupt {
        clock: Some(std::sync::Arc::new(Advancing(0.into()))),
        deadline_ms: Some(1.0),
        ..Interrupt::default()
    };
    assert_eq!(
        simplify_with(&input, &SimplifyOptions::default(), &ctx),
        Err(Abort::Timeout)
    );
}
proptest! {
    #![proptest_config(ProptestConfig{cases:32,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d382e34),..ProptestConfig::default()})]
    #[test]
    fn search_never_increases_cost_or_changes_a_rational_function(a in -4i64..=4,b in -4i64..=4) {
        let input=e(&format!("(x+({a}))*(y+({b}))+x+({a})"));
        let got=simplify(&input);
        prop_assert!(complexity(&got)<=complexity(&input));
        prop_assert_eq!(is_zero(&om_core::sub(got.clone(),input)),Tri::Zero);
        prop_assert_eq!(simplify(&got),got);
    }
}
