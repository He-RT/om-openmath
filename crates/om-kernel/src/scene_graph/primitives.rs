//! Real finite primitive coordinates and triangle topology, including transported tube frames.
use super::*;
use transform::{basis, cross};
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: [f64; 3], s: f64) -> [f64; 3] {
    a.map(|v| v * s)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn unit(v: [f64; 3]) -> Result<[f64; 3], PlotError> {
    Ok(basis(v)?.0)
}
fn radius(e: &Expr, ev: &Evaluator, ctx: &Interrupt) -> Result<f64, PlotError> {
    let r = machine(e, ev, ctx)?;
    if r < 0. {
        return Err(error("半径不能为负"));
    }
    Ok(r)
}
impl Builder<'_> {
    pub(super) fn reserve(&self, n: usize) -> Result<(), PlotError> {
        let count = self
            .data
            .meshes
            .iter()
            .map(|m| m.positions.len())
            .sum::<usize>()
            + self
                .data
                .lines
                .iter()
                .map(|l| l.positions.len())
                .sum::<usize>()
            + self.data.points.len()
            + self.data.labels.len();
        if count.saturating_add(n) > 200000 {
            return Err(error("scene总顶点超过200000"));
        }
        Ok(())
    }
    fn point(&mut self, p: [f64; 3], t: Transform, style: &Style) -> Result<(), PlotError> {
        self.reserve(1)?;
        let mut c = self.colors(style, "blue")?;
        self.data.points.push(ScenePoint {
            position: t.apply(p)?,
            color: self.color(&mut c, p, style)?,
            radius: style.width,
        });
        Ok(())
    }
    pub(super) fn mesh_result(
        &mut self,
        mesh: SceneMesh,
        original: &[[f64; 3]],
        style: &Style,
    ) -> Result<(), PlotError> {
        if !mesh.triangles.is_empty() {
            self.reserve(mesh.positions.len())?;
            self.data.meshes.push(mesh);
            return Ok(());
        }
        if original.is_empty() {
            return Err(error("图元没有可用几何"));
        }
        let mut low = original[0];
        let mut high = original[0];
        for p in original {
            for i in 0..3 {
                low[i] = low[i].min(p[i]);
                high[i] = high[i].max(p[i]);
            }
        }
        if low == high {
            return self.point(low, Transform::identity(), style);
        }
        let axis = (0..3)
            .max_by(|a, b| (high[*a] - low[*a]).total_cmp(&(high[*b] - low[*b])))
            .expect("invariant: three coordinate axes are nonempty");
        let a = *original
            .iter()
            .min_by(|a, b| a[axis].total_cmp(&b[axis]))
            .expect("invariant: original geometry is nonempty above");
        let b = *original
            .iter()
            .max_by(|a, b| a[axis].total_cmp(&b[axis]))
            .expect("invariant: original geometry is nonempty above");
        let extent = (high[axis] - low[axis]).max(f64::MIN_POSITIVE);
        let d = sub(b, a).map(|v| v / extent);
        if original.iter().any(|p| {
            let n = cross(sub(*p, a).map(|v| v / extent), d);
            n.iter().any(|v| v.abs() > 1e-9)
        }) {
            return Err(error("退化变换没有稳定的面/线/点"));
        }
        self.line(&[a, b], Transform::identity(), style)
    }
    fn surface(
        &mut self,
        points: &[[f64; 3]],
        faces: &[[usize; 3]],
        label: &str,
        t: Transform,
        style: &Style,
    ) -> Result<(), PlotError> {
        let mut mesh = geometry::mesh(label.into());
        let mut color = self.colors(style, "green")?;
        for face in faces {
            self.ctx.tick()?;
            self.triangle(&mut mesh, face.map(|i| points[i]), t, style, &mut color)?;
        }
        let world = points
            .iter()
            .map(|p| t.apply(*p))
            .collect::<Result<Vec<_>, _>>()?;
        self.mesh_result(mesh, &world, style)
    }
    pub(super) fn primitive(
        &mut self,
        e: &Expr,
        name: &str,
        t: Transform,
        base: &Style,
    ) -> Result<(), PlotError> {
        let min = match name {
            "Point" | "Line" | "Polygon" => 1,
            "Arrow" | "Circle" | "Disk" | "Sphere" | "Ellipsoid" | "Box" | "Tube" | "Label" => 2,
            "Cylinder" | "Cone" => 3,
            _ => return Err(error("未知scene图元，不能把源码当完成")),
        };
        let (args, opts) = options(e, min)?;
        let style = styled(base, &opts, name, &self.ev, self.ctx)?;
        match name {
            "Point" => self.point(vector(&args[0], self.dim, &self.ev, self.ctx)?, t, &style),
            "Line" => {
                let p = path(&args[0], self.dim, &self.ev, self.ctx)?;
                self.reserve(p.len())?;
                self.line(&p, t, &style)
            }
            "Arrow" => {
                let a = vector(&args[0], self.dim, &self.ev, self.ctx)?;
                let b = vector(&args[1], self.dim, &self.ev, self.ctx)?;
                self.arrow(a, b, t, &style)
            }
            "Circle" | "Disk" => {
                let center = vector(&args[0], self.dim, &self.ev, self.ctx)?;
                let r = radius(&args[1], &self.ev, self.ctx)?;
                self.circular(center, r, name == "Disk", t, &style)
            }
            "Polygon" => {
                let mut p = path(&args[0], self.dim, &self.ev, self.ctx)?;
                if p.first() == p.last() {
                    p.pop();
                }
                let faces = polygon::faces(&p, self.ctx)?;
                self.surface(&p, &faces, "Polygon", t, &style)
            }
            "Sphere" | "Ellipsoid" => {
                if self.dim != 3 {
                    return Err(error("sphere/ellipsoid只能用于dimensions:3"));
                }
                let center = vector(&args[0], 3, &self.ev, self.ctx)?;
                let radii = if name == "Sphere" {
                    [radius(&args[1], &self.ev, self.ctx)?; 3]
                } else {
                    let r = vector(&args[1], 3, &self.ev, self.ctx)?;
                    if r.iter().any(|v| *v < 0.) {
                        return Err(error("radii不能为负"));
                    }
                    r
                };
                self.ellipsoid(center, radii, t, &style)
            }
            "Box" => {
                let lo = vector(&args[0], self.dim, &self.ev, self.ctx)?;
                let hi = vector(&args[1], self.dim, &self.ev, self.ctx)?;
                if (0..self.dim).any(|i| lo[i] > hi[i]) {
                    return Err(error("box min不能大于max"));
                }
                self.box_shape(lo, hi, t, &style)
            }
            "Cylinder" | "Cone" => {
                if self.dim != 3 {
                    return Err(error("cylinder/cone只能用于dimensions:3"));
                }
                let a = vector(&args[0], 3, &self.ev, self.ctx)?;
                let b = vector(&args[1], 3, &self.ev, self.ctx)?;
                let r = radius(&args[2], &self.ev, self.ctx)?;
                self.round_body(a, b, r, name == "Cone", t, &style)
            }
            "Tube" => {
                if self.dim != 3 {
                    return Err(error("tube只能用于dimensions:3"));
                }
                let p = path(&args[0], 3, &self.ev, self.ctx)?;
                let r = radius(&args[1], &self.ev, self.ctx)?;
                self.tube(&p, r, t, &style)
            }
            "Label" => {
                let label = self.ev.fork_readonly().evaluate(&args[0], self.ctx)?;
                let ExprKind::String(text) = label.kind() else {
                    return Err(error("label需要纯字符串"));
                };
                if text.len() > 1024 || text.chars().any(|c| c.is_control()) {
                    return Err(error("label文本超限/有控制字符"));
                }
                let p = vector(&args[1], self.dim, &self.ev, self.ctx)?;
                let mut colors = self.colors(&style, "green")?;
                self.reserve(1)?;
                self.data.labels.push(SceneLabel {
                    position: t.apply(p)?,
                    text: text.to_string(),
                    color: self.color(&mut colors, p, &style)?,
                    offset: style.offset,
                });
                Ok(())
            }
            _ => unreachable!(),
        }
    }
    fn arrow(
        &mut self,
        a: [f64; 3],
        b: [f64; 3],
        t: Transform,
        style: &Style,
    ) -> Result<(), PlotError> {
        let d = sub(b, a);
        let length = d[0].hypot(d[1]).hypot(d[2]);
        if !length.is_finite() {
            return Err(error("arrow尺度超限"));
        }
        if length == 0. {
            return self.point(a, t, style);
        }
        self.line(&[a, b], t, style)?;
        let (_, u, v) = basis(d)?;
        let back = sub(b, mul(unit(d)?, length * 0.14));
        let radius = length * 0.05;
        if self.dim == 2 {
            let perpendicular = unit([-d[1], d[0], 0.])?;
            let p = [
                b,
                add(back, mul(perpendicular, radius)),
                sub(back, mul(perpendicular, radius)),
            ];
            self.surface(&p, &[[0, 1, 2]], "Arrow head", t, style)
        } else {
            let n = 12;
            let mut p = vec![b, back];
            for i in 0..n {
                let angle = std::f64::consts::TAU * i as f64 / n as f64;
                p.push(add(
                    back,
                    add(mul(u, radius * angle.cos()), mul(v, radius * angle.sin())),
                ));
            }
            let mut f = vec![];
            for i in 0..n {
                let j = (i + 1) % n;
                f.push([0, 2 + i, 2 + j]);
                f.push([1, 2 + j, 2 + i]);
            }
            self.surface(&p, &f, "Arrow head", t, style)
        }
    }
    fn circular(
        &mut self,
        center: [f64; 3],
        r: f64,
        fill: bool,
        t: Transform,
        style: &Style,
    ) -> Result<(), PlotError> {
        if r == 0. {
            return self.point(center, t, style);
        }
        let normal = if self.dim == 2 {
            if style.normal[0] != 0. || style.normal[1] != 0. {
                return Err(error("二维circle/disk法线须垂直XY平面"));
            }
            [0., 0., style.normal[2]]
        } else {
            style.normal
        };
        let (_, u, v) = basis(normal)?;
        let n = self.n * 2;
        let mut points = vec![];
        for i in 0..n {
            self.ctx.tick()?;
            let angle = std::f64::consts::TAU * i as f64 / n as f64;
            points.push(add(
                center,
                add(mul(u, r * angle.cos()), mul(v, r * angle.sin())),
            ));
        }
        if !fill {
            points.push(points[0]);
            return self.line(&points, t, style);
        }
        points.push(center);
        let faces = (0..n).map(|i| [n, i, (i + 1) % n]).collect::<Vec<_>>();
        self.surface(&points, &faces, "Disk", t, style)
    }
    fn ellipsoid(
        &mut self,
        center: [f64; 3],
        radii: [f64; 3],
        t: Transform,
        style: &Style,
    ) -> Result<(), PlotError> {
        if radii == [0.; 3] {
            return self.point(center, t, style);
        }
        let n = self.n;
        let m = n * 2;
        let mut p = vec![];
        for j in 0..=n {
            let v = std::f64::consts::PI * j as f64 / n as f64;
            for i in 0..=m {
                self.ctx.tick()?;
                let u = std::f64::consts::TAU * i as f64 / m as f64;
                p.push(if j == 0 {
                    [center[0], center[1], center[2] + radii[2]]
                } else if j == n {
                    [center[0], center[1], center[2] - radii[2]]
                } else {
                    [
                        center[0] + radii[0] * u.cos() * v.sin(),
                        center[1] + radii[1] * u.sin() * v.sin(),
                        center[2] + radii[2] * v.cos(),
                    ]
                });
            }
        }
        let mut faces = vec![];
        for j in 0..n {
            for i in 0..m {
                let a = j * (m + 1) + i;
                let b = a + 1;
                let c = (j + 1) * (m + 1) + i;
                let d = c + 1;
                faces.extend([[a, c, b], [b, c, d]]);
            }
        }
        self.surface(&p, &faces, "Ellipsoid", t, style)
    }
    fn box_shape(
        &mut self,
        lo: [f64; 3],
        hi: [f64; 3],
        t: Transform,
        style: &Style,
    ) -> Result<(), PlotError> {
        let p = [
            [lo[0], lo[1], lo[2]],
            [hi[0], lo[1], lo[2]],
            [hi[0], hi[1], lo[2]],
            [lo[0], hi[1], lo[2]],
            [lo[0], lo[1], hi[2]],
            [hi[0], lo[1], hi[2]],
            [hi[0], hi[1], hi[2]],
            [lo[0], hi[1], hi[2]],
        ];
        let f = if self.dim == 2 {
            vec![[0, 1, 2], [0, 2, 3]]
        } else {
            vec![
                [0, 2, 1],
                [0, 3, 2],
                [4, 5, 6],
                [4, 6, 7],
                [0, 1, 5],
                [0, 5, 4],
                [1, 2, 6],
                [1, 6, 5],
                [2, 3, 7],
                [2, 7, 6],
                [3, 0, 4],
                [3, 4, 7],
            ]
        };
        self.surface(&p, &f, "Box", t, style)
    }
    fn round_body(
        &mut self,
        a: [f64; 3],
        b: [f64; 3],
        r: f64,
        cone: bool,
        t: Transform,
        style: &Style,
    ) -> Result<(), PlotError> {
        if r == 0. {
            return self.line(&[a, b], t, style);
        }
        let (_, u, v) = basis(sub(b, a))?;
        let n = self.n * 2;
        let mut p = vec![a, b];
        for i in 0..n {
            let angle = std::f64::consts::TAU * i as f64 / n as f64;
            let d = add(mul(u, r * angle.cos()), mul(v, r * angle.sin()));
            p.extend([add(a, d), if cone { b } else { add(b, d) }]);
        }
        let mut f = vec![];
        for i in 0..n {
            let j = (i + 1) % n;
            f.push([0, 2 + 2 * j, 2 + 2 * i]);
            f.push([2 + 2 * i, 2 + 2 * j, 3 + 2 * j]);
            if !cone {
                f.extend([[2 + 2 * i, 3 + 2 * j, 3 + 2 * i], [1, 3 + 2 * i, 3 + 2 * j]]);
            }
        }
        self.surface(&p, &f, if cone { "Cone" } else { "Cylinder" }, t, style)
    }
    fn tube(
        &mut self,
        path: &[[f64; 3]],
        r: f64,
        t: Transform,
        style: &Style,
    ) -> Result<(), PlotError> {
        if path.len() < 2 {
            return Err(error("tube路径须至少两点"));
        }
        if r == 0. {
            return self.line(path, t, style);
        }
        let mut tangent = vec![];
        for i in 0..path.len() {
            self.ctx.tick()?;
            let direction = if i == 0 {
                sub(path[1], path[0])
            } else if i + 1 == path.len() {
                sub(path[i], path[i - 1])
            } else {
                add(
                    unit(sub(path[i], path[i - 1]))?,
                    unit(sub(path[i + 1], path[i]))?,
                )
            };
            tangent.push(unit(direction)?);
        }
        let (_, mut u, _) = basis(tangent[0])?;
        let n = self.n * 2;
        let mut p = vec![];
        for i in 0..path.len() {
            if i > 0 {
                let axis = cross(tangent[i - 1], tangent[i]);
                let sine = axis[0].hypot(axis[1]).hypot(axis[2]);
                let cosine = dot(tangent[i - 1], tangent[i]).clamp(-1., 1.);
                if sine > 1e-12 {
                    let axis = mul(axis, 1. / sine);
                    u = add(
                        add(mul(u, cosine), mul(cross(axis, u), sine)),
                        mul(axis, dot(axis, u) * (1. - cosine)),
                    );
                } else if cosine < 0. {
                    return Err(error("tube不能完全折返"));
                }
                u = unit(sub(u, mul(tangent[i], dot(u, tangent[i]))))?;
            }
            let v = cross(tangent[i], u);
            for j in 0..n {
                self.ctx.tick()?;
                let angle = std::f64::consts::TAU * j as f64 / n as f64;
                p.push(add(
                    path[i],
                    add(mul(u, r * angle.cos()), mul(v, r * angle.sin())),
                ));
            }
        }
        let mut f = vec![];
        for i in 0..path.len() - 1 {
            for j in 0..n {
                let k = (j + 1) % n;
                let a = i * n + j;
                let b = i * n + k;
                let c = (i + 1) * n + j;
                let d = (i + 1) * n + k;
                f.extend([[a, b, c], [b, d, c]]);
            }
        }
        let start = p.len();
        p.push(path[0]);
        let end = p.len();
        p.push(path[path.len() - 1]);
        for j in 0..n {
            let k = (j + 1) % n;
            f.extend([
                [start, k, j],
                [end, (path.len() - 1) * n + j, (path.len() - 1) * n + k],
            ]);
        }
        self.surface(&p, &f, "Tube", t, style)
    }
}
