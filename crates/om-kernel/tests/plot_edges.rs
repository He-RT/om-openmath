//! Domain/topology/validation/cancellation edges through actual plots.
pub mod support;
use om_core::Symbol;
use om_kernel::{Session, protocol::*};
use om_num::ctx::Clock;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use support::*;
fn r(kind: PlotKind, expr: &str, x: (f64, f64), y: Option<(f64, f64)>) -> PlotRequest {
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
        solve: None,
    }
}
fn sample(s: &mut Session, r: PlotRequest) -> PlotData {
    let Response::Plot { data } = s.handle(Request::SamplePlot { request: r }).0 else {
        panic!()
    };
    data
}
#[test]
fn shifted_poles_and_tangent_are_disconnected_without_splitting_smooth_steep_curves() {
    let mut s = sequential();
    let d = sample(
        &mut s,
        r(PlotKind::Function, "1/(x-0.123)", (-1.0, 1.0), None),
    );
    assert_eq!(d.curves[0].segments.len(), 2);
    assert!(
        d.curves[0]
            .segments
            .iter()
            .all(|p| p.iter().all(|(x, _)| *x < 0.123) || p.iter().all(|(x, _)| *x > 0.123))
    );
    let d = sample(&mut s, r(PlotKind::Function, "Tan[x]", (-2.0, 2.0), None));
    assert_eq!(d.curves[0].segments.len(), 3);
    let d = sample(&mut s, r(PlotKind::Function, "100000*x", (-1.0, 1.0), None));
    assert_eq!(d.curves[0].segments.len(), 1);
}
#[test]
fn implicit_components_saddles_and_no_zero_discontinuities_keep_actual_topology() {
    let mut s = sequential();
    let d = sample(
        &mut s,
        r(
            PlotKind::Implicit,
            "(x^2+y^2-1)*(x^2+y^2-4)",
            (-3.0, 3.0),
            Some((-3.0, 3.0)),
        ),
    );
    assert_eq!(d.curves[0].segments.len(), 2);
    for p in &d.curves[0].segments {
        assert_eq!(p.first(), p.last());
        let radius = p[0].0.hypot(p[0].1);
        assert!(p.iter().all(|(x, y)| (x.hypot(*y) - radius).abs() < 0.02));
    }
    let d = sample(
        &mut s,
        r(
            PlotKind::Implicit,
            "x*y-0.1",
            (-1.0, 1.0),
            Some((-1.0, 1.0)),
        ),
    );
    assert_eq!(d.curves[0].segments.len(), 2);
    assert!(
        d.curves[0]
            .segments
            .iter()
            .flatten()
            .all(|(x, y)| (x * y - 0.1).abs() < 0.005)
    );
    let d = sample(
        &mut s,
        r(
            PlotKind::Implicit,
            "1/(x-0.123)",
            (-1.0, 1.0),
            Some((-1.0, 1.0)),
        ),
    );
    assert!(d.curves[0].segments.is_empty());
}
#[test]
fn plot_local_axes_option_dependencies_and_reactive_sampling_are_real() {
    let mut s = Session::new(Default::default(), None);
    s.handle(Request::Evaluate {
        cell_id: "a".into(),
        source: "a=2".into(),
        dialect: Dialect::Wolfram,
    });
    let (o, _) = s.handle(Request::Evaluate {
        cell_id: "p".into(),
        source: "Plot[a*x,{x,-1,1},PlotRange->{-10,10}]".into(),
        dialect: Dialect::Wolfram,
    });
    assert!(matches!(o, Response::Evaluated { .. }));
    assert_eq!(s.notebook.cells[1].uses, [Symbol::intern("a")].into());
    let (o, e) = s.handle(Request::Evaluate {
        cell_id: "a".into(),
        source: "a=3".into(),
        dialect: Dialect::Wolfram,
    });
    let Response::Evaluated { reran, .. } = o else {
        panic!()
    };
    assert_eq!(reran, ["p"]);
    let Event::CellOutput {
        output: cell_output,
        ..
    } = e.last().unwrap()
    else {
        panic!()
    };
    let OutputItem::Plot { data, .. } = &cell_output.items[0] else {
        panic!()
    };
    assert_eq!(data.y_range, (-10.0, 10.0));
    assert!(
        data.curves[0]
            .segments
            .iter()
            .flatten()
            .all(|(x, y)| (y - 3.0 * x).abs() < 1e-12)
    );
    let o = output(
        &mut sequential(),
        "modern",
        "implicitplot(x^2+y^2=1,[x,-2,2],[y,-2,2])",
        Dialect::Modern,
    );
    assert!(matches!(o.items[0], OutputItem::Plot { .. }));
}
#[test]
fn all_range_axes_options_and_effectful_sources_fail_atomically() {
    let mut s = sequential();
    output(&mut s, "a", "a=2", Dialect::Wolfram);
    let mut bad = vec![
        r(PlotKind::Function, "Sin[x]", (1.0, -1.0), None),
        r(PlotKind::Function, "a=8", (-1.0, 1.0), None),
        r(PlotKind::Function, "x", (f64::NEG_INFINITY, 1.0), None),
        r(PlotKind::Function, "x", (-f64::MAX, f64::MAX), None),
    ];
    let mut req = r(PlotKind::Implicit, "x+y", (-1.0, 1.0), Some((-1.0, 1.0)));
    req.var_y = Some("x".into());
    bad.push(req);
    let mut req = r(PlotKind::Function, "x", (-1.0, 1.0), None);
    req.params.insert("x".into(), 1.0);
    bad.push(req);
    let mut req = r(PlotKind::Function, "x", (-1.0, 1.0), None);
    req.points.push((f64::NAN, 0.0));
    bad.push(req);
    for request in bad {
        assert!(matches!(
            s.handle(Request::SamplePlot { request }).0,
            Response::Error { .. }
        ));
        assert_eq!(s.notebook.cells.len(), 1);
    }
    for src in [
        "Plot[x,{x,1,1}]",
        "Plot[x,{Pi,0,1}]",
        "Plot[x,{x,0,1},Unknown->2]",
    ] {
        let o = output(&mut s, "invalid", src, Dialect::Wolfram);
        assert!(
            o.items
                .iter()
                .any(|i| matches!(i, OutputItem::Error { .. }))
        );
        assert_eq!(s.notebook.cells[1].status, CellStatus::Error);
    }
    let o = output(&mut s, "probe", "a", Dialect::Wolfram);
    assert_eq!(expressions(&o)[0].1, "2");
}
struct Cancel {
    enabled: AtomicBool,
    calls: AtomicUsize,
    handle: Mutex<Option<Arc<AtomicBool>>>,
}
impl Clock for Cancel {
    fn now_ms(&self) -> f64 {
        if self.enabled.load(Ordering::Relaxed)
            && self.calls.fetch_add(1, Ordering::Relaxed) > 2
            && let Some(flag) = self.handle.lock().unwrap().as_ref()
        {
            flag.store(true, Ordering::Relaxed);
        }
        0.0
    }
}
#[test]
fn injected_cancellation_stops_mid_sampling_and_next_request_recovers_without_history() {
    let clock = Arc::new(Cancel {
        enabled: AtomicBool::new(true),
        calls: Default::default(),
        handle: Mutex::new(None),
    });
    let mut s = Session::new(Default::default(), Some(clock.clone()));
    *clock.handle.lock().unwrap() = Some(s.interrupt_handle());
    let req = r(
        PlotKind::Implicit,
        "x^2+y^2==1",
        (-2.0, 2.0),
        Some((-2.0, 2.0)),
    );
    let Response::Error { message } = s
        .handle(Request::SamplePlot {
            request: req.clone(),
        })
        .0
    else {
        panic!()
    };
    assert!(message.contains("$Aborted"));
    assert!(s.notebook.cells.is_empty());
    clock.enabled.store(false, Ordering::Relaxed);
    let d = sample(&mut s, req);
    assert_eq!(d.curves[0].segments.len(), 1);
    let o = output(&mut s, "history", "1", Dialect::Wolfram);
    assert_eq!(expressions(&o)[0].0, 1);
}

#[test]
fn exact_grid_vertex_crossings_and_saddle_junctions_do_not_join_unrelated_branches() {
    let mut s = sequential();
    let d = sample(
        &mut s,
        r(PlotKind::Implicit, "x+y", (-1.0, 1.0), Some((-1.0, 1.0))),
    );
    assert_eq!(d.curves[0].segments.len(), 1);
    assert!(d.curves[0].segments[0].len() > 50);
    assert!(
        d.curves[0].segments[0]
            .iter()
            .all(|(x, y)| (x + y).abs() < 1e-12)
    );
    let d = sample(
        &mut s,
        r(
            PlotKind::Implicit,
            "(x-0.123)*(y-0.117)",
            (-1.0, 1.0),
            Some((-1.0, 1.0)),
        ),
    );
    assert!(!d.curves[0].segments.is_empty());
    assert!(
        d.curves[0]
            .segments
            .iter()
            .flatten()
            .all(|(x, y)| ((x - 0.123) * (y - 0.117)).abs() < 1e-12)
    );
}
#[test]
fn rendering_errors_preserve_completed_held_history_and_block_later_execution() {
    let mut s = sequential();
    let o = output(
        &mut s,
        "bad",
        "plot(unknown(x),[x,0,1])\nlet never=7",
        Dialect::Modern,
    );
    assert!(matches!(o.items[0], OutputItem::Error { .. }));
    assert_eq!(s.notebook.cells[0].exec_count, Some(1));
    assert_eq!(s.notebook.cells[0].status, CellStatus::Error);
    assert!(s.notebook.cells[0].input(1).is_some());
    let o = output(&mut s, "probe", "never", Dialect::Wolfram);
    assert_eq!(expressions(&o)[0], (2, "never".into()));
}

#[test]
fn nonconstant_tiny_curves_use_their_actual_quantile_viewport() {
    let mut s = sequential();
    let d = sample(
        &mut s,
        r(
            PlotKind::Function,
            "10^-15*Sin[x]",
            (0.0, std::f64::consts::TAU),
            None,
        ),
    );
    assert!(d.y_range.0 < -1e-15 && d.y_range.1 > 1e-15);
    assert!(d.y_range.1 - d.y_range.0 < 3e-15);
    assert_eq!(d.curves[0].segments.len(), 1);
}
