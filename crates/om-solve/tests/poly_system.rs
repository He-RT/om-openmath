//! Polynomial systems require all components and a certified primitive shape when needed.
use om_core::{Expr, canonicalize};
use om_num::{
    BigFloat,
    ctx::{Abort, Interrupt},
};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    Domain, NoSteps, SolutionSet, SolveOptions, Step, StepKind, StepRecorder, Verification,
    poly_system,
};
use proptest::prelude::*;
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn result(s: &str, vars: &[&str], domain: Domain) -> om_solve::univariate::PolynomialRoots {
    poly_system(
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
    let r = result(s, vars, domain);
    let SolutionSet::Finite(roots) = r.set else {
        panic!("finite {s}")
    };
    assert!(roots.iter().all(|r| matches!(
        r.verification,
        Verification::Exact | Verification::ByConstruction
    )));
    roots
}
fn flatten(steps: &[Step]) -> Vec<&Step> {
    steps
        .iter()
        .flat_map(|s| std::iter::once(s).chain(flatten(&s.children)))
        .collect()
}
fn compare(roots: &[om_solve::Solution], wants: &[&str]) {
    let mut used = vec![false; roots.len()];
    assert_eq!(roots.len(), wants.len());
    for want in wants {
        let w = e(want);
        let i = roots
            .iter()
            .enumerate()
            .position(|(i, r)| {
                !used[i]
                    && r.rules.iter().map(|(_, v)| v).zip(w.args()).all(|(a, b)| {
                        om_simplify::zero::is_zero(&om_core::sub(a.clone(), b.clone()))
                            == om_simplify::zero::Tri::Zero
                    })
            })
            .unwrap();
        used[i] = true
    }
}
#[test]
fn authority_circle_line_products_and_cyclic_systems_have_every_solution() {
    compare(
        &finite("{x^2+y^2==1,y==x}", &["x", "y"], Domain::Complexes),
        &["{-1/Sqrt[2],-1/Sqrt[2]}", "{1/Sqrt[2],1/Sqrt[2]}"],
    );
    compare(
        &finite("{x^2+y^2==5,x y==2}", &["x", "y"], Domain::Complexes),
        &["{-2,-1}", "{-1,-2}", "{1,2}", "{2,1}"],
    );
    compare(
        &finite("{x^2+y^2==25,y==x+1}", &["x", "y"], Domain::Reals),
        &["{-4,-3}", "{3,4}"],
    );
    let source = "{x+y+z==0,x y+y z+z x==0,x y z==1}";
    let roots = finite(source, &["x", "y", "z"], Domain::Complexes);
    assert_eq!(roots.len(), 6);
    for root in roots {
        for equation in raw(source).args() {
            let residual = om_core::sub(equation.args()[0].clone(), equation.args()[1].clone())
                .replace_all(&root.rules);
            let z = om_simplify::numeval::enclose(&residual, 768, &Interrupt::default())
                .unwrap()
                .unwrap();
            let bound = BigFloat::from_parts(1.into(), -500);
            assert!(
                z.re.contains_zero()
                    && z.im.contains_zero()
                    && z.re.rad < bound
                    && z.im.rad < bound
            )
        }
    }
}
#[test]
fn shape_failure_uses_a_real_primitive_element_and_records_actual_basis() {
    let mut sink = StepRecorder::new();
    let got = poly_system(
        &raw("{x^2==1,y^2==1}"),
        &[e("x"), e("y")],
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut sink,
    )
    .unwrap();
    let SolutionSet::Finite(roots) = got.set else {
        panic!("finite")
    };
    compare(&roots, &["{-1,-1}", "{-1,1}", "{1,-1}", "{1,1}"]);
    let steps = sink.finish();
    let all = flatten(&steps.root);
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::Substitute { .. }))
    );
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::Groebner { .. }))
    );
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::Eliminant { .. }))
    );
    assert!(
        all.iter()
            .any(|s| matches!(s.kind, StepKind::BackSubstitute { .. }))
    );
}
#[test]
fn root_coordinates_original_poles_and_real_domains_are_retained() {
    let roots = finite("{x-y^2==0,y^5-y+1==0}", &["x", "y"], Domain::Complexes);
    assert_eq!(roots.len(), 5);
    assert!(
        roots
            .iter()
            .all(|r| r.rules[1].1.is_head(om_core::BUILTIN::ROOT))
    );
    for r in roots {
        assert_eq!(
            om_simplify::zero::is_zero(&om_core::sub(
                r.rules[0].1.clone(),
                om_core::pow(r.rules[1].1.clone(), e("2"))
            )),
            om_simplify::zero::Tri::Zero
        )
    }
    compare(
        &finite(
            "{x^2==1,y==x,((x-1)/(x-1))==1}",
            &["x", "y"],
            Domain::Complexes,
        ),
        &["{-1,-1}"],
    );
    assert!(finite("{x^2+1==0,y==x}", &["x", "y"], Domain::Reals).is_empty());
    assert!(
        finite(
            "{x^2+1==0,y==x,Element[x,Reals]}",
            &["x", "y"],
            Domain::Complexes
        )
        .is_empty()
    );
}
#[test]
fn positive_dimension_prefers_later_free_axes_and_keeps_unsolved_relations() {
    let got = result("x-y^2==0", &["x", "y"], Domain::Complexes);
    assert!(got.messages.iter().any(|m| m.tag == "svars"));
    let SolutionSet::Finite(roots) = got.set else {
        panic!("finite")
    };
    assert_eq!(roots[0].rules, [(e("x"), e("y^2"))]);
    assert_eq!(finite("x^2+y^2==1", &["x", "y"], Domain::Reals).len(), 2);
    let got = result("x^3+y==0", &["x", "y"], Domain::Complexes);
    assert!(got.messages.iter().any(|m| m.tag == "svars"));
    let SolutionSet::Finite(r) = got.set else {
        panic!("conditional partial")
    };
    assert!(r[0].condition.is_some());
    let roots = finite("x^2-a==0", &["x"], Domain::Complexes);
    assert_eq!(roots.len(), 2);
    assert!(
        roots
            .iter()
            .all(|r| r.rules.iter().all(|(v, _)| v == &e("x")))
    );
}
#[test]
fn unit_zero_nonlinear_foreign_kernels_disabled_recording_and_abort_are_honest() {
    assert!(finite("{x^2==1,x^2==2}", &["x"], Domain::Complexes).is_empty());
    assert!(matches!(
        result("x==x", &["x", "y"], Domain::Complexes).set,
        SolutionSet::All
    ));
    for source in ["Sin[x]+y==1", "x==1||x==2"] {
        assert!(matches!(
            result(source, &["x", "y"], Domain::Complexes).set,
            SolutionSet::Unevaluated
        ))
    }
    let mut sink = StepRecorder::new();
    poly_system(
        &raw("{x^2+y^2==5,x y==2}"),
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
        poly_system(
            &raw("x^2==1"),
            &[e("x")],
            &SolveOptions::default(),
            &ctx,
            &mut NoSteps
        ),
        Err(om_solve::SolveError::Abort(Abort::Budget))
    ));
}

#[test]
fn positive_parameter_assumptions_and_free_real_axes_are_retained() {
    let got = result("x y==0", &["x", "y"], Domain::Complexes);
    assert!(got.assumptions.contains(&e("y!=0")));
    let SolutionSet::Finite(roots) = got.set else {
        panic!("finite generic component")
    };
    assert_eq!(roots[0].rules, [(e("x"), e("0"))]);
    assert!(roots[0].condition.is_none());
    let got = result("x^2+y^2==1", &["x", "y"], Domain::Reals);
    let SolutionSet::Finite(roots) = got.set else {
        panic!("real family")
    };
    for root in roots {
        let cond = root.condition.unwrap();
        let mut stack = vec![&cond];
        let mut found = false;
        while let Some(c) = stack.pop() {
            found |= *c == e("Element[y,Reals]");
            stack.extend(c.args())
        }
        assert!(found)
    }
}

#[test]
fn repeated_and_nonmonogenic_components_never_emit_indeterminate_or_partial_finite_sets() {
    let roots = finite("{x==y,y^2==0}", &["x", "y"], Domain::Complexes);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].multiplicity, 2);
    assert_eq!(roots[0].rules, [(e("x"), e("0")), (e("y"), e("0"))]);
    let got = result("{x^2==0,y^2==0}", &["x", "y"], Domain::Complexes);
    let SolutionSet::Finite(r) = got.set else {
        panic!("finite nonmonogenic point")
    };
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].rules, [(e("x"), e("0")), (e("y"), e("0"))]);
    assert_eq!(r[0].multiplicity, 4);
    let r = finite("{x^2==0,y^2==0,x y==0}", &["x", "y"], Domain::Complexes);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].multiplicity, 3);
}

#[test]
fn conjugate_and_distinct_primary_points_keep_original_local_lengths() {
    let roots = finite(
        "{(x-1)^2*(x+1)==0,(y-2)^3==0}",
        &["x", "y"],
        Domain::Complexes,
    );
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].rules, [(e("x"), e("-1")), (e("y"), e("2"))]);
    assert_eq!(roots[0].multiplicity, 3);
    assert_eq!(roots[1].rules, [(e("x"), e("1")), (e("y"), e("2"))]);
    assert_eq!(roots[1].multiplicity, 6);
    let roots = finite("{(x^2-2)^2==0,y==x}", &["x", "y"], Domain::Complexes);
    assert_eq!(roots.len(), 2);
    assert!(roots.iter().all(|r| r.multiplicity == 2));
    let roots = finite("{x^2==a,a^2==1}", &["x"], Domain::Complexes);
    assert_eq!(roots.len(), 4);
    assert!(
        roots
            .iter()
            .all(|r| r.condition.is_some() && r.rules.len() == 1)
    );
}

#[test]
fn close_coordinates_sort_by_certified_values_before_later_variables() {
    let roots = finite("{x==1-y/2^100,y^2==2}", &["x", "y"], Domain::Complexes);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].rules[1].1, e("Sqrt[2]"));
    assert_eq!(roots[1].rules[1].1, e("-Sqrt[2]"));
}
proptest! {
    #![proptest_config(ProptestConfig{cases:12,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d392e310),..ProptestConfig::default()})]
    #[test]
    fn planted_product_grids_are_complete(a in -3i64..1,b in 1i64..4,c in -3i64..1,d in 1i64..4){
        let source=format!("{{(x-({a}))*(x-({b}))==0,(y-({c}))*(y-({d}))==0}}");
        let roots=finite(&source,&["x","y"],Domain::Complexes);let wants=[format!("{{{a},{c}}}"),format!("{{{a},{d}}}"),format!("{{{b},{c}}}"),format!("{{{b},{d}}}")];compare(&roots,&wants.iter().map(String::as_str).collect::<Vec<_>>());
    }
}
