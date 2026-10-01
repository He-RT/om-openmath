//! P0/P1 preserve raw domain restrictions before arithmetic normalization.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::{Domain, NoSteps, SolveError, StepKind, StepRecorder, normalize::normalize};
use proptest::prelude::*;
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn n(s: &str) -> om_solve::normalize::Normalized {
    normalize(
        &raw(s),
        Some(&[e("x")]),
        Domain::Complexes,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap()
}
#[test]
fn lists_conjunctions_chained_equalities_and_booleans() {
    let got = n("{x==1,And[True,x+1==2==y]}");
    assert_eq!(got.branches.len(), 1);
    assert_eq!(got.branches[0].equations, [e("x-1"), e("x-1"), e("2-y")]);
    assert!(!got.unsupported);
    for input in ["True", "{}", "And[]", "Equal[x]"] {
        let got = n(input);
        assert_eq!(got.branches.len(), 1);
        assert!(got.branches[0].equations.is_empty());
    }
    for input in ["False", "{x==1,False}", "Or[]"] {
        assert!(n(input).branches.is_empty());
    }
}
#[test]
fn disjunctions_distribute_without_losing_branch_constraints() {
    let got = n("(x==1||x==2)&&(x!=0||x!=3)");
    assert_eq!(got.branches.len(), 4);
    assert!(
        got.branches
            .iter()
            .all(|b| b.equations.len() == 1 && b.exclusions.len() == 1)
    );
    assert_eq!(got.branches[0].equations, [e("x-1")]);
    assert_eq!(got.branches[3].equations, [e("x-2")]);
}
#[test]
fn raw_denominators_are_retained_even_when_canonical_construction_cancels() {
    let got = n("(x^2-1)/(x-1)==0");
    let b = &got.branches[0];
    assert_eq!(b.equations, [e("x^2-1")]);
    assert_eq!(b.exclusions[0].value, e("x-1"));
    assert_eq!(b.original.len(), 1);
    let got = n("x/x==1");
    let b = &got.branches[0];
    assert!(b.equations.is_empty());
    assert_eq!(b.exclusions[0].value, e("x"));
    assert!(n("1/(x-x)==1").branches.is_empty());
}
#[test]
fn negative_real_part_powers_logs_and_trigonometric_poles() {
    let got = n("x^(-1+I)+Log[x]+Tan[x]+Sec[x]+Cot[x]+Csc[x]==1");
    let ex = &got.branches[0].exclusions;
    assert_eq!(
        ex.iter().map(|r| r.value.clone()).collect::<Vec<_>>(),
        [e("x"), e("Cos[x]"), e("Sin[x]")]
    );
    let got = n("x^(1+I)==0");
    assert!(got.branches[0].exclusions.is_empty());
    let got = n("Log[2,x]==0");
    assert!(got.branches[0].exclusions.iter().any(|r| r.value == e("x")));
    for input in ["x^(-Pi)==0", "x^(-Sqrt[2])==0"] {
        assert_eq!(n(input).branches[0].exclusions[0].value, e("x"));
    }
}
#[test]
fn unequal_chains_are_pairwise_and_empty_equations_keep_conditions() {
    let got = n("x!=y!=z");
    let b = &got.branches[0];
    assert!(b.equations.is_empty());
    assert_eq!(
        b.exclusions
            .iter()
            .map(|r| r.value.clone())
            .collect::<Vec<_>>(),
        [e("x-y"), e("x-z"), e("y-z")]
    );
    assert!(n("Unequal[x,x]").branches.is_empty());
}
#[test]
fn automatic_variables_are_name_sorted_and_only_first_n_are_solved() {
    let got = normalize(
        &raw("z+x==Pi+E+I+C[1]"),
        None,
        Domain::Complexes,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert_eq!(got.vars, [e("x")]);
    assert_eq!(got.messages[0].tag, "svars");
    let got = normalize(
        &raw("{z==Sin[x],x==1}"),
        None,
        Domain::Complexes,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert_eq!(got.vars, [e("x"), e("z")]);
}
#[test]
fn membership_domains_and_inequality_routing_remain_explicit() {
    let got = n("x^2==1&&Element[x,Reals]&&Element[x,Integers]");
    assert_eq!(got.branches[0].domains, [(e("x"), Domain::Integers)]);
    let got = n("-2<x<=2");
    assert_eq!(got.branches[0].domains, [(e("x"), Domain::Reals)]);
    assert_eq!(got.branches[0].inequalities.len(), 2);
    let got = normalize(
        &raw("x<y"),
        Some(&[e("x"), e("y")]),
        Domain::Reals,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(got.unsupported);
    assert!(got.messages.iter().any(|m| m.tag == "ineq"));
    let got = n("x==1&&Element[a,Reals]");
    assert_eq!(got.branches[0].conditions, [e("Element[a,Reals]")]);
}
#[test]
fn normalization_emits_real_steps_and_honest_resource_failures() {
    let ctx = Interrupt::default();
    let mut sink = StepRecorder::new();
    normalize(
        &raw("(x^2-1)/(x-1)==0"),
        Some(&[e("x")]),
        Domain::Complexes,
        &ctx,
        &mut sink,
    )
    .unwrap();
    let steps = sink.finish();
    assert!(
        steps
            .root
            .iter()
            .any(|s| matches!(s.kind, StepKind::RecordExclusion { .. }))
    );
    assert!(steps.root.iter().any(|s| s.rule_id == "normalize"));
    assert!(steps.root.iter().any(|s| s.rule_id == "clear_denominators"));
    ctx.steps_left.set(0);
    assert!(matches!(
        normalize(&raw("x==1"), None, Domain::Complexes, &ctx, &mut NoSteps),
        Err(SolveError::Abort(Abort::Budget))
    ));
    assert!(matches!(
        normalize(
            &raw("x"),
            None,
            Domain::Complexes,
            &Interrupt::default(),
            &mut NoSteps
        ),
        Err(SolveError::Invalid(_))
    ));
    let expr = Expr::call(
        om_core::BUILTIN::AND,
        (0..7).map(|i| raw(&format!("x=={i}||x=={}", i + 1))),
    );
    let got = normalize(
        &expr,
        Some(&[e("x")]),
        Domain::Complexes,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(got.unsupported);
    assert!(got.messages.iter().any(|m| m.tag == "nsmet"));
    let impossible = Expr::call(
        om_core::BUILTIN::AND,
        [expr.clone(), Expr::sym(om_core::BUILTIN::FALSE)],
    );
    let got = normalize(
        &impossible,
        Some(&[e("x")]),
        Domain::Complexes,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(!got.unsupported);
    assert!(got.branches.is_empty());
    let always = Expr::call(
        om_core::BUILTIN::OR,
        [expr, Expr::sym(om_core::BUILTIN::TRUE)],
    );
    let got = normalize(
        &always,
        Some(&[e("x")]),
        Domain::Complexes,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(!got.unsupported);
    assert_eq!(got.branches.len(), 1);
    assert!(got.branches[0].equations.is_empty());
}
proptest! {
    #![proptest_config(ProptestConfig{cases:64,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e32),..ProptestConfig::default()})]
    #[test]
    fn distributed_branches_agree_with_an_independent_boolean_oracle(a in -2i64..=2,b in -2i64..=2,c in -2i64..=2,d in -2i64..=2) {
        let got=normalize(&raw(&format!("(x==({a})||x==({b}))&&(y==({c})||y==({d}))")),Some(&[e("x"),e("y")]),Domain::Complexes,&Interrupt::default(),&mut NoSteps).unwrap();
        for x in -2..=2 {for y in -2..=2 {
            let truth=got.branches.iter().any(|branch|branch.equations.iter().all(|eq|eq.replace_all(&[(e("x"),Expr::int(x)),(e("y"),Expr::int(y))]).is_zero()));
            prop_assert_eq!(truth,(x==a||x==b)&&(y==c||y==d));
        }}
    }
}
