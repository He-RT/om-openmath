//! Composed geometry is generated in a readonly mathematical scope; transforms never write notebook variables.
mod polygon;
mod primitives;
mod projection;
pub(crate) use projection::{from_expr as plot_from_expr, sample as sample_plot};
mod transform;
use crate::{
    plot::PlotError,
    protocol::*,
    scene3d::{Colorizer, geometry},
};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
use om_eval::Evaluator;
use transform::Transform;
fn error(s: &str) -> PlotError {
    PlotError::Invalid(format!("scene: {s}"))
}
fn machine(e: &Expr, ev: &Evaluator, ctx: &Interrupt) -> Result<f64, PlotError> {
    crate::plot::machine_value(e, ev, ctx)
}
fn vector(e: &Expr, dim: usize, ev: &Evaluator, ctx: &Interrupt) -> Result<[f64; 3], PlotError> {
    let e = ev.fork_readonly().evaluate(e, ctx)?;
    if !e.is_head(B::LIST) || e.args().len() != dim {
        return Err(error("坐标/向量维数必须与scene dimensions相同"));
    }
    let mut out = [0.; 3];
    for (i, e) in e.args().iter().enumerate() {
        out[i] = machine(e, ev, ctx)?;
    }
    Ok(out)
}
fn path(e: &Expr, dim: usize, ev: &Evaluator, ctx: &Interrupt) -> Result<Vec<[f64; 3]>, PlotError> {
    let e = ev.fork_readonly().evaluate(e, ctx)?;
    if !e.is_head(B::LIST) || e.args().is_empty() || e.args().len() > 2048 {
        return Err(error("几何路径需要1..2048点列表"));
    }
    e.args()
        .iter()
        .map(|e| {
            ctx.tick()?;
            vector(e, dim, ev, ctx)
        })
        .collect()
}
#[derive(Clone)]
struct Style {
    color: Option<String>,
    opacity: f64,
    width: f64,
    offset: (f64, f64),
    normal: [f64; 3],
}
impl Default for Style {
    fn default() -> Self {
        Self {
            color: None,
            opacity: 1.,
            width: 2.,
            offset: (8., -10.),
            normal: [0., 0., 1.],
        }
    }
}
type NodeOptions<'a> = (&'a [Expr], Vec<(&'a str, &'a Expr)>);
fn options(e: &Expr, min: usize) -> Result<NodeOptions<'_>, PlotError> {
    if e.args().len() < min {
        return Err(error("图元/变换缺少必要参数"));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut result = vec![];
    for option in &e.args()[min..] {
        if !option.is_head(B::RULE) || option.args().len() != 2 {
            return Err(error("尾参数需要命名选项"));
        }
        let name = option.args()[0]
            .as_symbol()
            .ok_or_else(|| error("选项名需要符号"))?
            .name();
        if !seen.insert(name) {
            return Err(error("重复选项"));
        }
        result.push((name, &option.args()[1]));
    }
    Ok((&e.args()[..min], result))
}
fn styled(
    base: &Style,
    options: &[(&str, &Expr)],
    name: &str,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<Style, PlotError> {
    let mut style = base.clone();
    for (key, value) in options {
        ctx.tick()?;
        match *key {
            "Color" => style.color = Some(om_format::input_form(value)),
            "Opacity" => {
                style.opacity = machine(value, ev, ctx)?;
                if !(0.0..=1.0).contains(&style.opacity) {
                    return Err(error("opacity需要0..1实数"));
                }
            }
            "Width" if matches!(name, "Line" | "Arrow" | "Circle" | "Point") => {
                style.width = machine(value, ev, ctx)?;
                if !(0.0..=32.0).contains(&style.width) {
                    return Err(error("width需要0..32逻辑像素"));
                }
            }
            "Normal" if matches!(name, "Circle" | "Disk") => {
                style.normal = vector(value, 3, ev, ctx)?
            }
            "Offset" if name == "Label" => {
                let v = vector(value, 2, ev, ctx)?;
                style.offset = (v[0], v[1]);
            }
            _ => return Err(error("此图元不支持给定选项")),
        }
    }
    Ok(style)
}
pub(crate) fn dimensions(e: &Expr, ev: &Evaluator, ctx: &Interrupt) -> Result<usize, PlotError> {
    if e.head_symbol().is_none_or(|s| s.name() != "Scene") {
        return Err(error("需要scene容器"));
    }
    let (_, opts) = options(e, 1)?;
    let mut dim = 2;
    for (name, value) in opts {
        if name == "Dimensions" {
            let value = ev.fork_readonly().evaluate(value, ctx)?;
            dim = match value.as_number() {
                Some(om_num::Number::Integer(n)) => usize::try_from(n)
                    .ok()
                    .filter(|v| matches!(v, 2 | 3))
                    .ok_or_else(|| error("dimensions仅2或3"))?,
                _ => return Err(error("dimensions需要整数2或3")),
            };
        }
    }
    Ok(dim)
}
pub(crate) fn request(
    e: &Expr,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<Scene3DRequest, PlotError> {
    let _ = dimensions(e, ev, ctx)?;
    let (_, opts) = options(e, 1)?;
    let mut n = 32;
    for (name, v) in &opts {
        match *name {
            "MeshPoints" => {
                let value = ev.fork_readonly().evaluate(v, ctx)?;
                n = match value.as_number() {
                    Some(om_num::Number::Integer(n)) => u32::try_from(n)
                        .ok()
                        .filter(|v| (8..=64).contains(v))
                        .ok_or_else(|| error("mesh_points需要8..64整数"))?,
                    _ => return Err(error("mesh_points需要整数")),
                };
            }
            "Dimensions" | "Color" | "Opacity" => {}
            _ => return Err(error("scene不支持给定选项")),
        }
    }
    Ok(Scene3DRequest {
        kind: SceneKind::Scene,
        expressions: vec![om_format::input_form(e)],
        axes: vec![],
        mesh_points: n,
        color: None,
        parameters: Default::default(),
    })
}
pub(crate) fn sample_request(
    r: &Scene3DRequest,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<Scene3DData, PlotError> {
    if r.expressions.len() != 1
        || !r.axes.is_empty()
        || !(8..=64).contains(&r.mesh_points)
        || r.parameters.len() > 64
    {
        return Err(error("scene数学请求无效"));
    }
    let raw = crate::scene3d::raw(&r.expressions[0])?;
    let dim = dimensions(&raw, ev, ctx)?;
    let mut locals = vec![];
    for (name, v) in &r.parameters {
        ctx.tick()?;
        if !v.is_finite() {
            return Err(error("scene参数非有限"));
        }
        locals.push((crate::plot::axis(name)?, Expr::real(*v)));
    }
    let ev = ev.fork_with_locals(&locals);
    let mut builder = Builder {
        ev,
        ctx,
        dim,
        n: r.mesh_points as usize,
        data: Scene3DData {
            sampled: false,
            ..geometry::empty()
        },
        nodes: 0,
    };
    let (args, opts) = options(&raw, 1)?;
    let common = opts
        .iter()
        .copied()
        .filter(|(name, _)| matches!(*name, "Color" | "Opacity"))
        .collect::<Vec<_>>();
    let style = styled(&Style::default(), &common, "Scene", &builder.ev, ctx)?;
    let nodes = builder.ev.fork_readonly().evaluate(&args[0], ctx)?;
    if !nodes.is_head(B::LIST) {
        return Err(error("scene nodes需要列表"));
    }
    for node in nodes.args() {
        builder.add(node, Transform::identity(), &style, 0)?;
    }
    geometry::finish(&mut builder.data, ctx)?;
    geometry::validate(&builder.data, ctx)?;
    Ok(builder.data)
}
struct Builder<'a> {
    ev: Evaluator,
    ctx: &'a Interrupt,
    dim: usize,
    n: usize,
    data: Scene3DData,
    nodes: usize,
}
impl Builder<'_> {
    fn colors(&self, style: &Style, default: &str) -> Result<Colorizer, PlotError> {
        let source = style
            .color
            .clone()
            .or_else(|| Some(om_format::input_form(&Expr::string(default))));
        Colorizer::new(&source, &self.ev, self.ctx)
    }
    fn color(&self, c: &mut Colorizer, p: [f64; 3], style: &Style) -> Result<[f64; 4], PlotError> {
        let mut color = c.at_coordinates(&p[..self.dim], &[], self.ctx)?;
        color[3] *= style.opacity;
        Ok(color)
    }
    fn line(&mut self, p: &[[f64; 3]], t: Transform, style: &Style) -> Result<(), PlotError> {
        if p.is_empty() {
            return Err(error("路径为空"));
        }
        let mut c = self.colors(style, "blue")?;
        self.reserve(p.len())?;
        let mut line = SceneLine {
            positions: vec![],
            colors: vec![],
            width: style.width,
        };
        for &p in p {
            self.ctx.tick()?;
            let world = t.apply(p)?;
            if line.positions.last().is_some_and(|last| *last == world) {
                continue;
            }
            line.positions.push(world);
            line.colors.push(self.color(&mut c, p, style)?);
        }
        if line.positions.len() == 1 {
            self.data.points.push(ScenePoint {
                position: line.positions[0],
                color: line.colors[0],
                radius: style.width,
            });
        } else {
            self.data.lines.push(line);
        }
        Ok(())
    }
    fn triangle(
        &mut self,
        mesh: &mut SceneMesh,
        p: [[f64; 3]; 3],
        t: Transform,
        style: &Style,
        c: &mut Colorizer,
    ) -> Result<(), PlotError> {
        let mut points = [t.apply(p[0])?, t.apply(p[1])?, t.apply(p[2])?];
        let mut colors = [
            self.color(c, p[0], style)?,
            self.color(c, p[1], style)?,
            self.color(c, p[2], style)?,
        ];
        if t.flipped {
            points.swap(1, 2);
            colors.swap(1, 2);
        }
        geometry::triangle(mesh, points, colors, self.ctx)
    }
    fn add(&mut self, e: &Expr, t: Transform, base: &Style, depth: usize) -> Result<(), PlotError> {
        self.ctx.tick()?;
        self.nodes += 1;
        if depth > 32 || self.nodes > 1024 {
            return Err(error("scene深度/节点超过32/1024"));
        }
        if e.is_head(B::LIST) {
            for item in e.args() {
                self.add(item, t, base, depth + 1)?;
            }
            return Ok(());
        }
        let e = self.ev.fork_readonly().evaluate(e, self.ctx)?;
        // A binding or Map may yield a group; expand it after actual readonly evaluation too.
        if e.is_head(B::LIST) {
            for item in e.args() {
                self.add(item, t, base, depth + 1)?;
            }
            return Ok(());
        }
        let name = e
            .head_symbol()
            .map(|s| s.name())
            .ok_or_else(|| error("scene节点需要实际图元/变换"))?;
        if matches!(name, "Translate" | "Rotate" | "Scale") {
            return self.transformed(&e, t, base, depth);
        }
        if name == "Scene" {
            let (args, opts) = options(&e, 1)?;
            for (key, v) in &opts {
                if *key == "Dimensions" {
                    let d = machine(v, &self.ev, self.ctx)?;
                    if d != self.dim as f64 {
                        return Err(error("嵌套scene维数不同"));
                    }
                }
            }
            let common = opts
                .iter()
                .copied()
                .filter(|(key, _)| matches!(*key, "Color" | "Opacity"))
                .collect::<Vec<_>>();
            if opts
                .iter()
                .any(|(key, _)| !matches!(*key, "Color" | "Opacity" | "Dimensions"))
            {
                return Err(error("嵌套scene不支持给定选项"));
            }
            let style = styled(base, &common, name, &self.ev, self.ctx)?;
            let nodes = self.ev.fork_readonly().evaluate(&args[0], self.ctx)?;
            if !nodes.is_head(B::LIST) {
                return Err(error("scene nodes需要列表"));
            }
            for node in nodes.args() {
                self.add(node, t, &style, depth + 1)?;
            }
            return Ok(());
        }
        if name == "Style" {
            let (args, opts) = options(&e, 1)?;
            let style = styled(base, &opts, name, &self.ev, self.ctx)?;
            return self.add(&args[0], t, &style, depth + 1);
        }
        if matches!(name, "Plot3D" | "ParametricPlot3D" | "ImplicitPlot3D") {
            if self.dim != 3 {
                return Err(error("三维采样节点只能放入dimensions:3的scene"));
            }
            let mut request = crate::scene3d::from_expr(&e, &self.ev, self.ctx)?
                .ok_or_else(|| error("无效采样节点"))?;
            if request.color.is_none() {
                request.color = base.color.clone();
            }
            let data = crate::scene3d::sample(&request, &self.ev, self.ctx)?;
            return self.import(data, t, base);
        }
        self.primitive(&e, name, t, base)
    }
    fn import(&mut self, data: Scene3DData, t: Transform, base: &Style) -> Result<(), PlotError> {
        self.data.skipped += data.skipped;
        self.data.sampled |= data.sampled;
        for old in data.meshes {
            // Import expands indexed meshes to independent world-coordinate triangles.
            // Enforce the aggregate bound before allocating that expanded representation.
            self.reserve(old.triangles.len().saturating_mul(3))?;
            let mut mesh = geometry::mesh(old.label);
            for face in old.triangles {
                self.ctx.tick()?;
                let ids = face.map(|i| i as usize);
                let mut p = [
                    t.apply(old.positions[ids[0]])?,
                    t.apply(old.positions[ids[1]])?,
                    t.apply(old.positions[ids[2]])?,
                ];
                let mut c = [old.colors[ids[0]], old.colors[ids[1]], old.colors[ids[2]]];
                for color in &mut c {
                    color[3] *= base.opacity;
                }
                if t.flipped {
                    p.swap(1, 2);
                    c.swap(1, 2);
                }
                geometry::triangle(&mut mesh, p, c, self.ctx)?;
            }
            let world = old
                .positions
                .iter()
                .map(|p| t.apply(*p))
                .collect::<Result<Vec<_>, _>>()?;
            self.mesh_result(mesh, &world, base)?;
        }
        for mut line in data.lines {
            self.reserve(line.positions.len())?;
            for p in &mut line.positions {
                *p = t.apply(*p)?;
            }
            for c in &mut line.colors {
                c[3] *= base.opacity;
            }
            if line.positions.iter().all(|p| *p == line.positions[0]) {
                self.data.points.push(ScenePoint {
                    position: line.positions[0],
                    color: line.colors[0],
                    radius: line.width,
                });
            } else {
                self.data.lines.push(line);
            }
        }
        for mut p in data.points {
            self.reserve(1)?;
            p.position = t.apply(p.position)?;
            p.color[3] *= base.opacity;
            self.data.points.push(p);
        }
        for mut label in data.labels {
            self.reserve(1)?;
            label.position = t.apply(label.position)?;
            self.data.labels.push(label);
        }
        Ok(())
    }
}
