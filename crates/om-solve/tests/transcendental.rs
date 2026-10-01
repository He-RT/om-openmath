//! Inverse families retain periods, branch restrictions and original verification.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    Domain, NoSteps, Solution, SolutionSet, SolveOptions, Step, StepKind, StepRecorder,
    Verification, univariate::transcendental_path,
};
use proptest::prelude::*;

fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn opts(domain: Domain) -> SolveOptions {
    SolveOptions {
        domain,
        ..SolveOptions::default()
    }
}
fn solve(s: &str, domain: Domain) -> Vec<Solution> {
    let result = transcendental_path(
        &raw(s),
        &e("x"),
        &opts(domain),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(roots) = result.set else {
        panic!("finite {s}")
    };
    assert!(
        roots.iter().all(|r| matches!(
            r.verification,
            Verification::Exact | Verification::Numeric { .. }
        )),
        "verified {s}"
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
fn authority_exponential_log_and_periodic_families() {
    assert_eq!(values(&solve("Exp[x]-2", Domain::Reals)), [e("Log[2]")]);
    let roots = solve("Exp[x]-2", Domain::Complexes);
    assert_eq!(values(&roots), [e("Log[2]+2 Pi I C[1]")]);
    assert_eq!(roots[0].constants, [(e("C[1]"), Domain::Integers)]);
    assert_eq!(values(&solve("Log[x]-2", Domain::Complexes)), [e("E^2")]);
    assert_eq!(
        values(&solve("Sin[x]-1/2", Domain::Complexes)),
        [e("Pi/6+2 Pi C[1]"), e("5 Pi/6+2 Pi C[1]")]
    );
    assert_eq!(
        values(&solve("Tan[x]-1", Domain::Complexes)),
        [e("Pi/4+Pi C[1]")]
    );
    assert_eq!(
        values(&solve("Cos[x]+1", Domain::Complexes)),
        [e("Pi+2 Pi C[1]")]
    );
    assert!(solve("Sin[x]-2", Domain::Reals).is_empty());
    assert!(solve("Exp[x]", Domain::Complexes).is_empty());
    assert!(solve("Tan[x]-I", Domain::Complexes).is_empty());
}
#[test]
fn commensurate_exponentials_use_rational_slopes_shifts_and_integer_power_bases() {
    for (p, want) in [
        ("E^(2x)-3 E^x+2", vec![e("0"), e("Log[2]")]),
        ("4^x-5 2^x+4", vec![e("0"), e("2")]),
        ("E^(x/2+1)-E", vec![e("0")]),
        ("3^x-9", vec![e("2")]),
    ] {
        assert_eq!(values(&solve(p, Domain::Reals)), want, "{p}");
    }
}
#[test]
fn sine_cosine_unification_and_nested_inverse_stripping_are_complete() {
    assert_eq!(
        values(&solve("Sin[x]+Cos[x]-1", Domain::Reals)),
        [e("2 Pi C[1]"), e("Pi/2+2 Pi C[1]")]
    );
    assert_eq!(
        values(&solve("Sin[2x+Pi/6]-1/2", Domain::Reals)),
        [e("Pi C[1]"), e("Pi/3+Pi C[1]")]
    );
    let roots = solve("Sin[x^2]-1/2", Domain::Complexes);
    assert_eq!(roots.len(), 4);
    assert!(
        roots
            .iter()
            .all(|r| r.constants == vec![(e("C[1]"), Domain::Integers)])
    );
}
#[test]
fn principal_log_and_inverse_trig_ranges_are_enforced() {
    assert_eq!(values(&solve("Log[x]-I Pi", Domain::Complexes)), [e("-1")]);
    assert!(solve("Log[x]+I Pi", Domain::Complexes).is_empty());
    assert!(solve("Log[x]-2 I Pi", Domain::Complexes).is_empty());
    assert_eq!(
        values(&solve("ArcSin[x]-Pi/6", Domain::Complexes)),
        [e("1/2")]
    );
    assert!(solve("ArcSin[x]-Pi", Domain::Complexes).is_empty());
    assert_eq!(values(&solve("ArcCos[x]-Pi", Domain::Complexes)), [e("-1")]);
    assert!(solve("ArcTan[x]-Pi/2", Domain::Complexes).is_empty());
    let roots = solve("Log[x]-a", Domain::Complexes);
    assert_eq!(values(&roots), [e("E^a")]);
    assert!(roots[0].condition.is_some());
}
#[test]
fn reciprocal_trig_and_hyperbolic_inverse_table_has_verified_branches() {
    for (p, n) in [
        ("Sec[x]-2", 2),
        ("Csc[x]-2", 2),
        ("Cot[x]", 1),
        ("Sinh[x]", 2),
        ("Cosh[x]-1", 1),
        ("Tanh[x]-1/2", 1),
    ] {
        let roots = solve(p, Domain::Complexes);
        assert_eq!(roots.len(), n, "{p}");
        assert!(roots.iter().all(|r| r.constants.len() == 1), "{p}");
    }
    assert!(solve("Tanh[x]-1", Domain::Complexes).is_empty());
}
#[test]
fn product_log_uses_exact_defining_identity_and_real_branch_ranges() {
    let result = transcendental_path(
        &raw("x E^x-1"),
        &e("x"),
        &opts(Domain::Complexes),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(result.messages.iter().any(|m| m.tag == "ifun"));
    let SolutionSet::Finite(roots) = result.set else {
        panic!("finite")
    };
    assert_eq!(values(&roots), [e("ProductLog[1]")]);
    assert!(matches!(roots[0].verification, Verification::Exact));
    assert_eq!(solve("x E^x+1/(2 E)", Domain::Reals).len(), 2);
    assert!(solve("x E^x+1", Domain::Reals).is_empty());
}
#[test]
fn fresh_constants_original_poles_steps_and_options_are_honest() {
    let roots = solve("Exp[x]-Exp[C[1]]", Domain::Complexes);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].constants, [(e("C[2]"), Domain::Integers)]);
    assert!(solve("(x/x)*(Exp[x]-1)", Domain::Reals).is_empty());
    let mut sink = StepRecorder::new();
    transcendental_path(
        &raw("E^(2x)-3 E^x+2"),
        &e("x"),
        &opts(Domain::Reals),
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    let steps = sink.finish();
    let all = flatten(&steps.root);
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::Substitute { .. }))
    );
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::InvertFunction { .. }))
    );
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::Verify { .. }))
    );
    let mut sink = StepRecorder::new();
    transcendental_path(
        &raw("Exp[x]-2"),
        &e("x"),
        &SolveOptions {
            record_steps: false,
            ..opts(Domain::Reals)
        },
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    assert!(sink.finish().root.is_empty());
    for p in ["E^x+2^x-3", "Sin[x]+Cos[2x]-1", "x+Sin[x]", "f[x]-1"] {
        let result = transcendental_path(
            &raw(p),
            &e("x"),
            &opts(Domain::Complexes),
            &Interrupt::default(),
            &mut NoSteps,
        )
        .unwrap();
        assert!(matches!(result.set, SolutionSet::Unevaluated), "{p}");
    }
    let result = transcendental_path(
        &raw("Sin[x]-1/2"),
        &e("x"),
        &SolveOptions {
            inverse_functions: false,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(result.set, SolutionSet::Unevaluated));
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        transcendental_path(
            &raw("Exp[x]-2"),
            &e("x"),
            &SolveOptions::default(),
            &ctx,
            &mut NoSteps
        ),
        Err(om_solve::SolveError::Abort(Abort::Budget))
    ));
}

#[test]
fn degenerate_boundaries_symbolic_nonzero_values_and_polynomial_delegation() {
    let identity = transcendental_path(
        &raw("x-x"),
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(identity.set, SolutionSet::All));
    assert_eq!(solve("(x-1)^3", Domain::Complexes)[0].multiplicity, 3);
    assert!(solve("Exp[x]+1", Domain::Reals).is_empty());
    assert!(solve("ArcSin[x]-Pi/2-I", Domain::Complexes).is_empty());
    assert_eq!(solve("ArcSin[x]-Pi/2+I", Domain::Complexes).len(), 1);
    let roots = solve("Exp[x]-a", Domain::Complexes);
    assert_eq!(roots.len(), 1);
    assert!(roots[0].condition.is_some());
    let ctx = Interrupt::default();
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        transcendental_path(
            &raw("Sin[x]-1/2"),
            &e("x"),
            &SolveOptions::default(),
            &ctx,
            &mut NoSteps
        ),
        Err(om_solve::SolveError::Abort(Abort::Interrupted))
    ));
}

#[test]
fn excluded_members_of_a_complex_family_become_conditions() {
    let roots = solve("(x/x)*(Exp[x]-1)", Domain::Complexes);
    assert_eq!(roots.len(), 1);
    assert!(roots[0].condition.is_some());
    assert_eq!(roots[0].constants, [(e("C[1]"), Domain::Integers)]);
}

#[test]
fn symbolic_inverse_ranges_keep_original_branch_guards_and_uncertain_numeric_boundaries_decline() {
    let roots = solve("ArcSin[x]-a", Domain::Complexes);
    let cond = roots[0].condition.as_ref().unwrap();
    let guard = e("ArcSin[Sin[a]]==a");
    let mut stack = vec![cond];
    let mut found = false;
    while let Some(c) = stack.pop() {
        found |= *c == guard;
        stack.extend(c.args());
    }
    assert!(found, "the strip alone admits the wrong complex boundary");
    let got = transcendental_path(
        &raw("ArcTan[x]-Pi/2-I"),
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Unevaluated));
}

#[test]
fn outer_integer_and_fractional_powers_of_elementary_kernels_keep_principal_branches() {
    for (p, count) in [
        ("Sin[x]^2-1/4", 4),
        ("Sqrt[Sin[x]]-1/2", 2),
        ("Sin[x]^(2/3)-4", 2),
        ("Sin[x]^(-1/2)-2", 2),
    ] {
        assert_eq!(solve(p, Domain::Complexes).len(), count, "{p}");
    }
}

#[test]
fn real_absolute_values_split_real_arguments_and_refuse_unproven_complex_arguments() {
    assert_eq!(values(&solve("Abs[x]-2", Domain::Reals)), [e("-2"), e("2")]);
    assert_eq!(
        values(&solve("Abs[2x-1]-3", Domain::Reals)),
        [e("-1"), e("2")]
    );
    assert_eq!(solve("Abs[Sin[x]]-1/2", Domain::Reals).len(), 4);
    assert!(solve("Abs[x]+1", Domain::Reals).is_empty());
    for (p, domain) in [
        ("Abs[x+I]-2", Domain::Reals),
        ("Abs[x]-2", Domain::Complexes),
    ] {
        let result = transcendental_path(
            &raw(p),
            &e("x"),
            &opts(domain),
            &Interrupt::default(),
            &mut NoSteps,
        )
        .unwrap();
        assert!(matches!(result.set, SolutionSet::Unevaluated));
    }
}

#[test]
fn generic_leading_assumptions_skip_invalid_parameter_samples() {
    let result = transcendental_path(
        &raw("a Sin[x]-1"),
        &e("x"),
        &SolveOptions {
            seed: 5,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(result.assumptions.contains(&e("a!=0")));
    let SolutionSet::Finite(roots) = result.set else {
        panic!("finite generic inverse")
    };
    assert_eq!(roots.len(), 2);
}

#[test]
fn mandatory_checks_custom_constants_and_generic_conditions_obey_options() {
    let opts = SolveOptions {
        generated_parameter: om_core::Symbol::intern("K"),
        verify: om_solve::VerifyMode::Never,
        max_extra_conditions: om_solve::MaxExtra::All,
        ..SolveOptions::default()
    };
    let result = transcendental_path(
        &raw("a*(Sin[x]-1/2)"),
        &e("x"),
        &opts,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(roots) = result.set else {
        panic!("finite")
    };
    assert_eq!(roots.len(), 2);
    assert!(
        roots
            .iter()
            .all(|r| r.constants == vec![(e("K[1]"), Domain::Integers)] && r.condition.is_some())
    );
    let result = transcendental_path(
        &raw("Sqrt[Sin[x]]+1"),
        &e("x"),
        &opts,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(result.set,SolutionSet::Finite(r)if r.is_empty()));
}

proptest! {
    #![proptest_config(ProptestConfig{cases:16,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e38),..ProptestConfig::default()})]
    #[test]
    fn generated_affine_sine_families_satisfy_independent_ball_checks(a in 1i64..=5,b in -3i64..=3) {
        let source=raw(&format!("Sin[({a})x+({b}) Pi/6]-1/2"));
        let roots=solve(&format!("Sin[({a})x+({b}) Pi/6]-1/2"),Domain::Complexes);
        prop_assert_eq!(roots.len(),2);
        for root in roots {
            for k in [-2,-1,0,1,2] {
                let value=root.rules[0].1.replace_all(&[(root.constants[0].0.clone(),Expr::int(k))]);
                let residual=source.replace_all(&[(e("x"),value)]);
                let z=om_simplify::numeval::enclose(&residual,256,&Interrupt::default()).unwrap().unwrap();
                prop_assert!(z.re.contains_zero()&&z.im.contains_zero());
                let bound=om_num::BigFloat::from_parts(1.into(),-200);
                prop_assert!(z.re.rad<bound&&z.im.rad<bound);
            }
        }
    }
}
