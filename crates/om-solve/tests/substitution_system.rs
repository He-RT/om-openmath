//! Mixed systems substitute verified univariate branches and retain source restrictions.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    Domain, MaxExtra, NoSteps, SolutionSet, SolveOptions, Step, StepKind, StepRecorder,
    Verification, VerifyMode, substitution_system,
};
use proptest::prelude::*;
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn result(s: &str, vars: &[&str], domain: Domain) -> om_solve::univariate::PolynomialRoots {
    substitution_system(
        &raw(s),
        &vars.iter().map(|v| e(v)).collect::<Vec<_>>(),
        &SolveOptions {
            domain,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap()
}
fn finite(s: &str, vars: &[&str], domain: Domain) -> Vec<om_solve::Solution> {
    let SolutionSet::Finite(r) = result(s, vars, domain).set else {
        panic!("finite {s}")
    };
    assert!(r.iter().all(|r| r.verification != Verification::Unverified));
    r
}
fn equivalent(a: &Expr, b: &Expr) -> bool {
    om_simplify::zero::is_zero(&om_core::sub(a.clone(), b.clone())) == om_simplify::zero::Tri::Zero
}
fn flatten(steps: &[Step]) -> Vec<&Step> {
    steps
        .iter()
        .flat_map(|s| std::iter::once(s).chain(flatten(&s.children)))
        .collect()
}
#[test]
fn mixed_exponential_linear_and_radical_systems_check_original_branches() {
    let r = finite("{Exp[x]==2,y==x+1}", &["x", "y"], Domain::Reals);
    assert_eq!(r.len(), 1);
    assert!(equivalent(&r[0].rules[0].1, &e("Log[2]")));
    assert!(equivalent(&r[0].rules[1].1, &e("1+Log[2]")));
    let r = finite("{Sqrt[x]+y==3,x==y}", &["x", "y"], Domain::Reals);
    assert_eq!(r.len(), 1);
    assert!(equivalent(&r[0].rules[0].1, &e("(7-Sqrt[13])/2")));
    assert!(equivalent(&r[0].rules[0].1, &r[0].rules[1].1));
    let r = finite("{Sqrt[x+2]==y,y==x}", &["x", "y"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules, [(e("x"), e("2")), (e("y"), e("2"))]);
}
#[test]
fn original_canceled_poles_and_explicit_real_membership_are_enforced() {
    assert!(
        finite(
            "{Sqrt[x+2]==y,y==x,(x-2)/(x-2)==1}",
            &["x", "y"],
            Domain::Complexes
        )
        .is_empty()
    );
    assert!(
        finite(
            "{Exp[x]==2,y==I,Element[y,Reals]}",
            &["x", "y"],
            Domain::Complexes
        )
        .is_empty()
    );
}
#[test]
fn independent_periods_avoid_each_other_and_existing_generated_names() {
    let r = finite("{Exp[x]==2,Exp[y]==3}", &["x", "y"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].constants.len(), 2);
    assert_ne!(r[0].constants[0].0, r[0].constants[1].0);
    for a in -1..=1 {
        for b in -1..=1 {
            let rules = [
                (r[0].constants[0].0.clone(), Expr::int(a)),
                (r[0].constants[1].0.clone(), Expr::int(b)),
            ];
            for (i, target) in [2, 3].into_iter().enumerate() {
                let residual = om_core::sub(
                    om_core::exp(r[0].rules[i].1.replace_all(&rules)),
                    Expr::int(target),
                );
                let z = om_simplify::numeval::enclose(&residual, 256, &Interrupt::default())
                    .unwrap()
                    .unwrap();
                assert!(z.re.contains_zero() && z.im.contains_zero());
                assert!(z.re.rad < om_num::BigFloat::from_parts(1.into(), -200));
            }
        }
    }
    let r = finite("{Exp[x]==2,y==C[5]}", &["x", "y"], Domain::Complexes);
    assert_eq!(r[0].constants[0].0, e("C[6]"));
}
#[test]
fn unsupported_finite_branches_and_more_than_64_branches_decline_atomically() {
    assert!(matches!(
        result("{x^2==1,Sin[y]+y==x}", &["x", "y"], Domain::Complexes).set,
        SolutionSet::Unevaluated
    ));
    let names = ["a", "b", "c", "d", "f", "g", "h", "x"];
    let source = "{a^2==1,b^2==1,c^2==1,d^2==1,f^2==1,g^2==1,h^2==1,Exp[x]==2}";
    assert!(matches!(
        result(source, &names, Domain::Reals).set,
        SolutionSet::Unevaluated
    ));
    let roots = finite(
        "{a^2==1,b^2==1,c^2==1,d^2==1,f^2==1,g^2==1,Exp[x]==2}",
        &["a", "b", "c", "d", "f", "g", "x"],
        Domain::Reals,
    );
    assert_eq!(roots.len(), 64);
}
#[test]
fn underdetermined_mixed_systems_keep_later_original_free_variables() {
    let r = result("{Exp[x]==2,y==x+z}", &["x", "y", "z"], Domain::Reals);
    assert!(r.messages.iter().any(|m| m.tag == "svars"));
    let SolutionSet::Finite(r) = r.set else {
        panic!("family")
    };
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules.len(), 2);
    assert_eq!(r[0].rules[0].0, e("x"));
    assert_eq!(r[0].rules[1].0, e("y"));
    assert!(equivalent(&r[0].rules[1].1, &e("Log[2]+z")));
    assert!(r[0].condition.is_some());
}
#[test]
fn kernel_lifting_recovers_coupled_arguments_without_auxiliary_axes() {
    let r = finite(
        "{Sin[x+y]+Cos[x-y]==0,Sin[x+y]-Cos[x-y]==0}",
        &["x", "y"],
        Domain::Complexes,
    );
    assert_eq!(r.len(), 4);
    for root in r {
        assert_eq!(root.rules.len(), 2);
        assert!(root.rules.iter().all(|(_, v)| {
            v.free_symbols()
                .iter()
                .all(|s| !s.name().starts_with("om$"))
        }));
        assert!(root.constants.len() >= 2);
        for a in -1..=1 {
            for b in -1..=1 {
                let constants = [
                    (root.constants[0].0.clone(), Expr::int(a)),
                    (root.constants[1].0.clone(), Expr::int(b)),
                ];
                let rules = root
                    .rules
                    .iter()
                    .map(|(v, value)| (v.clone(), value.replace_all(&constants)))
                    .collect::<Vec<_>>();
                for residual in ["Sin[x+y]+Cos[x-y]", "Sin[x+y]-Cos[x-y]"] {
                    let z = om_simplify::numeval::enclose(
                        &e(residual).replace_all(&rules),
                        256,
                        &Interrupt::default(),
                    )
                    .unwrap()
                    .unwrap();
                    assert!(z.re.contains_zero() && z.im.contains_zero());
                    assert!(z.re.rad < om_num::BigFloat::from_parts(1.into(), -200));
                }
            }
        }
    }
}
#[test]
fn lifting_uses_same_argument_circle_relations_and_retains_period_constraints() {
    let source = "{Sin[x+y]+Cos[x+y]+Sin[x-y]+Cos[x-y]==0,Sin[x+y]-Cos[x+y]+Sin[x-y]-Cos[x-y]==0,Sin[x+y]-Sin[x-y]==0}";
    let r = finite(source, &["x", "y"], Domain::Complexes);
    assert!(!r.is_empty());
    assert!(r.iter().all(|r| r.condition.is_some()));
}
#[test]
fn generic_guards_constraint_only_inputs_and_options_are_preserved() {
    let vars = [e("x"), e("y")];
    let got = substitution_system(
        &raw("{a y==1,Exp[x]==2}"),
        &vars,
        &SolveOptions {
            max_extra_conditions: MaxExtra::All,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(got.assumptions.contains(&e("a!=0")));
    let SolutionSet::Finite(r) = got.set else {
        panic!("generic family")
    };
    assert_eq!(r.len(), 1);
    assert!(r[0].condition.is_some());
    let r = finite("x!=1", &["x"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert!(r[0].rules.is_empty());
    let condition = r[0].condition.as_ref().unwrap();
    assert_eq!(condition.head_symbol(), e("x!=1").head_symbol());
    assert!(equivalent(
        &om_core::sub(condition.args()[0].clone(), condition.args()[1].clone()),
        &e("x-1")
    ));
    let got = substitution_system(
        &raw("{Exp[x]==2,y==x}"),
        &vars,
        &SolveOptions {
            inverse_functions: false,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Unevaluated));
    let got = substitution_system(
        &raw("{Sqrt[x+2]==y,y==x}"),
        &vars,
        &SolveOptions {
            verify: VerifyMode::Never,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(r) = got.set else {
        panic!("mandatory check")
    };
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules, [(e("x"), e("2")), (e("y"), e("2"))]);
}
#[test]
fn compound_axes_and_duplicate_requested_variables_use_normalized_axes() {
    let r = finite(
        "{Sin[f[1]+g[1]]+Cos[f[1]-g[1]]==0,Sin[f[1]+g[1]]-Cos[f[1]-g[1]]==0}",
        &["f[1]", "g[1]"],
        Domain::Complexes,
    );
    assert_eq!(r.len(), 4);
    assert!(
        r.iter()
            .all(|r| r.rules[0].0 == e("f[1]") && r.rules[1].0 == e("g[1]"))
    );
    let r = result("{Exp[x]==2,y==x+1}", &["x", "x", "y"], Domain::Reals);
    assert!(!r.messages.iter().any(|m| m.tag == "svars"));
    let SolutionSet::Finite(r) = r.set else {
        panic!("normalized axes")
    };
    assert_eq!(r[0].rules.len(), 2);
}
proptest! {
    #![proptest_config(ProptestConfig {cases:12,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e11),..ProptestConfig::default()})]
    #[test]
    fn planted_mixed_systems_recover_exact_coordinates(a in 0i64..6,b in 1i64..6) {
        let source=format!("{{x=={a},y==Sqrt[x+{b}],Exp[z]==2}}");
        let r=finite(&source,&["x","y","z"],Domain::Reals);
        prop_assert_eq!(r.len(),1);
        prop_assert_eq!(&r[0].rules[0].1,&Expr::int(a));
        prop_assert!(equivalent(&r[0].rules[1].1,&om_core::sqrt(Expr::int(a+b))));
        prop_assert!(equivalent(&r[0].rules[2].1,&e("Log[2]")));
    }
}
#[test]
fn actual_substitution_steps_disabled_recording_and_abort_are_preserved() {
    let input = raw("{Exp[x]==2,y==x+1}");
    let vars = [e("x"), e("y")];
    let mut sink = StepRecorder::new();
    substitution_system(
        &input,
        &vars,
        &SolveOptions {
            domain: Domain::Reals,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    let steps = sink.finish();
    let all = flatten(&steps.root);
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::BackSubstitute { .. }))
    );
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::Verify { .. }))
    );
    let mut sink = StepRecorder::new();
    substitution_system(
        &input,
        &vars,
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
        substitution_system(&input, &vars, &SolveOptions::default(), &ctx, &mut NoSteps),
        Err(om_solve::SolveError::Abort(Abort::Budget))
    ));
    let r = result("x==x", &["x", "y"], Domain::Complexes);
    assert!(matches!(r.set, SolutionSet::All));
}
