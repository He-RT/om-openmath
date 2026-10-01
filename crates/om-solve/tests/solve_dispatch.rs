//! Public P0-P3 solving preserves original syntax across every candidate path.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    Domain, MaxExtra, SolutionSet, SolveError, SolveOptions, Step, StepKind, Verification,
    VerifyMode, solve,
};
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn outcome(s: &str, vars: &[&str], domain: Domain) -> om_solve::SolveOutcome {
    solve(
        &raw(s),
        &vars.iter().map(|v| e(v)).collect::<Vec<_>>(),
        &SolveOptions {
            domain,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
    )
    .unwrap()
}
fn finite(s: &str, vars: &[&str], domain: Domain) -> Vec<om_solve::Solution> {
    let got = outcome(s, vars, domain);
    let SolutionSet::Finite(r) = got.set else {
        panic!("finite {s}")
    };
    assert!(r.iter().all(|r| r.verification != Verification::Unverified));
    r
}
fn flatten(steps: &[Step]) -> Vec<&Step> {
    steps
        .iter()
        .flat_map(|s| std::iter::once(s).chain(flatten(&s.children)))
        .collect()
}
#[test]
fn authority_linear_polynomial_multiplicity_and_root_provenance() {
    let r = finite("2x+3==7", &["x"], Domain::Complexes);
    assert_eq!(r[0].rules, [(e("x"), e("2"))]);
    assert_eq!(r[0].verification, Verification::Exact);
    let r = finite("x^2-5x+6==0", &["x"], Domain::Complexes);
    assert_eq!(
        r.iter().map(|r| r.rules[0].1.clone()).collect::<Vec<_>>(),
        [e("2"), e("3")]
    );
    let r = finite("(x-1)^2==0", &["x"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].multiplicity, 2);
    let r = finite("x^5-x+1==0", &["x"], Domain::Complexes);
    assert_eq!(r.len(), 5);
    assert!(r.iter().all(|r| matches!(
        r.verification,
        Verification::Exact | Verification::ByConstruction
    )));
}
#[test]
fn original_rational_holes_are_preserved_before_canonicalization() {
    let r = finite("(x^2-1)/(x-1)==0", &["x"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules, [(e("x"), e("-1"))]);
    assert!(finite("x/(x-1)==1/(x-1)", &["x"], Domain::Complexes).is_empty());
    let r = finite("1/x+1/(x+1)==1", &["x"], Domain::Complexes);
    assert_eq!(r.len(), 2);
    for root in r {
        let original = e("1/x+1/(x+1)-1").replace_all(&root.rules);
        assert_eq!(
            om_simplify::zero::is_zero(&original),
            om_simplify::zero::Tri::Zero
        );
    }
}
#[test]
fn radical_principal_checks_cannot_be_disabled() {
    for verify in [VerifyMode::Auto, VerifyMode::Always, VerifyMode::Never] {
        let got = solve(
            &raw("Sqrt[x+2]==x"),
            &[e("x")],
            &SolveOptions {
                verify,
                ..SolveOptions::default()
            },
            &Interrupt::default(),
        )
        .unwrap();
        let SolutionSet::Finite(r) = got.set else {
            panic!("principal roots")
        };
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].rules, [(e("x"), e("2"))]);
        assert!(r[0].verification != Verification::Unverified);
    }
    assert!(finite("Sqrt[x]==-1", &["x"], Domain::Complexes).is_empty());
    let r = finite("Sqrt[x]+Sqrt[x-5]==5", &["x"], Domain::Complexes);
    assert_eq!(r[0].rules[0].1, e("9"));
}
#[test]
fn systems_discrete_domains_and_mixed_sources_share_the_public_pipeline() {
    let r = finite("{x^2+y^2==5,x y==2}", &["x", "y"], Domain::Complexes);
    assert_eq!(r.len(), 4);
    assert!(r.iter().all(|r| matches!(
        r.verification,
        Verification::Exact | Verification::ByConstruction
    )));
    let r = finite("{Sqrt[x+2]==y,y==x}", &["x", "y"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules, [(e("x"), e("2")), (e("y"), e("2"))]);
    assert!(finite("x^2==2", &["x"], Domain::Rationals).is_empty());
    let r = finite("2x+3y==1", &["x", "y"], Domain::Integers);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].constants.len(), 1);
}
#[test]
fn logical_unions_deduplicate_and_unsupported_branches_remain_atomic() {
    let r = finite("x^2==1||x==1", &["x"], Domain::Complexes);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].rules[0].1, e("-1"));
    assert_eq!(r[1].rules[0].1, e("1"));
    let r = finite("x^2==1&&x!=1", &["x"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules[0].1, e("-1"));
    assert!(matches!(
        outcome("x==1||Sin[x]+x==0", &["x"], Domain::Complexes).set,
        SolutionSet::Unevaluated
    ));
    assert!(matches!(
        outcome("x==x", &["x"], Domain::Complexes).set,
        SolutionSet::All
    ));
    let r = finite("x==x&&x!=1", &["x"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert!(r[0].rules.is_empty() && r[0].condition.is_some());
}
#[test]
fn parameter_assumptions_remain_visible_when_requested() {
    let got = solve(
        &raw("a x+b==0"),
        &[e("x")],
        &SolveOptions {
            max_extra_conditions: MaxExtra::All,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
    )
    .unwrap();
    let SolutionSet::Finite(r) = got.set else {
        panic!("generic linear")
    };
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules[0].1, e("-b/a"));
    assert!(r[0].condition.is_some());
    let r = finite("{Exp[x]==2,y==x+z}", &["x", "y", "z"], Domain::Reals);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules.len(), 2);
    assert!(r[0].condition.is_some());
}
#[test]
fn independent_periods_and_close_coordinate_sorting_are_certified() {
    let r = finite("{Exp[x]==2,Exp[y]==3}", &["x", "y"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].constants.len(), 2);
    assert_ne!(r[0].constants[0].0, r[0].constants[1].0);
    let r = finite("{x==1-y/2^100,y^2==2}", &["x", "y"], Domain::Complexes);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].rules[1].1, e("Sqrt[2]"));
    assert_eq!(r[1].rules[1].1, e("-Sqrt[2]"));
}
#[test]
fn public_steps_and_interrupt_contracts_are_complete() {
    let got = outcome("Sqrt[x+2]==x", &["x"], Domain::Complexes);
    let steps = got.steps.unwrap();
    let all = flatten(&steps.root);
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::Verify { .. }))
    );
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::DropExtraneous { .. }))
    );
    let got = solve(
        &raw("x^2==2"),
        &[e("x")],
        &SolveOptions {
            record_steps: false,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
    )
    .unwrap();
    assert!(got.steps.is_none());
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        solve(&raw("x^2==2"), &[e("x")], &SolveOptions::default(), &ctx),
        Err(SolveError::Abort(Abort::Budget))
    ));
}

#[test]
fn constrained_identities_keep_domains_and_plain_identities_stay_all() {
    for (domain, name) in [
        (Domain::Reals, "Reals"),
        (Domain::Integers, "Integers"),
        (Domain::Rationals, "Rationals"),
    ] {
        assert!(matches!(
            outcome("x==x", &["x"], domain).set,
            SolutionSet::All
        ));
        let roots = finite("x==x&&x!=1", &["x"], domain);
        assert_eq!(roots.len(), 1);
        let condition = roots[0].condition.as_ref().unwrap();
        assert!(!condition.free_of(&e(&format!("Element[x,{name}]"))));
        assert!(!condition.free_of(&e("x-1!=0")));
    }
}

#[test]
fn equivalent_union_guards_deduplicate_without_summing_multiplicities() {
    let roots = finite(
        "(x==1&&a!=0&&b!=0)||((x-1)^2==0&&b!=0&&a!=0)",
        &["x"],
        Domain::Complexes,
    );
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].multiplicity, 2);
    assert!(roots[0].condition.is_some());
    assert!(matches!(
        outcome("True||(Sin[x]+x==0)", &["x"], Domain::Complexes).set,
        SolutionSet::All
    ));
    assert!(matches!(
        solve(
            &raw("x==1"),
            &[e("1")],
            &SolveOptions::default(),
            &Interrupt::default()
        ),
        Err(SolveError::Invalid(_))
    ));
}
