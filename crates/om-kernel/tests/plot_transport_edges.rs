//! Actual source/domain/provenance and lifecycle edge cases for automatic plots.
pub mod support;
use om_kernel::{Session, protocol::*};
use om_num::ctx::Clock;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use support::*;
fn initial(s: &mut Session, src: &str) -> PlotRequest {
    let o = output(s, "solve", src, Dialect::Wolfram);
    let OutputItem::Solutions { plot: Some(p), .. } = &o.items[0] else {
        panic!("{o:?}")
    };
    p.clone()
}
fn sample(s: &mut Session, p: PlotRequest) -> PlotData {
    let r = s.handle(Request::SamplePlot { request: p }).0;
    let Response::Plot { data } = r else {
        panic!("{r:?}")
    };
    data
}
#[test]
fn coupled_defaults_and_original_branch_constraints_never_invent_out_of_domain_points() {
    let mut s = sequential();
    let p = initial(&mut s, "Solve[x==1/(a-b),x]");
    assert_eq!(p.params["a"], 2.0);
    assert_eq!(p.params["b"], 1.0);
    assert_eq!(p.points, [(1.0, 1.0)]);
    let mut p = initial(&mut s, "Solve[{x^2==a,x!=1},x,Reals]");
    assert_eq!(p.points, [(-1.0, 1.0)]);
    p.params.insert("a".into(), 4.0);
    assert_eq!(
        sample(&mut sequential(), p).highlights.unwrap().points,
        [(-2.0, 4.0), (2.0, 4.0)]
    );
}
#[test]
fn replay_is_independent_of_later_global_definitions_and_legacy_highlights_are_returned() {
    let mut s = sequential();
    output(&mut s, "a", "a=4", Dialect::Wolfram);
    let p = initial(&mut s, "Solve[x^2==a,x]");
    assert!(p.params.is_empty());
    output(&mut s, "a", "a=9", Dialect::Wolfram);
    let d = sample(&mut s, p.clone());
    assert_eq!(d.highlights.unwrap().points, [(-2.0, 4.0), (2.0, 4.0)]);
    let request: Request =
        serde_json::from_str(&serde_json::to_string(&Request::SamplePlot { request: p }).unwrap())
            .unwrap();
    let (r, _) = sequential().handle(request);
    let Response::Plot { data } = r else { panic!() };
    assert_eq!(data.highlights.unwrap().points, [(-2.0, 4.0), (2.0, 4.0)]);
    let p:PlotRequest=serde_json::from_value(serde_json::json!({"kind":"Function","exprs":["x"],"var_x":"x","var_y":null,"x_range":[-1.0,1.0],"y_range":null,"params":{},"points":[[0.0,0.0]],"shade":[[-1.0,0.0]],"param_ranges":{}})).unwrap();
    let d = sample(&mut s, p);
    assert_eq!(d.highlights.unwrap().shade, [(-1.0, 0.0)]);
}
#[test]
fn closed_named_roots_embedded_domains_and_multiple_real_system_points_use_actual_coordinates() {
    let mut s = sequential();
    let p = initial(&mut s, "Solve[x==Root[Function[t,t^2-2],2],x]");
    assert!(p.params.is_empty());
    assert!((p.points[0].0 - 2.0_f64.sqrt()).abs() < 1e-14);
    let mut p = initial(&mut s, "Solve[{x^2==a,Element[x,Reals]},x]");
    p.params.insert("a".into(), -1.0);
    assert!(sample(&mut s, p).highlights.unwrap().points.is_empty());
    let p = initial(&mut s, "Solve[{x^2+y^2==1,x-y==0},{x,y}]");
    assert_eq!(p.points.len(), 2);
    assert!(
        p.points
            .iter()
            .all(|(x, y)| (x - y).abs() < 1e-12 && (x * x + y * y - 1.0).abs() < 1e-12)
    );
}
struct Cancel {
    enabled: AtomicBool,
    calls: AtomicUsize,
    flag: Mutex<Option<Arc<AtomicBool>>>,
}
impl Clock for Cancel {
    fn now_ms(&self) -> f64 {
        if self.enabled.load(Ordering::Relaxed)
            && self.calls.fetch_add(1, Ordering::Relaxed) >= 2
            && let Some(flag) = self.flag.lock().unwrap().as_ref()
        {
            flag.store(true, Ordering::Relaxed);
        }
        0.0
    }
}
#[test]
fn parameter_solving_and_geometry_honor_cancellation_deadline_and_next_request_recovery() {
    let p = initial(&mut sequential(), "Solve[{x+y==a,x-y==0},{x,y}]");
    let clock = Arc::new(Cancel {
        enabled: AtomicBool::new(true),
        calls: Default::default(),
        flag: Mutex::new(None),
    });
    let mut s = Session::new(Default::default(), Some(clock.clone()));
    *clock.flag.lock().unwrap() = Some(s.interrupt_handle());
    let r = s.handle(Request::SamplePlot { request: p.clone() }).0;
    assert!(matches!(r,Response::Error{message} if message.contains("$Aborted")));
    assert!(s.notebook.cells.is_empty());
    clock.enabled.store(false, Ordering::Relaxed);
    let d = sample(&mut s, p.clone());
    assert_eq!(d.highlights.unwrap().points, [(0.5, 0.5)]);
    s.config.general.eval_timeout_ms = 0;
    assert!(
        matches!(s.handle(Request::SamplePlot{request:p.clone()}).0,Response::Error{message} if message.contains("time")||message.contains("时限"))
    );
    s.config.general.eval_timeout_ms = 30_000;
    sample(&mut s, p);
    let o = output(&mut s, "history", "1", Dialect::Wolfram);
    assert_eq!(expressions(&o)[0].0, 1);
}
