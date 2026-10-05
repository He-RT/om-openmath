//! Handwritten small dense LU/Householder QR algorithms; all loops use the caller's budget.
use crate::Error;
mod solve;
mod svd;
mod symmetric;
use om_num::ctx::Interrupt;
pub use solve::Solution;
pub use svd::{Svd, svd};
pub use symmetric::{Eigen, cholesky, eigensystem};
/// A checked, finite row-major real matrix, at most 64 by 64.
#[derive(Clone, Debug)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}
impl Matrix {
    /// Validate shape, capacity and finite input values.
    pub fn new(rows: usize, cols: usize, data: Vec<f64>) -> Result<Self, Error> {
        if rows == 0 || cols == 0 || rows > 64 || cols > 64 || data.len() != rows * cols {
            return Err(Error::Input("矩阵需要1..64行/列和一致的数据长度"));
        }
        if data.iter().any(|x| !x.is_finite()) {
            return Err(Error::NonFinite);
        }
        Ok(Self { rows, cols, data })
    }
    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }
    /// Number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }
    /// Borrow row-major data.
    pub fn data(&self) -> &[f64] {
        &self.data
    }
    /// Read a checked caller-selected row/column.
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row * self.cols + col]
    }
    fn set(&mut self, row: usize, col: usize, value: f64) -> Result<(), Error> {
        if !value.is_finite() {
            return Err(Error::NonFinite);
        }
        self.data[row * self.cols + col] = value;
        Ok(())
    }
    /// Identity matrix within the supported dimension range.
    pub fn identity(n: usize) -> Result<Self, Error> {
        if n == 0 || n > 64 {
            return Err(Error::Input("单位矩阵尺寸需要1..64"));
        }
        let mut data = vec![0.0; n * n];
        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Self::new(n, n, data)
    }
    /// Numerically checked matrix product; no function reevaluation.
    pub fn multiply(&self, rhs: &Self, ctx: &Interrupt) -> Result<Self, Error> {
        if self.cols != rhs.rows {
            return Err(Error::Input("矩阵乘法维度不匹配"));
        }
        let mut out = vec![0.0; self.rows * rhs.cols];
        for i in 0..self.rows {
            for j in 0..rhs.cols {
                let mut sum = 0.0;
                for k in 0..self.cols {
                    ctx.tick()?;
                    sum = self.get(i, k).mul_add(rhs.get(k, j), sum);
                }
                if !sum.is_finite() {
                    return Err(Error::NonFinite);
                }
                out[i * rhs.cols + j] = sum;
            }
        }
        Self::new(self.rows, rhs.cols, out)
    }
    /// Pure transpose of stored samples.
    pub fn transpose(&self, ctx: &Interrupt) -> Result<Self, Error> {
        let mut out = vec![0.0; self.data.len()];
        for i in 0..self.rows {
            for j in 0..self.cols {
                ctx.tick()?;
                out[j * self.rows + i] = self.get(i, j);
            }
        }
        Self::new(self.cols, self.rows, out)
    }
    fn max_abs(&self) -> f64 {
        self.data.iter().fold(0.0_f64, |a, x| a.max(x.abs()))
    }
}
/// Partial-pivot LU factors satisfy P*A=L*U; permutation[i] names the original row.
#[derive(Clone, Debug)]
pub struct Lu {
    /// Unit-diagonal lower factor.
    pub l: Matrix,
    /// Upper factor.
    pub u: Matrix,
    /// Actual row permutation.
    pub permutation: Vec<usize>,
    /// Number of row swaps, for determinant sign.
    pub swaps: usize,
    scale: f64,
}
/// Compute a real rectangular LU factorization with deterministic largest-magnitude row pivots.
pub fn lu(a: &Matrix, ctx: &Interrupt) -> Result<Lu, Error> {
    ctx.tick()?;
    let (m, n) = (a.rows, a.cols);
    let mut l = Matrix::identity(m)?;
    let mut u = a.clone();
    let mut p: Vec<_> = (0..m).collect();
    let mut swaps = 0;
    for k in 0..m.min(n) {
        let mut pivot = k;
        for i in k..m {
            ctx.tick()?;
            if u.get(i, k).abs() > u.get(pivot, k).abs() {
                pivot = i;
            }
        }
        if pivot != k {
            for j in 0..n {
                ctx.tick()?;
                u.data.swap(k * n + j, pivot * n + j);
            }
            for j in 0..k {
                ctx.tick()?;
                l.data.swap(k * m + j, pivot * m + j);
            }
            p.swap(k, pivot);
            swaps += 1;
        }
        if u.get(k, k) == 0.0 {
            continue;
        }
        for i in k + 1..m {
            ctx.tick()?;
            let factor = u.get(i, k) / u.get(k, k);
            l.set(i, k, factor)?;
            u.set(i, k, 0.0)?;
            for j in k + 1..n {
                ctx.tick()?;
                u.set(i, j, (-factor).mul_add(u.get(k, j), u.get(i, j)))?;
            }
        }
    }
    Ok(Lu {
        l,
        u,
        permutation: p,
        swaps,
        scale: a.max_abs(),
    })
}
impl Lu {
    /// Solve with an explicit relative pivot tolerance; singular and ill-conditioned remain errors.
    pub fn solve(&self, rhs: &[f64], rel_tol: f64, ctx: &Interrupt) -> Result<Vec<f64>, Error> {
        let n = self.u.rows;
        if self.u.rows != self.u.cols {
            return Err(Error::Input("LU求解需要方阵"));
        }
        if rhs.len() != n
            || rhs.iter().any(|x| !x.is_finite())
            || !rel_tol.is_finite()
            || rel_tol < 0.0
        {
            return Err(Error::Input("右端或容差无效"));
        }
        let mut x = vec![0.0; n];
        for i in 0..n {
            ctx.tick()?;
            let mut value = rhs[self.permutation[i]];
            for (j, entry) in x.iter().enumerate().take(i) {
                ctx.tick()?;
                value = (-self.l.get(i, j)).mul_add(*entry, value);
            }
            if !value.is_finite() {
                return Err(Error::NonFinite);
            }
            x[i] = value;
        }
        for i in (0..n).rev() {
            let pivot = self.u.get(i, i);
            if pivot == 0.0 {
                return Err(Error::Singular);
            }
            if pivot.abs() / self.scale <= rel_tol {
                return Err(Error::IllConditioned);
            }
            let mut value = x[i];
            for (j, entry) in x.iter().enumerate().skip(i + 1) {
                ctx.tick()?;
                value = (-self.u.get(i, j)).mul_add(*entry, value);
            }
            x[i] = value / pivot;
            if !x[i].is_finite() {
                return Err(Error::NonFinite);
            }
        }
        Ok(x)
    }
    /// Actual product of diagonal factors with permutation sign.
    pub fn determinant(&self, ctx: &Interrupt) -> Result<f64, Error> {
        if self.u.rows != self.u.cols {
            return Err(Error::Input("行列式需要方阵"));
        }
        let mut value = if self.swaps.is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        for i in 0..self.u.rows {
            ctx.tick()?;
            value *= self.u.get(i, i);
            if !value.is_finite() {
                return Err(Error::NonFinite);
            }
        }
        Ok(value)
    }
}
/// Full Householder QR: Q is m by m, R is m by n; A=Q*R.
#[derive(Clone, Debug)]
pub struct Qr {
    /// Orthogonal factor.
    pub q: Matrix,
    /// Upper trapezoidal factor.
    pub r: Matrix,
    scale: f64,
}
/// Budgeted Householder reflections avoid squared-norm overflow and cancellation in the first component.
pub fn qr(a: &Matrix, ctx: &Interrupt) -> Result<Qr, Error> {
    ctx.tick()?;
    let (m, n) = (a.rows, a.cols);
    let mut q = Matrix::identity(m)?;
    let mut r = a.clone();
    for k in 0..m.min(n) {
        let mut norm = 0.0_f64;
        for i in k..m {
            ctx.tick()?;
            norm = norm.hypot(r.get(i, k));
        }
        if norm == 0.0 {
            continue;
        }
        if !norm.is_finite() {
            return Err(Error::NonFinite);
        }
        let sign = if r.get(k, k) >= 0.0 { 1.0 } else { -1.0 };
        let mut v: Vec<_> = (k..m).map(|i| r.get(i, k) / norm).collect();
        v[0] += sign;
        let length = v.iter().fold(0.0_f64, |a, x| a.hypot(*x));
        for x in &mut v {
            *x /= length;
        }
        for j in k..n {
            let mut dot = 0.0;
            for (offset, x) in v.iter().enumerate() {
                ctx.tick()?;
                dot = x.mul_add(r.get(k + offset, j), dot);
            }
            for (offset, x) in v.iter().enumerate() {
                ctx.tick()?;
                r.set(k + offset, j, (-2.0 * x).mul_add(dot, r.get(k + offset, j)))?;
            }
        }
        r.set(k, k, -sign * norm)?;
        for i in k + 1..m {
            r.set(i, k, 0.0)?;
        }
        for i in 0..m {
            let mut dot = 0.0;
            for (offset, x) in v.iter().enumerate() {
                ctx.tick()?;
                dot = q.get(i, k + offset).mul_add(*x, dot);
            }
            for (offset, x) in v.iter().enumerate() {
                ctx.tick()?;
                q.set(i, k + offset, (-2.0 * x).mul_add(dot, q.get(i, k + offset)))?;
            }
        }
    }
    Ok(Qr {
        q,
        r,
        scale: a.max_abs(),
    })
}
impl Qr {
    /// Unique full-column-rank least squares for m>=n; rank-deficient problems are not disguised as success.
    pub fn least_squares(
        &self,
        rhs: &[f64],
        rel_tol: f64,
        ctx: &Interrupt,
    ) -> Result<Vec<f64>, Error> {
        let (m, n) = (self.r.rows, self.r.cols);
        if m < n {
            return Err(Error::Input("QR最小二乘需要行数≥列数"));
        }
        if rhs.len() != m
            || rhs.iter().any(|x| !x.is_finite())
            || !rel_tol.is_finite()
            || rel_tol < 0.0
        {
            return Err(Error::Input("右端或容差无效"));
        }
        let mut x = vec![0.0; n];
        for (i, entry) in x.iter_mut().enumerate() {
            for (j, value) in rhs.iter().enumerate() {
                ctx.tick()?;
                *entry = self.q.get(j, i).mul_add(*value, *entry);
            }
        }
        for i in (0..n).rev() {
            ctx.tick()?;
            let pivot = self.r.get(i, i);
            if pivot == 0.0 {
                return Err(Error::Singular);
            }
            if pivot.abs() / self.scale <= rel_tol {
                return Err(Error::IllConditioned);
            }
            let mut value = x[i];
            for (j, entry) in x.iter().enumerate().skip(i + 1) {
                ctx.tick()?;
                value = (-self.r.get(i, j)).mul_add(*entry, value);
            }
            x[i] = value / pivot;
            if !x[i].is_finite() {
                return Err(Error::NonFinite);
            }
        }
        Ok(x)
    }
}
