//! Real shared-kernel 2D geometry with analytic residuals, count conservation and source-domain checks.
use om_kernel::{Session, protocol::*};
fn plot(s: &mut Session, id: &str, source: &str) -> (PlotRequest, PlotData) {
    let (r, _) = s.handle(Request::Evaluate {
        cell_id: id.into(),
        source: source.into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { output, .. } = r else {
        panic!("{r:?}")
    };
    assert!(output.messages.is_empty(), "{output:?}");
    let OutputItem::Plot { request, data } = &output.items[0] else {
        panic!("{output:?}")
    };
    (request.clone(), data.clone())
}
#[test]
fn parametric_circle_samples_the_actual_fixed_domain_and_does_not_rebind_it_on_pan() {
    let mut s = Session::new(Default::default(), None);
    let (mut request, data) = plot(
        &mut s,
        "circle",
        "parametric_plot([cos(t),sin(t)],t:0..2*pi)",
    );
    assert_eq!(request.kind, PlotKind::Parametric);
    let domain = request.options.as_ref().unwrap().axes[0].range;
    let points: Vec<_> = data.curves[0].segments.iter().flatten().collect();
    assert!(points.len() > 1000);
    for (x, y) in points {
        assert!((x * x + y * y - 1.).abs() < 1e-12);
    }
    request.x_range = (2., 3.);
    request.y_range = Some((4., 5.));
    let (r, _) = s.handle(Request::SamplePlot { request });
    let Response::Plot { data } = r else {
        panic!("{r:?}")
    };
    assert_eq!(data.x_range, (2., 3.));
    assert_eq!(data.y_range, (4., 5.));
    assert!(domain.1 > 6.);
}
#[test]
fn boolean_region_and_scalar_density_have_real_grid_values_not_fabricated_shapes() {
    let mut s = Session::new(Default::default(), None);
    let (_, data) = plot(&mut s, "disk", "region_plot(x^2+y^2<1,x:-2..2,y:-2..2)");
    let geometry = data.geometry.unwrap();
    let area: f64 = geometry
        .tiles
        .iter()
        .map(|t| (t.bounds.1.0 - t.bounds.0.0) * (t.bounds.1.1 - t.bounds.0.1))
        .sum();
    assert!((area - std::f64::consts::PI).abs() < 0.04);
    for tile in geometry.tiles {
        let x = (tile.bounds.0.0 + tile.bounds.1.0) / 2.;
        let y = (tile.bounds.0.1 + tile.bounds.1.1) / 2.;
        assert!(x * x + y * y < 1.);
    }
    let (_, data) = plot(
        &mut s,
        "density",
        "plot(x*y,x:-2..2,y:-2..2,view:\"density\")",
    );
    let g = data.geometry.unwrap();
    assert_eq!(g.tiles.len(), 96 * 96);
    for t in g.tiles {
        let x = (t.bounds.0.0 + t.bounds.1.0) * 0.5;
        let y = (t.bounds.0.1 + t.bounds.1.1) * 0.5;
        assert!((t.value - x * y).abs() < 1e-12);
        assert!(t.color.starts_with('#'));
    }
}
#[test]
fn vector_values_and_integral_curve_radius_are_independently_checked() {
    let mut s = Session::new(Default::default(), None);
    let (_, data) = plot(&mut s, "field", "field_plot([-y,x],x:-2..2,y:-2..2)");
    let g = data.geometry.unwrap();
    assert_eq!(g.arrows.len(), 400);
    for a in g.arrows {
        assert_eq!(a.value, (-a.start.1, a.start.0));
    }
    let (_, data) = plot(
        &mut s,
        "stream",
        "field_plot([-y,x],x:-2..2,y:-2..2,view:\"stream\")",
    );
    assert!(data.curves.len() > 50);
    for c in data.curves {
        let line = &c.segments[0];
        let r = line[0].0.hypot(line[0].1);
        for (x, y) in line {
            assert!((x.hypot(*y) - r).abs() < 1e-4);
        }
    }
}
#[test]
fn histogram_conserves_every_sample_and_data_plot_does_not_reorder_input() {
    let mut s = Session::new(Default::default(), None);
    let (_, data) = plot(&mut s, "h", "histogram([1,1,2,3,3],bins:3)");
    let bins = data.geometry.unwrap().tiles;
    assert_eq!(
        bins.iter().map(|t| t.value).collect::<Vec<_>>(),
        vec![2., 1., 2.]
    );
    assert_eq!(bins.iter().map(|t| t.value).sum::<f64>(), 5.);
    let (_, data) = plot(&mut s, "d", "data_plot([[2,4],[0,0],[1,1]],kind:\"line\")");
    assert_eq!(
        data.curves[0].segments[0],
        vec![(2., 4.), (0., 0.), (1., 1.)]
    );
    let (_, data) = plot(&mut s, "hm", "data_plot([[0,2],[4,6]],kind:\"heatmap\")");
    let g = data.geometry.unwrap();
    assert_eq!(g.color_range, Some((0., 6.)));
    assert_eq!(
        g.tiles.iter().map(|t| t.value).collect::<Vec<_>>(),
        vec![0., 2., 4., 6.]
    );
}
#[test]
fn named_scalar_contours_and_log_function_geometry_keep_original_coordinates() {
    let mut s = Session::new(Default::default(), None);
    let (_, data) = plot(
        &mut s,
        "levels",
        "implicit_plot(x^2+y^2,x:-2..2,y:-2..2,levels:[1])",
    );
    for c in data.curves {
        for (x, y) in c.segments.iter().flatten() {
            assert!((x * x + y * y - 1.).abs() < 0.003);
        }
    }
    let (_, data) = plot(&mut s, "log", "plot(x^2,x:1..1000,scale:\"log_log\")");
    assert_eq!(data.scale, Some(PlotScale::LogLog));
    for c in data.curves {
        for (x, y) in c.segments.iter().flatten() {
            assert!((*y - x * x).abs() < 1e-7);
        }
    }
}
#[test]
fn nonfinite_raw_poles_wrong_shapes_and_readonly_writes_are_not_successful_samples() {
    let mut s = Session::new(Default::default(), None);
    for source in [
        "parametric_plot([t,t,t],t:0..1)",
        "field_plot([x],x:-1..1,y:-1..1)",
        "data_plot([[1,2],[3]])",
        "histogram([1,2],bins:0)",
        "plot(x,x:-1..1,scale:\"log_x\")",
        "region_plot(assign(a,2),x:-1..1,y:-1..1)",
    ] {
        let (r, _) = s.handle(Request::Evaluate {
            cell_id: "bad".into(),
            source: source.into(),
            dialect: Dialect::Modern,
        });
        let Response::Evaluated { output, .. } = r else {
            panic!()
        };
        assert!(
            output
                .items
                .iter()
                .any(|i| matches!(i, OutputItem::Error { .. }))
                || !output.messages.is_empty(),
            "{source}: {output:?}"
        );
    }
    let (_, data) = plot(&mut s, "poles", "parametric_plot([t,1/(t-0.5)],t:0..1)");
    assert!(data.curves[0].segments.len() >= 2);
}

#[test]
fn histogram_camera_roundtrip_and_log_domain_filtering_do_not_change_real_counts() {
    let mut s = Session::new(Default::default(), None);
    let (mut request, _) = plot(&mut s, "h", "histogram([1,1,2,3,3],bins:3)");
    request.y_range = Some((1., 10.));
    let request: PlotRequest =
        serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
    let Response::Plot { data } = s.handle(Request::SamplePlot { request }).0 else {
        panic!()
    };
    assert_eq!(data.y_range, (1., 10.));
    assert_eq!(
        data.geometry
            .unwrap()
            .tiles
            .iter()
            .map(|t| t.value)
            .sum::<f64>(),
        5.
    );
    let (_, data) = plot(
        &mut s,
        "positive",
        "data_plot([[0,0],[1,2],[10,20]],scale:\"log_log\")",
    );
    assert!(data.x_range.0 > 0. && data.y_range.0 > 0.);
    let g = data.geometry.unwrap();
    assert_eq!(g.points.len(), 2);
    assert_eq!(g.skipped, 1);
    let (_, data) = plot(&mut s, "mixed", "plot(x,x:-2..2,scale:\"log_y\")");
    assert!(data.geometry.unwrap().skipped > 0);
    assert!(
        data.curves
            .iter()
            .flat_map(|c| &c.segments)
            .flatten()
            .all(|p| p.1 > 0.)
    );
}

#[test]
fn sampled_sources_remain_readonly_and_axes_are_local_with_reactive_parameter_dependencies() {
    let mut s = Session::new(Default::default(), None);
    for (id, source) in [
        ("globals", "let x=99;let y=88;let a=2"),
        ("curve", "field_plot([a*x,y],x:-1..1,y:-1..1)"),
    ] {
        let r = s
            .handle(Request::Evaluate {
                cell_id: id.into(),
                source: source.into(),
                dialect: Dialect::Modern,
            })
            .0;
        assert!(matches!(r, Response::Evaluated { .. }));
    }
    let (_, data) = plot(&mut s, "again", "field_plot([a*x,y],x:-1..1,y:-1..1)");
    for a in data.geometry.unwrap().arrows {
        assert_eq!(a.value, (2. * a.start.0, a.start.1));
    }
    let r = s
        .handle(Request::Evaluate {
            cell_id: "globals".into(),
            source: "let x=99;let y=88;let a=3".into(),
            dialect: Dialect::Modern,
        })
        .0;
    assert!(matches!(r, Response::Evaluated { .. }));
    let (_, data) = plot(&mut s, "again", "field_plot([a*x,y],x:-1..1,y:-1..1)");
    for a in data.geometry.unwrap().arrows {
        assert_eq!(a.value, (3. * a.start.0, a.start.1));
    }
    let r = s
        .handle(Request::Evaluate {
            cell_id: "badwrite".into(),
            source: "region_plot(assign(a,100)<x,x:-1..1,y:-1..1)".into(),
            dialect: Dialect::Modern,
        })
        .0;
    let Response::Evaluated { output, .. } = r else {
        panic!()
    };
    assert!(
        output
            .items
            .iter()
            .any(|i| matches!(i, OutputItem::Error { .. }))
    );
    let r = s
        .handle(Request::Evaluate {
            cell_id: "state".into(),
            source: "[x,y,a]".into(),
            dialect: Dialect::Modern,
        })
        .0;
    let Response::Evaluated { output, .. } = r else {
        panic!()
    };
    assert!(matches!(&output.items[0],OutputItem::Expr{input_form,..}if input_form=="{99, 88, 3}"));
}

#[test]
fn forged_transport_options_bad_precision_and_unsupported_palette_override_are_rejected() {
    let mut s = Session::new(Default::default(), None);
    for source in [
        "parametric_plot([t,t],t:0..decimal(\"1\",precision:50))",
        "plot(decimal(\"0.1\",precision:50)*x,x:0..1,color:\"red\")",
        "data_plot([[1,2],[3,4]],kind:\"heatmap\",color:\"red\")",
        "plot(x*y,x:0..1,y:0..1,view:\"density\",color:\"red\")",
        "field_plot([x,y],x:-1..1,y:-1..1,scal:\"linear\")",
    ] {
        let r = s
            .handle(Request::Evaluate {
                cell_id: "invalid".into(),
                source: source.into(),
                dialect: Dialect::Modern,
            })
            .0;
        let Response::Evaluated { output, .. } = r else {
            panic!()
        };
        assert!(
            !output.messages.is_empty()
                || output
                    .items
                    .iter()
                    .any(|i| matches!(i, OutputItem::Error { .. })),
            "{source}: {output:?}"
        );
    }
    let (mut request, _) = plot(&mut s, "valid", "parametric_plot([t,t^2],t:0..1)");
    request.options.as_mut().unwrap().axes.push(PlotAxis {
        name: "t".into(),
        range: (0., 1.),
    });
    assert!(matches!(
        s.handle(Request::SamplePlot { request }).0,
        Response::Error { .. }
    ));
}

#[test]
fn extended_sampling_cancels_without_partial_success_and_recovers_on_the_next_request() {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    struct Cancel {
        enabled: AtomicBool,
        calls: AtomicUsize,
        flag: Mutex<Option<Arc<AtomicBool>>>,
    }
    impl om_core::Clock for Cancel {
        fn now_ms(&self) -> f64 {
            if self.enabled.load(Ordering::Relaxed)
                && self.calls.fetch_add(1, Ordering::Relaxed) > 2
                && let Some(flag) = self.flag.lock().unwrap().as_ref()
            {
                flag.store(true, Ordering::Relaxed);
            }
            0.
        }
    }
    for source in [
        "region_plot(x^2+y^2<1,x:-2..2,y:-2..2)",
        "field_plot([-y,x],x:-2..2,y:-2..2,view:\"stream\")",
        "plot(x*y,x:-2..2,y:-2..2,view:\"density\")",
        "parametric_plot([cos(t),sin(t)],t:0..2*pi)",
    ] {
        let (request, _) = plot(
            &mut Session::new(Default::default(), None),
            "request",
            source,
        );
        let clock = Arc::new(Cancel {
            enabled: AtomicBool::new(true),
            calls: AtomicUsize::new(0),
            flag: Mutex::new(None),
        });
        let mut s = Session::new(Default::default(), Some(clock.clone()));
        *clock.flag.lock().unwrap() = Some(s.interrupt_handle());
        let response = s
            .handle(Request::SamplePlot {
                request: request.clone(),
            })
            .0;
        assert!(
            matches!(response,Response::Error{message} if message.contains("$Aborted")),
            "{source}"
        );
        assert!(s.notebook.cells.is_empty());
        clock.enabled.store(false, Ordering::Relaxed);
        assert!(matches!(
            s.handle(Request::SamplePlot { request }).0,
            Response::Plot { .. }
        ));
    }
}
