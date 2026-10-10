//! Real immutable producer values and readonly views; no expectation derives from display fields.
use om_kernel::{
    Session,
    protocol::*,
    retained_results::{
        GeometryChannel, ResultSourceFormat, RetainedGeometryQuery, RetainedOutputKind,
    },
};
use om_num::ctx::Interrupt;
fn eval(session: &mut Session, id: &str, source: &str) -> CellOutput {
    let Response::Evaluated { output, .. } = session
        .handle(Request::Evaluate {
            cell_id: id.into(),
            source: source.into(),
            dialect: Dialect::Modern,
        })
        .0
    else {
        panic!("no real output")
    };
    output
}
fn query(cell: &str, out: u32, view: &str) -> ValueQuery {
    ValueQuery {
        cell_id: cell.into(),
        out_index: out,
        view_id: view.into(),
        path: Vec::new(),
        offset: 0,
        limit: 1,
        column_offset: 0,
        column_limit: 1,
        include_source: false,
    }
}
#[test]
fn exact_values_unicode_rows_and_occurrences_stay_immutable_across_main_edits() {
    let mut session = Session::new(Default::default(), None);
    eval(
        &mut session,
        "matrix",
        "[[1/3, decimal(\"0.1\",precision:80), \"中文🙂\"],[2,3,4]]",
    );
    let retained = session.retain_result("matrix").unwrap();
    let summary = retained.summary();
    let record = &summary.statements[0];
    assert_eq!(record.value_kind, ValueKind::Matrix);
    assert_eq!(record.output_kind, RetainedOutputKind::Expression);
    let mut q = query("matrix", record.out_index, &record.view_id);
    let first = retained.value_page(&q, &Interrupt::default()).unwrap();
    assert_eq!(first.rows[0].cells[0].nature, ValueNature::Exact);
    assert_eq!(
        first.rows[0].cells[0].source.as_ref().unwrap().input_form,
        "1/3"
    );
    q.column_offset = 1;
    let column = retained.value_page(&q, &Interrupt::default()).unwrap();
    assert_eq!(column.rows[0].id, first.rows[0].id);
    assert_eq!(column.rows[0].cells[0].nature, ValueNature::HighPrecision);
    q.column_offset = 2;
    assert_eq!(
        retained.value_page(&q, &Interrupt::default()).unwrap().rows[0].cells[0].nature,
        ValueNature::Text
    );
    eval(&mut session, "matrix", "[9]");
    assert_eq!(
        retained
            .value_source(
                record.out_index,
                &record.view_id,
                &[0, 0],
                ResultSourceFormat::InputForm,
                &Interrupt::default()
            )
            .unwrap(),
        "1/3"
    );
    q.cell_id = "other".into();
    assert!(retained.value_page(&q, &Interrupt::default()).is_err());
    q.cell_id = "matrix".into();
    q.view_id = "old-wrong-view".into();
    assert!(retained.value_page(&q, &Interrupt::default()).is_err());
    assert!(
        retained
            .value_source(
                record.out_index,
                &record.view_id,
                &[2],
                ResultSourceFormat::Modern,
                &Interrupt::default()
            )
            .is_err()
    );
}
#[test]
fn readonly_set_seed_and_hidden_function_effects_do_not_change_old_or_main_context() {
    let mut session = Session::new(Default::default(), None);
    session.handle(Request::Evaluate {
        cell_id: "a".into(),
        source: "a=2; SeedRandom[42]; bad[x_]:=(a=x); {a,a+1}".into(),
        dialect: Dialect::Wolfram,
    });
    let retained = session.retain_result("a").unwrap();
    let summary = retained.summary();
    let record = summary.statements.last().unwrap();
    assert_eq!(
        retained
            .readonly_expression("a", false, &Interrupt::default())
            .unwrap()
            .input_form,
        "2"
    );
    assert!(
        retained
            .readonly_expression("Set[a,9]", false, &Interrupt::default())
            .is_err()
    );
    assert!(
        retained
            .readonly_expression("SeedRandom[7]", false, &Interrupt::default())
            .is_err()
    );
    assert!(
        retained
            .readonly_expression("bad[9]", false, &Interrupt::default())
            .is_err()
    );
    let rand1 = retained
        .readonly_expression("RandomUniform[]", false, &Interrupt::default())
        .unwrap();
    let rand2 = retained
        .readonly_expression("RandomUniform[]", false, &Interrupt::default())
        .unwrap();
    assert_eq!(rand1.input_form, rand2.input_form);
    let out = retained
        .readonly_expression(
            &format!("Out[{}]", record.out_index),
            false,
            &Interrupt::default(),
        )
        .unwrap();
    assert_eq!(out.input_form, "{2, 3}");
    eval(&mut session, "a", "let a=5; a");
    assert_eq!(
        retained
            .readonly_expression("a", false, &Interrupt::default())
            .unwrap()
            .input_form,
        "2"
    );
    let current = eval(&mut session, "probe", "a");
    let OutputItem::Expr { input_form, .. } = &current.items[0] else {
        panic!("no scalar")
    };
    assert_eq!(input_form, "5");
    let current = eval(&mut session, "random", "random_uniform()");
    let OutputItem::Expr { input_form, .. } = &current.items[0] else {
        panic!("no random")
    };
    assert_eq!(input_form, &rand1.input_form);
}
#[test]
fn original_steps_are_paged_and_numeric_value_uses_the_actual_retained_value() {
    let mut session = Session::new(Default::default(), None);
    eval(&mut session, "solve", "solve(x^2-5*x+6==0,x)");
    let retained = session.retain_result("solve").unwrap();
    let summary = retained.summary();
    let record = &summary.statements[0];
    assert!(record.steps_recorded);
    let first = retained
        .steps_page(
            record.out_index,
            &record.view_id,
            &[],
            0,
            1,
            &Interrupt::default(),
        )
        .unwrap();
    assert!(first.recorded && first.total > 0);
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.entries[0].step.id, "S1");
    assert!(first.entries[0].step.children.is_empty());
    assert!(
        retained
            .steps_page(
                record.out_index,
                "wrong-view",
                &[],
                0,
                1,
                &Interrupt::default()
            )
            .is_err()
    );
    eval(&mut session, "rational", "1/7");
    let exact = session.retain_result("rational").unwrap();
    let desc = exact.summary();
    let r = &desc.statements[0];
    let projection = exact
        .numeric(r.out_index, &r.view_id, &[], 50, &Interrupt::default())
        .unwrap();
    assert!(projection.input_form.contains("0.142857"));
    assert_eq!(
        exact
            .value_source(
                r.out_index,
                &r.view_id,
                &[],
                ResultSourceFormat::InputForm,
                &Interrupt::default()
            )
            .unwrap(),
        "1/7"
    );
    let page = exact
        .steps_page(r.out_index, &r.view_id, &[], 0, 1, &Interrupt::default())
        .unwrap();
    assert!(!page.recorded && page.entries.is_empty());
}
#[test]
fn geometry_pages_equal_original_kernel_samples_without_any_resampling() {
    let mut session = Session::new(Default::default(), None);
    let original = eval(&mut session, "plot", "plot(sin(x),x:-pi..pi)");
    let OutputItem::Plot { data, .. } = &original.items[0] else {
        panic!("no actual plot")
    };
    let retained = session.retain_result("plot").unwrap();
    let desc = retained.summary();
    let r = &desc.statements[0];
    let summary = retained
        .geometry_summary(r.out_index, &r.view_id, &Interrupt::default())
        .unwrap();
    assert!(summary.available);
    assert_eq!(summary.kind, "plot2d");
    let page = retained
        .geometry_page(
            r.out_index,
            &r.view_id,
            &RetainedGeometryQuery {
                channel: GeometryChannel::CurvePoints,
                object_index: 0,
                segment_index: 0,
                offset: 0,
                limit: 2,
            },
            &Interrupt::default(),
        )
        .unwrap();
    assert_eq!(
        page.values,
        serde_json::to_value(&data.curves[0].segments[0][..2]).unwrap()
    );
    assert!(
        retained
            .geometry_page(
                r.out_index,
                &r.view_id,
                &RetainedGeometryQuery {
                    channel: GeometryChannel::MeshPositions,
                    object_index: 0,
                    segment_index: 0,
                    offset: 0,
                    limit: 2
                },
                &Interrupt::default()
            )
            .is_err()
    );
    let original = eval(
        &mut session,
        "sphere",
        "scene([sphere([0,0,0],1)],dimensions:3,mesh_points:8)",
    );
    let OutputItem::Scene3D {
        data: Some(data), ..
    } = &original.items[0]
    else {
        panic!("no actual scene")
    };
    let retained = session.retain_result("sphere").unwrap();
    let desc = retained.summary();
    let r = &desc.statements[0];
    let page = retained
        .geometry_page(
            r.out_index,
            &r.view_id,
            &RetainedGeometryQuery {
                channel: GeometryChannel::MeshPositions,
                object_index: 0,
                segment_index: 0,
                offset: 0,
                limit: 2,
            },
            &Interrupt::default(),
        )
        .unwrap();
    assert_eq!(
        page.values,
        serde_json::to_value(&data.meshes[0].positions[..2]).unwrap()
    );
    let indices = retained
        .geometry_page(
            r.out_index,
            &r.view_id,
            &RetainedGeometryQuery {
                channel: GeometryChannel::MeshTriangles,
                object_index: 0,
                segment_index: 0,
                offset: 0,
                limit: 2,
            },
            &Interrupt::default(),
        )
        .unwrap();
    assert_eq!(
        indices.values,
        serde_json::to_value(&data.meshes[0].triangles[..2]).unwrap()
    );
}
#[test]
fn held_diagnostics_and_initial_explore_geometry_remain_actual_retained_data() {
    let mut session = Session::new(Default::default(), None);
    eval(&mut session, "diagnostic", "let partial=7; gamma(-1)");
    let retained = session.retain_result("diagnostic").unwrap();
    let desc = retained.summary();
    assert!(desc.message_count > 0);
    assert_eq!(desc.statements.len(), 2);
    assert!(
        retained
            .readonly_expression("Gamma[-1]", false, &Interrupt::default())
            .is_err()
    );
    let original = eval(
        &mut session,
        "explore",
        "explore(plot(sin(a*x),x:-pi..pi),controls:{a:1..3})",
    );
    let OutputItem::Explore { result, .. } = &original.items[0] else {
        panic!("no real exploration")
    };
    let OutputItem::Plot { data, .. } = result.item.as_ref() else {
        panic!("no actual initial plot")
    };
    let retained = session.retain_result("explore").unwrap();
    let desc = retained.summary();
    let record = &desc.statements[0];
    assert!(
        retained
            .geometry_summary(record.out_index, &record.view_id, &Interrupt::default())
            .unwrap()
            .available
    );
    let page = retained
        .geometry_page(
            record.out_index,
            &record.view_id,
            &RetainedGeometryQuery {
                channel: GeometryChannel::CurvePoints,
                object_index: 0,
                segment_index: 0,
                offset: 0,
                limit: 1,
            },
            &Interrupt::default(),
        )
        .unwrap();
    assert_eq!(
        page.values,
        serde_json::to_value(&data.curves[0].segments[0][..1]).unwrap()
    );
}
