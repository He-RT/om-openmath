//! Radical elimination must verify original principal branches and source exclusions.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_simplify::zero::Tri;
use om_solve::{
    Domain, NoSteps, Solution, SolutionSet, SolveOptions, Step, StepKind, StepRecorder,
    Verification, VerifyMode, univariate::radical_path,
};
use proptest::prelude::*;
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn solve(s: &str, opts: &SolveOptions) -> Vec<Solution> {
    let got = radical_path(&raw(s), &e("x"), opts, &Interrupt::default(), &mut NoSteps).unwrap();
    let SolutionSet::Finite(roots) = got.set else {
        panic!("finite radical solutions: {s}")
    };
    assert!(
        roots.iter().all(|r| matches!(
            r.verification,
            Verification::Exact | Verification::Numeric { .. }
        )),
        "{s}"
    );
    roots
}
fn values(roots: &[Solution]) -> Vec<Expr> {
    roots.iter().map(|r| r.rules[0].1.clone()).collect()
}
fn flatten(steps: &[Step]) -> Vec<&Step> {
    steps
        .iter()
        .flat_map(|s| std::iter::once(s).chain(flatten(&s.children)))
        .collect()
}
#[test]
fn authority_square_root_equations_drop_extraneous_candidates_even_when_verify_never() {
    for verify in [VerifyMode::Auto, VerifyMode::Always, VerifyMode::Never] {
        let opts = SolveOptions {
            verify,
            ..SolveOptions::default()
        };
        assert_eq!(values(&solve("Sqrt[x+2]-x", &opts)), [e("2")]);
        assert_eq!(values(&solve("Sqrt[x]+Sqrt[x-5]-5", &opts)), [e("9")]);
    }
}
#[test]
fn powers_denominators_and_shared_fractional_bases_use_principal_values() {
    for (p, want) in [
        ("x^(2/3)-4", "8"),
        ("x^(1/3)+x^(1/2)-2", "1"),
        ("x^(-1/2)-2", "1/4"),
        ("Sqrt[(x+1)/(x-1)]-2", "5/3"),
    ] {
        assert_eq!(
            values(&solve(p, &SolveOptions::default())),
            [e(want)],
            "{p}"
        );
    }
    assert!(solve("Sqrt[x]+1", &SolveOptions::default()).is_empty());
    assert_eq!(e("Sqrt[-1]"), e("I"));
    assert_eq!(
        values(&solve(
            "Sqrt[x]-I",
            &SolveOptions {
                domain: Domain::Reals,
                ..SolveOptions::default()
            }
        )),
        [e("-1")]
    );
    assert!(
        solve(
            "Sqrt[x]-1-I",
            &SolveOptions {
                domain: Domain::Reals,
                ..SolveOptions::default()
            }
        )
        .is_empty()
    );
    assert_eq!(
        values(&solve("Sqrt[x]-I", &SolveOptions::default())),
        [e("-1")]
    );
}
#[test]
fn nested_and_three_radical_equations_have_complete_resultant_elimination() {
    for (p, want) in [
        ("Sqrt[Sqrt[x]+1]-2", "9"),
        ("Sqrt[x]+Sqrt[x+3]+Sqrt[x+8]-6", "1"),
        ("x^(1/2)+x^(1/3)+x^(1/6)-3", "1"),
    ] {
        let mut sink = StepRecorder::new();
        let got = radical_path(
            &raw(p),
            &e("x"),
            &SolveOptions::default(),
            &Interrupt::default(),
            &mut sink,
        )
        .unwrap();
        let SolutionSet::Finite(roots) = got.set else {
            panic!("finite {p}")
        };
        assert_eq!(values(&roots), [e(want)], "{p}");
        let steps = sink.finish();
        let all = flatten(&steps.root);
        assert!(
            all.iter()
                .any(|s| matches!(s.kind, StepKind::Resultant { .. })),
            "{p}"
        );
        assert!(
            all.iter()
                .any(|s| matches!(s.kind, StepKind::Verify { .. })),
            "{p}"
        );
    }
}
#[test]
fn isolation_power_and_verification_steps_record_the_actual_equations_and_rejection() {
    let mut sink = StepRecorder::new();
    radical_path(
        &raw("Sqrt[x+2]-x"),
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    let steps = sink.finish();
    let all = flatten(&steps.root);
    let isolated = all
        .iter()
        .find(|s| matches!(s.kind, StepKind::IsolateTerm { .. }))
        .unwrap();
    assert_eq!(isolated.after, [e("Sqrt[x+2]==x")]);
    let power = all
        .iter()
        .find(|s| matches!(s.kind, StepKind::RaiseToPower { n: 2 }))
        .unwrap();
    assert_eq!(power.before, [e("Sqrt[x+2]==x")]);
    assert_eq!(power.after, [e("x+2==x^2")]);
    assert!(all.iter().any(|s|matches!(&s.kind,StepKind::DropExtraneous{candidate,..} if candidate==&vec![(e("x"),e("-1"))])));
    assert!(all.iter().any(|s|matches!(&s.kind,StepKind::Verify{candidate,outcome:Tri::NonZero,residual:Some(r)} if candidate==&vec![(e("x"),e("-1"))] && r==&e("2"))));
}
#[test]
fn raw_cancelled_denominator_holes_are_checked_before_accepting_roots() {
    assert!(solve("((x-2)/(x-2))*(Sqrt[x+2]-2)", &SolveOptions::default()).is_empty());
    assert_eq!(
        values(&solve("((x-2)/(x-2))*Sqrt[x+2]", &SolveOptions::default())),
        [e("-2")]
    );
}
#[test]
fn elimination_multiplicity_is_not_reported_as_duplicate_radical_solutions() {
    let roots = solve("(Sqrt[x]-1)^2", &SolveOptions::default());
    assert_eq!(values(&roots), [e("1")]);
    assert_eq!(roots[0].multiplicity, 1);
}
#[test]
fn polynomial_delegation_retains_yun_multiplicity() {
    let roots = solve("(x-1)^3", &SolveOptions::default());
    assert_eq!(values(&roots), [e("1")]);
    assert_eq!(roots[0].multiplicity, 3);
}
#[test]
fn degenerate_eliminants_unsupported_kernels_disabled_steps_and_abort_are_honest() {
    for p in [
        "Sqrt[x^2]-x",
        "Sqrt[Sin[x]]-1",
        "Sqrt[x]+Sin[x]-1",
        "Sqrt[(x-1)/(x-1)]-1",
    ] {
        let got = radical_path(
            &raw(p),
            &e("x"),
            &SolveOptions::default(),
            &Interrupt::default(),
            &mut NoSteps,
        )
        .unwrap();
        assert!(matches!(got.set, SolutionSet::Unevaluated), "{p}");
    }
    let mut recorder = StepRecorder::new();
    radical_path(
        &raw("Sqrt[x+2]-x"),
        &e("x"),
        &SolveOptions {
            record_steps: false,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut recorder,
    )
    .unwrap();
    assert!(recorder.finish().root.is_empty());
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        radical_path(
            &raw("Sqrt[x+2]-x"),
            &e("x"),
            &SolveOptions::default(),
            &ctx,
            &mut NoSteps
        ),
        Err(om_solve::SolveError::Abort(Abort::Budget))
    ));
}
#[test]
fn parameter_content_and_source_denominator_assumptions_are_retained() {
    let opts = SolveOptions {
        max_extra_conditions: om_solve::MaxExtra::All,
        ..SolveOptions::default()
    };
    for p in ["a*(Sqrt[x+2]-x)", "(Sqrt[x+2]-x)/a"] {
        let got =
            radical_path(&raw(p), &e("x"), &opts, &Interrupt::default(), &mut NoSteps).unwrap();
        assert!(!got.assumptions.is_empty());
        let SolutionSet::Finite(roots) = got.set else {
            panic!("finite")
        };
        assert_eq!(values(&roots), [e("2")]);
        assert!(roots[0].condition.is_some());
    }
}
proptest! {
    #![proptest_config(ProptestConfig{cases:24,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e37),..ProptestConfig::default()})]
    #[test]
    fn planted_nonnegative_square_root_roots_are_verified(s in 1i64..=4,k in -3i64..=3) {
        let p=format!("Sqrt[x+({k})]-({s})");
        let roots=solve(&p,&SolveOptions::default());
        prop_assert_eq!(values(&roots),[Expr::int(s*s-k)]);
        prop_assert_eq!(roots[0].multiplicity,1);
        prop_assert!(roots[0].rules[0].0==e("x"));
    }
}
