//! Real compiled/read-only mobile-safe ODE and interpolation operations, not fixture-generated values.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
fn eval(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, om_parse::Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn field<'a>(e: &'a Expr, key: &str) -> &'a Expr {
    &e.args()
        .iter()
        .find(|r| r.args().first() == Some(&Expr::string(key)))
        .unwrap_or_else(|| panic!("missing {key}: {e:?}"))
        .args()[1]
}
#[test]
fn actual_ode_solution_is_callable_dense_and_reports_genuine_work() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let solution=ode(fn(t,y)=>y,initial:1,t:0..1)");
    let value = eval(&mut ev, "solution.solution(0.37)");
    assert!(
        value.as_number().is_some(),
        "{value:?}; {:?}",
        ev.messages.take()
    );
    assert!((value.as_number().unwrap().to_f64().unwrap() - 0.37f64.exp()).abs() < 1e-7);
    let report = eval(&mut ev, "solution");
    assert_eq!(field(&report, "converged"), &Expr::sym(B::TRUE));
    assert_eq!(field(&report, "termination"), &Expr::string("end"));
    assert!(
        field(&report, "evaluations")
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap()
            > 0.
    );
    eval(
        &mut ev,
        "let harmonic=ode(fn(t,y)=>[y[2],-y[1]],initial:[1,0],t:0..3)",
    );
    let value = eval(&mut ev, "harmonic.solution(1.25)");
    assert!((value.args()[0].as_number().unwrap().to_f64().unwrap() - 1.25f64.cos()).abs() < 1e-7);
}
#[test]
fn interpolation_hermite_reverse_time_event_and_sampling_are_actual() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let f=interpolate([[0,0],[1,2],[2,4]])");
    assert_eq!(
        eval(&mut ev, "f(0.25)").as_number().unwrap().to_f64(),
        Some(0.5)
    );
    eval(
        &mut ev,
        "let g=interpolate([[0,0,0],[1,1,3]],method:\"hermite\")",
    );
    assert!(
        (eval(&mut ev, "g(0.37)")
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap()
            - 0.37f64.powi(3))
        .abs()
            < 1e-14
    );
    eval(
        &mut ev,
        "let back=ode(fn(t,y)=>y,initial:numeric(e),t:1..0)",
    );
    assert!(
        (eval(&mut ev, "back.solution(0.25)")
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap()
            - 0.25f64.exp())
        .abs()
            < 1e-7
    );
    let event = eval(
        &mut ev,
        "ode(fn(t,y)=>y,initial:1,t:0..2,event:fn(t,y)=>y-2)",
    );
    assert_eq!(field(&event, "termination"), &Expr::string("event"));
    assert!(
        (field(&event, "domain").args()[1]
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap()
            - 2f64.ln())
        .abs()
            < 1e-7
    );
    let table = eval(&mut ev, "sample(f,x:0..2,count:5)");
    assert!(table.is_head(B::DATA_TABLE));
    assert_eq!(table.args()[1].args().len(), 5);
    assert_eq!(
        table.args()[1].args()[2].args()[1].args()[1]
            .as_number()
            .unwrap()
            .to_f64(),
        Some(2.)
    );
}
#[test]
fn nonfinite_wrong_dimension_precision_mutation_and_extrapolation_fail_honestly() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let n=0");
    for s in [
        "ode(fn(t,y)=>[1,2],initial:[1],t:0..1)",
        "ode(fn(t,y)=>1/(t-0.5),initial:0,t:0..1)",
        "ode(fn(t,y)=>sequence(assign(n,1),y),initial:1,t:0..1)",
        "ode(fn(t,y)=>y,initial:decimal(\"1\",precision:50),t:0..1)",
        "ode(fn(t,y)=>y,initial:1,t:0..100,max_steps:1)",
        "interpolate([[0,1],[0,2]])",
        "interpolate([[0,1],[1,2]])(2)",
    ] {
        ev.messages.take();
        eval(&mut ev, s);
        assert!(!ev.messages.take().is_empty(), "{s}");
    }
    assert_eq!(eval(&mut ev, "n"), Expr::int(0));
    let p = om_parse::parse_expr(
        "ode(fn(t,y)=>y,initial:1,t:0..10)",
        om_parse::Dialect::Modern,
    )
    .unwrap();
    let ctx = Interrupt::default();
    ctx.steps_left.set(30);
    assert!(ev.evaluate(&p, &ctx).is_err());
}
#[test]
fn named_functions_parameters_local_time_and_vector_shape_roundtrip() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let t=70");
    eval(&mut ev, "let y=90");
    eval(&mut ev, "let rate=2");
    eval(&mut ev, "let rhs(t,y)=rate*y");
    let report = eval(&mut ev, "ode(rhs,initial:1,t:0..1,max_step:0.1)");
    let object = field(&report, "solution");
    let serialized = om_format::input_form(object);
    let decoded = om_parse::parse_expr(&serialized, om_parse::Dialect::Wolfram).unwrap();
    let call = Expr::normal(decoded, [Expr::real(0.25)]);
    let value = ev.evaluate(&call, &Interrupt::default()).unwrap();
    assert!((value.as_number().unwrap().to_f64().unwrap() - 0.5f64.exp()).abs() < 1e-7);
    assert_eq!(eval(&mut ev, "t"), Expr::int(70));
    assert_eq!(eval(&mut ev, "y"), Expr::int(90));
    eval(&mut ev, "let vector=interpolate([[0,[1]],[1,[3]]])");
    assert!(
        ev.defs
            .known_functions()
            .contains(&om_core::Symbol::intern("vector"))
    );
    let value = eval(&mut ev, "vector(0.5)");
    assert!(value.is_head(B::LIST));
    assert_eq!(value.args()[0].as_number().unwrap().to_f64(), Some(2.));
    let data = eval(&mut ev, "sample(fn(x)=>[x,x^2],x:1..0,count:3)");
    assert_eq!(
        data.args()[1].args()[0].args()[0].args()[1]
            .as_number()
            .unwrap()
            .to_f64(),
        Some(1.)
    );
    eval(&mut ev, "let rec={f:fn(x)=>x^2}");
    let call = eval(&mut ev, "rec.f(3)");
    assert_eq!(call, Expr::int(9));
}
#[test]
fn forged_interpolation_data_and_event_mutation_never_execute_as_saved_math() {
    let mut ev = Evaluator::new();
    eval(&mut ev, "let counter=0");
    for source in [
        "InterpolationData([0,1],[[0],[1]],[[[0,1,0,0]]],\"linear\",true,\"none\")(0.5)",
        "InterpolationData([0,1],[[0],[1]],[[[assign(counter,9),0,0,0]]],\"linear\",true,\"none\")(0.5)",
        "ode(fn(t,y)=>y,initial:1,t:0..1,event:fn(t,y)=>sequence(assign(counter,5),y))",
        "sample(fn(x)=>1/(x-0.5),x:0..1,count:3)",
        "ode(fn(t,y)=>y,initial:1,t:0..1,initial_step:0)",
    ] {
        let parsed = om_parse::parse_expr(source, om_parse::Dialect::Modern).unwrap();
        ev.messages.take();
        ev.evaluate(&parsed, &Interrupt::default()).unwrap();
        assert!(!ev.messages.take().is_empty(), "{source}");
    }
    assert!(
        om_parse::parse_expr(
            "interpolate([[0,0],[1,1]],precision:50)",
            om_parse::Dialect::Modern
        )
        .is_err()
    );
    assert_eq!(eval(&mut ev, "counter"), Expr::int(0));
}
#[test]
fn sixty_four_states_and_time_compile_together_without_shape_loss() {
    let initial = "1,".repeat(63) + "1";
    let mut ev = Evaluator::new();
    let report = eval(
        &mut ev,
        &format!("ode(fn(t,y)=>map(fn(v)=>t+v,y),initial:[{initial}],t:0..0.1)"),
    );
    let object = field(&report, "solution");
    let value = ev
        .evaluate(
            &Expr::normal(object.clone(), [Expr::real(0.1)]),
            &Interrupt::default(),
        )
        .unwrap();
    assert_eq!(value.args().len(), 64);
    for e in value.args() {
        assert!(
            (e.as_number().unwrap().to_f64().unwrap() - (2. * 0.1f64.exp() - 1.1)).abs() < 1e-8
        );
    }
}
