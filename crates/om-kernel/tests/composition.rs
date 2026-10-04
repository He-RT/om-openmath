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
