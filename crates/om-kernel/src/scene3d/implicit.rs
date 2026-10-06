//! Marching tetrahedra with shared real edge roots; poles are not accepted as zero crossings.
use super::*;
use std::collections::BTreeMap;
struct Field<'a> {
    function: CompiledFn,
    work: Vec<f64>,
    points: Vec<[f64; 3]>,
    values: Vec<f64>,
    edges: BTreeMap<(usize, usize), Option<[f64; 3]>>,
    ctx: &'a Interrupt,
}
impl Field<'_> {
    fn root(&mut self, a: usize, b: usize) -> Result<Option<[f64; 3]>, PlotError> {
        self.ctx.tick()?;
        let key = (a.min(b), a.max(b));
        if let Some(p) = self.edges.get(&key) {
            return Ok(*p);
        }
        let (pa, pb) = (self.points[a], self.points[b]);
        let (va, vb) = (self.values[a], self.values[b]);
        let result = if va == 0. {
            Some(pa)
        } else if vb == 0. {
            Some(pb)
        } else {
            let mut lo = 0_f64;
            let mut hi = 1_f64;
            let mut v = va;
            let mut result = None;
            let tolerance = 1e-7 * va.abs().min(vb.abs()).max(f64::MIN_POSITIVE);
            for _ in 0..40 {
                self.ctx.tick()?;
                let t = lo + (hi - lo) * 0.5;
                let p = std::array::from_fn::<_, 3, _>(|k| pa[k] + (pb[k] - pa[k]) * t);
                let value = self.function.eval_with_ctx(&p, &mut self.work, self.ctx)?;
                if !value.is_finite() {
                    break;
                }
                if value.abs() <= tolerance {
                    result = Some(p);
                    break;
                }
                if (value < 0.) == (v < 0.) {
                    lo = t;
                    v = value;
                } else {
                    hi = t;
                }
            }
            result
        };
        self.edges.insert(key, result);
        Ok(result)
    }
    fn tetrahedron(
        &mut self,
        ids: [usize; 4],
        mesh: &mut SceneMesh,
        colors: &mut Colorizer,
    ) -> Result<bool, PlotError> {
        self.ctx.tick()?;
        if ids.iter().any(|i| !self.values[*i].is_finite()) {
            return Ok(false);
        }
        let negative = ids
            .iter()
            .copied()
            .filter(|i| self.values[*i] < 0.)
            .collect::<Vec<_>>();
        let positive = ids
            .iter()
            .copied()
            .filter(|i| self.values[*i] >= 0.)
            .collect::<Vec<_>>();
        if negative.is_empty() || positive.is_empty() {
            return Ok(true);
        }
        let mut direction = [0.; 3];
        for (k, component) in direction.iter_mut().enumerate() {
            *component = positive
                .iter()
                .map(|i| self.points[*i][k] / positive.len() as f64)
                .sum::<f64>()
                - negative
                    .iter()
                    .map(|i| self.points[*i][k] / negative.len() as f64)
                    .sum::<f64>();
        }
        let magnitude = direction.iter().fold(0_f64, |m, v| m.max(v.abs()));
        if !magnitude.is_finite() || magnitude == 0. {
            return Ok(false);
        }
        direction = direction.map(|v| v / magnitude);
        let edges = if negative.len() == 1 {
            vec![
                (negative[0], positive[0]),
                (negative[0], positive[1]),
                (negative[0], positive[2]),
            ]
        } else if positive.len() == 1 {
            vec![
                (positive[0], negative[0]),
                (positive[0], negative[1]),
                (positive[0], negative[2]),
            ]
        } else {
            vec![
                (negative[0], positive[0]),
                (negative[0], positive[1]),
                (negative[1], positive[1]),
                (negative[1], positive[0]),
            ]
        };
        let mut points = vec![];
        for (a, b) in edges {
            let Some(point) = self.root(a, b)? else {
                return Ok(false);
            };
            points.push(point);
        }
        let faces = if points.len() == 3 {
            vec![[0, 1, 2]]
        } else {
            vec![[0, 1, 2], [0, 2, 3]]
        };
        for face in faces {
            let mut p = face.map(|i| points[i]);
            if let Some(n) = geometry::normal(p[0], p[1], p[2]) {
                if n.iter().zip(direction).map(|(a, b)| a * b).sum::<f64>() < 0. {
                    p.swap(1, 2);
                }
                let c = [
                    colors.at(p[0], &p[0], self.ctx)?,
                    colors.at(p[1], &p[1], self.ctx)?,
                    colors.at(p[2], &p[2], self.ctx)?,
                ];
                geometry::triangle(mesh, p, c, self.ctx)?;
            }
        }
        Ok(true)
    }
}
pub(super) fn sample(
    r: &Scene3DRequest,
    ev: &Evaluator,
    colors: &mut Colorizer,
    ctx: &Interrupt,
) -> Result<Scene3DData, PlotError> {
    let n = r.mesh_points as usize;
    let w = n + 1;
    let mut field = Field {
        function: compile(&r.expressions[0], r, ev, ctx)?,
        work: vec![],
        points: vec![],
        values: vec![],
        edges: BTreeMap::new(),
        ctx,
    };
    let index = |x: usize, y: usize, z: usize| (z * w + y) * w + x;
    for z in 0..=n {
        for y in 0..=n {
            for x in 0..=n {
                ctx.tick()?;
                let coords = [x, y, z];
                let point = std::array::from_fn::<_, 3, _>(|i| {
                    if coords[i] == n {
                        r.axes[i].range.1
                    } else {
                        r.axes[i].range.0
                            + (r.axes[i].range.1 - r.axes[i].range.0) * coords[i] as f64 / n as f64
                    }
                });
                field
                    .values
                    .push(field.function.eval_with_ctx(&point, &mut field.work, ctx)?);
                field.points.push(point);
            }
        }
    }
    let tetrahedra = [
        [0, 5, 1, 6],
        [0, 1, 2, 6],
        [0, 2, 3, 6],
        [0, 3, 7, 6],
        [0, 7, 4, 6],
        [0, 4, 5, 6],
    ];
    let mut data = geometry::empty();
    let mut mesh = geometry::mesh(r.expressions[0].clone());
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                ctx.tick()?;
                let corner = [
                    index(x, y, z),
                    index(x + 1, y, z),
                    index(x + 1, y + 1, z),
                    index(x, y + 1, z),
                    index(x, y, z + 1),
                    index(x + 1, y, z + 1),
                    index(x + 1, y + 1, z + 1),
                    index(x, y + 1, z + 1),
                ];
                for t in tetrahedra {
                    if !field.tetrahedron(t.map(|i| corner[i]), &mut mesh, colors)? {
                        data.skipped += 1;
                    }
                }
            }
        }
    }
    if !mesh.triangles.is_empty() {
        data.meshes.push(mesh);
    }
    Ok(data)
}
