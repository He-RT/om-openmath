//! Affine systems use original free axes, generic pivots and actual row certificates.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    NoSteps, SolutionSet, SolveOptions, Step, StepKind, StepRecorder, Verification, linear_system,
};
use proptest::prelude::*;
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn run(s: &str, vars: &[&str]) -> om_solve::univariate::PolynomialRoots {
    linear_system(
        &raw(s),
        &vars.iter().map(|v| e(v)).collect::<Vec<_>>(),
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap()
}
fn rules(result: &om_solve::univariate::PolynomialRoots) -> Vec<(Expr, Expr)> {
    let SolutionSet::Finite(r) = &result.set else {
        panic!("finite {:?}", result.set)
    };
    assert_eq!(r.len(), 1);
    assert!(matches!(
        r[0].verification,
        Verification::Exact | Verification::ByConstruction
    ));
    r[0].rules.clone()
}
fn flatten(steps: &[Step]) -> Vec<&Step> {
    steps
        .iter()
        .flat_map(|s| std::iter::once(s).chain(flatten(&s.children)))
        .collect()
}
#[test]
fn authority_unique_underdetermined_inconsistent_and_parameter_systems() {
    assert_eq!(
        rules(&run("{x+y+z==6,2x-y+z==3,x+2y-z==2}", &["x", "y", "z"])),
        [(e("x"), e("1")), (e("y"), e("2")), (e("z"), e("3"))]
    );
    let got = run("{x+y+z==1,x-y==0}", &["x", "y", "z"]);
    let values = rules(&got);
    assert_eq!(
        values.iter().map(|(v, _)| v.clone()).collect::<Vec<_>>(),
        [e("x"), e("y")]
    );
    for (_, value) in values {
        assert_eq!(
            om_simplify::zero::is_zero(&om_core::sub(value, e("(1-z)/2"))),
            om_simplify::zero::Tri::Zero
        );
    }
    assert!(got.messages.iter().any(|m| m.tag == "svars"));
    assert!(matches!(run("{x+y==1,x+y==2}",&["x","y"]).set,SolutionSet::Finite(r)if r.is_empty()));
    let got = run("{a x+y==1,x-y==0}", &["x", "y"]);
    assert_eq!(
        rules(&got),
        [(e("x"), e("1/(1+a)")), (e("y"), e("1/(1+a)"))]
    );
    assert!(got.assumptions.contains(&e("1+a!=0")));
    assert_eq!(
        rules(&run("{y==1,x+y==3}", &["x", "y"])),
        [(e("x"), e("2")), (e("y"), e("1"))]
    );
}
#[test]
fn fractions_free_columns_original_poles_and_degenerate_equations() {
    assert_eq!(
        rules(&run("{x/2+y/3==1,x-y==0}", &["x", "y"])),
        [(e("x"), e("6/5")), (e("y"), e("6/5"))]
    );
    assert_eq!(rules(&run("y==2", &["x", "y"])), [(e("y"), e("2"))]);
    assert!(matches!(run("x==x", &["x"]).set, SolutionSet::All));
    assert!(matches!(run("x==x+1",&["x"]).set,SolutionSet::Finite(r)if r.is_empty()));
    assert!(
        matches!(run("((x-1)/(x-1))*(x-1)==0",&["x"]).set,SolutionSet::Finite(r)if r.is_empty())
    );
    let got = run("{x+y==0,x!=0}", &["x", "y"]);
    let SolutionSet::Finite(r) = got.set else {
        panic!("finite")
    };
    assert!(r[0].condition.is_some());
}
#[test]
fn conditions_generic_contradictions_and_disabled_recording_are_honest() {
    let got = linear_system(
        &raw("{a x+y==1,x-y==0}"),
        &[e("x"), e("y")],
        &SolveOptions {
            max_extra_conditions: om_solve::MaxExtra::All,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(r) = got.set else {
        panic!("finite")
    };
    assert!(r[0].condition.is_some());
    let got = run("{a x==1,a x==2}", &["x"]);
    assert!(!got.assumptions.is_empty());
    assert!(matches!(got.set,SolutionSet::Finite(r)if r.is_empty()));
    for p in ["x^2+y==1", "Sin[x]+y==1", "x==1||x==2"] {
        assert!(matches!(run(p, &["x", "y"]).set, SolutionSet::Unevaluated));
    }
    let mut sink = StepRecorder::new();
    linear_system(
        &raw("{x+y==3,2x-y==0}"),
        &[e("x"), e("y")],
        &SolveOptions {
            record_steps: false,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    assert!(sink.finish().root.is_empty());
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        linear_system(
            &raw("x==1"),
            &[e("x")],
            &SolveOptions::default(),
            &ctx,
            &mut NoSteps
        ),
        Err(om_solve::SolveError::Abort(Abort::Budget))
    ));
}
#[test]
fn row_steps_are_exact_operations_and_back_substitution_matches_the_result() {
    let mut sink = StepRecorder::new();
    let got = linear_system(
        &raw("{2x+y==1,x+3y==2}"),
        &[e("x"), e("y")],
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    assert_eq!(rules(&got), [(e("x"), e("1/5")), (e("y"), e("3/5"))]);
    let steps = sink.finish();
    let all = flatten(&steps.root);
    let operations = all
        .iter()
        .filter(|s| matches!(s.kind, StepKind::RowReduce { .. }))
        .collect::<Vec<_>>();
    assert!(!operations.is_empty());
    for step in operations {
        let StepKind::RowReduce { op, matrix } = &step.kind else {
            unreachable!()
        };
        let mut before = step.before[0]
            .args()
            .iter()
            .map(|r| r.args().to_vec())
            .collect::<Vec<_>>();
        match op {
            om_solve::RowOp::Swap { a, b } => before.swap(*a, *b),
            om_solve::RowOp::Scale { row, factor } => {
                for v in &mut before[*row] {
                    *v = om_core::mul([factor.clone(), v.clone()])
                }
            }
            om_solve::RowOp::Add {
                target,
                source,
                factor,
            } => {
                let from = before[*source].clone();
                for (v, f) in before[*target].iter_mut().zip(from) {
                    *v = om_core::add([v.clone(), om_core::mul([factor.clone(), f])])
                }
            }
        }
        for (a, b) in before.iter().flatten().zip(matrix.iter().flatten()) {
            assert_eq!(
                om_simplify::zero::is_zero(&om_core::sub(a.clone(), b.clone())),
                om_simplify::zero::Tri::Zero
            )
        }
    }
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::BackSubstitute { .. }))
    );
}

#[test]
fn explicit_real_domains_symbolic_denominators_and_empty_conditions_are_preserved() {
    assert!(
        matches!(run("{x==I,Element[x,Reals]}",&["x"]).set,SolutionSet::Finite(r)if r.is_empty())
    );
    let got = run("x/a==1", &["x"]);
    assert_eq!(rules(&got), [(e("x"), e("a"))]);
    let SolutionSet::Finite(r) = got.set else {
        panic!("finite")
    };
    assert!(r[0].condition.is_some());
    let got = run("x!=0", &["x"]);
    let SolutionSet::Finite(r) = got.set else {
        panic!("conditional all")
    };
    assert!(r[0].rules.is_empty());
    assert!(r[0].condition.is_some());
    let got = linear_system(
        &raw("x==I"),
        &[e("x")],
        &SolveOptions {
            domain: om_solve::Domain::Reals,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(got.set,SolutionSet::Finite(r)if r.is_empty()));
}

#[test]
fn row_swaps_and_nonunit_previous_pivots_have_exact_full_matrix_steps() {
    for source in [
        "{y+z==1,2x+3y==4,3x+z==2}",
        "{2x+y+z==1,x+3y+z==2,4x+2y+5z==3}",
    ] {
        let mut sink = StepRecorder::new();
        linear_system(
            &raw(source),
            &[e("x"), e("y"), e("z")],
            &SolveOptions::default(),
            &Interrupt::default(),
            &mut sink,
        )
        .unwrap();
        let steps = sink.finish();
        for s in flatten(&steps.root) {
            if let StepKind::RowReduce { op, matrix } = &s.kind {
                let mut before = s.before[0]
                    .args()
                    .iter()
                    .map(|r| r.args().to_vec())
                    .collect::<Vec<_>>();
                match op {
                    om_solve::RowOp::Swap { a, b } => before.swap(*a, *b),
                    om_solve::RowOp::Scale { row, factor } => {
                        for v in &mut before[*row] {
                            *v = om_core::mul([factor.clone(), v.clone()])
                        }
                    }
                    om_solve::RowOp::Add {
                        target,
                        source,
                        factor,
                    } => {
                        let row = before[*source].clone();
                        for (v, f) in before[*target].iter_mut().zip(row) {
                            *v = om_core::add([v.clone(), om_core::mul([factor.clone(), f])])
                        }
                    }
                }
                for (a, b) in before.iter().flatten().zip(matrix.iter().flatten()) {
                    assert_eq!(
                        om_simplify::zero::is_zero(&om_core::sub(a.clone(), b.clone())),
                        om_simplify::zero::Tri::Zero
                    )
                }
            }
        }
    }
}

#[test]
fn huge_exact_solutions_do_not_attach_nonfinite_display_coordinates() {
    let got = run("x==2^2000", &["x"]);
    let SolutionSet::Finite(r) = got.set else {
        panic!("finite huge value")
    };
    assert_eq!(r[0].rules, [(e("x"), e("2^2000"))]);
    assert!(r[0].numeric.is_none());
}
proptest! {
    #![proptest_config(ProptestConfig{cases:32,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e39),..ProptestConfig::default()})]
    #[test]
    fn planted_two_by_two_systems_recover_exact_rational_values(a in -5i64..6,b in -5i64..6,c in -5i64..6,d in -5i64..6,u in -4i64..5,v in -4i64..5){
        prop_assume!(a*d-b*c!=0);
        let got=run(&format!("{{({a})x+({b})y=={},({c})x+({d})y=={}}}",a*u+b*v,c*u+d*v),&["x","y"]);
        prop_assert_eq!(rules(&got),[(e("x"),Expr::int(u)),(e("y"),Expr::int(v))]);
    }
}
