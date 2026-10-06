//! Actual immutable result pages, precise representation and producer provenance over the wire.
use om_kernel::{Session, protocol::*};
fn evaluate(s: &mut Session, id: &str, src: &str) -> CellOutput {
    let (r, _) = s.handle(Request::Evaluate {
        cell_id: id.into(),
        source: src.into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { output, .. } = r else {
        panic!("{r:?}")
    };
    assert!(output.messages.is_empty(), "{output:?}");
    output
}
fn first(o: &CellOutput) -> (&ValuePage, u32) {
    let OutputItem::Expr {
        presentation: Some(p),
        out_index,
        ..
    } = &o.items[0]
    else {
        panic!("{o:?}")
    };
    (p, *out_index)
}
fn query(cell: &str, out: u32, page: &ValuePage) -> ValueQuery {
    ValueQuery {
        cell_id: cell.into(),
        out_index: out,
        view_id: page.view_id.clone(),
        path: page.path.clone(),
        offset: 0,
        limit: 32,
        column_offset: 0,
        column_limit: 8,
        include_source: false,
    }
}
#[test]
fn tables_page_real_rows_and_columns_keep_occurrence_ids_and_full_source() {
    let mut s = Session::new(Default::default(), None);
    let o = evaluate(&mut s, "table", "parse_json(\"[1,2,3]\")");
    let (p, index) = first(&o);
    assert_eq!(p.kind, ValueKind::List);
    assert_eq!(p.rows[0].cells[0].nature, ValueNature::Exact);
    let source = "parse_csv(\"α,备注\\n1,中文\\n2,emoji🙂\")";
    let o = evaluate(&mut s, "csv", source);
    let (p, index2) = first(&o);
    assert_eq!(p.row_count, 2);
    assert_eq!(p.columns, vec!["α", "备注"]);
    let mut q = query("csv", index2, p);
    q.column_limit = 1;
    q.offset = 1;
    let (reply, _) = s.handle(Request::InspectValue { query: q.clone() });
    let Response::ValuePage { page } = reply else {
        panic!("{reply:?}")
    };
    assert_eq!(page.rows[0].id, p.rows[1].id);
    q.column_offset = 1;
    q.include_source = true;
    let (reply, _) = s.handle(Request::InspectValue { query: q });
    let Response::ValuePage { page } = reply else {
        panic!("{reply:?}")
    };
    assert_eq!(
        page.rows[0].cells[0].source.as_ref().unwrap().input_form,
        "\"emoji🙂\""
    );
    assert!(page.source.unwrap().input_form.contains("DataTable"));
    assert!(index < index2);
}
#[test]
fn fresh_science_has_provenance_but_arbitrary_or_cached_records_do_not() {
    let mut s = Session::new(Default::default(), None);
    let o = evaluate(
        &mut s,
        "calc",
        "let result=optimize((x-2)^2,x,scope:\"global\")",
    );
    let (p, _) = first(&o);
    assert_eq!(p.origin.as_ref().unwrap().function_id, "fn_000179");
    let o = evaluate(&mut s, "cached", "result");
    assert!(first(&o).0.origin.is_none());
    let o = evaluate(
        &mut s,
        "fake",
        "{converged:true,guarantee:\"certified_global\",point:[99]}",
    );
    assert!(first(&o).0.origin.is_none());
    let o = evaluate(&mut s, "wrapped", "[optimize((x-2)^2,x,scope:\"global\")]");
    assert!(first(&o).0.origin.is_none());
}
#[test]
fn old_or_stale_snapshots_and_bad_paths_are_rejected_without_reexecution() {
    let mut s = Session::new(Default::default(), None);
    let o = evaluate(&mut s, "one", "[1,2,3]");
    let (p, out) = first(&o);
    let mut q = query("one", out, p);
    q.path = vec![500];
    assert!(matches!(
        s.handle(Request::InspectValue { query: q.clone() }).0,
        Response::Error { .. }
    ));
    q.path = vec![];
    evaluate(&mut s, "one", "[4,5,6]");
    assert!(matches!(
        s.handle(Request::InspectValue { query: q }).0,
        Response::Error { .. }
    ));
    let o = evaluate(&mut s, "danger", "Hold(assign(n,9))");
    let OutputItem::Expr { presentation, .. } = &o.items[0] else {
        panic!()
    };
    assert!(presentation.is_none());
    let o = evaluate(&mut s, "source", "let n=0; [hold(assign(n,9))]");
    let (p, out) = first(&o);
    let mut q = query("source", out, p);
    q.path = p.rows[0].cells[0].path.clone();
    q.include_source = true;
    let r = s.handle(Request::InspectValue { query: q }).0;
    assert!(matches!(r, Response::ValuePage { .. }));
    let o = evaluate(&mut s, "check", "n");
    assert!(matches!(&o.items[0],OutputItem::Expr{input_form,..} if input_form=="0"));
}
#[test]
fn scalar_wire_is_unchanged_and_empty_tables_preserve_columns() {
    let mut s = Session::new(Default::default(), None);
    let o = evaluate(&mut s, "scalar", "2");
    let value = serde_json::to_value(&o).unwrap();
    assert!(value["items"][0].get("presentation").is_none());
    let o = evaluate(&mut s, "empty", "parse_csv(\"x,y\")");
    let (p, _) = first(&o);
    assert_eq!(p.row_count, 0);
    assert_eq!(p.columns, vec!["x", "y"]);
    let o = evaluate(
        &mut s,
        "matrix",
        "[[1,decimal(\"0.1\",precision:50)],[2,3.0]]",
    );
    let (p, _) = first(&o);
    assert_eq!(p.kind, ValueKind::Matrix);
    assert_eq!(p.rows[0].cells[1].nature, ValueNature::HighPrecision);
    assert_eq!(p.rows[1].cells[1].nature, ValueNature::Machine);
}
