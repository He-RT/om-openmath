//! Discrete domains require exact membership and a complete integer affine lattice.
use om_core::{Expr, canonicalize};
use om_num::ctx::{Abort, Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::univariate::poly_uni;
use om_solve::{
    Domain, NoSteps, SolutionSet, SolveOptions, Step, StepKind, StepRecorder, Verification,
    linear_system, poly_system, substitution_system,
};
use proptest::prelude::*;
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn polynomial(s: &str, domain: Domain) -> Vec<om_solve::Solution> {
    let r = poly_uni(
        &e(s),
        &e("x"),
        &SolveOptions {
            domain,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(r) = r.set else {
        panic!("polynomial {s}")
    };
    r
}
fn linear(s: &str, vars: &[&str], domain: Domain) -> Vec<om_solve::Solution> {
    let r = linear_system(
        &raw(s),
        &vars.iter().map(|v| e(v)).collect::<Vec<_>>(),
        &SolveOptions {
            domain,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(r) = r.set else {
        panic!("linear {s}")
    };
    r
}
fn flatten(steps: &[Step]) -> Vec<&Step> {
    steps
        .iter()
        .flat_map(|s| std::iter::once(s).chain(flatten(&s.children)))
        .collect()
}
#[test]
fn authority_rational_integer_roots_and_exact_near_integer_rejection() {
    assert!(polynomial("x^2-2", Domain::Rationals).is_empty());
    let r = polynomial("x^2-4", Domain::Integers);
    assert_eq!(
        r.iter().map(|r| r.rules[0].1.clone()).collect::<Vec<_>>(),
        [e("-2"), e("2")]
    );
    assert_eq!(polynomial("(2x-1)*(x-2)", Domain::Rationals).len(), 2);
    let r = polynomial("(2x-1)*(x-2)", Domain::Integers);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules[0].1, e("2"));
    assert!(polynomial("x-(1+1/2^100)", Domain::Integers).is_empty());
    assert_eq!(
        polynomial("(x-2)^3*(x^2-2)", Domain::Integers)[0].multiplicity,
        3
    );
}
#[test]
fn integer_hnf_diophantine_family_is_complete_in_small_box() {
    let r = linear("2x+3y==1", &["x", "y"], Domain::Integers);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].constants.len(), 1);
    assert_eq!(r[0].constants[0].1, Domain::Integers);
    assert_eq!(r[0].verification, Verification::Exact);
    let c = r[0].constants[0].0.clone();
    let mut found = vec![];
    for k in -50..=50 {
        let x = r[0].rules[0].1.replace_all(&[(c.clone(), Expr::int(k))]);
        let y = r[0].rules[1].1.replace_all(&[(c.clone(), Expr::int(k))]);
        assert_eq!(
            om_core::add([
                om_core::mul([e("2"), x.clone()]),
                om_core::mul([e("3"), y.clone()])
            ]),
            e("1")
        );
        found.push((x, y));
    }
    for x in -12..=12 {
        for y in -12..=12 {
            if 2 * x + 3 * y == 1 {
                assert!(found.contains(&(Expr::int(x), Expr::int(y))))
            }
        }
    }
    assert!(r[0].rules.iter().all(
        |(_, v)| !v.free_symbols().contains(&om_core::Symbol::intern("x"))
            && !v.free_symbols().contains(&om_core::Symbol::intern("y"))
    ));
}
#[test]
fn integer_systems_clear_rationals_and_reject_lattice_inconsistency() {
    assert!(linear("2x+4y==1", &["x", "y"], Domain::Integers).is_empty());
    assert!(linear("{x+y==1,x-y==0}", &["x", "y"], Domain::Integers).is_empty());
    let r = linear("{x/2+y/3==2,x-y==4}", &["x", "y"], Domain::Integers);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules, [(e("x"), e("4")), (e("y"), e("0"))]);
    let r = linear("{x+y==1,x-y==0}", &["x", "y"], Domain::Rationals);
    assert_eq!(r[0].rules, [(e("x"), e("1/2")), (e("y"), e("1/2"))]);
}
#[test]
fn source_poles_and_generated_parameter_names_survive_integer_lattices() {
    let r = linear("{2x+3y==1,(x-2)/(x-2)==1}", &["x", "y"], Domain::Integers);
    assert_eq!(r.len(), 1);
    assert!(r[0].condition.is_some());
    let r = linear("{2x+3y==1,C[5]!=0}", &["x", "y"], Domain::Integers);
    assert_eq!(r[0].constants[0].0, e("C[6]"));
    assert!(linear("{x==2,(x-2)/(x-2)==1}", &["x"], Domain::Integers).is_empty());
}
#[test]
fn polynomial_coordinates_reduce_to_rationals_and_explicit_domains_filter() {
    for domain in [Domain::Rationals, Domain::Integers] {
        let r = poly_system(
            &raw("{x==y^2,y^2==2}"),
            &[e("x"), e("y")],
            &SolveOptions {
                domain,
                ..SolveOptions::default()
            },
            &Interrupt::default(),
            &mut NoSteps,
        )
        .unwrap();
        let SolutionSet::Finite(r) = r.set else {
            panic!("finite")
        };
        assert!(r.is_empty());
    }
    let r = poly_system(
        &raw("{x==y^2,y^2==2}"),
        &[e("x")],
        &SolveOptions {
            domain: Domain::Rationals,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(r) = r.set else {
        panic!("coordinate")
    };
    assert_eq!(r.len(), 2);
    assert!(
        r.iter()
            .all(|r| r.rules[0].1 == e("2") && r.condition.is_some())
    );
    assert!(linear("{2x==1,Element[x,Integers]}", &["x"], Domain::Complexes).is_empty());
    let r = linear(
        "{2x+3y==1,Element[{x,y},Integers]}",
        &["x", "y"],
        Domain::Complexes,
    );
    assert_eq!(r[0].constants.len(), 1);
    let r = linear("x+y==1", &["x", "y"], Domain::Rationals);
    assert!(r[0].condition.is_some());
}
#[test]
fn mixed_systems_filter_integer_coordinates_after_complete_substitution() {
    let r = substitution_system(
        &raw("{Sqrt[x+2]==y,y==x}"),
        &[e("x"), e("y")],
        &SolveOptions {
            domain: Domain::Integers,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(r) = r.set else {
        panic!("finite")
    };
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules, [(e("x"), e("2")), (e("y"), e("2"))]);
}
#[test]
fn every_system_entry_uses_integer_lattices_and_closed_unknowns_decline() {
    let source = raw("2x+3y==1");
    let vars = [e("x"), e("y")];
    let opts = SolveOptions {
        domain: Domain::Integers,
        ..SolveOptions::default()
    };
    for got in [
        poly_system(&source, &vars, &opts, &Interrupt::default(), &mut NoSteps).unwrap(),
        substitution_system(&source, &vars, &opts, &Interrupt::default(), &mut NoSteps).unwrap(),
    ] {
        let SolutionSet::Finite(r) = got.set else {
            panic!("lattice")
        };
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].constants.len(), 1);
    }
    let got = poly_uni(
        &e("x-Log[2]"),
        &e("x"),
        &SolveOptions {
            domain: Domain::Rationals,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Unevaluated));
    assert!(got.messages.iter().any(|m| m.tag == "dom"));
    let got = om_solve::univariate::transcendental_path(
        &e("Exp[x]-1"),
        &e("x"),
        &opts,
        &Interrupt::default(),
        &mut NoSteps,
    )
    .unwrap();
    let SolutionSet::Finite(r) = got.set else {
        panic!("integer exponential zero")
    };
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules[0].1, e("0"));
    assert!(r[0].constants.is_empty());
}
#[test]
fn discrete_steps_disabled_recording_and_abort_are_real() {
    let opts = SolveOptions {
        domain: Domain::Integers,
        ..SolveOptions::default()
    };
    let mut sink = StepRecorder::new();
    linear_system(
        &raw("2x+3y==1"),
        &[e("x"), e("y")],
        &opts,
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
    poly_uni(
        &e("x^2-2"),
        &e("x"),
        &SolveOptions {
            domain: Domain::Rationals,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    let steps = sink.finish();
    assert!(flatten(&steps.root).iter().any(|s| matches!(
        s.kind,
        StepKind::DomainFilter {
            domain: Domain::Rationals,
            kept: 0,
            dropped: 2
        }
    )));
    let mut sink = StepRecorder::new();
    linear_system(
        &raw("2x+3y==1"),
        &[e("x"), e("y")],
        &SolveOptions {
            record_steps: false,
            ..opts.clone()
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
            &raw("2x+3y==1"),
            &[e("x"), e("y")],
            &opts,
            &ctx,
            &mut NoSteps
        ),
        Err(om_solve::SolveError::Abort(Abort::Budget))
    ));
}
proptest! {
    #![proptest_config(ProptestConfig {cases:24,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e12),..ProptestConfig::default()})]
    #[test]
    fn planted_integer_affine_families_have_exact_residual(a in 1i64..7,b in 1i64..7,x in -4i64..5,y in -4i64..5) {
        let source=format!("{a} x+{b} y=={}",a*x+b*y);
        let r=linear(&source,&["x","y"],Domain::Integers);
        prop_assert_eq!(r.len(),1);
        let r=&r[0];let c=r.constants[0].0.clone();
        for k in [-3,0,2] { let rules=r.rules.iter().map(|(v,z)| (v.clone(),z.replace_all(&[(c.clone(),Expr::int(k))]))).collect::<Vec<_>>();let equation=raw(&source);let residual=om_core::sub(equation.args()[0].clone(),equation.args()[1].clone()).replace_all(&rules);prop_assert_eq!(residual,e("0")); }
    }
}
