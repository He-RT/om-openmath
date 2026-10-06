//! Genuine fitted models, independent residual definitions, source scopes and honest failure paths.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn field<'a>(e: &'a Expr, k: &str) -> &'a Expr {
    &e.args()
        .iter()
        .find(|r| r.args()[0] == Expr::string(k))
        .unwrap_or_else(|| panic!("missing {k}: {e:?}"))
        .args()[1]
}
fn value(e: &Expr) -> f64 {
    e.as_number().unwrap().to_f64().unwrap()
}
#[test]
fn linear_fit_is_callable_with_actual_ordered_parameters_and_noisy_residuals() {
    let mut ev = Evaluator::new();
    eval(
        &mut ev,
        "let line=fit([[0,1],[1,3],[2,5]],model:a*x+b,parameters:{a:1,b:0})",
    );
    assert!((value(&eval(&mut ev, "line.model(3)")) - 7.).abs() < 1e-12);
    let line = eval(&mut ev, "line");
    assert_eq!(field(&line, "method"), &Expr::string("linear_qr"));
    assert_eq!(field(&line, "numerical_rank"), &Expr::int(2));
    let noisy = eval(
        &mut ev,
        "fit([[0,1],[1,2],[2,2]],model:a*x+b,parameters:[a,b])",
    );
    assert!((value(field(field(&noisy, "parameters"), "a")) - 0.5).abs() < 1e-14);
    assert!((value(field(field(&noisy, "parameters"), "b")) - 7. / 6.).abs() < 1e-14);
    assert!((value(field(&noisy, "sum_squares")) - 1. / 6.).abs() < 1e-14);
    assert_eq!(field(&noisy, "degrees_of_freedom"), &Expr::int(1));
    assert!(field(&noisy, "data").is_head(B::DATA_TABLE));
    let tiny = eval(
        &mut ev,
        "fit([[0,1e-300],[1,-1e-300]],model:a,parameters:[a])",
    );
    assert_eq!(field(&tiny, "sum_squares"), &Expr::sym(B::NULL));
    assert_eq!(
        field(&tiny, "sum_squares_status"),
        &Expr::string("underflow")
    );
    assert!(value(field(&tiny, "residual_norm")) > 0.);
    let large = eval(
        &mut ev,
        "fit([[0,1e200],[1,-1e200]],model:a,parameters:[a])",
    );
    assert_eq!(field(&large, "sum_squares"), &Expr::sym(B::NULL));
    assert_eq!(
        field(&large, "sum_squares_status"),
        &Expr::string("overflow")
    );
}
#[test]
fn real_nonlinear_exponential_fit_reports_actual_work_and_model_predictions() {
    let mut ev = Evaluator::new();
    eval(
        &mut ev,
        "let curve=fit([[0,2],[1,numeric(2*exp(0.3))],[2,numeric(2*exp(0.6))],[3,numeric(2*exp(0.9))]],model:a*exp(b*x),parameters:{a:1,b:0},method:\"nonlinear\")",
    );
    let result = eval(&mut ev, "curve");
    assert!((value(field(field(&result, "parameters"), "a")) - 2.).abs() < 1e-7);
    assert!((value(field(field(&result, "parameters"), "b")) - 0.3).abs() < 1e-7);
    assert!(value(field(&result, "evaluations")) > 1.);
    assert_eq!(field(&result, "converged"), &Expr::sym(B::TRUE));
    assert!((value(&eval(&mut ev, "curve.model(0.5)")) - 2. * 0.15f64.exp()).abs() < 1e-7);
}
#[test]
fn multivariate_table_named_functions_and_shadowed_globals_are_local_and_readonly() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let x=100");
    eval(&mut ev, "let a=200");
    eval(&mut ev, "let f(x,a,b)=a*x+b");
    let line = eval(
        &mut ev,
        "fit([[0,1],[1,3],[2,5]],model:f(x,a,b),parameters:[a,b])",
    );
    assert!((value(field(field(&line, "parameters"), "a")) - 2.).abs() < 1e-12);
    assert_eq!(eval(&mut ev, "a"), Expr::int(200));
    assert_eq!(eval(&mut ev, "x"), Expr::int(100));
    eval(
        &mut ev,
        "let surface=fit([[0,0,1],[1,0,3],[0,1,4],[1,1,6]],model:a*x+b*t+c,parameters:[c,a,b],variables:[x,t])",
    );
    assert!((value(&eval(&mut ev, "surface.model(2,3)")) - 14.).abs() < 1e-12);
    let table = eval(
        &mut ev,
        r#"fit(parse_csv("x,y\n0,1\n1,3"),model:a*x+b,parameters:[a,b])"#,
    );
    assert!(!table.is_head(B::RECORD));
    let table = eval(
        &mut ev,
        r#"fit(DataTable(["x","score"],[{x:0,score:1},{x:1,score:3},{x:2,score:5}]),model:a*x+b,parameters:[a,b],target:"score")"#,
    );
    assert!((value(field(field(&table, "parameters"), "a")) - 2.).abs() < 1e-12);
}
#[test]
fn invalid_linear_nonlinear_domains_rank_precision_mutations_and_limits_fail_honestly() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let counter=0");
    for source in [
        "fit([[0,1]],model:a*x+b,parameters:[a,b])",
        "fit([[0,1],[0,2]],model:a*x+b,parameters:[a,b])",
        "fit([[0,1],[1,2]],model:exp(a*x),parameters:{a:1})",
        "fit([[0,1],[1,2]],model:sequence(assign(counter,4),a*x),parameters:{a:1})",
        "fit([[0,1],[1,2]],model:a*x,parameters:{a:decimal(\"1\",precision:50)})",
        "fit([[0,1],[1,2]],model:a*(x/x),parameters:[a])",
        "fit([[0,1],[1,2]],model:a*x,parameters:[a],method:\"nonlinear\")",
        "fit([[0,4]],model:a^2,parameters:{a:0.1},method:\"nonlinear\",max_iterations:1,abs_tol:1e-14,rel_tol:0)",
        "fit([[0,1],[1,2]],model:a*x,parameters:[a],abs_tol:0.1)",
        "fit([[0,1],[1,3]],model:1e20+b-1e20,parameters:[b])",
    ] {
        ev.messages.take();
        eval(&mut ev, source);
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    assert_eq!(eval(&mut ev, "counter"), Expr::int(0));
    let parsed = om_parse::parse_expr(
        "fit([[0,1],[1,3],[2,5]],model:a*x+b,parameters:[a,b])",
        om_parse::Dialect::Modern,
    )
    .unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(50);
    assert!(ev.evaluate(&parsed, &ctx).is_err());
}
#[test]
fn fitted_model_data_roundtrip_sampling_and_symbolic_substitution_preserve_the_actual_model() {
    let mut ev = Evaluator::new();
    eval(
        &mut ev,
        "let report=fit([[0,1],[1,3],[2,5]],model:a*x+b,parameters:[a,b])",
    );
    eval(&mut ev, "let model=report.model");
    assert!(
        ev.defs
            .known_functions()
            .contains(&om_core::Symbol::intern("model"))
    );
    let table = eval(&mut ev, "sample(model,x:0..2,count:5)");
    assert!(table.is_head(B::DATA_TABLE));
    assert!((value(&eval(&mut ev, "diff(model(x),x)")) - 2.).abs() < 1e-12);
    let report = eval(&mut ev, "report");
    let raw = field(&report, "model");
    let source = om_format::input_form(raw);
    let parsed = om_parse::parse_expr(&source, om_parse::Dialect::Wolfram).unwrap();
    let value = ev
        .evaluate(
            &Expr::normal(parsed, [Expr::real(2.5)]),
            &Interrupt::default(),
        )
        .unwrap();
    assert!((self::value(&value) - 6.).abs() < 1e-12);
    eval(
        &mut ev,
        "let surface=fit([[0,0,0],[1,0,2],[0,1,3],[1,1,5]],model:a*x+b*y,parameters:[a,b],variables:[x,y])",
    );
    let symbolic = eval(&mut ev, "surface.model(y,x)");
    let free = symbolic.free_symbols();
    assert!(
        free.contains(&om_core::Symbol::intern("x"))
            && free.contains(&om_core::Symbol::intern("y"))
    );
}
#[test]
fn forged_model_source_cannot_execute_or_claim_a_precision_it_does_not_have() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let counter=0");
    for s in [
        "FittedModelData([x],sequence(assign(counter,9),x),\"machine\")(1)",
        "FittedModelData([x],x,\"exact\")(1)",
        "FittedModelData([x],x,\"machine\")(decimal(\"1\",precision:50))",
        "FittedModelData([x],x,\"machine\")([1,2])",
    ] {
        ev.messages.take();
        eval(&mut ev, s);
        assert!(!ev.messages.take().is_empty(), "{s}");
    }
    assert_eq!(eval(&mut ev, "counter"), Expr::int(0));
}
