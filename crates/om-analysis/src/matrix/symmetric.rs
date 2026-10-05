//! Real symmetric algorithms are scaled before products and reject non-positive pivots.
use super::{Error, Interrupt, Matrix};
fn symmetric(a: &Matrix, ctx: &Interrupt) -> Result<Matrix, Error> {
    if a.rows != a.cols {
        return Err(Error::Input("需要实对称方阵"));
    }
    let scale = a.max_abs();
    let mut out = a.clone();
    for i in 0..a.rows {
        for j in 0..a.cols {
            ctx.tick()?;
            let x = if scale == 0.0 {
                0.0
            } else {
                a.get(i, j) / scale
            };
            let y = if scale == 0.0 {
                0.0
            } else {
                a.get(j, i) / scale
            };
            if (x - y).abs() > 32.0 * f64::EPSILON {
                return Err(Error::Input("矩阵不是实对称矩阵"));
            }
            out.set(i, j, 0.5 * x + 0.5 * y)?;
        }
    }
    Ok(out)
}
/// Cholesky A=L*L^T for real positive-definite matrices; semidefinite matrices are explicitly rejected.
pub fn cholesky(a: &Matrix, ctx: &Interrupt) -> Result<Matrix, Error> {
    let scaled = symmetric(a, ctx)?;
    let n = a.rows;
    let scale = a.max_abs();
    if scale == 0.0 {
        return Err(Error::Input("Cholesky需要正定矩阵"));
    }
    let mut l = Matrix::new(n, n, vec![0.0; n * n])?;
    for i in 0..n {
        for j in 0..=i {
            let mut value = scaled.get(i, j);
            for k in 0..j {
                ctx.tick()?;
                value = (-l.get(i, k)).mul_add(l.get(j, k), value);
            }
            if i == j {
                if value <= 0.0 {
                    return Err(Error::Input("矩阵不正定，不能计算Cholesky"));
                }
                if value <= 32.0 * f64::EPSILON {
                    return Err(Error::IllConditioned);
                }
                l.set(i, j, value.sqrt())?;
            } else {
                l.set(i, j, value / l.get(j, j))?;
            }
        }
    }
    let root = scale.sqrt();
    for value in &mut l.data {
        ctx.tick()?;
        *value *= root;
        if !value.is_finite() {
            return Err(Error::NonFinite);
        }
    }
    Ok(l)
}
/// A genuine orthogonal eigenbasis, with eigenvectors stored as columns and values ascending.
#[derive(Clone, Debug)]
pub struct Eigen {
    /// Eigenvalues in ascending order.
    pub values: Vec<f64>,
    /// Matching eigenvector columns.
    pub vectors: Matrix,
    /// Actual number of plane rotations.
    pub rotations: usize,
}
/// Scaled maximum-pivot Jacobi iteration for real symmetric matrices only.
pub fn eigensystem(a: &Matrix, ctx: &Interrupt) -> Result<Eigen, Error> {
    let mut d = symmetric(a, ctx)?;
    let n = a.rows;
    let scale = a.max_abs();
    let mut v = Matrix::identity(n)?;
    let mut rotations = 0;
    let max_rotations = 100 * n * n;
    loop {
        let (mut p, mut q, mut largest) = (0, 0, 0.0);
        for i in 0..n {
            for j in i + 1..n {
                ctx.tick()?;
                if d.get(i, j).abs() > largest {
                    (p, q, largest) = (i, j, d.get(i, j).abs());
                }
            }
        }
        if largest <= 32.0 * f64::EPSILON {
            break;
        }
        if rotations >= max_rotations {
            return Err(Error::NoConvergence);
        }
        let apq = d.get(p, q);
        let tau = (d.get(q, q) - d.get(p, p)) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + tau.hypot(1.0))
        } else {
            -1.0 / ((-tau) + tau.hypot(1.0))
        };
        let c = 1.0 / t.hypot(1.0);
        let s = t * c;
        let pp = d.get(p, p) - t * apq;
        let qq = d.get(q, q) + t * apq;
        for k in 0..n {
            ctx.tick()?;
            if k != p && k != q {
                let x = d.get(k, p);
                let y = d.get(k, q);
                let a = c * x - s * y;
                let b = s * x + c * y;
                d.set(k, p, a)?;
                d.set(p, k, a)?;
                d.set(k, q, b)?;
                d.set(q, k, b)?;
            }
            let x = v.get(k, p);
            let y = v.get(k, q);
            v.set(k, p, c * x - s * y)?;
            v.set(k, q, s * x + c * y)?;
        }
        d.set(p, p, pp)?;
        d.set(q, q, qq)?;
        d.set(p, q, 0.0)?;
        d.set(q, p, 0.0)?;
        rotations += 1;
    }
    let mut order: Vec<_> = (0..n).collect();
    order.sort_by(|i, j| d.get(*i, *i).total_cmp(&d.get(*j, *j)));
    let mut values = vec![];
    let mut vectors = vec![0.0; n * n];
    for (j, column) in order.iter().enumerate() {
        ctx.tick()?;
        let value = d.get(*column, *column) * scale;
        if !value.is_finite() {
            return Err(Error::NonFinite);
        }
        values.push(value);
        for i in 0..n {
            ctx.tick()?;
            vectors[i * n + j] = v.get(i, *column);
        }
    }
    Ok(Eigen {
        values,
        vectors: Matrix::new(n, n, vectors)?,
        rotations,
    })
}
