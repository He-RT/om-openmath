//! Real vector samples and finite normalized RK4 integral curves, never frontend math.
use super::*;
use om_eval::numeric::CompiledFn;
fn vector(
    fs: &[CompiledFn],
    p: Point,
    work: &mut Vec<f64>,
    ctx: &Interrupt,
) -> Result<Option<Point>, PlotError> {
    let x = fs[0].eval_with_ctx(&[p.0, p.1], work, ctx)?;
    let y = fs[1].eval_with_ctx(&[p.0, p.1], work, ctx)?;
    Ok((x.is_finite() && y.is_finite()).then_some((x, y)))
}
fn direction(
    fs: &[CompiledFn],
    p: Point,
    work: &mut Vec<f64>,
    ctx: &Interrupt,
) -> Result<Option<Point>, PlotError> {
    Ok(vector(fs, p, work, ctx)?.and_then(|v| {
        let n = v.0.hypot(v.1);
        (n > 0. && n.is_finite()).then_some((v.0 / n, v.1 / n))
    }))
}
pub(super) fn sample(
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    if r.exprs.len() != 2 {
        return Err(extended::invalid("field_plot需要两坐标向量"));
    }
    let fs = r
        .exprs
        .iter()
        .map(|s| extended::compile(s, r, eval, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    let y = r
        .y_range
        .ok_or_else(|| extended::invalid("field_plot需要y范围"))?;
    let mut work = vec![];
    let mut geometry = PlotGeometry2D::default();
    let mut curves = vec![];
    let w = r.x_range.1 - r.x_range.0;
    let h = y.1 - y.0;
    if !r.options.as_ref().unwrap().stream {
        let mut points = vec![];
        let mut max = 0_f64;
        for j in 0..20 {
            for i in 0..20 {
                ctx.tick()?;
                let p = (
                    r.x_range.0 + w * (i as f64 + 0.5) / 20.,
                    y.0 + h * (j as f64 + 0.5) / 20.,
                );
                if let Some(v) = vector(&fs, p, &mut work, ctx)? {
                    max = max.max(v.0.hypot(v.1));
                    points.push((p, v));
                } else {
                    geometry.skipped += 1;
                }
            }
        }
        if !max.is_finite() {
            return Err(extended::invalid("向量范数超出机器表示"));
        }
        let length = 0.75 * w.min(h) / 20.;
        for (p, v) in points {
            ctx.tick()?;
            let end = if max == 0. {
                p
            } else {
                (p.0 + v.0 / max * length, p.1 + v.1 / max * length)
            };
            geometry.arrows.push(PlotArrow {
                start: p,
                end,
                value: v,
            });
        }
    } else {
        let inside =
            |p: Point| p.0 >= r.x_range.0 && p.0 <= r.x_range.1 && p.1 >= y.0 && p.1 <= y.1;
        let step = w.min(h) / 200.;
        for j in 0..8 {
            for i in 0..8 {
                let start = (
                    r.x_range.0 + w * (i as f64 + 0.5) / 8.,
                    y.0 + h * (j as f64 + 0.5) / 8.,
                );
                for sign in [-1., 1.] {
                    let mut p = start;
                    let mut line = vec![p];
                    for n in 0..240 {
                        ctx.tick()?;
                        let Some(a) = direction(&fs, p, &mut work, ctx)? else {
                            geometry.skipped += 1;
                            break;
                        };
                        let Some(b) = direction(
                            &fs,
                            (p.0 + sign * step * a.0 * 0.5, p.1 + sign * step * a.1 * 0.5),
                            &mut work,
                            ctx,
                        )?
                        else {
                            break;
                        };
                        let Some(c) = direction(
                            &fs,
                            (p.0 + sign * step * b.0 * 0.5, p.1 + sign * step * b.1 * 0.5),
                            &mut work,
                            ctx,
                        )?
                        else {
                            break;
                        };
                        let Some(d) = direction(
                            &fs,
                            (p.0 + sign * step * c.0, p.1 + sign * step * c.1),
                            &mut work,
                            ctx,
                        )?
                        else {
                            break;
                        };
                        p = (
                            p.0 + sign * step * (a.0 + 2. * b.0 + 2. * c.0 + d.0) / 6.,
                            p.1 + sign * step * (a.1 + 2. * b.1 + 2. * c.1 + d.1) / 6.,
                        );
                        if !inside(p) {
                            break;
                        }
                        line.push(p);
                        if n > 20 && (p.0 - start.0).hypot(p.1 - start.1) < step * 0.7 {
                            break;
                        }
                    }
                    if line.len() > 1 {
                        curves.push(Curve {
                            label: "stream".into(),
                            segments: vec![line],
                        });
                    }
                }
            }
        }
    }
    Ok(PlotData {
        geometry: Some(geometry),
        scale: None,
        curves,
        x_range: r.x_range,
        y_range: y,
        highlights: None,
    })
}
