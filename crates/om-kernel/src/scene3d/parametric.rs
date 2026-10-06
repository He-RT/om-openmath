//! Finite real samples for scalar surfaces, parameter surfaces and parameter curves.
use super::*;
type Sample = ([f64; 3], [f64; 4]);
struct Programs {
    functions: Vec<CompiledFn>,
    work: Vec<f64>,
    surface: bool,
}
impl Programs {
    fn at(
        &mut self,
        parameters: &[f64],
        colors: &mut Colorizer,
        ctx: &Interrupt,
    ) -> Result<Option<Sample>, PlotError> {
        let point = if self.surface {
            [
                parameters[0],
                parameters[1],
                self.functions[0].eval_with_ctx(parameters, &mut self.work, ctx)?,
            ]
        } else {
            let mut p = [0.; 3];
            for (i, f) in self.functions.iter().enumerate() {
                p[i] = f.eval_with_ctx(parameters, &mut self.work, ctx)?;
            }
            p
        };
        if point.iter().any(|p| !p.is_finite()) {
            return Ok(None);
        }
        Ok(Some((point, colors.at(point, parameters, ctx)?)))
    }
}
fn coordinate(axis: &PlotAxis, i: usize, n: usize) -> f64 {
    if i == n {
        axis.range.1
    } else {
        axis.range.0 + (axis.range.1 - axis.range.0) * i as f64 / n as f64
    }
}
pub(super) fn sample(
    r: &Scene3DRequest,
    ev: &Evaluator,
    colors: &mut Colorizer,
    ctx: &Interrupt,
) -> Result<Scene3DData, PlotError> {
    let functions = r
        .expressions
        .iter()
        .map(|s| compile(s, r, ev, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    let mut programs = Programs {
        functions,
        work: vec![],
        surface: r.kind == SceneKind::Surface,
    };
    if r.axes.len() == 1 {
        return curve(r, &mut programs, colors, ctx);
    }
    let n = r.mesh_points as usize;
    let mut samples = vec![];
    let mut data = geometry::empty();
    for j in 0..=n {
        for i in 0..=n {
            ctx.tick()?;
            let parameters = [coordinate(&r.axes[0], i, n), coordinate(&r.axes[1], j, n)];
            let p = programs.at(&parameters, colors, ctx)?;
            if p.is_none() {
                data.skipped += 1;
            }
            samples.push(p);
        }
    }
    let mut mesh = geometry::mesh(r.expressions.join(", "));
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for (p, _) in samples.iter().flatten() {
        for k in 0..3 {
            low[k] = low[k].min(p[k]);
            high[k] = high[k].max(p[k]);
        }
    }
    let spans = std::array::from_fn::<_, 3, _>(|i| high[i] - low[i]);
    for j in 0..n {
        for i in 0..n {
            ctx.tick()?;
            let indices = [
                j * (n + 1) + i,
                j * (n + 1) + i + 1,
                (j + 1) * (n + 1) + i + 1,
                (j + 1) * (n + 1) + i,
            ];
            let corners = indices.map(|k| samples[k]);
            let [Some(a), Some(b), Some(c), Some(d)] = corners else {
                data.skipped += 1;
                continue;
            };
            let parameters = [
                (coordinate(&r.axes[0], i, n) + coordinate(&r.axes[0], i + 1, n)) * 0.5,
                (coordinate(&r.axes[1], j, n) + coordinate(&r.axes[1], j + 1, n)) * 0.5,
            ];
            let Some((middle, _)) = programs.at(&parameters, colors, ctx)? else {
                data.skipped += 1;
                continue;
            };
            let curved = (0..3).any(|k| {
                spans[k].is_finite()
                    && spans[k] > 0.
                    && ((middle[k] / spans[k])
                        - (a.0[k] / spans[k]
                            + b.0[k] / spans[k]
                            + c.0[k] / spans[k]
                            + d.0[k] / spans[k])
                            * 0.25)
                        .abs()
                        > 0.1
            });
            if curved {
                data.skipped += 1;
                continue;
            }
            geometry::triangle(&mut mesh, [a.0, b.0, c.0], [a.1, b.1, c.1], ctx)?;
            geometry::triangle(&mut mesh, [a.0, c.0, d.0], [a.1, c.1, d.1], ctx)?;
        }
    }
    if !mesh.triangles.is_empty() {
        data.meshes.push(mesh);
    }
    Ok(data)
}
fn curve(
    r: &Scene3DRequest,
    programs: &mut Programs,
    colors: &mut Colorizer,
    ctx: &Interrupt,
) -> Result<Scene3DData, PlotError> {
    let n = r.mesh_points as usize * 16;
    let mut data = geometry::empty();
    let mut samples = vec![];
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for i in 0..=n {
        ctx.tick()?;
        let t = coordinate(&r.axes[0], i, n);
        let p = programs.at(&[t], colors, ctx)?;
        if let Some((p, _)) = &p {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        } else {
            data.skipped += 1;
        }
        samples.push((t, p));
    }
    if lo == hi && lo.iter().all(|v| v.is_finite()) {
        let (_, Some((position, color))) = samples[0] else {
            return Err(invalid("参数曲线没有有限样本"));
        };
        data.points.push(ScenePoint {
            position,
            color,
            radius: 4.,
        });
        return Ok(data);
    }
    let mut line = SceneLine {
        positions: vec![],
        colors: vec![],
        width: 2.,
    };
    for pair in samples.windows(2) {
        ctx.tick()?;
        let good = if let [(ta, Some(a)), (tb, Some(b))] = pair {
            let middle = programs.at(&[ta + (tb - ta) * 0.5], colors, ctx)?;
            let jump =
                (0..3).any(|k| hi[k] > lo[k] && ((a.0[k] - b.0[k]) / (hi[k] - lo[k])).abs() > 0.2);
            if let Some(middle) = middle.filter(|_| !jump) {
                if line.positions.is_empty() {
                    line.positions.push(a.0);
                    line.colors.push(a.1);
                }
                line.positions.extend([middle.0, b.0]);
                line.colors.extend([middle.1, b.1]);
                true
            } else {
                data.skipped += 1;
                false
            }
        } else {
            false
        };
        if !good {
            if line.positions.len() > 1 {
                data.lines.push(line);
            }
            line = SceneLine {
                positions: vec![],
                colors: vec![],
                width: 2.,
            };
        }
    }
    if line.positions.len() > 1 {
        data.lines.push(line);
    }
    Ok(data)
}
