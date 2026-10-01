//! Exact polynomial roots, multiplicity and genuine formula emission.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    Bound, Domain, Interval, MaxExtra, NoSteps, Solution, SolutionSet, SolveError, SolveOptions,
    Step, StepKind, StepRecorder, Verification, VerifyMode, univariate::poly_uni,
};
use proptest::prelude::*;
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
fn roots(s: &str) -> om_solve::univariate::PolynomialRoots {
    poly_uni(
        &e(s),
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap()
}
fn values(set: &SolutionSet) -> Vec<(Expr, u32)> {
    let SolutionSet::Finite(s) = set else {
        panic!("expected finite roots");
    };
    s.iter()
        .map(|s| (s.rules[0].1.clone(), s.multiplicity))
        .collect()
}
#[test]
fn authority_linear_quadratic_and_reducible_cubic_vectors() {
    for (input, expected) in [
        ("2*x+3-7", vec!["2"]),
        ("x^2-5*x+6", vec!["2", "3"]),
        ("x^2-2", vec!["-Sqrt[2]", "Sqrt[2]"]),
        ("x^2-2*x-1", vec!["1-Sqrt[2]", "1+Sqrt[2]"]),
        ("x^2+1", vec!["-I", "I"]),
        ("x^2+2*x+5", vec!["-1-2*I", "-1+2*I"]),
        ("x^3-6*x^2+11*x-6", vec!["1", "2", "3"]),
        ("x^3-2*x-4", vec!["2", "-1-I", "-1+I"]),
    ] {
        let got = roots(input);
        assert_eq!(
            values(&got.set),
            expected.iter().map(|s| (e(s), 1)).collect::<Vec<_>>(),
            "{input}"
        );
        for (value, _) in values(&got.set) {
            assert_eq!(
                om_simplify::zero::is_zero(&e(input).replace_all(&[(e("x"), value)])),
                om_simplify::zero::Tri::Zero
            );
        }
    }
}
#[test]
fn fractional_contents_multiplicity_and_complex_quadratics() {
    for (input, expected) in [
        ("(x-1)^2", vec![("1", 2)]),
        (
            "-7*(x-1)^3*(x^2+1)^2/15",
            vec![("1", 3), ("-I", 2), ("I", 2)],
        ),
        ("x^2/3-2/3", vec![("-Sqrt[2]", 1), ("Sqrt[2]", 1)]),
        ("(2*x-1)^2*(3*x+2)", vec![("-2/3", 1), ("1/2", 2)]),
    ] {
        assert_eq!(
            values(&roots(input).set),
            expected
                .into_iter()
                .map(|(s, m)| (e(s), m))
                .collect::<Vec<_>>(),
            "{input}"
        );
    }
    for (value, _) in values(&roots("x^2+x+1").set) {
        assert_eq!(
            om_simplify::zero::is_zero(&e("x^2+x+1").replace_all(&[(e("x"), value)])),
            om_simplify::zero::Tri::Zero
        );
    }
    assert_eq!(roots("(x-1)^2").set.to_expr(), e("{{x->1},{x->1}}"));
}
#[test]
fn generic_parameters_have_exact_formulas_and_leading_assumptions() {
    let got = roots("a*x+b");
    assert_eq!(values(&got.set), [(e("-b/a"), 1)]);
    assert_eq!(got.assumptions, [e("a!=0")]);
    let got = roots("a*x^2+b*x+c");
    assert_eq!(got.assumptions, [e("a!=0")]);
    let expected = [
        e("(-b-Sqrt[b^2-4*a*c])/(2*a)"),
        e("(-b+Sqrt[b^2-4*a*c])/(2*a)"),
    ];
    let vals = values(&got.set);
    assert_eq!(vals.len(), 2);
    assert!(
        vals.iter().all(|(v, m)| *m == 1 && expected.contains(v)),
        "{vals:?}"
    );
    for (v, _) in vals {
        let residual = om_simplify::algebra::expand(&e("a*x^2+b*x+c").replace_all(&[(e("x"), v)]));
        assert_eq!(
            om_simplify::zero::is_zero(&residual),
            om_simplify::zero::Tri::Zero
        );
    }
    assert_eq!(values(&roots("a*(x-1)^2").set), [(e("1"), 2)]);
    let opts = SolveOptions {
        max_extra_conditions: MaxExtra::All,
        ..SolveOptions::default()
    };
    let got = poly_uni(
        &e("a*x+b"),
        &e("x"),
        &opts,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert_eq!(
        got.set.to_expr(),
        e("{{x->ConditionalExpression[-b/a,a!=0]}}")
    );
}
#[test]
fn constants_exact_special_coefficients_and_unsupported_paths_are_honest() {
    for input in ["0", "Sin[Pi]*x", "(x+1)^2-x^2-2*x-1"] {
        assert!(matches!(roots(input).set, SolutionSet::All), "{input}");
    }
    for input in ["7/3", "a"] {
        assert!(values(&roots(input).set).is_empty(), "{input}");
    }
    for input in ["x^3-2", "x^5-x+1", "Sin[x]-1", "1/x-1", "(x-1)*(x^3-2)"] {
        assert!(
            matches!(roots(input).set, SolutionSet::Unevaluated),
            "{input}"
        );
    }
}
#[test]
fn symbolic_monomial_factors_are_extracted_with_multiplicity() {
    for (input, expected) in [
        ("a*x^7", vec![("0", 7)]),
        ("x^3*(a*x+b)", vec![("0", 3), ("-b/a", 1)]),
        ("a*x^4*(x-1)^2", vec![("0", 4), ("1", 2)]),
    ] {
        let got = roots(input);
        let actual = values(&got.set);
        assert_eq!(actual.len(), expected.len(), "{input}");
        assert!(
            expected
                .into_iter()
                .all(|(v, m)| actual.contains(&(e(v), m))),
            "{input}: {actual:?}"
        );
        assert_eq!(got.assumptions, [e("a!=0")]);
    }
    assert!(matches!(
        roots("x^2*(a*x^3+x+1)").set,
        SolutionSet::Unevaluated
    ));
}
#[test]
fn symbolic_coefficient_content_is_removed_before_root_formulas() {
    for (input, expected) in [
        ("a*(x^2-2)", vec!["-Sqrt[2]", "Sqrt[2]"]),
        ("(a+b)*(x^3-6*x^2+11*x-6)", vec!["1", "2", "3"]),
        (
            "(a+b)*(x^2+c*x+d)",
            vec!["(-c-Sqrt[c^2-4*d])/2", "(-c+Sqrt[c^2-4*d])/2"],
        ),
    ] {
        let got = roots(input);
        let actual = values(&got.set);
        assert_eq!(actual.len(), expected.len());
        assert!(
            expected.into_iter().all(|v| actual.contains(&(e(v), 1))),
            "{input}: {actual:?}"
        );
        assert_eq!(got.assumptions.len(), 1);
    }
}
#[test]
fn coefficient_denominators_and_composite_heads_preserve_dependency() {
    let got = roots("x/a+b");
    assert_eq!(values(&got.set), [(e("-a*b"), 1)]);
    assert!(got.assumptions.contains(&e("a!=0")));
    // A variable hidden in a function head still prevents polynomial classification.
    let p = om_core::add([
        om_core::mul([Expr::normal(e("f[x]"), vec![e("a")]), e("x")]),
        Expr::int(1),
    ]);
    let got = poly_uni(
        &p,
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Unevaluated));
    let p = e("I*x-1");
    assert_eq!(
        values(
            &poly_uni(
                &p,
                &e("x"),
                &SolveOptions::default(),
                &Interrupt::default(),
                &mut NoSteps
            )
            .unwrap()
            .set
        ),
        [(e("-I"), 1)]
    );
}
#[test]
fn options_defaults_and_solution_set_wolfram_shapes() {
    let opts = SolveOptions::default();
    assert_eq!(opts.domain, Domain::Complexes);
    assert!(!opts.cubics && !opts.quartics);
    assert!(opts.record_steps && opts.inverse_functions);
    assert!(matches!(opts.verify, VerifyMode::Auto));
    assert!(matches!(opts.max_extra_conditions, MaxExtra::Zero));
    assert_eq!(opts.generated_parameter, om_core::BUILTIN::C);
    assert_eq!(opts.seed, 0x0A5E_ED00_0000_0001);
    assert_eq!(SolutionSet::All.to_expr(), e("{{}}"));
    assert_eq!(SolutionSet::Finite(vec![]).to_expr(), e("{}"));
    let s = Solution {
        rules: vec![(e("x"), e("C[1]"))],
        condition: Some(e("a!=0")),
        constants: vec![(e("C[1]"), Domain::Integers)],
        multiplicity: 1,
        verification: Verification::ByConstruction,
        numeric: None,
    };
    assert_eq!(
        SolutionSet::Finite(vec![s]).to_expr(),
        e("{{x->ConditionalExpression[C[1],a!=0&&Element[C[1],Integers]]}}")
    );
    let region = SolutionSet::Region {
        cond: e("x>1"),
        intervals: vec![Interval {
            lo: Bound::Open(e("1")),
            hi: Bound::PosInf,
        }],
    };
    assert_eq!(region.to_expr(), e("x>1"));
    assert_eq!(SolutionSet::Unevaluated.to_expr(), e("Unevaluated"));
}
#[test]
fn computation_emits_square_free_factor_and_formula_events() {
    let ctx = Interrupt::default();
    let mut sink = StepRecorder::new();
    let got = poly_uni(
        &e("x^2+2*x-3"),
        &e("x"),
        &SolveOptions::default(),
        &ctx,
        &mut sink,
    )
    .unwrap();
    assert_eq!(values(&got.set), [(e("-3"), 1), (e("1"), 1)]);
    let steps = sink.finish();
    assert!(
        steps
            .root
            .iter()
            .any(|s| matches!(s.kind, StepKind::SquareFree { .. }))
    );
    assert!(
        steps
            .root
            .iter()
            .any(|s| matches!(s.kind, StepKind::Factor { .. }))
    );
    assert!(steps.root.iter().any(|s| s.rule_id == "zero_product"));
    fn walk(s: &Step, out: &mut Vec<String>) {
        out.push(format!("{} {}", s.id, s.rule_id));
        for child in &s.children {
            walk(child, out);
        }
    }
    let mut snapshot = vec![];
    for s in &steps.root {
        walk(s, &mut snapshot);
    }
    insta::assert_snapshot!(snapshot.join("\n"),@r"
S1 square_free
S2 factor
S3 zero_product
S4 branch
S4.1 split_component
S4.2 linear_formula
S5 branch
S5.1 split_component
S5.2 linear_formula
");
}
#[test]
fn factor_events_reconstruct_signed_content_and_fractional_inputs() {
    for input in ["-7*(x-1)^3*(x^2+1)^2/15", "(a+b)*(x^2-2)", "a*x^4*(x-1)^2"] {
        let mut sink = StepRecorder::new();
        poly_uni(
            &e(input),
            &e("x"),
            &SolveOptions::default(),
            &Interrupt::default(),
            &mut sink,
        )
        .unwrap();
        for step in sink.finish().root {
            if matches!(step.kind, StepKind::Factor { .. }) {
                assert_eq!(
                    om_simplify::zero::is_zero(&om_core::sub(
                        step.before[0].clone(),
                        step.after[0].clone()
                    )),
                    om_simplify::zero::Tri::Zero,
                    "{input}"
                );
            }
        }
    }
}
#[test]
fn conditional_limit_attaches_complete_assumption_conjunctions() {
    for (limit, show) in [
        (MaxExtra::Zero, false),
        (MaxExtra::Count(0), false),
        (MaxExtra::Count(1), true),
        (MaxExtra::All, true),
    ] {
        let opts = SolveOptions {
            max_extra_conditions: limit,
            ..SolveOptions::default()
        };
        let got = poly_uni(
            &e("a*x+b"),
            &e("x"),
            &opts,
            &Interrupt::default(),
            &mut NoSteps,
        )
        .unwrap();
        let SolutionSet::Finite(rows) = got.set else {
            panic!("finite result")
        };
        assert_eq!(rows[0].condition, show.then(|| e("a!=0")));
    }
}
#[test]
fn checked_polynomial_work_rejects_limits_and_propagates_abort() {
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        poly_uni(
            &e("x^2-2"),
            &e("x"),
            &SolveOptions::default(),
            &ctx,
            &mut NoSteps
        ),
        Err(SolveError::Abort(Abort::Budget))
    ));
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        poly_uni(
            &e("0"),
            &e("x"),
            &SolveOptions::default(),
            &ctx,
            &mut NoSteps
        ),
        Err(SolveError::Abort(Abort::Interrupted))
    ));
    assert!(matches!(
        poly_uni(
            &e("x^1000000-1"),
            &e("x"),
            &SolveOptions::default(),
            &Interrupt::default(),
            &mut NoSteps
        ),
        Err(SolveError::Unsupported(_))
    ));
}
#[test]
fn recorded_denominator_clearing_propagates_budget_without_panicking() {
    let p = e("x^2/3-2/3");
    let x = e("x");
    for budget in 0..650 {
        let ctx = Interrupt::default();
        ctx.steps_left.set(budget);
        let result = poly_uni(
            &p,
            &x,
            &SolveOptions::default(),
            &ctx,
            &mut StepRecorder::new(),
        );
        assert!(
            result.is_ok() || matches!(result, Err(SolveError::Abort(Abort::Budget))),
            "budget {budget}: {result:?}"
        );
    }
}
proptest! {
    #![proptest_config(ProptestConfig{cases:64,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e33),..ProptestConfig::default()})]
    #[test]
    fn planted_gaussian_roots_and_repeated_factors_are_recovered(a in -5i64..=5,b in 1i64..=4,c in -5i64..=5,m in 1u32..=3,n in 1u32..=3) {
        let p=e(&format!("(x-({c}))^{m}*((x-({a}))^2+({b})^2)^{n}"));
        let got=poly_uni(&p,&e("x"),&SolveOptions::default(),&Interrupt::default(),&mut NoSteps).unwrap();
        prop_assert_eq!(values(&got.set),vec![(Expr::int(c),m),(e(&format!("({a})-({b})*I")),n),(e(&format!("({a})+({b})*I")),n)]);
        prop_assert_eq!(got.set.to_expr().args().len(),(m+2*n) as usize);
        for (value,_) in values(&got.set) {prop_assert!(p.replace_all(&[(e("x"),value)]).is_zero());}
    }
}
