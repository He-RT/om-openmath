//! Two-dimensional scene output uses only true XY coordinates; no hidden third coordinate in PlotData.
use super::*;
fn rgb(c: [f64; 4]) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (c[0] * 255.).round() as u8,
        (c[1] * 255.).round() as u8,
        (c[2] * 255.).round() as u8
    )
}
fn range(lo: f64, hi: f64) -> Result<(f64, f64), PlotError> {
    let padding = if hi > lo {
        (hi - lo) * 0.05
    } else {
        lo.abs().max(1.) * 0.05
    };
    let r = (lo - padding, hi + padding);
    crate::plot::range(r)?;
    Ok(r)
}
pub(crate) fn from_expr(
    e: &Expr,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<(PlotRequest, PlotData), PlotError> {
    if dimensions(e, ev, ctx)? != 2 {
        return Err(error("二维scene必须dimensions:2"));
    }
    let scene = request(e, ev, ctx)?;
    let data = sample_request(&scene, ev, ctx)?;
    let request = PlotRequest {
        kind: PlotKind::Scene,
        exprs: scene.expressions,
        var_x: "x".into(),
        var_y: None,
        x_range: range(data.bounds.0[0], data.bounds.1[0])?,
        y_range: Some(range(data.bounds.0[1], data.bounds.1[1])?),
        params: Default::default(),
        points: vec![],
        shade: vec![],
        param_ranges: Default::default(),
        solve: None,
        options: None,
    };
    let plot = project(&request, data, ctx)?;
    Ok((request, plot))
}
pub(crate) fn sample(
    r: &PlotRequest,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    if r.exprs.len() != 1 || r.var_y.is_some() || r.params.len() > 64 {
        return Err(error("二维scene请求无效"));
    }
    let raw = crate::scene3d::raw(&r.exprs[0])?;
    let mut scene = request(&raw, ev, ctx)?;
    scene.parameters = r.params.clone();
    if dimensions(&raw, ev, ctx)? != 2 {
        return Err(error("不能把三维scene当二维采样"));
    }
    project(r, sample_request(&scene, ev, ctx)?, ctx)
}
fn project(r: &PlotRequest, data: Scene3DData, ctx: &Interrupt) -> Result<PlotData, PlotError> {
    crate::plot::range(r.x_range)?;
    let y = r.y_range.ok_or_else(|| error("scene二维视窗缺少纵轴"))?;
    crate::plot::range(y)?;
    let mut g = PlotGeometry2D {
        skipped: data.skipped,
        ..Default::default()
    };
    for mesh in data.meshes {
        for face in mesh.triangles {
            ctx.tick()?;
            let ids = face.map(|i| i as usize);
            let points = ids.map(|i| {
                let p = mesh.positions[i];
                (p[0], p[1])
            });
            let color = std::array::from_fn::<_, 4, _>(|k| {
                ids.iter().map(|i| mesh.colors[*i][k] / 3.).sum()
            });
            g.polygons.push(PlotPolygon {
                points: points.to_vec(),
                color: rgb(color),
                opacity: color[3],
            });
        }
    }
    for line in data.lines {
        ctx.tick()?;
        if line.colors.iter().all(|c| *c == line.colors[0]) {
            let c = line.colors[0];
            g.paths.push(PlotPath {
                points: line.positions.iter().map(|p| (p[0], p[1])).collect(),
                color: rgb(c),
                opacity: c[3],
                width: line.width,
            });
        } else {
            for (p, c) in line.positions.windows(2).zip(line.colors.windows(2)) {
                ctx.tick()?;
                let color = std::array::from_fn::<_, 4, _>(|k| (c[0][k] + c[1][k]) / 2.);
                g.paths.push(PlotPath {
                    points: vec![(p[0][0], p[0][1]), (p[1][0], p[1][1])],
                    color: rgb(color),
                    opacity: color[3],
                    width: line.width,
                });
            }
        }
    }
    for p in data.points {
        ctx.tick()?;
        g.markers.push(PlotMarker {
            position: (p.position[0], p.position[1]),
            color: rgb(p.color),
            opacity: p.color[3],
            radius: p.radius,
        });
    }
    for label in data.labels {
        ctx.tick()?;
        g.labels.push(PlotLabel {
            position: (label.position[0], label.position[1]),
            text: label.text,
            color: rgb(label.color),
            opacity: label.color[3],
            offset: label.offset,
        });
    }
    Ok(PlotData {
        curves: vec![],
        x_range: r.x_range,
        y_range: y,
        highlights: None,
        geometry: Some(g),
        scale: None,
    })
}
