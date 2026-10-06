//! Uniform real cell samples are explicit visualization approximations, not exact region certificates.
use super::*;
const N: usize = 96;
pub(super) fn palette(value: f64, range: (f64, f64)) -> String {
    let t = if range.1 > range.0 {
        ((value - range.0) / (range.1 - range.0)).clamp(0., 1.)
    } else {
        0.5
    };
    let r = (35. + 210. * t).round() as u8;
    let g = (75. + 110. * (1. - (2. * t - 1.).abs())).round() as u8;
    let b = (205. - 160. * t).round() as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}
pub(super) fn region(
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    if r.exprs.len() != 1 {
        return Err(extended::invalid("region_plot需要一个布尔条件"));
    }
    let (vars, locals) = extended::setup(r, eval, ctx)?;
    let raw = eval.prepare_numeric(&parse(&r.exprs[0])?, &locals, ctx)?;
    extended::checked_precision(&raw, ctx)?;
    let predicate = predicate::Predicate::compile(&raw, &vars, ctx, 0)?;
    let y = r
        .y_range
        .ok_or_else(|| extended::invalid("区域需要y范围"))?;
    let mut geometry = PlotGeometry2D::default();
    let mut work = vec![];
    for j in 0..N {
        for i in 0..N {
            ctx.tick()?;
            let x0 = r.x_range.0 + (r.x_range.1 - r.x_range.0) * i as f64 / N as f64;
            let x1 = r.x_range.0 + (r.x_range.1 - r.x_range.0) * (i + 1) as f64 / N as f64;
            let y0 = y.0 + (y.1 - y.0) * j as f64 / N as f64;
            let y1 = y.0 + (y.1 - y.0) * (j + 1) as f64 / N as f64;
            match predicate.evaluate(&[(x0 + x1) * 0.5, (y0 + y1) * 0.5], &mut work, ctx)? {
                Some(true) => geometry.tiles.push(PlotTile {
                    bounds: ((x0, y0), (x1, y1)),
                    value: 1.,
                    color: r
                        .options
                        .as_ref()
                        .and_then(|o| o.color.clone())
                        .unwrap_or_else(|| "#21854a".into()),
                }),
                None => geometry.skipped += 1,
                _ => {}
            }
        }
    }
    Ok(PlotData {
        geometry: Some(geometry),
        scale: None,
        curves: vec![],
        x_range: r.x_range,
        y_range: y,
        highlights: None,
    })
}
pub(super) fn density(
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    if r.exprs.len() != 1 {
        return Err(extended::invalid("density需要一个标量表达式"));
    }
    let f = extended::compile(&r.exprs[0], r, eval, ctx)?;
    let y = r
        .y_range
        .ok_or_else(|| extended::invalid("density需要y范围"))?;
    let mut work = vec![];
    let mut sampled = vec![];
    let mut geometry = PlotGeometry2D::default();
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for j in 0..N {
        for i in 0..N {
            ctx.tick()?;
            let x0 = r.x_range.0 + (r.x_range.1 - r.x_range.0) * i as f64 / N as f64;
            let x1 = r.x_range.0 + (r.x_range.1 - r.x_range.0) * (i + 1) as f64 / N as f64;
            let y0 = y.0 + (y.1 - y.0) * j as f64 / N as f64;
            let y1 = y.0 + (y.1 - y.0) * (j + 1) as f64 / N as f64;
            let value = f.eval_with_ctx(&[(x0 + x1) * 0.5, (y0 + y1) * 0.5], &mut work, ctx)?;
            if value.is_finite() {
                lo = lo.min(value);
                hi = hi.max(value);
                sampled.push((((x0, y0), (x1, y1)), value));
            } else {
                geometry.skipped += 1;
            }
        }
    }
    if sampled.is_empty() || !(hi - lo).is_finite() {
        return Err(extended::invalid("density颜色范围无有限样本或超出机器尺度"));
    }
    geometry.color_range = Some((lo, hi));
    geometry.tiles = sampled
        .into_iter()
        .map(|(bounds, value)| PlotTile {
            bounds,
            value,
            color: palette(value, (lo, hi)),
        })
        .collect();
    Ok(PlotData {
        geometry: Some(geometry),
        scale: None,
        curves: vec![],
        x_range: r.x_range,
        y_range: y,
        highlights: None,
    })
}
