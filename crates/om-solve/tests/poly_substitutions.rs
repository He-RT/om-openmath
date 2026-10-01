//! Radical reductions of binomials, power-compositions and reciprocal polynomials.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_num::{
    BigFloat,
    ctx::{Abort, Interrupt},
};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    NoSteps, SolutionSet, SolveError, SolveOptions, StepKind, StepRecorder, univariate::poly_uni,
};
use proptest::prelude::*;
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
fn solve(s: &str) -> Vec<(Expr, u32)> {
    let got = poly_uni(
        &e(s),
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(roots) = got.set else {
        panic!("finite roots expected: {s}")
    };
    roots
        .into_iter()
        .map(|r| (r.rules[0].1.clone(), r.multiplicity))
        .collect()
}
fn certify(input: &str, roots: &[(Expr, u32)], degree: usize) {
    assert_eq!(
        roots.iter().map(|(_, m)| *m as usize).sum::<usize>(),
        degree,
        "{input}"
    );
    for (value, _) in roots {
        let residual = e(input).replace_all(&[(e("x"), value.clone())]);
        let z = om_simplify::numeval::enclose(&residual, 768, &Interrupt::default())
            .unwrap()
            .unwrap();
        let bound = BigFloat::from_parts(1.into(), -400);
        for b in [z.re, z.im] {
            let abs = if b.mid < BigFloat::ZERO {
                -b.mid
            } else {
                b.mid
            };
            assert!(abs + b.rad < bound, "{input}: {value:?}");
        }
    }
}
#[test]
fn binomial_authority_values_and_complex_principal_branches() {
    let got = solve("x^3-2");
    let expected = [
        e("2^(1/3)"),
        e("-(-1)^(1/3)*2^(1/3)"),
        e("(-1)^(2/3)*2^(1/3)"),
    ];
    assert_eq!(got.len(), 3);
    assert!(expected.into_iter().all(|v| got.contains(&(v, 1))));
    assert_eq!(got[0], (e("2^(1/3)"), 1));
    for input in ["x^3-2", "x^3+2", "x^4-2", "x^5-2", "3*x^7+5"] {
        certify(
            input,
            &solve(input),
            if input.contains('7') {
                7
            } else if input.contains('5') {
                5
            } else if input.contains('4') {
                4
            } else {
                3
            },
        );
    }
    assert_eq!(
        solve("x^4-2"),
        vec![
            (e("-2^(1/4)"), 1),
            (e("2^(1/4)"), 1),
            (e("-I*2^(1/4)"), 1),
            (e("I*2^(1/4)"), 1)
        ]
    );
}
#[test]
fn maximal_power_substitution_and_nested_reductions() {
    assert_eq!(
        solve("x^4-5*x^2+4"),
        vec![(e("-2"), 1), (e("-1"), 1), (e("1"), 1), (e("2"), 1)]
    );
    for (input, degree) in [("x^6+x^3-1", 6), ("x^8-3*x^4+1", 8), ("x^6+x^3+1", 6)] {
        certify(input, &solve(input), degree);
    }
    certify("(x^6+x^3-1)^2", &solve("(x^6+x^3-1)^2"), 12);
}
#[test]
fn reciprocal_substitution_and_double_inverse_roots() {
    for (input, degree) in [("x^4-3*x^3+3*x^2-3*x+1", 4), ("x^4+x^3+x^2+x+1", 4)] {
        certify(input, &solve(input), degree);
    }
    let got = solve("a*(x-1)^4");
    assert_eq!(got, [(e("1"), 4)]);
    let got = poly_uni(
        &e("a*x^4+b*x^3+c*x^2+b*x+a"),
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert_eq!(got.assumptions, [e("a!=0")]);
    let SolutionSet::Finite(rows) = got.set else {
        panic!("symbolic palindrome")
    };
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r.multiplicity == 1));
}
#[test]
fn symbolic_binomial_content_zero_powers_and_unsupported_components() {
    let got = poly_uni(
        &e("a*x^5+b"),
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert_eq!(got.assumptions, [e("a!=0")]);
    let SolutionSet::Finite(rows) = got.set else {
        panic!("symbolic binomial")
    };
    assert_eq!(rows.len(), 5);
    certify(
        "2*x^5-3",
        &rows
            .into_iter()
            .map(|r| {
                (
                    r.rules[0]
                        .1
                        .replace_all(&[(e("a"), e("2")), (e("b"), e("-3"))]),
                    r.multiplicity,
                )
            })
            .collect::<Vec<_>>(),
        5,
    );
    assert_eq!(solve("a*x^7"), [(e("0"), 7)]);
    // M9.5 covers those general polynomial cases with Root fallback.
    for input in ["Sin[x]+1", "1/x-1", "(x-1)*(Sin[x]+1)"] {
        let got = poly_uni(
            &e(input),
            &e("x"),
            &SolveOptions::default(),
            &Interrupt::default(),
            &mut NoSteps,
        )
        .unwrap();
        assert!(matches!(got.set, SolutionSet::Unevaluated), "{input}");
    }
}
#[test]
fn substitution_and_formula_steps_are_emitted_during_computation() {
    for (input, wanted) in [
        ("x^3-2", "binomial_formula"),
        ("x^6+x^3-1", "substitute"),
        ("x^4-3*x^3+3*x^2-3*x+1", "palindromic_formula"),
    ] {
        let mut sink = StepRecorder::new();
        poly_uni(
            &e(input),
            &e("x"),
            &SolveOptions::default(),
            &Interrupt::default(),
            &mut sink,
        )
        .unwrap();
        fn walk(steps: &[om_solve::Step], ids: &mut Vec<&'static str>) {
            for s in steps {
                ids.push(s.rule_id);
                walk(&s.children, ids);
            }
        }
        let mut ids = vec![];
        walk(&sink.finish().root, &mut ids);
        assert!(ids.contains(&wanted), "{input}: {ids:?}");
    }
    // Auxiliary variables cannot capture an independent user coefficient.
    let mut sink = StepRecorder::new();
    poly_uni(
        &e("x^6+OmSolve$1*x^3+1"),
        &e("x"),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    fn check(steps: &[om_solve::Step]) {
        for s in steps {
            if let StepKind::Substitute { new_var, .. } = &s.kind {
                assert_ne!(new_var, &e("OmSolve$1"));
            }
            check(&s.children);
        }
    }
    check(&sink.finish().root);
}
#[test]
fn reductions_propagate_checked_resource_exhaustion() {
    let ctx = Interrupt::default();
    ctx.steps_left.set(100);
    assert!(matches!(
        poly_uni(
            &e("x^7-2"),
            &e("x"),
            &SolveOptions::default(),
            &ctx,
            &mut StepRecorder::new()
        ),
        Err(SolveError::Abort(Abort::Budget))
    ));
}
#[test]
fn mixed_factors_order_equal_real_projections_by_exact_imaginary_parts() {
    let got = solve("(x^4-2)*(x^4-8)");
    let expected = [
        "-8^(1/4)",
        "-2^(1/4)",
        "2^(1/4)",
        "8^(1/4)",
        "-I*8^(1/4)",
        "-I*2^(1/4)",
        "I*2^(1/4)",
        "I*8^(1/4)",
    ];
    assert_eq!(
        got,
        expected.into_iter().map(|v| (e(v), 1)).collect::<Vec<_>>()
    );
}
proptest! {
    #![proptest_config(ProptestConfig{cases:16,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e34),..ProptestConfig::default()})]
    #[test]
    fn planted_positive_binomials_retain_all_roots_and_multiplicities(n in 3usize..=6,c in 2i64..=7,m in 1usize..=2) {
        let input=format!("(x^{n}-{c})^{m}");
        let roots=solve(&input);
        prop_assert_eq!(roots.len(),n);
        prop_assert!(roots.iter().all(|(_,mult)|*mult as usize==m));
        certify(&input,&roots,n*m);
        prop_assert!(!roots.iter().any(|(v,_)|v.is_head(B::ROOT)));
    }
}
