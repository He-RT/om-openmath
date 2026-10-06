//! Independent mathematical checks of actual 3D samples, original domains and genuine platform fallback.
use om_kernel::{Session, protocol::*};
fn evaluate(s: &mut Session, source: &str) -> CellOutput {
    let Response::Evaluated { output, .. } = s
        .handle(Request::Evaluate {
            cell_id: "scene".into(),
            source: source.into(),
            dialect: Dialect::Modern,
        })
        .0
    else {
        panic!()
    };
    output
}
fn scene(s: &mut Session, source: &str) -> (Scene3DRequest, Scene3DData) {
    let output = evaluate(s, source);
    assert!(output.messages.is_empty(), "{output:?}");
    let OutputItem::Scene3D {
        request,
        data: Some(data),
        unavailable: None,
    } = &output.items[0]
    else {
        panic!("{output:?}")
    };
    (request.clone(), data.clone())
}
#[test]
fn scalar_and_parameter_surfaces_are_actual_samples_with_independent_residuals_and_unit_normals() {
    let mut s = Session::new(Default::default(), None);
    let (_, data) = scene(
        &mut s,
        "plot(x^2-y^2,x:-2..2,y:-2..2,view:\"surface\",mesh_points:16)",
    );
    assert!(data.meshes[0].triangles.len() > 100);
    for p in &data.meshes[0].positions {
        assert!((p[2] - (p[0] * p[0] - p[1] * p[1])).abs() < 1e-12);
    }
    for n in &data.meshes[0].normals {
        assert!((n[0] * n[0] + n[1] * n[1] + n[2] * n[2] - 1.).abs() < 1e-12);
    }
    let (r, data) = scene(
        &mut s,
        "parametric_plot([cos(u)*sin(v),sin(u)*sin(v),cos(v)],u:0..2*pi,v:0..pi,mesh_points:16)",
    );
    assert_eq!(r.axes.len(), 2);
    for p in &data.meshes[0].positions {
        assert!((p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - 1.).abs() < 1e-12);
    }
}
#[test]
fn parameter_curve_and_readonly_color_return_real_coordinates_and_rgb_not_frontend_values() {
    let mut s = Session::new(Default::default(), None);
    let (_, data) = scene(
        &mut s,
        "parametric_plot([cos(t),sin(t),t],t:0..2*pi,color:fn(p,t)=>[0.1,0.5+0.3*cos(12*t),0.2])",
    );
    assert!(!data.lines.is_empty());
    for line in data.lines {
        for (p, c) in line.positions.iter().zip(line.colors) {
            assert!((p[0] * p[0] + p[1] * p[1] - 1.).abs() < 1e-12);
            assert!((c[1] - (0.5 + 0.3 * (12. * p[2]).cos())).abs() < 1e-12);
        }
    }
    let (_, data) = scene(&mut s, "parametric_plot([1,2,3],t:0..1)");
    assert_eq!(data.points[0].position, [1., 2., 3.]);
}
#[test]
fn implicit_sphere_has_true_edge_zeros_and_outward_normals_poles_are_not_fake_surfaces() {
    let mut s = Session::new(Default::default(), None);
    let (_, data) = scene(
        &mut s,
        "implicit_plot(x^2+y^2+z^2=1,x:-2..2,y:-2..2,z:-2..2,mesh_points:12)",
    );
    let mesh = &data.meshes[0];
    assert!(mesh.triangles.len() > 100);
    for p in &mesh.positions {
        assert!((p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - 1.).abs() < 1e-6);
    }
    for (p, n) in mesh.positions.iter().zip(&mesh.normals) {
        assert!(p[0] * n[0] + p[1] * n[1] + p[2] * n[2] > 0.7);
    }
    let output = evaluate(&mut s, "implicit_plot(1/x=0,x:-1..1,y:-1..1,z:-1..1)");
    assert!(
        output
            .items
            .iter()
            .any(|i| matches!(i, OutputItem::Error { .. }))
    );
}
#[test]
fn local_axes_do_not_capture_globals_and_ios_keeps_source_without_generating_mesh() {
    let mut s = Session::new(Default::default(), None);
    s.handle(Request::Evaluate {
        cell_id: "globals".into(),
        source: "let x=99;let y=88;let a=2".into(),
        dialect: Dialect::Modern,
    });
    let (r, data) = scene(&mut s, "plot(a*x+y,x:0..1,y:0..1)");
    assert_eq!(r.kind, SceneKind::Surface);
    for p in &data.meshes[0].positions {
        assert!((p[2] - (2. * p[0] + p[1])).abs() < 1e-12);
    }
    s.handle(Request::SetHostPlatform {
        platform: HostPlatform::Ios,
    });
    let output = evaluate(&mut s, "plot(sin(x)*cos(y),x:-pi..pi,y:-pi..pi)");
    assert!(
        matches!(&output.items[0],OutputItem::Scene3D{data:None,unavailable:Some(text),..}if text.contains("尚未适配"))
    );
    assert!(output.messages.is_empty());
    assert!(matches!(
        s.handle(Request::SampleScene3D { request: r }).0,
        Response::Error { .. }
    ));
}

#[test]
fn obj_contains_actual_world_vertices_unit_normals_faces_lines_and_original_colors() {
    let mut s = Session::new(Default::default(), None);
    let (_, data) = scene(
        &mut s,
        "parametric_plot([cos(u)*sin(v),sin(u)*sin(v),cos(v)],u:0..2*pi,v:0..pi,mesh_points:12,color:fn(p,u,v)=>[0.1,0.5+0.3*cos(12*u),0.2])",
    );
    let Response::Artifact { artifact } = s
        .handle(Request::ExportScene3D {
            data: data.clone(),
            title: "实际球面".into(),
        })
        .0
    else {
        panic!()
    };
    let text = String::from_utf8(om_kernel::artifact::decode(&artifact).unwrap()).unwrap();
    let vertices = text
        .lines()
        .filter(|line| line.starts_with("v "))
        .map(|line| {
            line.split_whitespace()
                .skip(1)
                .map(|x| x.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let normals = text
        .lines()
        .filter(|line| line.starts_with("vn "))
        .map(|line| {
            line.split_whitespace()
                .skip(1)
                .map(|x| x.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let faces = text
        .lines()
        .filter(|line| line.starts_with("f "))
        .map(|line| {
            line.split_whitespace()
                .skip(1)
                .map(|x| x.split_once("//").unwrap())
                .map(|(v, n)| {
                    (
                        v.parse::<usize>().unwrap() - 1,
                        n.parse::<usize>().unwrap() - 1,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(vertices.len(), data.meshes[0].positions.len());
    assert_eq!(faces.len(), data.meshes[0].triangles.len());
    for (i, v) in vertices.iter().enumerate() {
        assert_eq!(&v[..3], &data.meshes[0].positions[i]);
        assert_eq!(&v[3..], &data.meshes[0].colors[i][..3]);
        assert!((v[0] * v[0] + v[1] * v[1] + v[2] * v[2] - 1.).abs() < 1e-12);
    }
    for f in faces {
        assert_eq!(f.len(), 3);
        for (v, n) in f {
            assert!(v < vertices.len() && n < normals.len());
            assert!((normals[n].iter().map(|x| x * x).sum::<f64>() - 1.).abs() < 1e-12);
        }
    }
    let (_, data) = scene(&mut s, "parametric_plot([cos(t),sin(t),t],t:0..2*pi)");
    let Response::Artifact { artifact } = s
        .handle(Request::ExportScene3D {
            data,
            title: "螺线".into(),
        })
        .0
    else {
        panic!()
    };
    assert!(
        String::from_utf8(om_kernel::artifact::decode(&artifact).unwrap())
            .unwrap()
            .lines()
            .any(|l| l.starts_with("l "))
    );
}
#[test]
fn unsupported_precision_dimensions_bad_colors_and_readonly_writes_never_fabricate_success() {
    let mut s = Session::new(Default::default(), None);
    for source in [
        "plot(x+y,x:0..1,y:0..1,mesh_points:0)",
        "parametric_plot([x,y],x:0..1,y:0..1)",
        "plot(x+y,x:0..1,y:0..1,color:fn(p,x,y)=>[2,0,0])",
        "plot(x+y,x:0..1,y:0..1,color:fn(p,x,y)=>[assign(leak,9),0,0])",
        "plot(decimal(\"0.1\",precision:50)*x,x:0..1,y:0..1)",
        "plot(x+y,x:0..1,x:0..1)",
    ] {
        let output = evaluate(&mut s, source);
        assert!(
            !output.messages.is_empty()
                || output
                    .items
                    .iter()
                    .any(|i| matches!(i, OutputItem::Error { .. })),
            "{source}: {output:?}"
        );
    }
    let output = evaluate(&mut s, "leak");
    assert!(matches!(&output.items[0],OutputItem::Expr{input_form,..}if input_form=="leak"));
}
#[test]
fn frozen_exploration_changes_only_local_3d_geometry_and_ios_does_not_auto_sample() {
    let mut s = Session::new(Default::default(), None);
    s.handle(Request::Evaluate {
        cell_id: "global".into(),
        source: "let a=99".into(),
        dialect: Dialect::Modern,
    });
    let output = evaluate(
        &mut s,
        "explore(plot(a*x+y,x:0..1,y:0..1,mesh_points:8),controls:{a:0..4},initial:{a:2})",
    );
    let OutputItem::Explore {
        out_index,
        view_id,
        result,
        ..
    } = &output.items[0]
    else {
        panic!("{output:?}")
    };
    let OutputItem::Scene3D { request, .. } = &*result.item else {
        panic!()
    };
    let q = ExploreQuery {
        cell_id: "scene".into(),
        out_index: *out_index,
        view_id: view_id.clone(),
    };
    let Response::ExploreContext { context, .. } =
        s.handle(Request::GetExploreContext { query: q }).0
    else {
        panic!()
    };
    let Response::Scene3D { data } = Session::new(Default::default(), None)
        .handle(Request::SampleExploreScene3D {
            context: context.clone(),
            values: std::collections::BTreeMap::from([("a".into(), 4.)]),
            request: request.clone(),
        })
        .0
    else {
        panic!()
    };
    for p in &data.meshes[0].positions {
        assert!((p[2] - (4. * p[0] + p[1])).abs() < 1e-12);
    }
    let mut ios = Session::new(Default::default(), None);
    ios.handle(Request::SetHostPlatform {
        platform: HostPlatform::Ios,
    });
    let Response::Explored { result } = ios
        .handle(Request::RunExploreContext {
            context,
            values: std::collections::BTreeMap::from([("a".into(), 3.)]),
            revision: 3,
        })
        .0
    else {
        panic!()
    };
    assert!(matches!(
        &*result.item,
        OutputItem::Scene3D {
            data: None,
            unavailable: Some(_),
            ..
        }
    ));
}

#[test]
fn three_dimensional_sampling_shares_cancellation_budget_and_a_new_request_recovers() {
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
                && let Some(f) = self.flag.lock().unwrap().as_ref()
            {
                f.store(true, Ordering::Relaxed);
            }
            0.
        }
    }
    let mut producer = Session::new(Default::default(), None);
    let (request, _) = scene(
        &mut producer,
        "implicit_plot(x^2+y^2+z^2=1,x:-2..2,y:-2..2,z:-2..2,mesh_points:12)",
    );
    let clock = Arc::new(Cancel {
        enabled: AtomicBool::new(true),
        calls: AtomicUsize::new(0),
        flag: Mutex::new(None),
    });
    let mut s = Session::new(Default::default(), Some(clock.clone()));
    *clock.flag.lock().unwrap() = Some(s.interrupt_handle());
    assert!(
        matches!(s.handle(Request::SampleScene3D{request:request.clone()}).0,Response::Error{message}if message.contains("$Aborted"))
    );
    clock.enabled.store(false, Ordering::Relaxed);
    assert!(matches!(
        s.handle(Request::SampleScene3D { request }).0,
        Response::Scene3D { .. }
    ));
    assert!(s.notebook.cells.is_empty());
}
#[test]
fn existing_user_aliases_keep_precedence_and_all_3d_axes_are_lexically_local() {
    let mut s = Session::new(Default::default(), None);
    let response = s
        .handle(Request::Evaluate {
            cell_id: "function".into(),
            source: "let parametric_plot(x)=x+10".into(),
            dialect: Dialect::Modern,
        })
        .0;
    assert!(matches!(response, Response::Evaluated { .. }));
    let o = evaluate(&mut s, "parametric_plot(3)");
    assert!(matches!(&o.items[0],OutputItem::Expr{input_form,..}if input_form=="13"));
    let mut s = Session::new(Default::default(), None);
    s.handle(Request::Evaluate {
        cell_id: "params".into(),
        source: "let a=2;let x=99;let y=88;let z=77".into(),
        dialect: Dialect::Modern,
    });
    scene(
        &mut s,
        "implicit_plot(x^2+y^2+z^2=a,x:-2..2,y:-2..2,z:-2..2,mesh_points:12)",
    );
    let cell = s.notebook.cells.iter().find(|c| c.id == "scene").unwrap();
    assert!(cell.uses.iter().any(|s| s.name() == "a"));
    assert!(
        !cell
            .uses
            .iter()
            .any(|s| matches!(s.name(), "x" | "y" | "z"))
    );
}
