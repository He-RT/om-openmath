//! Plot protocol acceptance uses compiled real expressions and finite geometry.
pub mod support;
use om_kernel::{Session, protocol::*};
use support::{expressions, output, sequential};
fn request(kind: PlotKind, expr: &str, x: (f64, f64), y: Option<(f64, f64)>) -> PlotRequest {
    PlotRequest {
        kind,
        exprs: vec![expr.into()],
        var_x: "x".into(),
        var_y: (kind == PlotKind::Implicit).then(|| "y".into()),
        x_range: x,
        y_range: y,
        params: Default::default(),
        points: vec![],
        shade: vec![],
        param_ranges: Default::default(),
    }
}
fn sample(s: &mut Session, r: PlotRequest) -> PlotData {
    let (resp, events) = s.handle(Request::SamplePlot { request: r });
    assert!(events.is_empty());
    let Response::Plot { data } = resp else {
        panic!("{resp:?}")
    };
    data
}
fn finite(data: &PlotData) {
    assert!(
        data.y_range.0.is_finite() && data.y_range.1.is_finite() && data.y_range.0 < data.y_range.1
    );
    assert!(
        data.curves
            .iter()
            .flat_map(|c| &c.segments)
            .flatten()
            .all(|(x, y)| x.is_finite() && y.is_finite())
    );
    assert!(serde_json::to_string(data).is_ok());
}
#[test]
fn authority_sine_has_at_least_400_points_and_small_error() {
    let mut s = sequential();
    let data = sample(
        &mut s,
        request(
            PlotKind::Function,
            "Sin[x]",
            (0.0, std::f64::consts::TAU),
            None,
        ),
    );
    finite(&data);
    let points: Vec<_> = data.curves[0].segments.iter().flatten().copied().collect();
    assert!(points.len() >= 400);
    assert!(points.iter().all(|(x, y)| (y - x.sin()).abs() < 1e-3));
    assert_eq!(data.curves[0].segments.len(), 1);
    assert!(data.y_range.0 < -0.99 && data.y_range.1 > 0.99);
}
#[test]
fn poles_and_original_holes_split_without_connecting_invalid_domains() {
    let mut s = sequential();
    let data = sample(
        &mut s,
        request(PlotKind::Function, "1/x", (-1.0, 1.0), None),
    );
    finite(&data);
    assert_eq!(data.curves[0].segments.len(), 2);
    for segment in &data.curves[0].segments {
        assert!(segment.iter().all(|(x, _)| *x < 0.0) || segment.iter().all(|(x, _)| *x > 0.0));
    }
    let data = sample(
        &mut s,
        request(PlotKind::Function, "(x-1)/(x-1)", (0.0, 2.0), None),
    );
    assert_eq!(data.curves[0].segments.len(), 2);
    assert!(
        data.curves[0]
            .segments
            .iter()
            .flatten()
            .all(|(x, _)| *x != 1.0)
    );
    let data = sample(
        &mut s,
        request(PlotKind::Function, "Sqrt[x]", (-1.0, 1.0), None),
    );
    finite(&data);
    assert!(
        data.curves[0]
            .segments
            .iter()
            .flatten()
            .all(|(x, _)| *x >= 0.0)
    );
}
#[test]
fn authority_circle_is_a_stitched_closed_contour_with_small_residual() {
    let mut s = sequential();
    let data = sample(
        &mut s,
        request(
            PlotKind::Implicit,
            "x^2+y^2==1",
            (-2.0, 2.0),
            Some((-2.0, 2.0)),
        ),
    );
    finite(&data);
    let segments = &data.curves[0].segments;
    assert_eq!(segments.len(), 1);
    assert!(segments[0].len() > 100);
    assert_eq!(segments[0].first(), segments[0].last());
    assert!(
        segments[0]
            .iter()
            .all(|(x, y)| (x.hypot(*y) - 1.0).abs() < 0.02)
    );
}
#[test]
fn user_functions_assigned_axes_and_parameters_use_local_readonly_definitions() {
    let mut s = sequential();
    output(&mut s, "defs", "x=99;a=2;f[t_]:=a*t^2", Dialect::Wolfram);
    let mut r = request(PlotKind::Function, "f[x]", (-1.0, 1.0), Some((-1.0, 4.0)));
    r.params.insert("a".into(), 3.0);
    let data = sample(&mut s, r);
    finite(&data);
    assert_eq!(data.y_range, (-1.0, 4.0));
    assert!(
        data.curves[0]
            .segments
            .iter()
            .flatten()
            .all(|(x, y)| (y - 3.0 * x * x).abs() < 1e-12)
    );
    assert_eq!(
        expressions(&output(&mut s, "history", "x+a", Dialect::Wolfram))[0],
        (2, "101".into())
    );
}
#[test]
fn explicit_held_plots_and_suppression_keep_real_history_and_event_outputs() {
    let mut s = sequential();
    output(&mut s, "x", "x=99", Dialect::Wolfram);
    let o = output(&mut s, "plot", "Plot[Sin[x],{x,0,2*Pi}]", Dialect::Wolfram);
    let OutputItem::Plot { request, data } = &o.items[0] else {
        panic!("{o:?}")
    };
    assert_eq!(request.var_x, "x");
    finite(data);
    assert!(
        data.curves[0]
            .segments
            .iter()
            .flatten()
            .all(|(x, y)| (y - x.sin()).abs() < 1e-12)
    );
    let o = output(
        &mut s,
        "contour",
        "ContourPlot[x^2+y^2==1,{x,-2,2},{y,-2,2}]",
        Dialect::Wolfram,
    );
    assert!(matches!(o.items[0], OutputItem::Plot { .. }));
    let o = output(
        &mut s,
        "hidden",
        "plot(sin(x),[x,0,1]);\n5",
        Dialect::Modern,
    );
    assert_eq!(expressions(&o), [(5, "5".into())]);
    let o = output(&mut s, "history", "Out[2]", Dialect::Wolfram);
    assert!(matches!(o.items[0], OutputItem::Plot { .. }));
    assert_eq!(s.notebook.cells[4].exec_count, Some(6));
}
#[test]
fn invalid_requests_are_atomic_and_empty_nonreal_curves_have_finite_viewports() {
    let mut s = sequential();
    for mut r in [
        request(PlotKind::Function, "Sin[x]", (1.0, 1.0), None),
        request(PlotKind::Function, "bad[x]", (-1.0, 1.0), None),
        request(PlotKind::Implicit, "x+y", (-1.0, 1.0), None),
    ] {
        r.points = vec![];
        let Response::Error { message } = s.handle(Request::SamplePlot { request: r }).0 else {
            panic!()
        };
        assert!(message.contains("err.plot"));
        assert!(s.notebook.cells.is_empty());
    }
    let data = sample(
        &mut s,
        request(PlotKind::Function, "I*x", (-1.0, 1.0), None),
    );
    finite(&data);
    assert!(data.curves[0].segments.is_empty());
    let data = sample(&mut s, request(PlotKind::Function, "2", (-1.0, 1.0), None));
    finite(&data);
    assert!(data.y_range.0 < 2.0 && data.y_range.1 > 2.0);
}
