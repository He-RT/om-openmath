//! Finite geometry, real winding and normals computed from actual mathematical samples.
use super::*;
pub(super) fn empty() -> Scene3DData {
    Scene3DData {
        meshes: vec![],
        lines: vec![],
        points: vec![],
        bounds: ([0.; 3], [0.; 3]),
        skipped: 0,
        sampled: true,
    }
}
pub(super) fn mesh(label: String) -> SceneMesh {
    SceneMesh {
        positions: vec![],
        normals: vec![],
        colors: vec![],
        triangles: vec![],
        label,
    }
}
pub(super) fn normal(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<[f64; 3]> {
    let u = std::array::from_fn::<_, 3, _>(|i| b[i] - a[i]);
    let v = std::array::from_fn::<_, 3, _>(|i| c[i] - a[i]);
    let scale = u.iter().chain(v.iter()).fold(0_f64, |m, x| m.max(x.abs()));
    if !scale.is_finite() || scale == 0. {
        return None;
    }
    let u = u.map(|x| x / scale);
    let v = v.map(|x| x / scale);
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = n[0].hypot(n[1]).hypot(n[2]);
    (length > 0. && length.is_finite()).then(|| n.map(|x| x / length))
}
pub(super) fn triangle(
    mesh: &mut SceneMesh,
    points: [[f64; 3]; 3],
    colors: [[f64; 4]; 3],
    ctx: &Interrupt,
) -> Result<(), PlotError> {
    ctx.tick()?;
    if let Some(normal) = normal(points[0], points[1], points[2]) {
        let at = mesh.positions.len() as u32;
        if at > 200000 {
            return Err(invalid("三维顶点超过200000预算"));
        }
        mesh.positions.extend(points);
        mesh.colors.extend(colors);
        mesh.normals.extend([normal; 3]);
        mesh.triangles.push([at, at + 1, at + 2]);
    }
    Ok(())
}
pub(super) fn finish(data: &mut Scene3DData, ctx: &Interrupt) -> Result<(), PlotError> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut count = 0usize;
    let mut add = |p: [f64; 3]| -> Result<(), PlotError> {
        ctx.tick()?;
        if p.iter().any(|v| !v.is_finite()) {
            return Err(invalid("几何坐标非有限"));
        }
        count += 1;
        if count > 200000 {
            return Err(invalid("三维几何超过200000顶点"));
        }
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
        Ok(())
    };
    for mesh in &data.meshes {
        if mesh.positions.len() != mesh.normals.len() || mesh.positions.len() != mesh.colors.len() {
            return Err(invalid("网格数据长度不一致"));
        }
        for &p in &mesh.positions {
            add(p)?;
        }
    }
    for line in &data.lines {
        if line.positions.len() != line.colors.len() {
            return Err(invalid("曲线颜色长度不一致"));
        }
        for &p in &line.positions {
            add(p)?;
        }
    }
    for p in &data.points {
        add(p.position)?;
    }
    if count == 0 {
        return Err(invalid(
            "未取得有限三维几何；空曲面/未采到交点/全部非有限，不伪造成功",
        ));
    }
    if (0..3).any(|i| !(hi[i] - lo[i]).is_finite()) {
        return Err(invalid("场景尺度超出机器范围"));
    }
    if lo == hi {
        for i in 0..3 {
            let padding = lo[i].abs().max(1.) * 0.01;
            lo[i] -= padding;
            hi[i] += padding;
        }
    }
    data.bounds = (lo, hi);
    Ok(())
}

pub(crate) fn validate(data: &Scene3DData, ctx: &Interrupt) -> Result<(), PlotError> {
    let mut count = 0usize;
    let valid_color = |color: &[f64; 4]| {
        color
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    };
    let valid_point = |point: &[f64; 3]| {
        point
            .iter()
            .enumerate()
            .all(|(i, v)| v.is_finite() && *v >= data.bounds.0[i] && *v <= data.bounds.1[i])
    };
    let finite_vector = |point: &[f64; 3]| point.iter().all(|v| v.is_finite());
    for mesh in &data.meshes {
        ctx.tick()?;
        count = count.saturating_add(mesh.positions.len());
        if mesh.positions.len() != mesh.colors.len() || mesh.positions.len() != mesh.normals.len() {
            return Err(invalid("网格数组长度无效"));
        }
        for ((p, n), c) in mesh.positions.iter().zip(&mesh.normals).zip(&mesh.colors) {
            ctx.tick()?;
            if !valid_point(p)
                || !finite_vector(n)
                || !valid_color(c)
                || (n[0].hypot(n[1]).hypot(n[2]) - 1.).abs() > 1e-8
            {
                return Err(invalid("网格坐标/法线/颜色无效"));
            }
        }
        if mesh.triangles.len() > 200000 {
            return Err(invalid("三角形数量超限"));
        }
        for face in &mesh.triangles {
            ctx.tick()?;
            if face.iter().any(|i| *i as usize >= mesh.positions.len())
                || face[0] == face[1]
                || face[0] == face[2]
                || face[1] == face[2]
            {
                return Err(invalid("三角形索引无效"));
            }
        }
    }
    for line in &data.lines {
        ctx.tick()?;
        count = count.saturating_add(line.positions.len());
        if line.positions.len() != line.colors.len()
            || !line.width.is_finite()
            || !(0.0..=32.0).contains(&line.width)
        {
            return Err(invalid("三维曲线形状/宽度无效"));
        }
        for (p, c) in line.positions.iter().zip(&line.colors) {
            ctx.tick()?;
            if !valid_point(p) || !valid_color(c) {
                return Err(invalid("曲线坐标/颜色无效"));
            }
        }
    }
    for point in &data.points {
        ctx.tick()?;
        count += 1;
        if !valid_point(&point.position)
            || !valid_color(&point.color)
            || !point.radius.is_finite()
            || !(0.0..=64.0).contains(&point.radius)
        {
            return Err(invalid("点坐标/颜色/半径无效"));
        }
    }
    if count == 0 || count > 200000 {
        return Err(invalid("三维总顶点数无效"));
    }
    let (lo, hi) = data.bounds;
    if !finite_vector(&lo)
        || !finite_vector(&hi)
        || (0..3).any(|i| lo[i] > hi[i] || !(hi[i] - lo[i]).is_finite())
        || lo == hi
    {
        return Err(invalid("场景范围无效"));
    }
    Ok(())
}
