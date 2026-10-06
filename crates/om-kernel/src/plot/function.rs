//! Uniform base sampling with bounded midpoint refinement and real domain breaks.
use super::{PlotError, Point};
use crate::protocol::{Curve, PlotData, PlotRequest};
use om_core::Interrupt;
use om_eval::numeric::CompiledFn;
const BASE: usize = 400;
fn point(f: &CompiledFn, x: f64, work: &mut Vec<f64>, ctx: &Interrupt) -> Result<Point, PlotError> {
    Ok((x, f.eval_with_ctx(&[x], work, ctx)?))
}
pub(super) fn viewport(mut values: Vec<f64>) -> (f64, f64) {
    values.retain(|v| v.is_finite());
    if values.is_empty() {
        return (-1.0, 1.0);
    }
    values.sort_by(f64::total_cmp);
    let span = values[values.len() - 1] - values[0];
    if span
        <= 1e-12
            * values
                .iter()
                .map(|v| v.abs())
                .fold(f64::MIN_POSITIVE, f64::max)
    {
        let value = values[values.len() / 2];
        let pad = (value.abs() * 0.1).max(1.0);
        if (value - pad).is_finite() && (value + pad).is_finite() {
            return (value - pad, value + pad);
        }
    }
    let lo = values[((values.len() - 1) as f64 * 0.02).floor() as usize];
    let hi = values[((values.len() - 1) as f64 * 0.98).ceil() as usize];
    let pad = if hi > lo {
        0.1 * (hi - lo)
    } else {
        (lo.abs() * 0.1).max(1.0)
    };
    let lower = (lo - pad).max(-f64::MAX);
    let upper = (hi + pad).min(f64::MAX);
    if lower < upper && (upper - lower).is_finite() {
        (lower, upper)
    } else {
        (-f64::MAX / 4.0, f64::MAX / 4.0)
    }
}
struct Refiner<'a> {
    f: &'a CompiledFn,
    ctx: &'a Interrupt,
    work: Vec<f64>,
    xspan: f64,
    yspan: f64,
    log_x: bool,
    points: Vec<Option<Point>>,
}
impl Refiner<'_> {
    fn add(&mut self, p: Point) {
        self.points.push(p.1.is_finite().then_some(p));
    }
    fn interval(&mut self, l: Point, r: Point, depth: u8) -> Result<(), PlotError> {
        self.ctx.tick()?;
        let x = if self.log_x {
            ((l.0.ln() + r.0.ln()) * 0.5).exp()
        } else {
            l.0 + (r.0 - l.0) * 0.5
        };
        if x == l.0 || x == r.0 {
            self.add(r);
            return Ok(());
        }
        let m = point(self.f, x, &mut self.work, self.ctx)?;
        let valid = l.1.is_finite() && m.1.is_finite() && r.1.is_finite();
        let angle = if valid {
            let a = ((m.1 - l.1) / self.yspan).atan2((m.0 - l.0) / self.xspan);
            let b = ((r.1 - m.1) / self.yspan).atan2((r.0 - m.0) / self.xspan);
            (a - b).abs()
        } else {
            0.0
        };
        let mixed = (l.1.is_finite() || m.1.is_finite() || r.1.is_finite()) && !valid;
        let jump = valid
            && (r.1 - l.1).abs() > 0.5 * self.yspan
            && (m.1 - l.1).signum() != (r.1 - m.1).signum();
        if depth < 6 && (mixed || angle > 10.0_f64.to_radians() || jump) {
            self.interval(l, m, depth + 1)?;
            self.interval(m, r, depth + 1)?;
        } else {
            if !m.1.is_finite() || jump {
                self.points.push(None);
            }
            if mixed && m.1.is_finite() {
                self.add(m);
            }
            self.add(r);
        }
        Ok(())
    }
}
pub(super) fn sample(
    r: &PlotRequest,
    fs: &[CompiledFn],
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    let mut bases = vec![];
    let mut ys = vec![];
    let mut work = vec![];
    for f in fs {
        let mut base = Vec::with_capacity(BASE);
        for i in 0..BASE {
            let x = if i + 1 == BASE {
                r.x_range.1
            } else if r.options.as_ref().is_some_and(|o| o.scale.log_x()) {
                (r.x_range.0.ln()
                    + (r.x_range.1.ln() - r.x_range.0.ln()) * (i as f64 / (BASE - 1) as f64))
                    .exp()
            } else {
                r.x_range.0 + (r.x_range.1 - r.x_range.0) * (i as f64 / (BASE - 1) as f64)
            };
            let p = point(f, x, &mut work, ctx)?;
            ys.push(p.1);
            base.push(p);
        }
        bases.push(base);
    }
    let log_y = r.options.as_ref().is_some_and(|o| o.scale.log_y());
    let y_range = if let Some(y) = r.y_range {
        y
    } else if log_y {
        let positives: Vec<_> = ys
            .iter()
            .copied()
            .filter(|v| v.is_finite() && *v > 0.)
            .collect();
        if positives.is_empty() {
            return Err(PlotError::Invalid("对数纵轴没有正的有限样本".into()));
        }
        let lo = positives.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = positives.iter().copied().fold(0_f64, f64::max);
        let y = (lo / 1.1, hi * 1.1);
        super::range(y)?;
        y
    } else {
        viewport(ys)
    };
    let mut curves = vec![];
    let mut skipped = 0;
    for ((f, base), label) in fs.iter().zip(bases).zip(&r.exprs) {
        let mut refine = Refiner {
            f,
            ctx,
            work: vec![],
            xspan: r.x_range.1 - r.x_range.0,
            yspan: y_range.1 - y_range.0,
            log_x: r.options.as_ref().is_some_and(|o| o.scale.log_x()),
            points: vec![],
        };
        refine.add(base[0]);
        for pair in base.windows(2) {
            refine.interval(pair[0], pair[1], 0)?;
        }
        let mut segments = vec![];
        let mut segment = vec![];
        for p in refine.points {
            if let Some(p) = p {
                if segment.last() != Some(&p) {
                    segment.push(p);
                }
            } else {
                skipped += 1;
                if segment.len() >= 2 {
                    segments.push(std::mem::take(&mut segment));
                }
                segment.clear();
            }
        }
        if segment.len() >= 2 {
            segments.push(segment);
        }
        curves.push(Curve {
            label: label.clone(),
            segments,
        });
    }
    Ok(PlotData {
        geometry: r.options.as_ref().map(|_| crate::protocol::PlotGeometry2D {
            skipped,
            ..Default::default()
        }),
        scale: None,
        curves,
        x_range: r.x_range,
        y_range,
        highlights: None,
    })
}
