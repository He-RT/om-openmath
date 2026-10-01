//! Numeric APIs retain complete roots, precision, source validation and injected aborts.
use om_core::{Expr, canonicalize};
use om_num::{
    BigFloat, Complex, Number, Precision, Real,
    ctx::{Abort, Interrupt},
};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    FindRootOptions, SolutionSet, SolveError, Verification, eliminate, find_root, nsolve,
};
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn finite(set: SolutionSet) -> Vec<om_solve::Solution> {
    let SolutionSet::Finite(v) = set else {
        panic!("finite numeric roots")
    };
    v
}
fn residual(source: &str, rules: &[(Expr, Expr)], bits: u32) -> om_num::CBall {
    om_simplify::numeval::enclose(&e(source).replace_all(rules), bits, &Interrupt::default())
        .unwrap()
        .unwrap()
}
fn small(source: &str, rules: &[(Expr, Expr)], bits: u32, goal: i64) {
    let z = residual(source, rules, bits);
    let limit = BigFloat::from_parts(1.into(), -goal as isize);
    assert!(
        [z.re, z.im].iter().all(|b| (if b.mid < BigFloat::ZERO {
            -&b.mid
        } else {
            b.mid.clone()
        }) + &b.rad
            < limit),
        "residual {source}"
    );
}
#[test]
fn nsolve_quintic_machine_and_arbitrary_precision_roots_are_complete() {
    for precision in [Precision::Machine, Precision::Bits(256)] {
        let got = nsolve(
            &raw("x^5-x+1==0"),
            &[e("x")],
            precision,
            &Interrupt::default(),
        )
        .unwrap();
        let roots = finite(got.set);
        assert_eq!(roots.len(), 5);
        for root in &roots {
            assert!(matches!(root.verification, Verification::Numeric { .. }));
            assert!(!root.rules[0].1.as_number().unwrap().is_exact());
            small(
                "x^5-x+1",
                &root.rules,
                512,
                if precision == Precision::Machine {
                    40
                } else {
                    235
                },
            );
        }
    }
}
#[test]
fn nsolve_systems_multiplicities_and_original_poles_survive() {
    let roots = finite(
        nsolve(
            &raw("{x^2+y^2==5,x y==2}"),
            &[e("x"), e("y")],
            Precision::Bits(128),
            &Interrupt::default(),
        )
        .unwrap()
        .set,
    );
    assert_eq!(roots.len(), 4);
    for root in &roots {
        small("x^2+y^2-5", &root.rules, 256, 110);
        small("x y-2", &root.rules, 256, 110);
    }
    let roots = finite(
        nsolve(
            &raw("(x-1)^3==0"),
            &[e("x")],
            Precision::Machine,
            &Interrupt::default(),
        )
        .unwrap()
        .set,
    );
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].multiplicity, 3);
    let roots = finite(
        nsolve(
            &raw("(x^2-1)/(x-1)==0"),
            &[e("x")],
            Precision::Machine,
            &Interrupt::default(),
        )
        .unwrap()
        .set,
    );
    assert_eq!(roots.len(), 1);
    assert_eq!(
        roots[0].rules[0].1.as_number().unwrap().to_f64(),
        Some(-1.0)
    );
}
#[test]
fn nsolve_rejects_incomplete_families_and_invalid_precision() {
    let got = nsolve(
        &raw("Exp[x]==2"),
        &[e("x")],
        Precision::Machine,
        &Interrupt::default(),
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Unevaluated));
    assert!(matches!(
        nsolve(
            &raw("x^2==2"),
            &[e("x")],
            Precision::Exact,
            &Interrupt::default()
        ),
        Err(SolveError::Invalid(_))
    ));
}
fn local(
    source: &str,
    starts: &[(&str, Number)],
    opts: &FindRootOptions,
) -> om_solve::SolveOutcome {
    find_root(
        &raw(source),
        &starts
            .iter()
            .map(|(v, n)| (e(v), n.clone()))
            .collect::<Vec<_>>(),
        opts,
        &Interrupt::default(),
    )
    .unwrap()
}
fn number(v: f64) -> Number {
    Number::Real(Real::Machine(v))
}
#[test]
fn damped_newton_scalar_transcendental_system_and_complex_roots_converge() {
    let opts = FindRootOptions::default();
    let roots = finite(local("x^3-2x+2==0", &[("x", number(-2.0))], &opts).set);
    small("x^3-2x+2", &roots[0].rules, 256, 36);
    let roots = finite(local("Cos[x]==x", &[("x", number(1.0))], &opts).set);
    small("Cos[x]-x", &roots[0].rules, 256, 36);
    let roots = finite(
        local(
            "{x^2+y^2==5,x y==2}",
            &[("x", number(2.2)), ("y", number(0.8))],
            &opts,
        )
        .set,
    );
    small("x^2+y^2-5", &roots[0].rules, 256, 36);
    small("x y-2", &roots[0].rules, 256, 36);
    let start = Number::Complex(Box::new(Complex {
        re: number(0.5),
        im: number(1.0),
    }));
    let roots = finite(local("x^2+1==0", &[("x", start)], &opts).set);
    small("x^2+1", &roots[0].rules, 256, 36);
}
#[test]
fn arbitrary_precision_newton_and_numeric_jacobian_fallback_are_honest() {
    let opts = FindRootOptions {
        precision: Precision::Bits(256),
        ..FindRootOptions::default()
    };
    let roots = finite(local("x^2==2", &[("x", number(1.0))], &opts).set);
    small("x^2-2", &roots[0].rules, 512, 230);
    let roots = finite(
        local(
            "Abs[x]==2",
            &[("x", number(-3.0))],
            &FindRootOptions::default(),
        )
        .set,
    );
    assert!(roots[0].rules[0].1.as_number().unwrap().to_f64().unwrap() < 0.0);
    small("Abs[x]-2", &roots[0].rules, 256, 36);
}
#[test]
fn bracketed_brent_roots_endpoints_and_invalid_brackets_are_checked() {
    let opts = FindRootOptions {
        bracket: Some((number(0.0), number(2.0))),
        ..FindRootOptions::default()
    };
    let roots = finite(local("x^3==2", &[("x", number(1.0))], &opts).set);
    small("x^3-2", &roots[0].rules, 256, 36);
    let roots = finite(local("x==0", &[("x", number(1.0))], &opts).set);
    assert_eq!(roots[0].rules[0].1.as_number().unwrap().to_f64(), Some(0.0));
    assert!(matches!(
        local("x^2+1==0", &[("x", number(1.0))], &opts).set,
        SolutionSet::Unevaluated
    ));
    let invalid = FindRootOptions {
        bracket: Some((number(2.0), number(0.0))),
        ..opts
    };
    assert!(matches!(
        find_root(
            &raw("x^2==2"),
            &[(e("x"), number(1.0))],
            &invalid,
            &Interrupt::default()
        ),
        Err(SolveError::Invalid(_))
    ));
}
#[test]
fn local_failures_poles_recording_and_budget_contracts_are_atomic() {
    let opts = FindRootOptions {
        max_iterations: 1,
        ..FindRootOptions::default()
    };
    assert!(matches!(
        local("Exp[x]==100", &[("x", number(0.0))], &opts).set,
        SolutionSet::Unevaluated
    ));
    assert!(matches!(
        local("x/x==0", &[("x", number(0.0))], &FindRootOptions::default()).set,
        SolutionSet::Unevaluated
    ));
    let opts = FindRootOptions {
        record_steps: false,
        ..FindRootOptions::default()
    };
    assert!(
        local("x^2==2", &[("x", number(1.0))], &opts)
            .steps
            .is_none()
    );
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        nsolve(&raw("x^2==2"), &[e("x")], Precision::Machine, &ctx),
        Err(SolveError::Abort(Abort::Budget))
    ));
    assert!(matches!(
        find_root(&raw("x^2==2"), &[(e("x"), number(1.0))], &opts, &ctx),
        Err(SolveError::Abort(Abort::Budget))
    ));
    assert!(matches!(
        eliminate(&raw("x==y"), &[e("x")], &ctx),
        Err(SolveError::Abort(Abort::Budget))
    ));
}
#[test]
fn lex_elimination_returns_exact_surviving_relations() {
    assert_eq!(
        eliminate(&raw("{x==y+1,y==2z}"), &[e("y")], &Interrupt::default()).unwrap(),
        e("x==1+2z")
    );
    let got = eliminate(&raw("{x^2+y^2==1,y==x}"), &[e("y")], &Interrupt::default()).unwrap();
    assert!(got.free_of(&e("y")));
    assert!(got.is_head(om_core::BUILTIN::EQUAL));
    let r = om_core::sub(got.args()[0].clone(), got.args()[1].clone());
    assert_eq!(
        om_simplify::zero::is_zero(&r.replace_all(&[(e("x"), e("1/Sqrt[2]"))])),
        om_simplify::zero::Tri::Zero
    );
    assert_eq!(
        eliminate(&raw("{x==1,x==2}"), &[e("x")], &Interrupt::default()).unwrap(),
        e("False")
    );
    assert_eq!(
        eliminate(&raw("x==y"), &[e("x")], &Interrupt::default()).unwrap(),
        e("True")
    );
    assert!(matches!(
        eliminate(&raw("Sin[x]==y"), &[e("x")], &Interrupt::default()),
        Err(SolveError::Unsupported(_))
    ));
}

#[test]
fn rounded_numeric_poles_inexact_coefficients_and_invalid_starts_are_checked() {
    let got = nsolve(
        &raw("(x-(1+1/2^100))/(x-1)==0"),
        &[e("x")],
        Precision::Machine,
        &Interrupt::default(),
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Unevaluated));
    let roots = finite(
        nsolve(
            &raw("x^2==2.0"),
            &[e("x")],
            Precision::Machine,
            &Interrupt::default(),
        )
        .unwrap()
        .set,
    );
    assert_eq!(roots.len(), 2);
    for root in roots {
        small("x^2-2", &root.rules, 256, 40);
    }
    assert!(matches!(
        find_root(
            &raw("x==1"),
            &[(e("x"), number(f64::NAN))],
            &FindRootOptions::default(),
            &Interrupt::default()
        ),
        Err(SolveError::Invalid(_))
    ));
    assert!(matches!(
        find_root(
            &raw("{x==1,x==1}"),
            &[(e("x"), number(0.0)), (e("x"), number(0.0))],
            &FindRootOptions::default(),
            &Interrupt::default()
        ),
        Err(SolveError::Invalid(_))
    ));
}

#[test]
fn high_precision_brent_and_original_poles_are_certified() {
    let opts = FindRootOptions {
        precision: Precision::Bits(256),
        bracket: Some((number(0.0), number(2.0))),
        ..FindRootOptions::default()
    };
    let roots = finite(local("x^3==2", &[("x", number(1.0))], &opts).set);
    small("x^3-2", &roots[0].rules, 512, 230);
    let opts = FindRootOptions {
        bracket: Some((number(-1.0), number(1.0))),
        ..FindRootOptions::default()
    };
    assert!(matches!(
        local("1/x==0", &[("x", number(0.5))], &opts).set,
        SolutionSet::Unevaluated
    ));
    let result = eliminate(&raw("x/y==1"), &[e("x")], &Interrupt::default()).unwrap();
    assert_eq!(result, e("y!=0"));
    assert!(matches!(
        eliminate(&raw("1/x==y"), &[e("x")], &Interrupt::default()),
        Err(SolveError::Unsupported(_))
    ));
}

#[test]
fn numeric_steps_include_the_rounded_verified_assignments_and_honest_evidence() {
    fn flatten(v: &[om_solve::Step]) -> Vec<&om_solve::Step> {
        v.iter()
            .flat_map(|s| std::iter::once(s).chain(flatten(&s.children)))
            .collect()
    }
    let result = local("x^2==2", &[("x", number(1.0))], &FindRootOptions::default());
    let roots = finite(result.set);
    assert!(flatten(&result.steps.unwrap().root).iter().any(|s|matches!(&s.kind,om_solve::StepKind::Verify {candidate,outcome:om_simplify::zero::Tri::Unknown(om_simplify::zero::UnknownReason::ProbablyZero),..} if candidate==&roots[0].rules)));
    let result = nsolve(
        &raw("x^2==2"),
        &[e("x")],
        Precision::Machine,
        &Interrupt::default(),
    )
    .unwrap();
    let roots = finite(result.set);
    let steps = result.steps.unwrap();
    for root in roots {
        assert!(flatten(&steps.root).iter().any(|s|matches!(&s.kind,om_solve::StepKind::Verify {candidate,outcome:om_simplify::zero::Tri::Unknown(om_simplify::zero::UnknownReason::ProbablyZero),..} if candidate==&root.rules)));
    }
}

#[test]
fn local_convergence_is_invariant_under_small_equation_scaling() {
    let roots = finite(
        local(
            "(x-1)/10^100==0",
            &[("x", number(0.0))],
            &FindRootOptions::default(),
        )
        .set,
    );
    assert_eq!(roots[0].rules[0].1.as_number().unwrap().to_f64(), Some(1.0));
    let roots = finite(
        local(
            "(Cos[x]-x)/10^100==0",
            &[("x", number(1.0))],
            &FindRootOptions::default(),
        )
        .set,
    );
    small("Cos[x]-x", &roots[0].rules, 256, 36);
    let opts = FindRootOptions {
        max_iterations: 1,
        ..FindRootOptions::default()
    };
    assert!(matches!(
        local("(x-1)^20==0", &[("x", number(1.1))], &opts).set,
        SolutionSet::Unevaluated
    ));
    let opts = FindRootOptions {
        max_iterations: 1,
        bracket: Some((number(0.99), number(1.2))),
        ..FindRootOptions::default()
    };
    assert!(matches!(
        local("(x-1)^21==0", &[("x", number(1.1))], &opts).set,
        SolutionSet::Unevaluated
    ));
}
