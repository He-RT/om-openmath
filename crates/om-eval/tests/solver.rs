//! Solver entry points retain raw sources while using the public solver kernels.
use om_core::{Expr, Interrupt, canonicalize};
use om_eval::Evaluator;
use om_parse::{Dialect, parse_expr};
use om_solve::{SolutionSet, SolveOptions};
fn raw(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn e(s: &str) -> Expr {
    canonicalize(&raw(s))
}
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate_statement(&raw(s), &Interrupt::default())
        .unwrap()
}
fn same(ev: &mut Evaluator, s: &str, expected: &str) {
    assert_eq!(
        om_format::input_form(&canonicalize(&eval(ev, s))),
        om_format::input_form(&e(expected)),
        "{s}"
    );
}
#[test]
fn raw_holes_match_the_public_solver_and_steps() {
    let mut ev = Evaluator::new();
    for s in [
        "Solve[x/x==1,x]",
        "Solve[(x^2-1)/(x-1)==0,x]",
        "Solve[x/(x-1)==1/(x-1),x]",
        "Solve[(x-1)/(x-1)==x,x]",
    ] {
        let call = raw(s);
        let result = om_solve::solve(
            &call.args()[0],
            &[e("x")],
            &SolveOptions::default(),
            &Interrupt::default(),
        )
        .unwrap();
        assert!(!matches!(result.set, SolutionSet::Unevaluated));
        assert_eq!(
            canonicalize(&eval(&mut ev, s)),
            canonicalize(&result.set.to_expr()),
            "{s}"
        );
        assert_eq!(
            ev.last_steps.as_ref().unwrap().root.len(),
            result.steps.unwrap().root.len()
        );
    }
}
#[test]
fn shapes_domains_and_values_preserve_axis_order_and_multiplicity() {
    let mut ev = Evaluator::new();
    for (s, out) in [
        ("Solve[x^2==4,x,Integers]", "{{x->-2},{x->2}}"),
        ("Solve[x^2==2,x,Rationals]", "{}"),
        ("SolveValues[(x-1)^2==0,x]", "{1,1}"),
        ("SolveValues[{x==2,y==3},{y,x}]", "{{3,2}}"),
        ("SolveValues[x+y==1,{x,y}]", "{{1-y,y}}"),
        ("SolveValues[x==x,x]", "{x}"),
        ("Roots[x^2==1,x]", "x==-1||x==1"),
        ("Reduce[x^2<4,x]", "-2<x<2"),
        ("Eliminate[{x==y+1,y==2z},y]", "x==1+2z"),
        ("ConditionalExpression[2+2,True]", "4"),
        ("ConditionalExpression[1/0,False]", "Undefined"),
    ] {
        same(&mut ev, s, out);
    }
}
#[test]
fn delayed_definitions_pure_functions_parameters_and_inference() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "a=2; f[t_]:=t/t; eq:=f[x]==1");
    same(&mut ev, "Solve[a x==4]", "{{x->2}}");
    let source = raw("x/x==1");
    let result = om_solve::solve(
        &source,
        &[e("x")],
        &SolveOptions::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert_eq!(
        canonicalize(&eval(&mut ev, "Solve[eq,x]")),
        canonicalize(&result.set.to_expr())
    );
    assert_eq!(
        canonicalize(&eval(&mut ev, "Solve[(#/#&)[x]==1,x]")),
        canonicalize(&result.set.to_expr())
    );
    let modern = parse_expr("solve(x^2 = 4, x)", Dialect::Modern).unwrap();
    assert_eq!(
        canonicalize(&ev.evaluate(&modern, &Interrupt::default()).unwrap()),
        e("{{x->-2},{x->2}}")
    );
}
#[test]
fn recording_readonly_and_statement_boundaries() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "Solve[x^2==2,x]");
    assert!(ev.last_steps.as_ref().is_some_and(|s| !s.root.is_empty()));
    let history = ev.history.clone();
    let mut fork = ev.fork_readonly();
    eval(&mut fork, "Solve[x==3,x]");
    assert_eq!(ev.history, history);
    ev.settings.record_steps = false;
    for s in [
        "Solve[x==2,x]",
        "NSolve[x^2==2,x]",
        "Reduce[x^2<4,x]",
        "FindRoot[x^2==2,{x,1}]",
    ] {
        assert_ne!(eval(&mut ev, s).head_symbol(), raw(s).head_symbol());
        assert!(ev.last_steps.is_none(), "{s}");
    }
    eval(&mut ev, "2+2");
    assert!(ev.last_steps.is_none());
    eval(&mut ev, "Eliminate[x==y,y]");
    assert!(ev.last_steps.is_none());
}
#[test]
fn malformed_and_unsupported_calls_remain_inspectable() {
    let mut ev = Evaluator::new();
    for s in [
        "Solve[]",
        "Solve[x==1,2]",
        "Solve[x==1,x,UnknownOption->True]",
        "Solve[x==1,x,Cubics->3]",
        "Solve[Sin[x]+x==0,x]",
        "FindRoot[x==1,{x}]",
        "NSolve[x==1,x,WorkingPrecision->0]",
        "Root[#^2-2&,0]",
        "Root[Sin[#]&,1]",
    ] {
        let call = raw(s);
        assert_eq!(eval(&mut ev, s), call, "{s}");
        assert!(!ev.messages.take().is_empty(), "{s}");
    }
}

fn small(expr: &Expr, bits: u32, threshold: u32) {
    let z = om_simplify::numeval::enclose(expr, bits, &Interrupt::default())
        .unwrap()
        .unwrap();
    let limit = om_num::BigFloat::from_parts(1.into(), -(threshold as isize));
    for b in [z.re, z.im] {
        let mid = if b.mid < om_num::BigFloat::ZERO {
            -b.mid
        } else {
            b.mid
        };
        assert!(mid + b.rad < limit, "residual {expr:?}");
    }
}
#[test]
fn numeric_precision_domains_and_local_methods_use_public_kernels() {
    let mut ev = Evaluator::new();
    let roots = eval(&mut ev, "NSolve[x^2==2,x,WorkingPrecision->200]");
    assert_eq!(roots.args().len(), 2);
    for row in roots.args() {
        let value = &row.args()[0].args()[1];
        assert!(matches!(
            value.as_number(),
            Some(om_num::Number::Real(om_num::Real::Big(_)))
        ));
        small(
            &om_core::sub(om_core::pow(value.clone(), Expr::int(2)), Expr::int(2)),
            700,
            600,
        );
    }
    same(&mut ev, "NSolve[x^2+1==0,x,Reals]", "{}");
    let values = eval(&mut ev, "NSolveValues[(x-1)^2==0,{x}]");
    assert_eq!(values.args().len(), 2);
    assert!(
        values
            .args()
            .iter()
            .all(|r| r.is_head(om_core::BUILTIN::LIST))
    );
    eval(&mut ev, "x=99;y=88;a=2");
    for s in [
        "FindRoot[x^2==a,{x,1},WorkingPrecision->100]",
        "FindRoot[x^2==a,{x,1,2},Method->\"Brent\",WorkingPrecision->100]",
    ] {
        let result = eval(&mut ev, s);
        assert!(result.is_head(om_core::BUILTIN::LIST), "{result:?}");
        assert_eq!(result.args()[0].args()[0], e("x"));
        small(
            &om_core::sub(
                om_core::pow(result.args()[0].args()[1].clone(), Expr::int(2)),
                Expr::int(2),
            ),
            400,
            290,
        );
        assert_eq!(eval(&mut ev, "x"), Expr::int(99));
    }
    for s in [
        "FindRoot[{x+y==3,x-y==1},{{x,0},{y,0}}]",
        "FindRoot[{x+y==3,x-y==1},{x,0},{y,0}]",
    ] {
        let result = eval(&mut ev, s);
        assert_eq!(result.args().len(), 2);
        assert_eq!(result.args()[0].args()[0], e("x"));
        assert_eq!(result.args()[1].args()[0], e("y"));
        small(
            &om_core::sub(result.args()[0].args()[1].clone(), Expr::int(2)),
            100,
            40,
        );
        small(
            &om_core::sub(result.args()[1].args()[1].clone(), Expr::int(1)),
            100,
            40,
        );
    }
    assert_eq!(eval(&mut ev, "y"), Expr::int(88));
}
#[test]
fn generated_conditions_and_roots_keep_full_solution_meaning() {
    let mut ev = Evaluator::new();
    same(
        &mut ev,
        "SolveValues[Exp[x]==2,x,GeneratedParameters->k]",
        "{ConditionalExpression[Log[2]+2Pi I k[1],Element[k[1],Integers]]}",
    );
    same(
        &mut ev,
        "Roots[Exp[x]==2,x,GeneratedParameters->k]",
        "x==Log[2]+2Pi I k[1]&&Element[k[1],Integers]",
    );
    same(
        &mut ev,
        "Solve[a x==b,x,MaxExtraConditions->All,VerifySolutions->True]",
        "{{x->ConditionalExpression[b/a,a!=0]}}",
    );
    same(
        &mut ev,
        "Solve[a x==b,x,MaxExtraConditions->1,VerifySolutions->False]",
        "{{x->ConditionalExpression[b/a,a!=0]}}",
    );
    same(
        &mut ev,
        "Solve[Exp[x]==2,x,InverseFunctions->False]",
        "Solve[Exp[x]==2,x,InverseFunctions->False]",
    );
    same(&mut ev, "Roots[x==x,x]", "True");
    same(&mut ev, "Roots[x==x+1,x]", "False");
    same(&mut ev, "Roots[{x+y==1,x-y==0},{y,x}]", "y==1/2&&x==1/2");
    same(
        &mut ev,
        "SolveValues[x/x==1,x]",
        "{ConditionalExpression[x,x!=0]}",
    );
}
#[test]
fn exact_roots_and_conditional_evaluation() {
    let mut ev = Evaluator::new();
    same(&mut ev, "Root[#-2&,1]", "2");
    let r = eval(&mut ev, "Root[Function[t,t^2-2],2]");
    let a = om_simplify::root_reduce::to_algebraic(&r, &Interrupt::default())
        .unwrap()
        .unwrap();
    let b = om_simplify::root_reduce::to_algebraic(&e("Sqrt[2]"), &Interrupt::default())
        .unwrap()
        .unwrap();
    assert_eq!(a.equals(&b, &Interrupt::default()).unwrap(), Some(true));
    same(&mut ev, "Root[#^3+a #+b&,2]", "Root[#^3+a #+b&,2]");
    same(
        &mut ev,
        "ConditionalExpression[2+2,x>0]",
        "ConditionalExpression[4,x>0]",
    );
    ev.messages.take();
    same(&mut ev, "ConditionalExpression[1/0,2<1]", "Undefined");
    assert!(ev.messages.take().is_empty());
    for s in [
        "Root[#^2-2&,3]",
        "Root[#2^2-2&,1]",
        "Root[1/#&,1]",
        "FindRoot[x==1,{x,0},Method->\"Brent\"]",
        "FindRoot[x==1,{x,0,2},Method->\"Newton\"]",
    ] {
        assert_eq!(eval(&mut ev, s), raw(s));
        assert!(!ev.messages.take().is_empty());
    }
}
#[test]
fn explicit_empty_variables_and_no_steps_adapters_keep_contracts() {
    let mut ev = Evaluator::new();
    for s in ["Solve[x==1,{}]", "SolveValues[x==x,{}]"] {
        let call = raw(s);
        let set = om_solve::solve(
            &call.args()[0],
            &[],
            &SolveOptions::default(),
            &Interrupt::default(),
        )
        .unwrap()
        .set;
        if s.starts_with("SolveValues") {
            same(&mut ev, s, "{{}}");
        } else {
            assert_eq!(
                canonicalize(&eval(&mut ev, s)),
                canonicalize(&set.to_expr())
            );
        }
    }
    let opts = SolveOptions {
        record_steps: false,
        domain: om_solve::Domain::Reals,
        ..Default::default()
    };
    let n = om_solve::nsolve_with_options(
        &raw("x^2+1==0"),
        &[e("x")],
        om_num::Precision::Machine,
        &opts,
        &Interrupt::default(),
    )
    .unwrap();
    assert!(n.steps.is_none());
    assert!(matches!(n.set,SolutionSet::Finite(ref roots) if roots.is_empty()));
    let r = om_solve::reduce_with_options(&raw("x^2<4"), &[e("x")], &opts, &Interrupt::default())
        .unwrap();
    assert!(r.steps.is_none());
    assert_eq!(
        om_format::input_form(&canonicalize(&r.set.to_expr())),
        om_format::input_form(&e("-2<x<2"))
    );
}
#[test]
fn solver_abort_restores_held_scopes_and_all_calls_have_docs() {
    use om_core::{Abort, Clock};
    use std::{
        cell::Cell,
        sync::{Arc, atomic::Ordering},
    };
    struct Expired;
    impl Clock for Expired {
        fn now_ms(&self) -> f64 {
            10.0
        }
    }
    let mut ev = Evaluator::new();
    eval(&mut ev, "x=99");
    let s = raw("FindRoot[x^2==2,{x,1},WorkingPrecision->100]");
    let probe = Interrupt::default();
    ev.evaluate_statement(&s, &probe).unwrap();
    let required = 500_000_000 - probe.steps_left.get();
    assert!(required > 100);
    for budget in [0, 8, 25, 100, required - 1] {
        let ctx = Interrupt {
            steps_left: Cell::new(budget),
            ..Default::default()
        };
        let result = ev.evaluate_statement(&s, &ctx);
        assert!(
            matches!(result, Err(om_eval::EvalError::Abort(Abort::Budget))),
            "budget {budget}: {result:?}"
        );
        assert_eq!(eval(&mut ev, "x"), Expr::int(99));
    }
    let ctx = Interrupt::default();
    ctx.flag.store(true, Ordering::Relaxed);
    assert!(matches!(
        ev.evaluate_statement(&raw("Solve[x==1,x]"), &ctx),
        Err(om_eval::EvalError::Abort(Abort::Interrupted))
    ));
    let ctx = Interrupt {
        deadline_ms: Some(1.0),
        clock: Some(Arc::new(Expired)),
        ..Default::default()
    };
    assert!(matches!(
        ev.evaluate_statement(&s, &ctx),
        Err(om_eval::EvalError::Abort(Abort::Timeout))
    ));
    for name in [
        "Solve",
        "NSolve",
        "FindRoot",
        "Reduce",
        "Eliminate",
        "SolveValues",
        "NSolveValues",
        "Roots",
        "Root",
        "ConditionalExpression",
    ] {
        let doc = Evaluator::doc(om_core::Symbol::intern(name)).unwrap();
        assert!(!doc.summary_en.is_empty() && !doc.summary_zh.is_empty());
        for example in doc.examples {
            let mut ev = Evaluator::new();
            assert_ne!(eval(&mut ev, example), raw(example), "{example}");
        }
    }
}
#[test]
fn local_and_numeric_raw_poles_are_never_erased() {
    let mut ev = Evaluator::new();
    let original = raw("(x-1)/(x-1)==x");
    let direct = om_solve::find_root(
        &original,
        &[(e("x"), om_num::Number::Integer(1.into()))],
        &Default::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert!(matches!(direct.set, SolutionSet::Unevaluated));
    let call = "FindRoot[(x-1)/(x-1)==x,{x,1}]";
    assert_eq!(eval(&mut ev, call), raw(call));
    assert!(
        ev.messages
            .take()
            .iter()
            .any(|m| m.symbol == "FindRoot" && m.tag == "cvmit")
    );
    same(&mut ev, "NSolve[(x-1)/(x-1)==x,x]", "{}");
    same(&mut ev, "NSolve[(x^2-1)/(x-1)==0,x]", "{{x->-1.}}");
}
#[test]
fn formula_options_cover_cubic_and_quartic_branches() {
    let mut ev = Evaluator::new();
    for s in [
        "Solve[x^3+x+1==0,x,Cubics->True]",
        "Solve[x^4+x+1==0,x,Quartics->True]",
    ] {
        let call = raw(s);
        let out = eval(&mut ev, s);
        assert_eq!(out.args().len(), if s.contains("x^3") { 3 } else { 4 });
        for row in out.args() {
            let v = &row.args()[0].args()[1];
            assert!(!v.is_head(om_core::BUILTIN::ROOT));
            let lhs = call.args()[0].args()[0].replace_all(&[(e("x"), v.clone())]);
            small(&lhs, 700, 400);
        }
        let numeric = eval(&mut ev, &s.replacen("Solve[", "NSolve[", 1));
        assert_eq!(numeric.args().len(), out.args().len());
        for row in numeric.args() {
            let v = &row.args()[0].args()[1];
            assert!(v.as_number().is_some());
            small(
                &call.args()[0].args()[0].replace_all(&[(e("x"), v.clone())]),
                128,
                40,
            );
        }
    }
}
#[test]
fn nested_root_parameters_resolve_before_equation_normalization() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "a=2");
    let r = eval(&mut ev, "Root[#^2-2&,2]");
    let out = eval(&mut ev, "Solve[x==Root[#^2-a&,2],x]");
    assert_eq!(out.args()[0].args()[0].args()[1], r);
}
#[test]
fn true_conditional_sources_and_named_function_lists_keep_raw_meaning() {
    let mut ev = Evaluator::new();
    let original = raw("x/x==1");
    let direct = om_solve::solve(
        &original,
        &[e("x")],
        &Default::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert_eq!(
        canonicalize(&eval(
            &mut ev,
            "Solve[ConditionalExpression[x/x,True]==1,x]"
        )),
        canonicalize(&direct.set.to_expr())
    );
    same(
        &mut ev,
        "Solve[x==ConditionalExpression[2,True],x]",
        "{{x->2}}",
    );
    same(&mut ev, "Root[Function[{t},t-2],1]", "2");
}
#[test]
fn explicit_arithmetic_heads_and_opaque_numeric_calls_preserve_poles() {
    let mut ev = Evaluator::new();
    let direct = om_solve::solve(
        &raw("x/x==1"),
        &[e("x")],
        &Default::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert_eq!(
        canonicalize(&eval(&mut ev, "Solve[Divide[x,x]==1,x]")),
        canonicalize(&direct.set.to_expr())
    );
    let call = "Solve[Floor[x/x]==1,x]";
    let result = om_solve::solve(
        &raw("Floor[x/x]==1"),
        &[e("x")],
        &Default::default(),
        &Interrupt::default(),
    )
    .unwrap();
    assert!(matches!(result.set, SolutionSet::Unevaluated));
    assert_eq!(eval(&mut ev, call), raw(call));
    same(&mut ev, "Solve[GCD[12,6]x==12,x]", "{{x->2}}");
    same(&mut ev, "Solve[Floor[1/0]==Floor[1/0],x]", "{}");
    same(&mut ev, "Solve[Subtract[x,1]==0,x]", "{{x->1}}");
}
#[test]
fn invalid_nested_roots_decline_the_whole_solving_call() {
    let mut ev = Evaluator::new();
    for s in [
        "Solve[x==Root[Sin[#]&,1],x]",
        "Solve[x==Root[],x]",
        "NSolve[x==Root[#^2-2&,0],x]",
    ] {
        assert_eq!(eval(&mut ev, s), raw(s), "{s}");
        assert!(!ev.messages.take().is_empty());
    }
    eval(&mut ev, "a=0");
    let s = "Solve[x==Root[#^2/a-1&,1],x]";
    assert_eq!(eval(&mut ev, s), raw(s));
}
