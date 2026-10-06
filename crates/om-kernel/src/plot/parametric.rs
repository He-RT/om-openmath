//! Two true coordinate callbacks share a fixed parameter domain, independently of the camera viewport.
use super::*;
pub(super) fn sample(
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    if r.exprs.len() != 2 {
        return Err(extended::invalid("参数曲线需要两坐标"));
    }
    let (a, b) = r.options.as_ref().unwrap().axes[0].range;
    let x = extended::compile(&r.exprs[0], r, eval, ctx)?;
    let y = extended::compile(&r.exprs[1], r, eval, ctx)?;
    let mut wx = vec![];
    let mut wy = vec![];
    let mut xs = vec![];
    let mut ys = vec![];
    let mut points = vec![];
    let mut skipped = 0;
    for i in 0..800 {
        ctx.tick()?;
        let t = if i == 799 {
            b
        } else {
            a + (b - a) * i as f64 / 799.
        };
        let p = (
            x.eval_with_ctx(&[t], &mut wx, ctx)?,
            y.eval_with_ctx(&[t], &mut wy, ctx)?,
        );
        if p.0.is_finite() && p.1.is_finite() {
            xs.push(p.0);
            ys.push(p.1);
            points.push(Some((t, p)));
        } else {
            skipped += 1;
            points.push(None);
        }
    }
    let opts = r.options.as_ref().unwrap();
    let auto_x = if opts.scale.log_x() {
        extended::log_bounds(&xs)?
    } else {
        extended::bounds(&xs)?
    };
    let auto_y = if opts.scale.log_y() {
        extended::log_bounds(&ys)?
    } else {
        extended::bounds(&ys)?
    };
    let x_range = if r.y_range.is_some() {
        r.x_range
    } else {
        auto_x
    };
    let y_range = r.y_range.unwrap_or(auto_y);
    let mut segments = vec![];
    let mut segment = vec![];
    for pair in points.windows(2) {
        ctx.tick()?;
        let good = if let (Some((ta, pa)), Some((tb, pb))) = (pair[0], pair[1]) {
            let t = (ta + tb) * 0.5;
            let middle = (
                x.eval_with_ctx(&[t], &mut wx, ctx)?,
                y.eval_with_ctx(&[t], &mut wy, ctx)?,
            );
            let finite = middle.0.is_finite() && middle.1.is_finite();
            let span = ((pa.0 - pb.0) / (auto_x.1 - auto_x.0))
                .hypot((pa.1 - pb.1) / (auto_y.1 - auto_y.0));
            if finite && span < 0.2 {
                if segment.is_empty() {
                    segment.push(pa);
                }
                segment.push(middle);
                segment.push(pb);
                true
            } else {
                skipped += 1;
                false
            }
        } else {
            false
        };
        if !good {
            if segment.len() >= 2 {
                segments.push(std::mem::take(&mut segment));
            }
            segment.clear();
        }
    }
    if segment.len() >= 2 {
        segments.push(segment);
    }
    Ok(PlotData {
        geometry: Some(PlotGeometry2D {
            skipped,
            ..Default::default()
        }),
        scale: None,
        curves: vec![Curve {
            label: format!("({}, {})", r.exprs[0], r.exprs[1]),
            segments,
        }],
        x_range,
        y_range,
        highlights: None,
    })
}
