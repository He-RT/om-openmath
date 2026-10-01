//! Exact interval membership is checked against independent rational arithmetic.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
    rng::SplitMix64,
};
use om_parse::{Dialect, parse_expr};
use om_solve::{
    Bound, Domain, Interval, SolutionSet, SolveError, SolveOptions, Step, StepKind, reduce, solve,
};
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn region(s: &str) -> (Expr, Vec<Interval>) {
    let result = reduce(&raw(s), &[e("x")], Domain::Reals, &Interrupt::default()).unwrap();
    let SolutionSet::Region { cond, intervals } = result.set else {
        panic!("region: {s}")
    };
    (cond, intervals)
}
fn bounds(v: &[Interval]) -> Vec<(String, String)> {
    fn s(b: &Bound) -> String {
        match b {
            Bound::NegInf => "-inf".into(),
            Bound::PosInf => "inf".into(),
            Bound::Open(e) => format!("open:{e:?}"),
            Bound::Closed(e) => format!("closed:{e:?}"),
        }
    }
    v.iter().map(|i| (s(&i.lo), s(&i.hi))).collect()
}
fn expected(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}
#[test]
fn authority_sign_charts_have_exact_open_and_closed_endpoints() {
    assert_eq!(
        bounds(&region("x^2<4").1),
        expected(&[("open:-2", "open:2")])
    );
    assert_eq!(
        bounds(&region("(x-1)/(x+2)>=0").1),
        expected(&[("-inf", "open:-2"), ("closed:1", "inf")])
    );
    assert_eq!(region("x^2>=0").0, e("True"));
    assert_eq!(region("x^2<0").0, e("False"));
    assert_eq!(
        bounds(&region("x^3-x>0").1),
        expected(&[("open:-1", "open:0"), ("open:1", "inf")])
    );
}
#[test]
fn raw_poles_survive_cancellation_and_zero_numerators() {
    assert_eq!(
        bounds(&region("(x-1)/(x-1)>=0").1),
        expected(&[("-inf", "open:1"), ("open:1", "inf")])
    );
    assert_eq!(region("1/(x-1)<1/(x-1)").0, e("False"));
    assert_eq!(
        bounds(&region("0/(x-1)<=0").1),
        expected(&[("-inf", "open:1"), ("open:1", "inf")])
    );
    assert_eq!(region("1/x^2<0").0, e("False"));
}
#[test]
fn conjunctions_unions_singletons_and_adjacent_cells_merge_exactly() {
    assert_eq!(
        bounds(&region("x^2<=4&&x>0").1),
        expected(&[("open:0", "closed:2")])
    );
    assert_eq!(region("x<0||x>=0").0, e("True"));
    assert_eq!(
        bounds(&region("x<0||x>0").1),
        expected(&[("-inf", "open:0"), ("open:0", "inf")])
    );
    assert_eq!(
        bounds(&region("(x-1)^2<=0").1),
        expected(&[("closed:1", "closed:1")])
    );
    assert_eq!(
        bounds(&region("x^2==1&&x>=0").1),
        expected(&[("closed:1", "closed:1")])
    );
    assert_eq!(region("x>1&&x<=1").0, e("False"));
    assert_eq!(
        bounds(&region("0<x<=1").1),
        expected(&[("open:0", "closed:1")])
    );
}
#[test]
fn algebraic_critical_points_use_exact_order_and_cross_polynomial_deduplication() {
    let (_, intervals) = region("(x^2-2)*(x^4-4)<=0");
    assert_eq!(intervals.len(), 2);
    for i in &intervals {
        let (Bound::Closed(a), Bound::Closed(b)) = (&i.lo, &i.hi) else {
            panic!("singletons")
        };
        assert_eq!(a, b);
        assert_eq!(
            om_simplify::zero::is_zero(&om_core::sub(
                om_core::pow(a.clone(), Expr::int(2)),
                Expr::int(2)
            )),
            om_simplify::zero::Tri::Zero
        );
    }
    assert_eq!(
        bounds(&region("(x-1)*(x-1-1/2^100)<0").1),
        expected(&[("open:1", &format!("open:{:?}", e("1+1/2^100")))])
    );
    let (_, intervals) = region("x^5-x+1<=0");
    assert_eq!(intervals.len(), 1);
    assert!(matches!(&intervals[0].hi, Bound::Closed(v) if v.is_head(B::ROOT)));
}
fn contains(v: &[Interval], q: &Rational) -> bool {
    let point = Expr::number(om_num::Number::Rational(q.clone()));
    let ctx = Interrupt::default();
    v.iter().any(|i| {
        let compare = |b: &Bound, lower: bool| match b {
            Bound::NegInf | Bound::PosInf => true,
            Bound::Open(a) | Bound::Closed(a) => {
                let a = om_simplify::root_reduce::to_algebraic(a, &ctx)
                    .unwrap()
                    .unwrap();
                let q = om_simplify::root_reduce::to_algebraic(&point, &ctx)
                    .unwrap()
                    .unwrap();
                let order = a.cmp_real(&q, &ctx).unwrap().unwrap();
                if lower {
                    order.is_lt() || order.is_eq() && matches!(b, Bound::Closed(_))
                } else {
                    order.is_gt() || order.is_eq() && matches!(b, Bound::Closed(_))
                }
            }
        };
        compare(&i.lo, true) && compare(&i.hi, false)
    })
}
#[test]
fn planted_sign_charts_match_independent_exact_fraction_membership() {
    let mut rng = SplitMix64::new(0x1490);
    for _ in 0..24 {
        let a = rng.next_range(0, 9) as i64 - 4;
        let b = rng.next_range(0, 9) as i64 - 4;
        let c = rng.next_range(0, 9) as i64 - 4;
        let source = format!("(x-({a}))^2*(x-({b}))/(x-({c}))>=0");
        let (_, v) = region(&source);
        for n in -18..=18 {
            let q = Rational::from(n) / Rational::from(3);
            let pole = q == Rational::from(c);
            let product =
                (&q - Rational::from(a)) * (&q - Rational::from(a)) * (&q - Rational::from(b));
            let yes = !pole && product / (&q - Rational::from(c)) >= Rational::ZERO;
            assert_eq!(contains(&v, &q), yes, "{source} at {q}");
        }
    }
}
#[test]
fn pure_equations_convert_to_boolean_relations() {
    assert_eq!(region("x^2==1").0, e("x==-1||x==1"));
    assert_eq!(region("x==x").0, e("True"));
    assert_eq!(region("x==x+1").0, e("False"));
}

#[test]
fn repeated_high_degree_inputs_use_the_small_square_free_root_certificate() {
    assert_eq!(
        bounds(&region("(x-1)^80<=0").1),
        expected(&[("closed:1", "closed:1")])
    );
    assert_eq!(
        bounds(&region("(x-1)^80>0").1),
        expected(&[("-inf", "open:1"), ("open:1", "inf")])
    );
}

#[test]
fn real_chart_does_not_silently_drop_complex_logical_alternatives() {
    let got = solve(
        &raw("x<0||x^2+1==0"),
        &[e("x")],
        &SolveOptions::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Unevaluated));
    assert_eq!(region("x<0||x^2+1==0").0, e("x<0"));
    let got = solve(
        &raw("x<0||x==1"),
        &[e("x")],
        &SolveOptions::default(),
        &Interrupt::default(),
    )
    .unwrap();
    let SolutionSet::Region { intervals, .. } = got.set else {
        panic!("real finite alternative")
    };
    assert_eq!(
        bounds(&intervals),
        expected(&[("-inf", "open:0"), ("closed:1", "closed:1")])
    );
    let got = solve(
        &raw("x<0||x==x"),
        &[e("x")],
        &SolveOptions::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::All));
    let got = solve(
        &raw("x^2+1==0||x==x||x<0"),
        &[e("x")],
        &SolveOptions::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::All));
}
fn flatten(v: &[Step]) -> Vec<&Step> {
    v.iter()
        .flat_map(|s| std::iter::once(s).chain(flatten(&s.children)))
        .collect()
}
#[test]
fn solve_routes_inequalities_with_real_sign_steps_and_disabled_recording() {
    let got = solve(
        &raw("x^2<4"),
        &[e("x")],
        &SolveOptions::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert!(matches!(got.set, SolutionSet::Region { .. }));
    assert!(flatten(&got.steps.unwrap().root).iter().any(
        |s| matches!(&s.kind,StepKind::SignChart {points,signs} if points.len()==2&&signs.len()==3)
    ));
    let got = solve(
        &raw("x^2<4"),
        &[e("x")],
        &SolveOptions {
            record_steps: false,
            ..SolveOptions::default()
        },
        &Interrupt::default(),
    )
    .unwrap();
    assert!(got.steps.is_none());
}
#[test]
fn unsupported_constraints_decline_and_invalid_inputs_or_abort_propagate() {
    for (s, v, d) in [
        ("x+y<1", vec![e("x"), e("y")], Domain::Reals),
        ("a x<1", vec![e("x")], Domain::Reals),
        ("Sin[x]>0", vec![e("x")], Domain::Reals),
        ("x<1", vec![e("x")], Domain::Integers),
    ] {
        let got = reduce(&raw(s), &v, d, &Interrupt::default()).unwrap();
        assert!(matches!(got.set, SolutionSet::Unevaluated), "{s}");
        assert!(
            got.messages
                .iter()
                .any(|m| m.symbol == "Reduce" && m.tag == "nsmet")
        );
    }
    assert!(matches!(
        reduce(&raw("x<1"), &[e("1")], Domain::Reals, &Interrupt::default()),
        Err(SolveError::Invalid(_))
    ));
    let ctx = Interrupt::default();
    ctx.steps_left.set(0);
    assert!(matches!(
        reduce(&raw("x^2<4"), &[e("x")], Domain::Reals, &ctx),
        Err(SolveError::Abort(Abort::Budget))
    ));
}
