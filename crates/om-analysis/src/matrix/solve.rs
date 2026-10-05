//! Numerical rank, null spaces and minimum-norm affine/least-squares solutions from real SVD factors.
use super::{Error, Interrupt, Matrix, Svd};
/// A numerical solution includes its actual residual and every numerical free direction.
#[derive(Clone, Debug)]
pub struct Solution {
    /// Minimum Euclidean norm particular solution.
    pub particular: Vec<f64>,
    /// Orthonormal null-space directions, one vector per row.
    pub nullspace: Vec<Vec<f64>>,
    /// Rank at the explicitly requested relative singular-value threshold.
    pub rank: usize,
    /// Actual Euclidean norm of A*x-b.
    pub residual_norm: f64,
    /// Residual satisfies the documented floating point backward tolerance; not an exact certificate.
    pub consistent: bool,
}
impl Svd {
    /// Count singular values above rel_tol times the largest value.
    pub fn rank(&self, rel_tol: f64) -> Result<usize, Error> {
        if !rel_tol.is_finite() || !(0.0..1.0).contains(&rel_tol) {
            return Err(Error::Input("相对秩容差需要0..1"));
        }
        let largest = self.values.first().copied().unwrap_or(0.0);
        if largest == 0.0 {
            return Ok(0);
        }
        Ok(self
            .values
            .iter()
            .filter(|x| **x > 0.0 && **x / largest > rel_tol)
            .count())
    }
    /// Numerical right null-space basis, including rectangular free dimensions.
    pub fn nullspace(&self, rel_tol: f64, ctx: &Interrupt) -> Result<Vec<Vec<f64>>, Error> {
        let rank = self.rank(rel_tol)?;
        let mut output = vec![];
        for j in rank..self.v.cols {
            let mut vector = vec![];
            for i in 0..self.v.rows {
                ctx.tick()?;
                vector.push(self.v.get(i, j));
            }
            output.push(vector);
        }
        Ok(output)
    }
    /// Minimum norm pseudoinverse solution plus residual/consistency and null directions.
    pub fn solve(
        &self,
        a: &Matrix,
        rhs: &[f64],
        rel_tol: f64,
        ctx: &Interrupt,
    ) -> Result<Solution, Error> {
        if a.rows != self.u.rows
            || a.cols != self.v.rows
            || rhs.len() != a.rows
            || rhs.iter().any(|x| !x.is_finite())
        {
            return Err(Error::Input("矩阵、因子及右端维度不一致"));
        }
        let rank = self.rank(rel_tol)?;
        let mut particular = vec![0.0; a.cols];
        for j in 0..rank {
            let mut coefficient = 0.0;
            for (i, value) in rhs.iter().enumerate() {
                ctx.tick()?;
                coefficient = self.u.get(i, j).mul_add(*value, coefficient);
            }
            coefficient /= self.values[j];
            if !coefficient.is_finite() {
                return Err(Error::NonFinite);
            }
            for (i, value) in particular.iter_mut().enumerate() {
                ctx.tick()?;
                *value = self.v.get(i, j).mul_add(coefficient, *value);
                if !value.is_finite() {
                    return Err(Error::NonFinite);
                }
            }
        }
        let (mut norm, mut maximum) = (0.0_f64, 0.0_f64);
        for (i, b) in rhs.iter().enumerate() {
            let mut r = -b;
            for (j, x) in particular.iter().enumerate() {
                ctx.tick()?;
                r = a.get(i, j).mul_add(*x, r);
            }
            if !r.is_finite() {
                return Err(Error::NonFinite);
            }
            norm = norm.hypot(r);
            maximum = maximum.max(r.abs());
        }
        if !norm.is_finite() {
            return Err(Error::NonFinite);
        }
        let bmax = rhs.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
        let xmax = particular.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
        let scale = a.max_abs().max(bmax);
        let consistent = if scale == 0.0 {
            true
        } else {
            let mut anorm = 0.0_f64;
            for i in 0..a.rows {
                let mut row = 0.0;
                for j in 0..a.cols {
                    ctx.tick()?;
                    row += a.get(i, j).abs() / scale;
                }
                anorm = anorm.max(row);
            }
            let reference = anorm * xmax + bmax / scale;
            if !reference.is_finite() {
                return Err(Error::NonFinite);
            }
            maximum / scale <= rel_tol.max(128.0 * f64::EPSILON) * reference
        };
        Ok(Solution {
            particular,
            nullspace: self.nullspace(rel_tol, ctx)?,
            rank,
            residual_norm: norm,
            consistent,
        })
    }
}
