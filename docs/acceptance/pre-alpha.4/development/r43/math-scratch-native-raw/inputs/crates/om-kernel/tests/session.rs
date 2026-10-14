//! Real evaluator, source/history storage and provenance through Session.
/// Shared fixture helpers.
pub mod support;
use om_core::canonicalize;
use om_kernel::{
    config::{ConfigDialect, Constants},
    protocol::*,
};
use serde_json::json;
use support::*;
#[test]
fn evaluation_uses_the_real_evaluator_and_retains_cells_history_and_source_order() {
    let mut session = sequential();
    let (response, events) = request(
        &mut session,
        json!({"type":"evaluate","cell_id":"a",
        "source":"let a = 2;\na + 1","dialect":"Modern"}),
    );
    assert!(events.is_empty());
    let Response::Evaluated { output: first, .. } = response else {
        panic!()
    };
    assert_eq!(expressions(&first), vec![(2, "3".into())]);
    assert!(first.timing_ms.is_finite());
    assert_eq!(session.notebook.cells[0].source, "let a = 2;\na + 1");
    assert_eq!(session.notebook.cells[0].status, CellStatus::Done);
    assert_eq!(session.notebook.cells[0].exec_count, Some(2));
    assert!(session.notebook.cells[0].output.is_some());
    assert_eq!(
        expressions(&output(&mut session, "b", "a^2", Dialect::Modern)),
        vec![(3, "4".into())]
    );
    assert_eq!(
        expressions(&output(&mut session, "a", "a=9;\na", Dialect::Wolfram)),
        vec![(5, "9".into())]
    );
    assert_eq!(
        session
            .notebook
            .cells
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(
        expressions(&output(&mut session, "history", "Out[1]", Dialect::Wolfram)),
        vec![(6, "2".into())]
    );
    let expr_output = output(&mut session, "forms", "x^2+2*x", Dialect::Wolfram);
    let OutputItem::Expr {
        input_form,
        modern_form,
        latex,
        ..
    } = &expr_output.items[0]
    else {
        panic!()
    };
    same(input_form, "x^2+2*x");
    assert_eq!(
        canonicalize(&om_parse::parse_expr(modern_form, om_parse::Dialect::Modern).unwrap()),
        canonicalize(&om_parse::parse_expr(input_form, om_parse::Dialect::Wolfram).unwrap())
    );
    assert_eq!(
        latex,
        &om_format::latex(&canonicalize(
            &om_parse::parse_expr(input_form, om_parse::Dialect::Wolfram).unwrap()
        ))
    );
}

#[test]
fn solving_keeps_raw_poles_delayed_definitions_and_invalid_roots_inspectable() {
    let mut session = sequential();
    for (index, source, expected) in [
        ("p", "Solve[x^2==4,x]", "{{x->-2},{x->2}}"),
        ("hole", "Solve[(x^2-1)/(x-1)==0,x]", "{{x->-1}}"),
        ("all", "Solve[x/x==1,x]", "{ConditionalExpression[{},x!=0]}"),
        ("reduce", "Reduce[x^2<4,x]", "-2<x<2"),
    ] {
        let value = output(&mut session, index, source, Dialect::Wolfram);
        same(&expressions(&value)[0].1, expected);
        let cell = session.notebook.cells.last().unwrap();
        let out_index = expressions(&value)[0].0;
        assert_eq!(
            cell.input(out_index),
            Some(&om_parse::parse_expr(source, om_parse::Dialect::Wolfram).unwrap())
        );
        assert!(
            cell.steps(out_index)
                .is_some_and(|steps| !steps.root.is_empty())
        );
    }
    output(
        &mut session,
        "def",
        "f[t_]:=t/t; eq:=f[x]==1",
        Dialect::Wolfram,
    );
    let value = output(&mut session, "delayed", "Solve[eq,x]", Dialect::Wolfram);
    same(
        &expressions(&value)[0].1,
        "{ConditionalExpression[{},x!=0]}",
    );
    let invalid = output(
        &mut session,
        "root",
        "Solve[x==Root[Sin[#]&,1],x]",
        Dialect::Wolfram,
    );
    same(&expressions(&invalid)[0].1, "Solve[x==Root[Sin[#]&,1],x]");
    assert!(!invalid.messages.is_empty());
    let numeric = output(
        &mut session,
        "numeric",
        "FindRoot[x^2==2,{x,1}]",
        Dialect::Wolfram,
    );
    let expr =
        om_parse::parse_expr(&expressions(&numeric)[0].1, om_parse::Dialect::Wolfram).unwrap();
    let value = expr.args()[0].args()[1]
        .as_number()
        .unwrap()
        .to_f64()
        .unwrap();
    assert!((value * value - 2.0).abs() < 1e-12);
}

#[test]
fn parser_env_follows_actual_functions_constant_mode_and_configured_auto_dialect() {
    let mut session = sequential();
    output(&mut session, "def", "let f(t)=t+1", Dialect::Modern);
    let call = output(&mut session, "call", "f(2)", Dialect::Modern);
    assert_eq!(expressions(&call)[0].1, "3");
    assert!(call.messages.iter().all(|m| m.tag != "W002"));
    output(&mut session, "clear", "Clear[f]", Dialect::Wolfram);
    let ambiguous = output(&mut session, "ambiguous", "f (x)", Dialect::Modern);
    same(&expressions(&ambiguous)[0].1, "f*x");
    assert!(ambiguous.messages.iter().any(|m| m.tag == "W001"));
    let unknown = output(&mut session, "unknown", "f(x)", Dialect::Modern);
    same(&expressions(&unknown)[0].1, "f[x]");
    assert!(unknown.messages.iter().any(|m| m.tag == "W002"));
    session.config.general.constants = Constants::Strict;
    assert_eq!(
        expressions(&output(&mut session, "strict", "e+i", Dialect::Modern))[0].1,
        "e + i"
    );
    session.config.general.constants = Constants::Math;
    same(
        &expressions(&output(&mut session, "math", "e+i", Dialect::Modern))[0].1,
        "E+I",
    );
    session.config.general.dialect = ConfigDialect::Wolfram;
    assert_eq!(
        expressions(&output(&mut session, "auto", "a=2", Dialect::Auto))[0].1,
        "2"
    );
    session.config.general.dialect = ConfigDialect::Modern;
    let equation = output(&mut session, "modern", "b=2", Dialect::Auto);
    same(&expressions(&equation)[0].1, "b==2");
    assert_eq!(
        session.notebook.cells.last().unwrap().dialect,
        Dialect::Auto
    );
    assert_eq!(
        expressions(&output(&mut session, "override", "b=3", Dialect::Wolfram))[0].1,
        "3"
    );
}

#[test]
fn parse_preflight_is_atomic_and_suppression_never_loses_messages() {
    let mut session = sequential();
    let invalid = output(&mut session, "bad", "let a = 2;\nsolve(", Dialect::Modern);
    assert!(expressions(&invalid).is_empty());
    assert!(
        invalid
            .items
            .iter()
            .any(|i| matches!(i, OutputItem::Error { span: Some(_), .. }))
    );
    assert_eq!(session.notebook.cells[0].status, CellStatus::Error);
    assert_eq!(session.notebook.cells[0].exec_count, None);
    assert_eq!(
        expressions(&output(&mut session, "a", "a", Dialect::Modern)),
        vec![(1, "a".into())]
    );
    let suppressed = output(&mut session, "msg", "1/0;\n2", Dialect::Wolfram);
    assert_eq!(expressions(&suppressed), vec![(3, "2".into())]);
    assert!(suppressed.messages.iter().any(|m| m.tag == "infy"));
    assert_eq!(
        expressions(&output(&mut session, "last", "Out[2]", Dialect::Wolfram))[0].1,
        "Null"
    );
    let recover = output(&mut session, "bad", "4", Dialect::Modern);
    assert_eq!(expressions(&recover)[0].1, "4");
    assert_eq!(session.notebook.cells[0].status, CellStatus::Done);
    assert!(recover.messages.is_empty());
    let empty = output(&mut session, "empty", "# comment\n", Dialect::Modern);
    assert!(empty.items.is_empty());
    assert_eq!(session.notebook.cells.last().unwrap().exec_count, None);
    let arity = output(&mut session, "arity", "Solve[]", Dialect::Wolfram);
    assert!(arity.messages.iter().any(|m| m.level == MsgLevel::Warning));
    assert_eq!(
        session.notebook.cells.last().unwrap().status,
        CellStatus::Done
    );
}

#[test]
fn load_save_preserves_only_sources_and_resets_definitions_without_changing_config() {
    let mut session = sequential();
    session.config.llm.profiles[0].api_key = Some("test-secret".into());
    output(&mut session, "old", "a=7", Dialect::Wolfram);
    let file = json!({"version":1,"title":"Notebook α","cells":[
        {"id":"new","kind":"Math","source":"a","dialect":"Auto"},
        {"id":"text","kind":"Text","source":"# read me","dialect":"Modern"},
        {"id":"ask","kind":"Ask","source":"解方程","dialect":"Auto"}]});
    let (response, _) = request(&mut session, json!({"type":"load_notebook","file":file}));
    assert!(matches!(response, Response::Ok));
    assert_eq!(session.notebook.title, "Notebook α");
    assert_eq!(
        session
            .notebook
            .cells
            .iter()
            .map(|c| c.status)
            .collect::<Vec<_>>(),
        vec![CellStatus::Stale, CellStatus::Done, CellStatus::Stale]
    );
    assert!(
        session
            .notebook
            .cells
            .iter()
            .all(|c| c.output.is_none() && c.exec_count.is_none() && c.defines.is_empty())
    );
    assert_eq!(
        session.notebook.cells[0].uses,
        [om_core::Symbol::intern("a")].into()
    );
    assert!(
        session.notebook.cells[1..]
            .iter()
            .all(|c| c.uses.is_empty())
    );
    let (saved, events) = session.handle(Request::SaveNotebook);
    assert!(events.is_empty());
    let json = serde_json::to_value(saved).unwrap();
    assert_eq!(json, json!({"type":"notebook","file":file}));
    assert!(!json.to_string().contains("test-secret"));
    assert_eq!(
        session.config.llm.profiles[0].api_key.as_deref(),
        Some("test-secret")
    );
    assert_eq!(
        expressions(&output(&mut session, "new", "a", Dialect::Wolfram)),
        vec![(1, "a".into())]
    );
    for id in ["text", "ask"] {
        let (response, _) = session.handle(Request::Evaluate {
            cell_id: id.into(),
            source: "1+1".into(),
            dialect: Dialect::Modern,
        });
        assert!(matches!(response, Response::Error { .. }));
    }
    assert_eq!(session.notebook.cells[1].source, "# read me");
    assert_eq!(session.notebook.cells[2].source, "解方程");
    let baseline = serde_json::to_value(session.handle(Request::SaveNotebook).0).unwrap();
    for bad in [
        json!({"version":2,"title":"bad","cells":[]}),
        json!({"version":1,"title":"bad","cells":[file["cells"][0].clone(),file["cells"][0].clone()]}),
        json!({"version":1,"title":"bad","cells":[{"id":"","kind":"Math","source":"1","dialect":"Auto"}]}),
    ] {
        assert!(matches!(
            request(&mut session, json!({"type":"load_notebook","file":bad})).0,
            Response::Error { .. }
        ));
        assert_eq!(
            serde_json::to_value(session.handle(Request::SaveNotebook).0).unwrap(),
            baseline
        );
    }
    assert!(matches!(
        session
            .handle(Request::Evaluate {
                cell_id: "".into(),
                source: "1".into(),
                dialect: Dialect::Modern
            })
            .0,
        Response::Error { .. }
    ));
}

#[test]
fn each_statement_retains_real_steps_with_original_source_and_honors_recording() {
    let mut session = sequential();
    let source = "solve((x^2-1)/(x-1)=0,x);\n2";
    let result = output(&mut session, "steps", source, Dialect::Modern);
    assert_eq!(expressions(&result), vec![(2, "2".into())]);
    let cell = &session.notebook.cells[0];
    let steps = cell.steps(1).unwrap();
    assert!(!steps.root.is_empty());
    assert!(cell.steps(2).is_none());
    let original = &om_parse::parse(source, om_parse::Dialect::Modern).statements[0].expr;
    assert_eq!(cell.input(1), Some(original));
    session.config.general.show_steps = false;
    for call in [
        "Solve[x==2,x]",
        "NSolve[x==2,x]",
        "Reduce[x^2<4,x]",
        "FindRoot[x^2==2,{x,1}]",
    ] {
        let result = output(&mut session, "steps", call, Dialect::Wolfram);
        let index = expressions(&result)[0].0;
        assert!(session.notebook.cells[0].steps(index).is_none());
        assert!(session.notebook.cells[0].input(1).is_none());
    }
    session.config.general.show_steps = true;
    let result = output(&mut session, "steps", "Solve[x==2,x]", Dialect::Wolfram);
    assert!(
        session.notebook.cells[0]
            .steps(expressions(&result)[0].0)
            .is_some()
    );
}

#[test]
fn evaluator_failure_stops_later_statements_retains_earlier_effects_and_recovers() {
    let mut session = sequential();
    let failed = output(&mut session, "loop", "a=4;\nf:=f\nf\nb=5", Dialect::Wolfram);
    assert!(
        failed
            .items
            .iter()
            .any(|item| matches!(item, OutputItem::Error { .. }))
    );
    assert!(
        failed
            .messages
            .iter()
            .any(|message| message.tag == "evaluation")
    );
    assert_eq!(session.notebook.cells[0].status, CellStatus::Error);
    assert_eq!(session.notebook.cells[0].exec_count, Some(2));
    let earlier = output(&mut session, "earlier", "a+b", Dialect::Modern);
    same(&expressions(&earlier)[0].1, "4+b");
    output(&mut session, "clear", "Clear[f]", Dialect::Wolfram);
    let recovered = output(&mut session, "loop", "2+2", Dialect::Modern);
    assert_eq!(expressions(&recovered)[0].1, "4");
    assert!(recovered.messages.is_empty());
    assert_eq!(session.notebook.cells[0].status, CellStatus::Done);
}

#[test]
fn function_environment_includes_real_pure_functions_aliases_and_removes_cleared_values() {
    let mut session = sequential();
    output(
        &mut session,
        "defs",
        "pure=(#+1&); alias:=pure; sine:=Sin; u:=v; v:=u",
        Dialect::Wolfram,
    );
    for (id, source, expected) in [
        ("pure", "pure(2)", "3"),
        ("alias", "alias(2)", "3"),
        ("sine", "sine(0)", "0"),
    ] {
        let value = output(&mut session, id, source, Dialect::Modern);
        assert_eq!(expressions(&value)[0].1, expected);
        assert!(
            value.messages.iter().all(|message| message.tag != "W002"),
            "{source}: {:?}",
            value.messages
        );
    }
    output(&mut session, "clear", "Clear[pure]", Dialect::Wolfram);
    let value = output(&mut session, "alias", "alias(2)", Dialect::Modern);
    same(&expressions(&value)[0].1, "pure[2]");
    assert!(value.messages.iter().any(|message| message.tag == "W002"));
}
