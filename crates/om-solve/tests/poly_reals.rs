//! Real-domain filtering checks root counts, principal radicals and unknown parameters.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_num::{
    BigFloat,
    ctx::{Abort, Interrupt},
};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    Domain, MaxExtra, NoSteps, Solution, SolutionSet, SolveOptions, StepKind, StepRecorder,
    univariate::poly_uni,
};
use proptest::prelude::*;
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
fn reals() -> SolveOptions {
    SolveOptions {
        domain: Domain::Reals,
        ..SolveOptions::default()
    }
}
fn solve(p: &str, opts: &SolveOptions) -> Vec<Solution> {
    let result = poly_uni(&e(p), &e("x"), opts, &Interrupt::default(), &mut NoSteps).unwrap();
    assert!(
        result.messages.is_empty(),
        "{p}: unexpected realness diagnostic"
    );
    let SolutionSet::Finite(roots) = result.set else {
        panic!("finite polynomial: {p}")
    };
    roots
}
#[test]
fn real_domain_keeps_exact_real_radicals_and_rejects_nonreal_branches() {
    for (p, n) in [
        ("x^2+1", 0),
        ("x^3-2", 1),
        ("x^4-16", 2),
        ("x^4+1", 0),
        ("x^6+x^3-1", 2),
        ("x-Sqrt[2]", 1),
        ("x-Sqrt[2]-I/2^300", 0),
    ] {
        let roots = solve(p, &reals());
        assert_eq!(roots.len(), n, "{p}");
        for root in roots {
            let z = om_simplify::numeval::enclose(&root.rules[0].1, 1024, &Interrupt::default())
                .unwrap()
                .unwrap();
            assert!(z.im.contains_zero(), "{p}");
            assert!(z.im.rad < BigFloat::from_parts(1.into(), -900), "{p}");
        }
    }
}
#[test]
fn native_root_indices_are_filtered_per_factor_with_distinct_root_counts() {
    for (p, n) in [
        ("x^5-x+1", 1),
        ("x^3-3*x+1", 3),
        ("x^4+x+1", 0),
        ("x^4-5*x^2+x+1", 4),
    ] {
        let roots = solve(p, &reals());
        assert_eq!(roots.len(), n, "{p}");
        assert!(roots.iter().all(|r| r.rules[0].1.is_head(B::ROOT)));
    }
    let roots = solve("(x^5-x+1)^2*(x-2)^3*(x^2+1)^4", &reals());
    assert_eq!(roots.len(), 2);
    assert!(
        roots
            .iter()
            .any(|r| r.rules[0].1 == e("2") && r.multiplicity == 3)
    );
    let root = roots
        .iter()
        .find(|r| r.rules[0].1.is_head(B::ROOT))
        .unwrap();
    assert_eq!(root.rules[0].1.args()[1], e("1"));
    assert_eq!(root.multiplicity, 2);
}
#[test]
fn cardano_and_ferrari_real_subsets_preserve_original_radical_values() {
    let opts = SolveOptions {
        cubics: true,
        quartics: true,
        ..reals()
    };
    for (p, n) in [
        ("x^3-3*x+1", 3),
        ("x^3+x+1", 1),
        ("x^4+x+1", 0),
        ("x^4-5*x^2+x+1", 4),
    ] {
        let full = solve(
            p,
            &SolveOptions {
                domain: Domain::Complexes,
                ..opts.clone()
            },
        );
        let kept = solve(p, &opts);
        assert_eq!(kept.len(), n, "{p}");
        assert!(kept.iter().all(|r| {
            full.iter()
                .any(|s| s.rules == r.rules && s.multiplicity == r.multiplicity)
        }));
    }
}
#[test]
fn unknown_parameter_root_realness_keeps_roots_assumptions_and_diagnostics() {
    let opts = SolveOptions {
        max_extra_conditions: MaxExtra::All,
        ..reals()
    };
    let mut recorder = StepRecorder::new();
    let result = poly_uni(
        &e("x^2*(a*x^3+x+1)"),
        &e("x"),
        &opts,
        &Interrupt::default(),
        &mut recorder,
    )
    .unwrap();
    assert_eq!(result.assumptions, [e("a!=0")]);
    assert_eq!(result.messages.len(), 3);
    assert!(
        result
            .messages
            .iter()
            .all(|m| m.symbol == "Solve" && m.tag == "real")
    );
    let SolutionSet::Finite(roots) = result.set else {
        panic!("unknown roots must be retained")
    };
    assert_eq!(roots.len(), 4);
    assert!(roots.iter().all(|r| r.condition == Some(e("a!=0"))));
    fn notes(steps: &[om_solve::Step]) -> usize {
        steps
            .iter()
            .map(|s| {
                usize::from(
                    matches!(&s.kind,StepKind::Note{msg} if msg.symbol=="Solve"&&msg.tag=="real"),
                ) + notes(&s.children)
            })
            .sum()
    }
    assert_eq!(notes(&recorder.finish().root), 3);
    let mut recorder = StepRecorder::new();
    let result = poly_uni(
        &e("x^2*(a*x^3+x+1)"),
        &e("x"),
        &SolveOptions {
            record_steps: false,
            ..opts
        },
        &Interrupt::default(),
        &mut recorder,
    )
    .unwrap();
    assert_eq!(result.messages.len(), 3);
    assert!(recorder.finish().root.is_empty());
}
#[test]
fn domain_filter_records_actual_counts_and_honors_disabled_recording_and_abort() {
    for record in [true, false] {
        let mut sink = StepRecorder::new();
        poly_uni(
            &e("x^4-16"),
            &e("x"),
            &SolveOptions {
                record_steps: record,
                ..reals()
            },
            &Interrupt::default(),
            &mut sink,
        )
        .unwrap();
        let steps = sink.finish();
        if record {
            assert!(steps.root.iter().any(|s| matches!(
                s.kind,
                StepKind::DomainFilter {
                    domain: Domain::Reals,
                    kept: 2,
                    dropped: 2
                }
            )));
        } else {
            assert!(steps.root.is_empty());
        }
    }
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        poly_uni(&e("x^5-x+1"), &e("x"), &reals(), &ctx, &mut NoSteps),
        Err(om_solve::SolveError::Abort(Abort::Budget))
    ));
}
#[test]
fn precision_refinement_separates_tiny_imaginary_parts_and_retains_numeric_unknowns() {
    // The two exponential terms are equal, but interval dependency keeps a nonzero
    // imaginary radius. A zero midpoint does not prove their difference real.
    let constant = "E^(I*Pi/7)-Conjugate[E^(-I*Pi/7)]";
    let result = poly_uni(
        &e(&format!("x-({constant})")),
        &e("x"),
        &reals(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(roots) = result.set else {
        panic!("unknown must be retained")
    };
    assert_eq!(roots.len(), 1);
    assert_eq!(result.messages.len(), 1);
    let result = poly_uni(
        &e(&format!("x-({constant}+I/2^200)")),
        &e("x"),
        &reals(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(roots) = result.set else {
        panic!("finite")
    };
    assert!(roots.is_empty());
    assert!(result.messages.is_empty());
}
proptest! {
    #![proptest_config(ProptestConfig{cases:32,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e36),..ProptestConfig::default()})]
    #[test]
    fn planted_real_and_gaussian_factors_keep_only_real_multiplicities(a in -4i64..=4,b in -4i64..=4) {
        let roots=solve(&format!("(x-({a}))^2*(x-({b}))*(x^2+1)"),&reals());
        prop_assert_eq!(roots.iter().map(|r|r.multiplicity).sum::<u32>(),3);
        prop_assert_eq!(roots.len(),if a==b {1}else{2});
        prop_assert!(roots.iter().all(|r|r.rules[0].1==Expr::int(a)||r.rules[0].1==Expr::int(b)));
    }
}
