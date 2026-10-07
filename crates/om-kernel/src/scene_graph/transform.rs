//! Actual affine point transforms; normals/winding are computed again from transformed triangles.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Transform {
    m: [[f64; 3]; 3],
    offset: [f64; 3],
    pub flipped: bool,
}
impl Transform {
    pub fn identity() -> Self {
        Self {
            m: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            offset: [0.; 3],
            flipped: false,
        }
    }
    pub fn apply(self, p: [f64; 3]) -> Result<[f64; 3], PlotError> {
        let out: [f64; 3] = std::array::from_fn(|i| {
            self.m[i].iter().zip(p).map(|(m, p)| m * p).sum::<f64>() + self.offset[i]
        });
        if out.iter().any(|v| !v.is_finite()) {
            return Err(error("变换坐标超出有限机器范围"));
        }
        Ok(out)
    }
    fn compose(self, next: Self) -> Result<Self, PlotError> {
        let mut out = Self::identity();
        for i in 0..3 {
            for j in 0..3 {
                out.m[i][j] = (0..3).map(|k| self.m[i][k] * next.m[k][j]).sum();
            }
        }
        out.offset = self.apply(next.offset)?;
        out.flipped = self.flipped ^ next.flipped;
        if out.m.iter().flatten().any(|v| !v.is_finite()) {
            return Err(error("变换矩阵超出机器范围"));
        }
        Ok(out)
    }
}
fn unit(v: [f64; 3]) -> Result<[f64; 3], PlotError> {
    let scale = v.iter().fold(0_f64, |m, v| m.max(v.abs()));
    if scale == 0. || !scale.is_finite() {
        return Err(error("方向向量须非零有限"));
    }
    let v = v.map(|v| v / scale);
    let norm = v[0].hypot(v[1]).hypot(v[2]);
    Ok(v.map(|v| v / norm))
}
pub(super) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
type Basis = ([f64; 3], [f64; 3], [f64; 3]);
pub(super) fn basis(normal: [f64; 3]) -> Result<Basis, PlotError> {
    let n = unit(normal)?;
    let index = (0..3)
        .min_by(|a, b| n[*a].abs().total_cmp(&n[*b].abs()))
        .expect("invariant: three coordinate axes are nonempty");
    let mut axis = [0.; 3];
    axis[index] = 1.;
    let u = unit(cross(n, axis))?;
    let v = cross(n, u);
    Ok((n, u, v))
}
impl Builder<'_> {
    pub(super) fn transformed(
        &mut self,
        e: &Expr,
        parent: Transform,
        base: &Style,
        depth: usize,
    ) -> Result<(), PlotError> {
        let name = e
            .head_symbol()
            .ok_or_else(|| error("变换需要符号头"))?
            .name();
        let (args, opts) = options(e, 2)?;
        let mut transform = Transform::identity();
        let mut center = [0.; 3];
        let mut axis = [0., 0., 1.];
        for (key, value) in &opts {
            match *key {
                "Center" if name != "Translate" => {
                    center = vector(value, self.dim, &self.ev, self.ctx)?
                }
                "Axis" if name == "Rotate" && self.dim == 3 => {
                    axis = vector(value, 3, &self.ev, self.ctx)?
                }
                _ => return Err(error("此变换不支持给定参数")),
            }
        }
        match name {
            "Translate" => transform.offset = vector(&args[1], self.dim, &self.ev, self.ctx)?,
            "Scale" => {
                let value = self.ev.fork_readonly().evaluate(&args[1], self.ctx)?;
                let factors = if value.is_head(B::LIST) {
                    vector(&value, self.dim, &self.ev, self.ctx)?
                } else {
                    let s = machine(&value, &self.ev, self.ctx)?;
                    if self.dim == 2 { [s, s, 1.] } else { [s; 3] }
                };
                transform.m = [
                    [factors[0], 0., 0.],
                    [0., factors[1], 0.],
                    [0., 0., if self.dim == 2 { 1. } else { factors[2] }],
                ];
                transform.flipped =
                    factors[..self.dim].iter().filter(|v| **v < 0.).count() % 2 == 1;
            }
            "Rotate" => {
                let angle = machine(&args[1], &self.ev, self.ctx)?;
                let (c, s) = (angle.cos(), angle.sin());
                let [x, y, z] = unit(axis)?;
                let t = 1. - c;
                transform.m = [
                    [c + x * x * t, x * y * t - z * s, x * z * t + y * s],
                    [y * x * t + z * s, c + y * y * t, y * z * t - x * s],
                    [z * x * t - y * s, z * y * t + x * s, c + z * z * t],
                ];
            }
            _ => return Err(error("未知变换")),
        }
        if name != "Translate" {
            for (i, value) in transform.offset.iter_mut().enumerate() {
                *value = center[i] - (0..3).map(|k| transform.m[i][k] * center[k]).sum::<f64>();
            }
        }
        self.add(&args[0], parent.compose(transform)?, base, depth + 1)
    }
}
