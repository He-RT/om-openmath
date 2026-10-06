//! Tall column-pivoted Householder QR stores only R work and Q^T b, never an m×m Q.
use super::*;
const RANK_TOL: f64 = 1e-12;
struct Factor {
    r: Vec<f64>,
    rhs: Vec<f64>,
    columns: Vec<f64>,
    permutation: Vec<usize>,
    rhs_scale: f64,
    rank: usize,
    cols: usize,
}
fn factor(
    m: usize,
    n: usize,
    input: &[f64],
    rhs: Option<&[f64]>,
    ctx: &Interrupt,
) -> Result<Factor, Error> {
    ctx.tick()?;
    // LM adds up to 16 damping rows to a checked problem.
    if !(1..=16).contains(&n)
        || m < n
        || m > 10016
        || m.saturating_mul(n) > 100256
        || input.len() != m * n
        || input.iter().any(|x| !x.is_finite())
        || rhs.is_some_and(|r| r.len() != m || r.iter().any(|x| !x.is_finite()))
    {
        return Err(Error::Input("QR设计矩阵、右端或资源限额无效"));
    }
    let mut scale = vec![0_f64; n];
    for i in 0..m {
        for k in 0..n {
            ctx.tick()?;
            scale[k] = scale[k].max(input[i * n + k].abs());
        }
    }
    let mut a = input.to_vec();
    for i in 0..m {
        for k in 0..n {
            ctx.tick()?;
            a[i * n + k] = if scale[k] == 0. {
                0.
            } else {
                a[i * n + k] / scale[k]
            };
            if a[i * n + k] == 0. && input[i * n + k] != 0. {
                return Err(Error::NonFinite);
            }
        }
    }
    let b_scale = rhs.map_or(1., |r| {
        r.iter()
            .fold(0_f64, |a, x| a.max(x.abs()))
            .max(f64::MIN_POSITIVE)
    });
    let mut b = rhs.map_or_else(|| vec![0.; m], |r| r.iter().map(|x| x / b_scale).collect());
    if rhs.is_some_and(|r| {
        r.iter()
            .zip(&b)
            .any(|(raw, scaled)| *raw != 0. && *scaled == 0.)
    }) {
        return Err(Error::NonFinite);
    }
    let mut permutation: Vec<_> = (0..n).collect();
    let mut rank = 0;
    let mut initial_norm = 0_f64;
    for k in 0..n {
        let mut pivot = k;
        let mut pivot_norm = 0.;
        for column in k..n {
            let mut norm = 0_f64;
            for i in k..m {
                ctx.tick()?;
                norm = norm.hypot(a[i * n + column]);
            }
            if norm > pivot_norm {
                pivot_norm = norm;
                pivot = column;
            }
        }
        if k == 0 {
            initial_norm = pivot_norm;
        }
        if pivot_norm == 0. || pivot_norm / initial_norm <= RANK_TOL {
            break;
        }
        if pivot != k {
            for i in 0..m {
                ctx.tick()?;
                a.swap(i * n + k, i * n + pivot);
            }
            permutation.swap(k, pivot);
        }
        let sign = if a[k * n + k] >= 0. { 1. } else { -1. };
        let mut v: Vec<_> = (k..m).map(|i| a[i * n + k] / pivot_norm).collect();
        v[0] += sign;
        let length = norm(&v, ctx)?;
        for v in &mut v {
            *v /= length;
        }
        for column in k..n {
            let mut dot = 0.;
            for (i, v) in v.iter().enumerate() {
                ctx.tick()?;
                dot = v.mul_add(a[(k + i) * n + column], dot);
            }
            for (i, v) in v.iter().enumerate() {
                ctx.tick()?;
                a[(k + i) * n + column] = (-2. * v).mul_add(dot, a[(k + i) * n + column]);
            }
        }
        let mut dot = 0.;
        for (i, v) in v.iter().enumerate() {
            ctx.tick()?;
            dot = v.mul_add(b[k + i], dot);
        }
        for (i, v) in v.iter().enumerate() {
            ctx.tick()?;
            b[k + i] = (-2. * v).mul_add(dot, b[k + i]);
        }
        a[k * n + k] = -sign * pivot_norm;
        for i in k + 1..m {
            a[i * n + k] = 0.;
        }
        rank += 1;
    }
    if a.iter().chain(&b).any(|x| !x.is_finite()) {
        return Err(Error::NonFinite);
    }
    Ok(Factor {
        r: a,
        rhs: b,
        columns: scale,
        permutation,
        rhs_scale: b_scale,
        rank,
        cols: n,
    })
}
/// Numerical column rank at relative 1e-12 after column equilibration, not an exact certificate.
pub fn rank(rows: usize, cols: usize, data: &[f64], ctx: &Interrupt) -> Result<usize, Error> {
    shape(rows, cols, data.len())?;
    Ok(factor(rows, cols, data, None, ctx)?.rank)
}
fn solve(f: Factor, ctx: &Interrupt) -> Result<Vec<f64>, Error> {
    let n = f.cols;
    if f.rank != n {
        return Err(Error::Input(
            "拟合设计/Jacobian在相对1e-12阈值下列秩不足，不能唯一识别参数",
        ));
    }
    let mut z = vec![0.; n];
    for i in (0..n).rev() {
        let mut value = f.rhs[i];
        for (j, z) in z.iter().enumerate().skip(i + 1) {
            ctx.tick()?;
            value = (-f.r[i * n + j]).mul_add(*z, value);
        }
        z[i] = value / f.r[i * n + i];
        if !z[i].is_finite() {
            return Err(Error::NonFinite);
        }
    }
    let mut out = vec![0.; n];
    for i in 0..n {
        ctx.tick()?;
        out[f.permutation[i]] = product(z[i], f.rhs_scale, f.columns[f.permutation[i]])?;
    }
    Ok(out)
}
/// Full-column-rank least squares with checked sample/parameter limits and stable scaling.
pub fn least_squares(
    rows: usize,
    cols: usize,
    data: &[f64],
    rhs: &[f64],
    ctx: &Interrupt,
) -> Result<Vec<f64>, Error> {
    shape(rows, cols, data.len())?;
    solve(factor(rows, cols, data, Some(rhs), ctx)?, ctx)
}
pub(super) fn augmented(
    rows: usize,
    cols: usize,
    data: &[f64],
    rhs: &[f64],
    ctx: &Interrupt,
) -> Result<Vec<f64>, Error> {
    solve(factor(rows, cols, data, Some(rhs), ctx)?, ctx)
}
