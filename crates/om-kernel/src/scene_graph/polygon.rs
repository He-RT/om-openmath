//! Planar simple polygons are actually triangulated by ear clipping; nonplanar/crossed boundaries are rejected.
use super::*;
fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
pub(super) fn faces(points: &[[f64; 3]], ctx: &Interrupt) -> Result<Vec<[usize; 3]>, PlotError> {
    if points.len() < 3 || points.len() > 2048 {
        return Err(error("polygon需要3..2048顶点"));
    }
    let base = points[0];
    let normal = (1..points.len() - 1)
        .find_map(|i| geometry::normal(base, points[i], points[i + 1]))
        .ok_or_else(|| error("polygon没有非零面积"))?;
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in points {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let scale = (0..3).map(|i| hi[i] - lo[i]).fold(0_f64, f64::max);
    if !scale.is_finite() || scale == 0. {
        return Err(error("polygon尺度不可表示"));
    }
    for p in points {
        ctx.tick()?;
        let distance = (0..3)
            .map(|i| (p[i] - base[i]) / scale * normal[i])
            .sum::<f64>();
        if distance.abs() > 1e-10 {
            return Err(error("polygon顶点必须共面"));
        }
    }
    let drop = (0..3)
        .max_by(|a, b| normal[*a].abs().total_cmp(&normal[*b].abs()))
        .expect("invariant: three coordinate axes are nonempty");
    let axes = (0..3).filter(|i| *i != drop).collect::<Vec<_>>();
    let q = points
        .iter()
        .map(|p| {
            [
                (p[axes[0]] - lo[axes[0]]) / scale,
                (p[axes[1]] - lo[axes[1]]) / scale,
            ]
        })
        .collect::<Vec<_>>();
    let n = q.len();
    for i in 0..n {
        let a = q[i];
        let b = q[(i + 1) % n];
        if a == b {
            return Err(error("polygon相邻顶点重复"));
        }
        for j in i + 1..n {
            ctx.tick()?;
            if j == (i + 1) % n || (j + 1) % n == i {
                continue;
            }
            let c = q[j];
            let d = q[(j + 1) % n];
            let on = |p: [f64; 2], x: [f64; 2], y: [f64; 2]| {
                orient(x, y, p).abs() < 1e-13
                    && p[0] >= x[0].min(y[0]) - 1e-13
                    && p[0] <= x[0].max(y[0]) + 1e-13
                    && p[1] >= x[1].min(y[1]) - 1e-13
                    && p[1] <= x[1].max(y[1]) + 1e-13
            };
            if orient(a, b, c) * orient(a, b, d) < 0. && orient(c, d, a) * orient(c, d, b) < 0.
                || on(c, a, b)
                || on(d, a, b)
                || on(a, c, d)
                || on(b, c, d)
            {
                return Err(error("polygon边界自交或重叠"));
            }
        }
    }
    let area = (0..n)
        .map(|i| q[i][0] * q[(i + 1) % n][1] - q[(i + 1) % n][0] * q[i][1])
        .sum::<f64>();
    if area.abs() < 1e-14 {
        return Err(error("polygon数值面积退化"));
    }
    let sign = area.signum();
    let mut active = (0..n).collect::<Vec<_>>();
    let mut output = vec![];
    while active.len() > 3 {
        ctx.tick()?;
        let mut ear = None;
        for k in 0..active.len() {
            let a = active[(k + active.len() - 1) % active.len()];
            let b = active[k];
            let c = active[(k + 1) % active.len()];
            let turn = orient(q[a], q[b], q[c]) * sign;
            if turn.abs() < 1e-14 {
                ear = Some((k, None));
                break;
            }
            if turn <= 0. {
                continue;
            }
            let mut inside = false;
            for &i in &active {
                ctx.tick()?;
                if i == a || i == b || i == c {
                    continue;
                }
                if orient(q[a], q[b], q[i]) * sign >= -1e-14
                    && orient(q[b], q[c], q[i]) * sign >= -1e-14
                    && orient(q[c], q[a], q[i]) * sign >= -1e-14
                {
                    inside = true;
                    break;
                }
            }
            if !inside {
                ear = Some((k, Some([a, b, c])));
                break;
            }
        }
        let (k, face) = ear.ok_or_else(|| error("polygon无法稳定三角化"))?;
        if let Some(face) = face {
            output.push(face);
        }
        active.remove(k);
    }
    output.push([active[0], active[1], active[2]]);
    Ok(output)
}
