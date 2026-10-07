//! One display command stream from validated actual samples, shared by SVG and the CPU rasterizer.
use super::*;
use std::fmt::Write;
pub(super) type Point = (f64, f64);
#[derive(Clone, Copy, Debug)]
pub(super) struct Color(pub [u8; 3]);
#[derive(Clone, Copy)]
pub(super) enum Anchor {
    Left,
    Center,
    Right,
}
pub(super) enum Shape {
    Line(Point, Point, Color, f64),
    Rect(Point, Point, Color, f64),
    Circle(Point, f64, Color),
    Text(Point, String, f32, Color, Anchor),
    Polygon(Vec<Point>, Color, f64),
    StyledLine(Point, Point, Color, f64, f64),
    StyledMarker(Point, f64, Color, f64),
    StyledText(Point, String, f32, Color, f64),
}
pub(super) struct Drawing {
    pub width: u32,
    pub height: u32,
    pub shapes: Vec<Shape>,
    pub metadata: String,
}
pub(super) fn color(s: &str) -> Result<Color, ArtifactError> {
    om_core::graphics_color::rgb(s)
        .map(Color)
        .ok_or_else(|| invalid("导出颜色需要受支持颜色名或#RRGGBB"))
}
fn text(s: &str, max: usize) -> Result<(), ArtifactError> {
    if s.len() > max || s.chars().any(|c| c.is_control()) {
        return Err(invalid("图形文字超限或包含控制字符"));
    }
    Ok(())
}
fn range(r: (f64, f64), log: bool) -> Result<(), ArtifactError> {
    if !r.0.is_finite()
        || !r.1.is_finite()
        || r.0 >= r.1
        || !(r.1 - r.0).is_finite()
        || log && r.0 <= 0.
    {
        return Err(invalid("导出视窗需要有效有限范围，对数轴为正"));
    }
    Ok(())
}
fn tick_values(r: (f64, f64), log: bool) -> Vec<f64> {
    let (lo, hi) = if log { (r.0.log10(), r.1.log10()) } else { r };
    let raw = (hi - lo) / 5.;
    let base = 10_f64.powf(raw.log10().floor()).max(f64::MIN_POSITIVE);
    let fraction = raw / base;
    let step = (if fraction <= 1. {
        1.
    } else if fraction <= 2. {
        2.
    } else if fraction <= 5. {
        5.
    } else {
        10.
    }) * base;
    if !step.is_finite() || step <= 0. {
        return vec![r.0, r.1];
    }
    let start = (lo / step).ceil() * step;
    let mut out = vec![];
    for i in 0..32 {
        let value = start + i as f64 * step;
        if value > hi {
            break;
        }
        if value >= lo && value.is_finite() {
            out.push(if log { 10_f64.powf(value) } else { value });
        }
    }
    if out.is_empty() { vec![r.0, r.1] } else { out }
}
fn label(v: f64) -> String {
    if v == 0. {
        "0".into()
    } else if v.abs() >= 1e5 || v.abs() < 1e-3 {
        format!("{v:.2e}")
    } else {
        format!("{v:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .into()
    }
}
fn project(value: f64, r: (f64, f64), log: bool) -> f64 {
    let (value, lo, hi) = if log {
        (value.ln(), r.0.ln(), r.1.ln())
    } else {
        (value, r.0, r.1)
    };
    let magnitude = lo.abs().max(hi.abs()).max(f64::MIN_POSITIVE);
    ((value / magnitude - lo / magnitude) / (hi / magnitude - lo / magnitude)).clamp(-1e6, 1e6)
}
fn checked(p: Point, log_x: bool, log_y: bool) -> Result<(), ArtifactError> {
    if !p.0.is_finite() || !p.1.is_finite() || log_x && p.0 <= 0. || log_y && p.1 <= 0. {
        Err(invalid("导出数据包含非有限/对数域外坐标"))
    } else {
        Ok(())
    }
}
fn clip(a: Point, b: Point, rect: (Point, Point)) -> Option<(Point, Point)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let mut lo = 0_f64;
    let mut hi = 1_f64;
    for (p, q) in [
        (-dx, a.0 - rect.0.0),
        (dx, rect.1.0 - a.0),
        (-dy, a.1 - rect.0.1),
        (dy, rect.1.1 - a.1),
    ] {
        if p == 0. {
            if q < 0. {
                return None;
            }
            continue;
        }
        let t = q / p;
        if p < 0. {
            lo = lo.max(t);
        } else {
            hi = hi.min(t);
        }
        if lo > hi {
            return None;
        }
    }
    Some((
        (a.0 + lo * dx, a.1 + lo * dy),
        (a.0 + hi * dx, a.1 + hi * dy),
    ))
}
fn line(
    shapes: &mut Vec<Shape>,
    a: Point,
    b: Point,
    rect: (Point, Point),
    color: Color,
    width: f64,
) {
    if let Some((a, b)) = clip(a, b, rect) {
        shapes.push(Shape::Line(a, b, color, width));
    }
}
fn clipped_polygon(mut points: Vec<Point>, rect: (Point, Point)) -> Vec<Point> {
    for (axis, bound, lower) in [
        (0, rect.0.0, true),
        (0, rect.1.0, false),
        (1, rect.0.1, true),
        (1, rect.1.1, false),
    ] {
        if points.is_empty() {
            break;
        }
        let mut out = vec![];
        let coordinate = |p: Point| if axis == 0 { p.0 } else { p.1 };
        let inside = |p: Point| {
            if lower {
                coordinate(p) >= bound
            } else {
                coordinate(p) <= bound
            }
        };
        let mut a = *points.last().unwrap();
        for &b in &points {
            let (ai, bi) = (inside(a), inside(b));
            if ai != bi {
                let t = (bound - coordinate(a)) / (coordinate(b) - coordinate(a));
                out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
            if bi {
                out.push(b);
            }
            a = b;
        }
        points = out;
    }
    points
}
fn opacity(value: f64) -> Result<(), ArtifactError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        Err(invalid("图元opacity无效"))
    } else {
        Ok(())
    }
}
fn drawing(f: &PlotFigure, ctx: &Interrupt) -> Result<Drawing, ArtifactError> {
    if !(320..=2048).contains(&f.width)
        || !(240..=2048).contains(&f.height)
        || u64::from(f.width) * u64::from(f.height) > 2_000_000
    {
        return Err(invalid("图形尺寸需320..2048×240..2048且总像素≤2000000"));
    }
    text(&f.axis_x, 128)?;
    text(&f.axis_y, 128)?;
    text(&f.title, 512)?;
    if f.parameters.len() > 64 {
        return Err(invalid("参数数超限"));
    }
    for (name, value) in &f.parameters {
        text(name, 128)?;
        if !value.is_finite() {
            return Err(invalid("图形参数非有限"));
        }
    }
    let d = &f.data;
    let scale = d.scale.unwrap_or_default();
    let log_x = matches!(scale, PlotScale::LogX | PlotScale::LogLog);
    let log_y = matches!(scale, PlotScale::LogY | PlotScale::LogLog);
    range(d.x_range, log_x)?;
    range(d.y_range, log_y)?;
    if d.curves.len() > 2048 {
        return Err(invalid("导出曲线数超限"));
    }
    let rect = ((76., 58.), (f.width as f64 - 24., f.height as f64 - 52.));
    let point = |p: Point| {
        (
            rect.0.0 + project(p.0, d.x_range, log_x) * (rect.1.0 - rect.0.0),
            rect.1.1 - project(p.1, d.y_range, log_y) * (rect.1.1 - rect.0.1),
        )
    };
    let green = color("green")?;
    let ink = Color([42, 55, 48]);
    let grid = Color([219, 226, 220]);
    let grey = Color([98, 110, 103]);
    let fixed = f.color.as_deref().map(color).transpose()?;
    let mut shapes = vec![];
    let mut samples = 0usize;
    let mut count = |n: usize| -> Result<(), ArtifactError> {
        samples = samples.saturating_add(n);
        if samples > 200000 {
            return Err(invalid("导出几何超过200000项"));
        }
        ctx.tick()?;
        Ok(())
    };
    shapes.push(Shape::Text(
        (76., 14.),
        f.title.clone(),
        20.,
        ink,
        Anchor::Left,
    ));
    for x in tick_values(d.x_range, log_x) {
        let p = point((x, d.y_range.0));
        shapes.push(Shape::Line((p.0, rect.0.1), (p.0, rect.1.1), grid, 1.));
        shapes.push(Shape::Text(
            (p.0, rect.1.1 + 8.),
            label(x),
            13.,
            grey,
            Anchor::Center,
        ));
    }
    for y in tick_values(d.y_range, log_y) {
        let p = point((d.x_range.0, y));
        shapes.push(Shape::Line((rect.0.0, p.1), (rect.1.0, p.1), grid, 1.));
        shapes.push(Shape::Text(
            (64., p.1 - 8.),
            label(y),
            13.,
            grey,
            Anchor::Right,
        ));
    }
    if let Some(h) = &d.highlights {
        count(h.shade.len() + h.points.len())?;
        for &(a, b) in &h.shade {
            if !a.is_finite() || !b.is_finite() || a > b {
                return Err(invalid("高亮范围无效"));
            }
            let a = a.max(d.x_range.0);
            let b = b.min(d.x_range.1);
            if a < b {
                shapes.push(Shape::Rect(
                    (point((a, d.y_range.0)).0, rect.0.1),
                    (point((b, d.y_range.0)).0, rect.1.1),
                    green,
                    0.12,
                ));
            }
        }
    }
    if let Some(g) = &d.geometry {
        count(
            g.tiles.len()
                + g.arrows.len()
                + g.points.len()
                + g.paths.len()
                + g.polygons.len()
                + g.markers.len()
                + g.labels.len(),
        )?;
        for polygon in &g.polygons {
            count(polygon.points.len())?;
            opacity(polygon.opacity)?;
            if polygon.points.len() < 3 {
                return Err(invalid("polygon数据至少三点"));
            }
            for &p in &polygon.points {
                checked(p, log_x, log_y)?;
            }
            let points = clipped_polygon(polygon.points.iter().copied().map(point).collect(), rect);
            if points.len() >= 3 {
                shapes.push(Shape::Polygon(
                    points,
                    color(&polygon.color)?,
                    polygon.opacity,
                ));
            }
        }
        for path in &g.paths {
            count(path.points.len())?;
            opacity(path.opacity)?;
            if !path.width.is_finite() || !(0.0..=32.0).contains(&path.width) {
                return Err(invalid("path宽度无效"));
            }
            for &p in &path.points {
                checked(p, log_x, log_y)?;
            }
            for pair in path.points.windows(2) {
                if let Some((a, b)) = clip(point(pair[0]), point(pair[1]), rect) {
                    shapes.push(Shape::StyledLine(
                        a,
                        b,
                        color(&path.color)?,
                        path.width,
                        path.opacity,
                    ));
                }
            }
        }
        for marker in &g.markers {
            checked(marker.position, log_x, log_y)?;
            opacity(marker.opacity)?;
            if !marker.radius.is_finite() || !(0.0..=64.0).contains(&marker.radius) {
                return Err(invalid("marker半径无效"));
            }
            let p = point(marker.position);
            if p.0 >= rect.0.0 && p.0 <= rect.1.0 && p.1 >= rect.0.1 && p.1 <= rect.1.1 {
                shapes.push(Shape::StyledMarker(
                    p,
                    marker.radius,
                    color(&marker.color)?,
                    marker.opacity,
                ));
            }
        }
        for label in &g.labels {
            checked(label.position, log_x, log_y)?;
            opacity(label.opacity)?;
            text(&label.text, 1024)?;
            if !label.offset.0.is_finite() || !label.offset.1.is_finite() {
                return Err(invalid("label偏移无效"));
            }
            let p = point(label.position);
            shapes.push(Shape::StyledText(
                (p.0 + label.offset.0, p.1 + label.offset.1),
                label.text.clone(),
                14.,
                color(&label.color)?,
                label.opacity,
            ));
        }
        if g.color_range
            .is_some_and(|(a, b)| !a.is_finite() || !b.is_finite() || a > b)
        {
            return Err(invalid("颜色范围无效"));
        }
        for t in &g.tiles {
            checked(t.bounds.0, log_x, log_y)?;
            checked(t.bounds.1, log_x, log_y)?;
            if t.bounds.0.0 >= t.bounds.1.0 || t.bounds.0.1 > t.bounds.1.1 || !t.value.is_finite() {
                return Err(invalid("采样图元的边界/值无效"));
            }
            let a = point((t.bounds.0.0, t.bounds.1.1));
            let b = point((t.bounds.1.0, t.bounds.0.1));
            let a = (a.0.max(rect.0.0), a.1.max(rect.0.1));
            let b = (b.0.min(rect.1.0), b.1.min(rect.1.1));
            if a.0 < b.0 && a.1 < b.1 {
                shapes.push(Shape::Rect(
                    a,
                    b,
                    color(&t.color)?,
                    if f.region { 0.28 } else { 0.85 },
                ));
            }
        }
        for arrow in &g.arrows {
            checked(arrow.start, log_x, log_y)?;
            checked(arrow.end, log_x, log_y)?;
            checked(arrow.value, false, false)?;
            let (a, b) = (point(arrow.start), point(arrow.end));
            let color = fixed.unwrap_or(green);
            line(&mut shapes, a, b, rect, color, 1.);
            if a != b {
                let angle = (b.1 - a.1).atan2(b.0 - a.0);
                for sign in [-1., 1.] {
                    let p = (
                        b.0 - 5. * (angle + sign * 0.45).cos(),
                        b.1 - 5. * (angle + sign * 0.45).sin(),
                    );
                    line(&mut shapes, b, p, rect, color, 1.);
                }
            }
        }
        for &p in &g.points {
            checked(p, log_x, log_y)?;
            let p = point(p);
            if p.0 >= rect.0.0 && p.0 <= rect.1.0 && p.1 >= rect.0.1 && p.1 <= rect.1.1 {
                shapes.push(Shape::Circle(p, 3., fixed.unwrap_or(green)));
            }
        }
    }
    for (i, c) in d.curves.iter().enumerate() {
        text(&c.label, 65536)?;
        let color = fixed.unwrap_or(color(
            ["green", "blue", "orange", "purple", "red", "cyan"][i % 6],
        )?);
        for segment in &c.segments {
            count(segment.len())?;
            for &p in segment {
                checked(p, log_x, log_y)?;
            }
            for pair in segment.windows(2) {
                ctx.tick()?;
                line(&mut shapes, point(pair[0]), point(pair[1]), rect, color, 2.);
            }
        }
    }
    if let Some(h) = &d.highlights {
        for &p in &h.points {
            checked(p, log_x, log_y)?;
            let p = point(p);
            if p.0 >= rect.0.0 && p.0 <= rect.1.0 && p.1 >= rect.0.1 && p.1 <= rect.1.1 {
                shapes.push(Shape::Circle(p, 4., green));
            }
        }
    }
    for (a, b) in [
        (rect.0, (rect.1.0, rect.0.1)),
        ((rect.1.0, rect.0.1), rect.1),
        (rect.1, (rect.0.0, rect.1.1)),
        ((rect.0.0, rect.1.1), rect.0),
    ] {
        shapes.push(Shape::Line(a, b, grey, 1.));
    }
    shapes.push(Shape::Text(
        (rect.1.0, f.height as f64 - 25.),
        f.axis_x.clone(),
        15.,
        ink,
        Anchor::Right,
    ));
    shapes.push(Shape::Text(
        (76., 36.),
        f.axis_y.clone(),
        15.,
        ink,
        Anchor::Left,
    ));
    let metadata=serde_json::to_string(&serde_json::json!({"format_version":1,"kernel_version":env!("CARGO_PKG_VERSION"),"figure":f})).map_err(|_|invalid("图形元数据编码失败"))?;
    if metadata.len() > 8 * 1024 * 1024 {
        return Err(invalid("图形数据超过8MiB"));
    }
    Ok(Drawing {
        width: f.width,
        height: f.height,
        shapes,
        metadata,
    })
}
fn escaped(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn svg(d: &Drawing, ctx: &Interrupt) -> Result<Vec<u8>, ArtifactError> {
    let mut text = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><metadata>{}</metadata><rect width=\"100%\" height=\"100%\" fill=\"white\"/>",
        d.width,
        d.height,
        d.width,
        d.height,
        escaped(&d.metadata)
    );
    for shape in &d.shapes {
        ctx.tick()?;
        match shape{
        Shape::Line(a,b,c,w)=>write!(text,"<path d=\"M{:.3} {:.3}L{:.3} {:.3}\" fill=\"none\" stroke=\"#{:02x}{:02x}{:02x}\" stroke-width=\"{w}\"/>",a.0,a.1,b.0,b.1,c.0[0],c.0[1],c.0[2]),
        Shape::Rect(a,b,c,o)=>write!(text,"<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{o}\"/>",a.0,a.1,b.0-a.0,b.1-a.1,c.0[0],c.0[1],c.0[2]),
        Shape::Circle(p,r,c)=>write!(text,"<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{r}\" fill=\"#{:02x}{:02x}{:02x}\"/>",p.0,p.1,c.0[0],c.0[1],c.0[2]),
        Shape::Polygon(points,c,a)=>write!(text,"<polygon points=\"{}\" fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{a}\"/>",points.iter().map(|p|format!("{:.3},{:.3}",p.0,p.1)).collect::<Vec<_>>().join(" "),c.0[0],c.0[1],c.0[2]),
        Shape::StyledLine(p,q,c,w,a)=>write!(text,"<path d=\"M{:.3} {:.3}L{:.3} {:.3}\" fill=\"none\" stroke=\"#{:02x}{:02x}{:02x}\" stroke-width=\"{w}\" stroke-opacity=\"{a}\"/>",p.0,p.1,q.0,q.1,c.0[0],c.0[1],c.0[2]),
        Shape::StyledMarker(p,r,c,a)=>write!(text,"<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{r}\" fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{a}\"/>",p.0,p.1,c.0[0],c.0[1],c.0[2]),
        Shape::StyledText(p,s,size,c,a)=>write!(text,"<text x=\"{:.3}\" y=\"{:.3}\" dominant-baseline=\"hanging\" font-family=\"sans-serif\" font-size=\"{size}\" fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{a}\">{}</text>",p.0,p.1,c.0[0],c.0[1],c.0[2],escaped(s)),
        Shape::Text(p,s,size,c,anchor)=>write!(text,"<text x=\"{:.3}\" y=\"{:.3}\" dominant-baseline=\"hanging\" text-anchor=\"{}\" font-family=\"sans-serif\" font-size=\"{size}\" fill=\"#{:02x}{:02x}{:02x}\">{}</text>",p.0,p.1,match anchor {Anchor::Left=>"start",Anchor::Center=>"middle",Anchor::Right=>"end"},c.0[0],c.0[1],c.0[2],escaped(s)),
    }.map_err(|_|invalid("SVG格式化失败"))?;
        if text.len() > 16 * 1024 * 1024 {
            return Err(invalid("SVG超过16MiB"));
        }
    }
    text.push_str("</svg>");
    Ok(text.into_bytes())
}
/// Export already sampled coordinates; never re-run the source function or any user definitions.
pub fn export_plot(
    figure: &PlotFigure,
    format: PlotExportFormat,
    ctx: &Interrupt,
) -> Result<Artifact, ArtifactError> {
    let drawing = drawing(figure, ctx)?;
    let (bytes, mime, ext) = match format {
        PlotExportFormat::Svg => (svg(&drawing, ctx)?, "image/svg+xml", "svg"),
        PlotExportFormat::Png => (
            super::png::encode(&drawing, super::raster::render(&drawing, ctx)?, ctx)?,
            "image/png",
            "png",
        ),
    };
    super::encode(&bytes, mime, ext, ctx)
}
