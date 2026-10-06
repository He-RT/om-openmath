//! Native/WASM shared session integration: real sampling, dependencies and source-only persistence.
use om_kernel::{KernelConfig, Session, protocol::*};
fn evaluate(s: &mut Session, id: &str, source: &str) -> (CellOutput, Vec<Event>) {
    let (response, events) = s.handle(Request::Evaluate {
        cell_id: id.into(),
        source: source.into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { output, .. } = response else {
        panic!("{response:?}")
    };
    (output, events)
}
#[test]
fn named_axis_plot_samples_genuine_kernel_data() {
    let mut s = Session::new(KernelConfig::default(), None);
    let (output, _) = evaluate(&mut s, "p", "plot(sin(x),x:-pi..pi)");
    assert!(output.messages.is_empty(), "{output:?}");
    assert!(
        output
            .items
            .iter()
            .any(|i| matches!(i, OutputItem::Plot { .. })),
        "{output:?}"
    );
    let (output, _) = evaluate(
        &mut s,
        "c",
        "plot(x^2+y^2=1,x:-2..2,y:-2..2,view:\"contour\")",
    );
    assert!(output.messages.is_empty(), "{output:?}");
    assert!(
        output
            .items
            .iter()
            .any(|i| matches!(i, OutputItem::Plot { .. })),
        "{output:?}"
    );
}
#[test]
fn lexical_closure_dependencies_recalculate_and_notebook_remains_source_only() {
    let mut s = Session::new(KernelConfig::default(), None);
    evaluate(&mut s, "a", "let a=2");
    evaluate(&mut s, "f", "let f=fn(x)=>fn(y)=>x+y+a");
    let (output, _) = evaluate(&mut s, "v", "f(1)(2)");
    assert!(
        output
            .items
            .iter()
            .any(|i| matches!(i,OutputItem::Expr {input_form,..} if input_form=="5"))
    );
    let (_, events) = evaluate(&mut s, "a", "let a=5");
    assert!(events.iter().any(|e|matches!(e,Event::CellOutput {cell_id,output} if cell_id=="v" && output.items.iter().any(|i|matches!(i,OutputItem::Expr {input_form,..} if input_form=="8")))),"{events:?}");
    let (r, _) = s.handle(Request::SaveNotebook);
    let Response::Notebook { file } = r else {
        panic!("{r:?}")
    };
    let json = serde_json::to_value(file).unwrap();
    assert_eq!(json["version"], 1);
    for cell in json["cells"].as_array().unwrap() {
        assert_eq!(cell.as_object().unwrap().len(), 4);
    }
    assert!(
        json["cells"][1]["source"]
            .as_str()
            .unwrap()
            .contains("fn(x)")
    );
}
#[test]
fn continuous_ode_dependencies_real_wire_values_and_source_only_saving() {
    fn last_number(output: &CellOutput) -> f64 {
        let source = output
            .items
            .iter()
            .filter_map(|i| match i {
                OutputItem::Expr { input_form, .. } => Some(input_form),
                _ => None,
            })
            .next_back()
            .expect("real numeric item");
        om_parse::parse_expr(source, om_parse::Dialect::Wolfram)
            .unwrap()
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap()
    }
    let mut s = Session::new(KernelConfig::default(), None);
    evaluate(&mut s, "rate", "let rate=1");
    let (report, _) = evaluate(
        &mut s,
        "solution",
        "let solution=ode(fn(t,y)=>rate*y,initial:1,t:0..1)",
    );
    assert!(report.messages.is_empty(), "{report:?}");
    assert!(
        (last_number(&evaluate(&mut s, "point", "solution.solution(0.5)").0) - 0.5f64.exp()).abs()
            < 1e-7
    );
    let (_, events) = evaluate(&mut s, "rate", "let rate=2");
    let actual = events
        .iter()
        .find_map(|e| match e {
            Event::CellOutput { cell_id, output } if cell_id == "point" => Some(output),
            _ => None,
        })
        .unwrap();
    assert!((last_number(actual) - std::f64::consts::E).abs() < 1e-7);
    let wire = serde_json::to_string(&Response::Evaluated {
        cell_id: "point".into(),
        output: actual.clone(),
        reran: vec![],
    })
    .unwrap();
    let round: Response = serde_json::from_str(&wire).unwrap();
    assert_eq!(serde_json::to_string(&round).unwrap(), wire);
    let (response, _) = s.handle(Request::SaveNotebook);
    let Response::Notebook { file } = response else {
        panic!("{response:?}")
    };
    let file = serde_json::to_value(file).unwrap();
    assert_eq!(file["version"], 1);
    let text = serde_json::to_string(&file).unwrap();
    assert!(text.contains("fn(t,y)"));
    assert!(!text.contains("InterpolationData"));
    for cell in file["cells"].as_array().unwrap() {
        assert_eq!(cell.as_object().unwrap().len(), 4);
    }
}
