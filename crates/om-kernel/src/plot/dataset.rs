//! Data points and frequency bins stay mathematical data; frontends never recompute counts or colors.
use super::*;
pub(super) fn sample(r: &PlotRequest, ctx: &Interrupt) -> Result<PlotData, PlotError> {
    let o = r.options.as_ref().unwrap();
    if o.samples.is_empty()
        || o.samples.iter().map(Vec::len).sum::<usize>() > 100000
        || o.samples.iter().flatten().any(|v| !v.is_finite())
    {
        return Err(extended::invalid("数据图样本为空、过多或非有限"));
    }
    let mut geometry = PlotGeometry2D::default();
    let mut curves = vec![];
    let mut y = r.y_range.unwrap_or((0., 1.));
    if r.kind == PlotKind::Histogram {
        if !(1..=200).contains(&o.bins) || o.samples.iter().any(|p| p.len() != 1) {
            return Err(extended::invalid("直方图样本/箱数无效"));
        }
        let values: Vec<_> = o.samples.iter().map(|p| p[0]).collect();
        let mut lo = values.iter().copied().fold(f64::INFINITY, f64::min);
        let mut hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if lo == hi {
            let b = extended::bounds(&values)?;
            lo = b.0;
            hi = b.1;
        }
        range((lo, hi))?;
        let bins = o.bins as usize;
        let mut counts = vec![0usize; bins];
        for value in values {
            ctx.tick()?;
            let i = if value == hi {
                bins - 1
            } else {
                (((value - lo) / (hi - lo)) * bins as f64).floor() as usize
            }
            .min(bins - 1);
            counts[i] += 1;
        }
        y = r.y_range.unwrap_or((
            if o.scale.log_y() { 0.5 } else { 0. },
            (*counts.iter().max().unwrap() as f64 * 1.08).max(1.),
        ));
        for (i, n) in counts.iter().enumerate() {
            ctx.tick()?;
            let a = lo + (hi - lo) * i as f64 / bins as f64;
            let b = lo + (hi - lo) * (i + 1) as f64 / bins as f64;
            geometry.tiles.push(PlotTile {
                bounds: ((a, 0.), (b, *n as f64)),
                value: *n as f64,
                color: o.color.clone().unwrap_or_else(|| "#21854a".into()),
            });
        }
    } else if o.style == DataPlotStyle::Heatmap {
        let n = o.samples[0].len();
        if o.samples.iter().any(|r| r.len() != n) {
            return Err(extended::invalid("热图必须矩形"));
        }
        let lo = o
            .samples
            .iter()
            .flatten()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let hi = o
            .samples
            .iter()
            .flatten()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        if !(hi - lo).is_finite() {
            return Err(extended::invalid("热图颜色范围超出机器表示"));
        }
        geometry.color_range = Some((lo, hi));
        for (j, row) in o.samples.iter().enumerate() {
            for (i, value) in row.iter().enumerate() {
                ctx.tick()?;
                geometry.tiles.push(PlotTile {
                    bounds: ((i as f64, j as f64), ((i + 1) as f64, (j + 1) as f64)),
                    value: *value,
                    color: grid::palette(*value, (lo, hi)),
                });
            }
        }
    } else {
        if o.samples.iter().any(|p| p.len() != 2) {
            return Err(extended::invalid("数据图需要成对坐标"));
        }
        let points: Vec<_> = o.samples.iter().map(|p| (p[0], p[1])).collect();
        if o.style == DataPlotStyle::Line {
            curves.push(Curve {
                label: "data".into(),
                segments: vec![points],
            });
        } else {
            geometry.points = points;
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
pub(super) fn validate_log(d: &mut PlotData) -> Result<(), PlotError> {
    let scale = d.scale.unwrap_or_default();
    if scale == PlotScale::Linear {
        return Ok(());
    }
    // Bounds and every retained primitive must lie in the chosen positive log domain.
    if scale.log_x() && d.x_range.0 <= 0. || scale.log_y() && d.y_range.0 <= 0. {
        return Err(extended::invalid(
            "对数绘图窗口必须为正范围，请显式给plot_range",
        ));
    }
    let mut skipped = 0;
    for curve in &mut d.curves {
        let mut segments = vec![];
        for segment in &curve.segments {
            let mut run = vec![];
            for p in segment {
                if (!scale.log_x() || p.0 > 0.) && (!scale.log_y() || p.1 > 0.) {
                    run.push(*p);
                } else {
                    skipped += 1;
                    if run.len() > 1 {
                        segments.push(std::mem::take(&mut run));
                    }
                    run.clear();
                }
            }
            if run.len() > 1 {
                segments.push(run);
            }
        }
        curve.segments = segments;
    }
    if skipped > 0 && d.geometry.is_none() {
        d.geometry = Some(Default::default());
    }
    if let Some(g) = &mut d.geometry {
        g.skipped += skipped;
        let valid = |p: (f64, f64)| (!scale.log_x() || p.0 > 0.) && (!scale.log_y() || p.1 > 0.);
        let before = g.points.len() + g.tiles.len() + g.arrows.len();
        g.points.retain(|p| valid(*p));
        for tile in &mut g.tiles {
            if scale.log_x() {
                tile.bounds.0.0 = tile.bounds.0.0.max(d.x_range.0);
            }
            if scale.log_y() {
                tile.bounds.0.1 = tile.bounds.0.1.max(d.y_range.0);
            }
        }
        g.tiles.retain(|p| {
            valid(p.bounds.0)
                && valid(p.bounds.1)
                && p.bounds.1.0 > p.bounds.0.0
                && p.bounds.1.1 > p.bounds.0.1
        });
        g.arrows.retain(|p| valid(p.start) && valid(p.end));
        g.skipped += (before - g.points.len() - g.tiles.len() - g.arrows.len()) as u32;
    }
    Ok(())
}
