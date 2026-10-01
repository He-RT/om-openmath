//! General exact Root, Cardano and Ferrari paths, with independent directed checks.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_num::{
    BigFloat,
    ctx::{Abort, Interrupt},
};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    MaxExtra, NoSteps, Solution, SolutionSet, SolveError, SolveOptions, StepKind, StepRecorder,
    univariate::poly_uni,
};
use proptest::prelude::*;
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
fn solve(s: &str, opts: &SolveOptions) -> Vec<Solution> {
    let got = poly_uni(&e(s), &e("x"), opts, &Interrupt::default(), &mut NoSteps).unwrap();
    let SolutionSet::Finite(roots) = got.set else {
        panic!("finite roots: {s}")
    };
    roots
}
fn contains_root(e: &Expr) -> bool {
    e.is_head(B::ROOT) || e.args().iter().any(contains_root)
}
fn certify(p: &str, roots: &[Solution], n: usize) {
    assert_eq!(
        roots.iter().map(|r| r.multiplicity as usize).sum::<usize>(),
        n,
        "{p}"
    );
    for root in roots {
        let value = &root.rules[0].1;
        let z = om_simplify::numeval::enclose(
            &e(p).replace_all(&[(e("x"), value.clone())]),
            1024,
            &Interrupt::default(),
        )
        .unwrap()
        .unwrap();
        let limit = BigFloat::from_parts(1.into(), -700);
        for b in [z.re, z.im] {
            let m = if b.mid < BigFloat::ZERO {
                -b.mid
            } else {
                b.mid
            };
            assert!(m + b.rad < limit, "{p}: {value:?}");
        }
    }
}
#[test]
fn default_root_objects_have_exact_source_polynomial_and_one_based_indices() {
    for (p, poly, n) in [
        ("x^3-3*x+1", "1-3*#1+#1^3", 3),
        ("x^4+x+1", "1+#1+#1^4", 4),
        ("x^5-x+1", "1-#1+#1^5", 5),
    ] {
        let roots = solve(p, &SolveOptions::default());
        assert_eq!(roots.len(), n);
        let expected = (1..=n)
            .map(|k| e(&format!("Root[({poly})&, {k}]")))
            .collect::<Vec<_>>();
        assert!(roots.iter().all(|r| expected.contains(&r.rules[0].1)));
        certify(p, &roots, n);
    }
    let roots = solve("x^5-x+1", &SolveOptions::default());
    let first = om_simplify::numeval::enclose(&roots[0].rules[0].1, 128, &Interrupt::default())
        .unwrap()
        .unwrap();
    assert!((first.re.to_f64() + 1.167304).abs() < 1e-6);
}
#[test]
fn cardano_authority_cubics_and_zero_plus_branch_have_all_radical_roots() {
    let opts = SolveOptions {
        cubics: true,
        ..SolveOptions::default()
    };
    for p in ["x^3-3*x+1", "x^3+x+1", "(x+1)^3+2", "3*x^3+2*x^2-5*x+7"] {
        let roots = solve(p, &opts);
        assert!(roots.iter().all(|r| !contains_root(&r.rules[0].1)), "{p}");
        certify(p, &roots, 3);
    }
    let roots = solve("x^3-3*x+1", &opts);
    for (r, want) in roots.iter().zip([-1.879385, 0.347296, 1.532089]) {
        let z = om_simplify::numeval::enclose(&r.rules[0].1, 128, &Interrupt::default())
            .unwrap()
            .unwrap();
        assert!((z.re.to_f64() - want).abs() < 1e-6);
    }
}
#[test]
fn ferrari_resolvent_cubic_and_depressed_biquadratic_are_branch_complete() {
    let opts = SolveOptions {
        quartics: true,
        ..SolveOptions::default()
    };
    for p in [
        "x^4+x+1",
        "x^4-5*x^2+x+1",
        "2*x^4+3*x^3-5*x^2+7*x-11",
        "(x+1)^4-3*(x+1)^2+1",
    ] {
        let roots = solve(p, &opts);
        assert!(roots.iter().all(|r| !contains_root(&r.rules[0].1)), "{p}");
        certify(p, &roots, 4);
    }
}
#[test]
fn repeated_general_factors_preserve_yun_and_mixed_factor_multiplicities() {
    let roots = solve("(x^5-x+1)^2*(x-2)^3", &SolveOptions::default());
    assert_eq!(roots.len(), 6);
    assert!(
        roots
            .iter()
            .any(|r| r.rules[0].1 == e("2") && r.multiplicity == 3)
    );
    assert!(
        roots
            .iter()
            .filter(|r| contains_root(&r.rules[0].1))
            .all(|r| r.multiplicity == 2)
    );
    certify("(x^5-x+1)^2*(x-2)^3", &roots, 13);
    let opts = SolveOptions {
        cubics: true,
        quartics: true,
        ..SolveOptions::default()
    };
    let roots = solve("(x^3+x+1)^2*(x^4+x+1)^3", &opts);
    assert_eq!(roots.len(), 7);
    certify("(x^3+x+1)^2*(x^4+x+1)^3", &roots, 18);
}
#[test]
fn parameter_roots_keep_original_leading_assumptions_and_zero_valuation() {
    let opts = SolveOptions {
        cubics: true,
        quartics: true,
        max_extra_conditions: MaxExtra::All,
        ..SolveOptions::default()
    };
    let roots = solve("x^2*(a*x^3+x+1)", &opts);
    assert_eq!(roots.len(), 4);
    assert!(
        roots
            .iter()
            .any(|r| r.rules[0].1.is_zero() && r.multiplicity == 2)
    );
    for root in roots.iter().filter(|r| contains_root(&r.rules[0].1)) {
        assert_eq!(root.condition, Some(e("a!=0")));
        assert!(root.rules[0].1.is_head(B::ROOT));
        let function = &root.rules[0].1.args()[0];
        assert_eq!(function, &e("(1+#1+a*#1^3)&"));
    }
}
#[test]
fn power_composition_uses_root_of_original_polynomial_until_reduced_roots_are_radical() {
    let default = solve("x^9+x^3+1", &SolveOptions::default());
    assert!(default.iter().all(|r| r.rules[0].1.is_head(B::ROOT)));
    assert_eq!(default.len(), 9);
    let opts = SolveOptions {
        cubics: true,
        ..SolveOptions::default()
    };
    let radicals = solve("x^9+x^3+1", &opts);
    assert!(radicals.iter().all(|r| !contains_root(&r.rules[0].1)));
    certify("x^9+x^3+1", &radicals, 9);
}
#[test]
fn root_and_general_formula_steps_are_actual_and_disabled_options_stay_empty() {
    for (p, opts, want) in [
        ("x^5-x+1", SolveOptions::default(), "root_objects"),
        (
            "x^3-3*x+1",
            SolveOptions {
                cubics: true,
                ..SolveOptions::default()
            },
            "cardano_formula",
        ),
        (
            "x^4+x+1",
            SolveOptions {
                quartics: true,
                ..SolveOptions::default()
            },
            "ferrari_formula",
        ),
    ] {
        let mut recorder = StepRecorder::new();
        poly_uni(&e(p), &e("x"), &opts, &Interrupt::default(), &mut recorder).unwrap();
        fn scan(steps: &[om_solve::Step], wanted: &str) -> bool {
            steps
                .iter()
                .any(|s| s.rule_id == wanted || scan(&s.children, wanted))
        }
        let steps = recorder.finish();
        assert!(scan(&steps.root, want));
        fn root_count(steps: &[om_solve::Step]) {
            for s in steps {
                if let StepKind::RootObjects { real_count, .. } = &s.kind {
                    assert_eq!(*real_count, 1);
                }
                root_count(&s.children);
            }
        }
        if p == "x^5-x+1" {
            root_count(&steps.root);
        }
        let mut recorder = StepRecorder::new();
        let opts = SolveOptions {
            record_steps: false,
            ..opts
        };
        poly_uni(&e(p), &e("x"), &opts, &Interrupt::default(), &mut recorder).unwrap();
        assert!(recorder.finish().root.is_empty());
    }
}
#[test]
fn general_paths_propagate_abort_and_refuse_nonpolynomial_input() {
    let opts = SolveOptions {
        cubics: true,
        quartics: true,
        ..SolveOptions::default()
    };
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        poly_uni(&e("x^5-x+1"), &e("x"), &opts, &ctx, &mut NoSteps),
        Err(SolveError::Abort(Abort::Budget))
    ));
    let got = poly_uni(
        &e("(x-1)*(Sin[x]+1)"),
        &e("x"),
        &opts,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Unevaluated));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:12,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e35),..ProptestConfig::default()})]
 #[test]
 fn generated_small_quartics_have_four_certified_radical_roots(a in 1i64..=3,b in -3i64..=3,c in -3i64..=3,d in -3i64..=3,e0 in -3i64..=3) {
  let p=format!("{a}*x^4+({b})*x^3+({c})*x^2+({d})*x+({e0})");
  let opts=SolveOptions{cubics:true,quartics:true,..SolveOptions::default()};
  let roots=solve(&p,&opts);
  prop_assert!(roots.iter().all(|r|!contains_root(&r.rules[0].1)));
  certify(&p,&roots,4);
 }
}
