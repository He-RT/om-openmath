//! Independent geometry and transform invariants for actual composed scenes.
use om_kernel::{Session, protocol::*};
fn eval(s: &mut Session, source: &str) -> CellOutput {
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
fn two(s: &mut Session, source: &str) -> PlotData {
    let o = eval(s, source);
    assert!(o.messages.is_empty(), "{o:?}");
    let OutputItem::Plot { data, .. } = &o.items[0] else {
        panic!("{o:?}")
    };
    data.clone()
}
fn three(s: &mut Session, source: &str) -> Scene3DData {
    let o = eval(s, source);
    assert!(o.messages.is_empty(), "{o:?}");
    let OutputItem::Scene3D {
        data: Some(data), ..
    } = &o.items[0]
    else {
        panic!("{o:?}")
    };
    data.clone()
}
#[test]
fn real_xy_markers_paths_labels_disks_polygons_and_affine_transforms() {
    let mut s = Session::new(Default::default(), None);
    let d = two(
        &mut s,
        "scene([point([1,2],color:\"red\"),line([[0,0],[1,1]]),label(\"地球🙂\",[1,2]),translate(rotate(line([[0,0],[1,0]]),pi/2),[2,3]),disk([0,0],1),polygon([[0,0],[2,0],[2,2],[1,1],[0,2]])])",
    );
    let g = d.geometry.unwrap();
    assert_eq!(g.markers[0].position, (1., 2.));
    assert_eq!(g.markers[0].color, "#bc3838");
    assert_eq!(g.labels[0].text, "地球🙂");
    let p = &g.paths[1].points;
    assert_eq!(p[0], (2., 3.));
    assert!((p[1].0 - 2.).abs() < 1e-12 && (p[1].1 - 4.).abs() < 1e-12);
    let area = g
        .polygons
        .iter()
        .map(|p| {
            let q = &p.points;
            ((q[1].0 - q[0].0) * (q[2].1 - q[0].1) - (q[1].1 - q[0].1) * (q[2].0 - q[0].0)).abs()
                / 2.
        })
        .sum::<f64>();
    assert!((area - (3. + std::f64::consts::PI)).abs() < 0.02);
}
#[test]
fn actual_sphere_ellipsoid_box_cylinder_cone_tube_and_transformed_normals() {
    let mut s = Session::new(Default::default(), None);
    let d = three(
        &mut s,
        "scene([sphere([0,0,0],1),ellipsoid([3,0,0],[1,2,3]),box([5,0,0],[6,1,1]),cylinder([0,3,0],[0,3,2],0.3),cone([3,3,0],[3,3,2],0.4),tube([[5,3,0],[5,3,1],[6,3,2]],0.1)],dimensions:3,mesh_points:8)",
    );
    assert_eq!(d.meshes.len(), 6);
    for p in &d.meshes[0].positions {
        assert!((p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - 1.).abs() < 1e-12);
    }
    for p in &d.meshes[1].positions {
        assert!(((p[0] - 3.).powi(2) + p[1] * p[1] / 4. + p[2] * p[2] / 9. - 1.).abs() < 1e-12);
    }
    for mesh in &d.meshes {
        for n in &mesh.normals {
            assert!((n.iter().map(|v| v * v).sum::<f64>() - 1.).abs() < 1e-12);
        }
    }
    let d = three(
        &mut s,
        "scene([translate(scale(sphere([0,0,0],1),[-2,1,1]),[3,0,0])],dimensions:3,mesh_points:8)",
    );
    for (p, n) in d.meshes[0].positions.iter().zip(&d.meshes[0].normals) {
        let gradient = [(p[0] - 3.) / 4., p[1], p[2]];
        let norm = gradient.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(gradient.iter().zip(n).map(|(a, b)| a * b).sum::<f64>() / norm > 0.7);
    }
}

#[test]
fn zero_scale_imports_become_actual_points_or_lines_and_total_import_budget_is_checked() {
    let mut s = Session::new(Default::default(), None);
    let d = three(
        &mut s,
        "scene([scale(plot(x+y,x:0..1,y:0..1,mesh_points:8),0)],dimensions:3)",
    );
    assert!(d.meshes.is_empty());
    assert_eq!(d.points[0].position, [0.; 3]);
    let d = three(
        &mut s,
        "scene([scale(plot(x+y,x:0..1,y:0..1,mesh_points:8),[1,0,0])],dimensions:3)",
    );
    assert!(d.meshes.is_empty());
    assert_eq!(d.lines[0].positions, vec![[0., 0., 0.], [1., 0., 0.]]);
    let o = eval(
        &mut s,
        "scene(table(plot(x+y,x:0..1,y:0..1,mesh_points:16),[i,1,160]),dimensions:3)",
    );
    assert!(
        o.items
            .iter()
            .any(|v| matches!(v,OutputItem::Error{message,..} if message.contains("200000"))),
        "{o:?}"
    );
}

#[test]
fn invalid_shapes_unknown_options_and_readonly_nodes_fail_without_writing_the_session() {
    let mut s = Session::new(Default::default(), None);
    for source in [
        "scene([])",
        "scene([point([1,2,3])])",
        "scene([sphere([0,0,0],-1)],dimensions:3)",
        "scene([polygon([[0,0],[1,1],[0,1],[1,0]])])",
        "scene([polygon([[0,0,0],[1,0,0],[1,1,1],[0,1,0]])],dimensions:3)",
        "scene([rotate(point([0,0,0]),1,axis:[0,0,0])],dimensions:3)",
        "scene([cylinder([0,0,0],[0,0,0],1)],dimensions:3)",
        "scene([tube([[0,0,0],[1,0,0],[0,0,0]],0.1)],dimensions:3)",
        "scene([point([assign(scene_leak,7),0])])",
        "scene([point([0,0],color:fn(p)=>[assign(scene_leak,7),0,0])])",
        "scene([point([decimal(\"0.1\",precision:50),0])])",
        "scene([point([0,0],opacity:2)])",
        "scene([point([0,0])],dimensions:4)",
    ] {
        let o = eval(&mut s, source);
        assert!(
            !o.messages.is_empty()
                || o.items
                    .iter()
                    .any(|v| matches!(v, OutputItem::Error { .. })),
            "{source}: {o:?}"
        );
    }
    let o = eval(&mut s, "scene_leak");
    assert!(matches!(&o.items[0],OutputItem::Expr{input_form,..}if input_form=="scene_leak"));
}

#[test]
fn primitive_normal_directions_center_rotation_and_flattened_geometry_are_independently_correct() {
    let mut s = Session::new(Default::default(), None);
    let d = three(
        &mut s,
        "scene([cylinder([0,0,0],[0,0,2],1),cone([3,0,0],[3,0,2],1),tube([[6,0,0],[6,0,2]],1)],dimensions:3,mesh_points:8)",
    );
    for (i, mesh) in d.meshes.iter().enumerate() {
        for face in &mesh.triangles {
            let p = std::array::from_fn::<_, 3, _>(|axis| {
                face.iter()
                    .map(|i| mesh.positions[*i as usize][axis] / 3.)
                    .sum::<f64>()
            });
            let n = mesh.normals[face[0] as usize];
            let center = i as f64 * 3.;
            if n[2].abs() < 0.99 {
                assert!(
                    (p[0] - center) * n[0] + p[1] * n[1] > 0.5,
                    "{i}: {p:?} {n:?}"
                );
            }
        }
    }
    let d = two(
        &mut s,
        "scene([rotate(point([2,1]),pi/2,center:[1,1]),scale(circle([0,0],1),0)])",
    );
    let g = d.geometry.unwrap();
    assert!((g.markers[0].position.0 - 1.).abs() < 1e-12);
    assert!((g.markers[0].position.1 - 2.).abs() < 1e-12);
    assert!(g.markers.iter().any(|v| v.position == (0., 0.)));
}

#[test]
fn real_watermelon_fixture_has_striped_skin_rind_red_section_seeds_and_readable_obj_geometry() {
    let mut s = Session::new(Default::default(), None);
    let o = eval(&mut s, include_str!("../../../docs/examples/watermelon.om"));
    assert!(o.messages.is_empty(), "{o:?}");
    let Some(OutputItem::Scene3D { data: Some(d), .. }) = o.items.last() else {
        panic!("{o:?}")
    };
    assert_eq!(d.meshes.len(), 16);
    assert_eq!(d.labels.len(), 2);
    let skin = &d.meshes[0];
    let (lo, hi) = skin
        .colors
        .iter()
        .map(|v| v[1])
        .fold((1_f64, 0_f64), |(lo, hi), v| (lo.min(v), hi.max(v)));
    assert!(hi - lo > 0.27);
    assert!(d.meshes[1].positions.iter().all(|p| p[2] <= 1e-12));
    assert!(d.meshes[2].colors.iter().all(|c| c[1] > 0.8 && c[0] > 0.8));
    assert!(d.meshes[3].colors.iter().all(|c| c[0] > 0.9 && c[1] < 0.4));
    for mesh in &d.meshes[4..] {
        assert!(mesh.colors.iter().all(|c| c[0] < 0.3 && c[1] < 0.2));
        for p in &mesh.positions {
            assert!(((p[0] - 2.) / 1.25).powi(2) + p[1] * p[1] < 0.91_f64.powi(2));
        }
    }
    let response = s
        .handle(Request::ExportScene3D {
            data: d.clone(),
            title: "西瓜".into(),
        })
        .0;
    let Response::Artifact { artifact } = response else {
        panic!("{response:?}")
    };
    let text = String::from_utf8(om_kernel::artifact::decode(&artifact).unwrap()).unwrap();
    assert_eq!(
        text.lines().filter(|v| v.starts_with("v ")).count(),
        d.meshes.iter().map(|m| m.positions.len()).sum::<usize>()
    );
    assert_eq!(
        text.lines().filter(|v| v.starts_with("f ")).count(),
        d.meshes.iter().map(|m| m.triangles.len()).sum::<usize>()
    );
    assert_eq!(
        text.lines().filter(|v| v.starts_with("vn ")).count(),
        d.meshes.iter().map(|m| m.positions.len()).sum::<usize>()
    );
}

#[test]
fn real_earth_moon_l2_fixture_has_high_precision_residual_and_proportional_position_diagram() {
    let mut s = Session::new(Default::default(), None);
    let o = eval(
        &mut s,
        include_str!("../../../docs/examples/earth-moon-l2.om"),
    );
    assert!(o.messages.is_empty(), "{o:?}");
    let Some(OutputItem::Plot { data, .. }) = o.items.last() else {
        panic!("{o:?}")
    };
    let g = data.geometry.as_ref().unwrap();
    assert_eq!(g.markers.len(), 4);
    assert_eq!(g.labels.len(), 4);
    let earth = g.markers[0].position.0;
    let moon = g.markers[2].position.0;
    let l2 = g.markers[3].position.0;
    assert!((moon - earth - 384400.).abs() < 1e-8);
    assert!((l2 - 444244.22260083933).abs() < 1e-7);
    assert!((l2 - earth - 448914.9072421657).abs() < 1e-7);
    assert!((l2 - moon - 64514.907242165726).abs() < 1e-7);
    // Independent force balance in normalized barycentric coordinates.
    let mu = 4902.800118 / (398600.435507 + 4902.800118);
    let x = l2 / 384400.;
    assert!((x - (1. - mu) / (x + mu).powi(2) - mu / (x - 1. + mu).powi(2)).abs() < 1e-14);
    let Response::Evaluated { output, .. } = s
        .handle(Request::Evaluate {
            cell_id: "probe".into(),
            source: "l2_report.dimensionless_residual".into(),
            dialect: Dialect::Modern,
        })
        .0
    else {
        panic!()
    };
    let OutputItem::Expr { input_form, .. } = &output.items[0] else {
        panic!("{output:?}")
    };
    let e = om_parse::parse_expr(input_form, om_parse::Dialect::Wolfram).unwrap();
    assert!(e.as_number().unwrap().to_f64().unwrap().abs() < 1e-45);
    assert!(matches!(
        e.as_number().unwrap().precision(),
        om_num::Precision::Bits(_)
    ));
    s.handle(Request::SetHostPlatform {
        platform: HostPlatform::Ios,
    });
    let o = eval(&mut s, "scene([sphere([0,0,0],1)],dimensions:3)");
    assert!(
        matches!(&o.items[0],OutputItem::Scene3D{data:None,unavailable:Some(t),..}if t.contains("尚未适配"))
    );
}

#[test]
fn serialized_modern_symbol_names_survive_2d_3d_and_frozen_exploration_resampling() {
    let mut s = Session::new(Default::default(), None);
    s.handle(Request::Evaluate {
        cell_id: "definitions".into(),
        source: "let some_value=2;let my_function(t)=some_value*t".into(),
        dialect: Dialect::Modern,
    });
    let d = two(&mut s, "plot(my_function(axis_x),axis_x:0..1)");
    for curve in &d.curves {
        for segment in &curve.segments {
            for &(x, y) in segment {
                assert!((y - 2. * x).abs() < 1e-12);
            }
        }
    }
    let d = three(
        &mut s,
        "plot(my_function(axis_x)+axis_y,axis_x:0..1,axis_y:0..1,mesh_points:8)",
    );
    for p in &d.meshes[0].positions {
        assert!((p[2] - 2. * p[0] - p[1]).abs() < 1e-12);
    }
    let o = eval(
        &mut s,
        "explore(scene([point([slider_value,0])]),controls:{slider_value:0..5},initial:{slider_value:2})",
    );
    let OutputItem::Explore {
        out_index, view_id, ..
    } = &o.items[0]
    else {
        panic!("{o:?}")
    };
    let Response::ExploreContext { context, .. } = s
        .handle(Request::GetExploreContext {
            query: ExploreQuery {
                cell_id: "scene".into(),
                out_index: *out_index,
                view_id: view_id.clone(),
            },
        })
        .0
    else {
        panic!()
    };
    let Response::Explored { result } = Session::new(Default::default(), None)
        .handle(Request::RunExploreContext {
            context,
            values: std::collections::BTreeMap::from([("slider_value".into(), 4.)]),
            revision: 1,
        })
        .0
    else {
        panic!()
    };
    assert!(
        matches!(*result.item,OutputItem::Plot{data:PlotData{geometry:Some(ref g),..},..}if g.markers[0].position==(4.,0.))
    );
}

#[test]
fn style_inheritance_and_real_rgba_blending_preserve_alpha_and_validate_components() {
    let mut s = Session::new(Default::default(), None);
    let d = two(
        &mut s,
        "scene([style([point([0,0]),point([1,0],color:\"blue\")],color:blend([\"black\",\"white\"],0.5),opacity:0.4)])",
    );
    let g = d.geometry.unwrap();
    assert_eq!(g.markers[0].color, "#808080");
    assert_eq!(g.markers[1].color, "#3266b0");
    assert_eq!(g.markers[0].opacity, 0.4);
    assert_eq!(g.markers[1].opacity, 0.4);
    let d = three(
        &mut s,
        "scene([style(sphere([0,0,0],1),color:blend([[1,0,0,0.2],[0,0,1,0.8]],0.25),opacity:0.5)],dimensions:3,mesh_points:8)",
    );
    for c in &d.meshes[0].colors {
        assert_eq!(&c[..3], &[0.75, 0., 0.25]);
        assert!((c[3] - 0.175).abs() < 1e-12);
    }
    for source in [
        "blend([\"red\"],0.5)",
        "blend([\"red\",\"blue\"],2)",
        "blend([[2,0,0],\"blue\"],0.5)",
        "blend([\"red\",\"blue\"],decimal(\"0.5\",precision:50))",
        "scene([style(point([0,0]),width:5)])",
    ] {
        let o = eval(&mut s, source);
        assert!(
            !o.messages.is_empty()
                || o.items
                    .iter()
                    .any(|i| matches!(i, OutputItem::Error { .. })),
            "{source}: {o:?}"
        );
    }
}
