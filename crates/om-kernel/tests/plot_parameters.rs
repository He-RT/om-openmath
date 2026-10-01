//! Transported source/domain makes parameter re-solving independent of Session caches.
pub mod support;
use om_kernel::{Session, protocol::*};
use support::*;
fn plot(o: CellOutput) -> PlotRequest {
    let OutputItem::Solutions { plot: Some(p), .. } = &o.items[0] else {
        panic!("{o:?}")
    };
    p.clone()
}
fn initial(s: &mut Session, src: &str) -> PlotRequest {
    plot(output(s, "solve", src, Dialect::Wolfram))
}
fn sample(s: &mut Session, p: PlotRequest) -> PlotData {
    let response = s.handle(Request::SamplePlot { request: p }).0;
    let Response::Plot { data } = response else {
        panic!("{response:?}")
    };
    data
}
#[test]
fn parameter_default_real_nsolve_and_updated_points_survive_json_roundtrip_in_fresh_session() {
    let mut s = sequential();
    let mut p = initial(&mut s, "Solve[x^2==a,x]");
    assert_eq!(p.params["a"], 1.0);
    assert_eq!(p.param_ranges["a"], (-5.0, 5.0));
    assert_eq!(p.points, [(-1.0, 1.0), (1.0, 1.0)]);
    assert!(p.solve.is_some());
    p.params.insert("a".into(), 9.0);
    let p: PlotRequest = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    let mut fresh = sequential();
    let data = sample(&mut fresh, p);
    assert_eq!(data.highlights.unwrap().points, [(-3.0, 9.0), (3.0, 9.0)]);
    assert!(fresh.notebook.cells.is_empty());
    assert_eq!(
        expressions(&output(&mut fresh, "history", "1", Dialect::Wolfram))[0].0,
        1
    );
}
#[test]
fn parameter_poles_selected_domains_and_empty_updates_never_keep_stale_points() {
    let mut s = sequential();
    let mut p = initial(&mut s, "Solve[x==1/(a-1),x]");
    assert_eq!(p.params["a"], 2.0);
    assert_eq!(p.points, [(1.0, 1.0)]);
    p.params.insert("a".into(), 3.0);
    assert_eq!(
        sample(&mut s, p.clone()).highlights.unwrap().points,
        [(0.5, 0.5)]
    );
    p.params.insert("a".into(), 1.0);
    let Response::Error { .. } = s.handle(Request::SamplePlot { request: p }).0 else {
        panic!()
    };
    let mut p = initial(&mut s, "Solve[x^2==a,x,Reals]");
    p.params.insert("a".into(), -1.0);
    assert!(sample(&mut s, p).highlights.unwrap().points.is_empty());
    let mut p = initial(&mut s, "Solve[x==a/2,x,Integers]");
    assert_eq!(p.params["a"], 1.0);
    assert!(p.points.is_empty());
    p.params.insert("a".into(), 4.0);
    assert_eq!(sample(&mut s, p).highlights.unwrap().points, [(2.0, 2.0)]);
}
#[test]
fn parameterized_regions_update_actual_shading_and_two_axis_points_update_actual_assignments() {
    let mut s = sequential();
    let mut p = initial(&mut s, "Reduce[x^2<1,x,Reals]");
    assert_eq!(p.shade, [(-1.0, 1.0)]);
    p.solve.as_mut().unwrap().source = "x^2<a".into();
    p.exprs = vec!["x^2 - a".into()];
    p.params.insert("a".into(), 1.0);
    p.param_ranges.insert("a".into(), (-5.0, 5.0));
    p.params.insert("a".into(), 4.0);
    assert_eq!(sample(&mut s, p).highlights.unwrap().shade, [(-2.0, 2.0)]);
    let mut p = initial(&mut s, "Solve[{x+y==a,x-y==0},{x,y}]");
    assert_eq!(p.points, [(0.5, 0.5)]);
    p.params.insert("a".into(), 4.0);
    let data = sample(&mut s, p);
    assert_eq!(data.highlights.unwrap().points, [(2.0, 2.0)]);
}
#[test]
fn pole_exclusions_and_complete_parameter_bindings_are_validated_without_relying_on_cache() {
    let mut s = sequential();
    let mut p = initial(&mut s, "Solve[(x^2-a)/(x-1)==0,x,Reals]");
    assert_eq!(p.points, [(-1.0, 0.0)]);
    p.params.insert("a".into(), 4.0);
    let data = sample(&mut sequential(), p.clone());
    assert_eq!(data.highlights.unwrap().points, [(-2.0, 0.0), (2.0, 0.0)]);
    p.params.clear();
    assert!(matches!(
        s.handle(Request::SamplePlot { request: p }).0,
        Response::Error { .. }
    ));
    let mut p = initial(&mut s, "Solve[x^2==a,x]");
    p.exprs[0] = "x+100".into();
    assert!(matches!(
        s.handle(Request::SamplePlot { request: p }).0,
        Response::Error { .. }
    ));
}
#[test]
fn legacy_wire_shapes_are_preserved_and_new_optional_metadata_is_explicit() {
    let old = serde_json::json!({"kind":"Function","exprs":["x"],"var_x":"x","var_y":null,"x_range":[-1.0,1.0],"y_range":null,"params":{},"points":[],"shade":[],"param_ranges":{}});
    let p: PlotRequest = serde_json::from_value(old.clone()).unwrap();
    assert!(p.solve.is_none());
    assert_eq!(serde_json::to_value(p).unwrap(), old);
    let old = serde_json::json!({"curves":[],"x_range":[-1.0,1.0],"y_range":[-1.0,1.0]});
    let d: PlotData = serde_json::from_value(old.clone()).unwrap();
    assert!(d.highlights.is_none());
    assert_eq!(serde_json::to_value(d).unwrap(), old);
    let mut s = sequential();
    let p = initial(&mut s, "Solve[x^2==a,x]");
    let wire = serde_json::to_value(p).unwrap();
    assert_eq!(wire["solve"]["domain"], "Complexes");
    assert!(wire["solve"]["source"].as_str().unwrap().contains('a'));
}

#[test]
fn moving_complex_domain_parameters_out_of_real_roots_clears_highlights_in_successful_sampling() {
    let mut s = sequential();
    let mut p = initial(&mut s, "Solve[x^2==a,x]");
    p.params.insert("a".into(), -1.0);
    let d = sample(&mut s, p);
    assert!(d.highlights.unwrap().points.is_empty());
    assert_eq!(d.curves.len(), 2);
}

#[test]
fn one_invalid_parameter_branch_does_not_discard_a_valid_inequality_union_branch() {
    let mut s = sequential();
    let mut p = initial(&mut s, "Reduce[x<0 || x>1,x,Reals]");
    p.solve.as_mut().unwrap().source = "x<0 || x>1/(a-1)".into();
    p.exprs = vec!["x - 0".into(), "x - (a - 1)^(-1)".into()];
    p.params.insert("a".into(), 1.0);
    p.param_ranges.insert("a".into(), (-5.0, 5.0));
    let data = sample(&mut s, p);
    assert_eq!(data.highlights.unwrap().shade, [(-1e308, 0.0)]);
}
