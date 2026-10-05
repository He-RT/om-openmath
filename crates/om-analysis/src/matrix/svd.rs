//! Scaled one-sided Jacobi SVD, with genuine rotations and a completed orthogonal null basis.
use super::{Error, Interrupt, Matrix};
/// Full real singular-value decomposition A=U*S*V^T.
#[derive(Clone, Debug)]
pub struct Svd {
    /// Full m by m left orthogonal factor.
    pub u: Matrix,
    /// m by n diagonal singular-value matrix.
    pub s: Matrix,
    /// Full n by n right orthogonal factor.
    pub v: Matrix,
    /// min(m,n) nonnegative values descending.
    pub values: Vec<f64>,
    /// Actual Jacobi sweeps performed.
    pub sweeps: usize,
}
/// Dense real Jacobi SVD; convergence or representational limits are returned as errors.
pub fn svd(a: &Matrix, ctx: &Interrupt) -> Result<Svd, Error> {
    if a.rows < a.cols {
        let transposed = a.transpose(ctx)?;
        let f = svd(&transposed, ctx)?;
        return Ok(Svd {
            u: f.v,
            s: f.s.transpose(ctx)?,
            v: f.u,
            values: f.values,
            sweeps: f.sweeps,
        });
    }
    let (m, n) = (a.rows, a.cols);
    let scale = a.max_abs();
    let mut b = a.clone();
    let mut v = Matrix::identity(n)?;
    if scale != 0.0 {
        for value in &mut b.data {
            ctx.tick()?;
            let original = *value;
            *value /= scale;
            if original != 0.0 && *value == 0.0 {
                return Err(Error::NonFinite);
            }
        }
    }
    let mut sweeps = 0;
    if scale != 0.0 {
        loop {
            let mut changed = false;
            for p in 0..n {
                for q in p + 1..n {
                    let mut np = 0.0_f64;
                    let mut nq = 0.0_f64;
                    for i in 0..m {
                        ctx.tick()?;
                        np = np.hypot(b.get(i, p));
                        nq = nq.hypot(b.get(i, q));
                    }
                    if np == 0.0 || nq == 0.0 {
                        continue;
                    }
                    let mut rho = 0.0_f64;
                    for i in 0..m {
                        ctx.tick()?;
                        rho = (b.get(i, p) / np).mul_add(b.get(i, q) / nq, rho);
                    }
                    if rho.abs() <= 64.0 * f64::EPSILON {
                        continue;
                    }
                    let common = np.max(nq);
                    let (alpha, beta) = (np / common, nq / common);
                    let gamma = rho * alpha * beta;
                    if gamma == 0.0 {
                        // A correlated residual column below a representable Jacobi rotation
                        // is numerically zero. Independent tiny columns keep their values.
                        if np.min(nq) <= 64.0 * f64::EPSILON * np.max(nq) {
                            let column = if np <= nq { p } else { q };
                            for i in 0..m {
                                ctx.tick()?;
                                b.set(i, column, 0.0)?;
                            }
                            changed = true;
                            continue;
                        }
                        return Err(Error::NonFinite);
                    }
                    let delta = beta * beta - alpha * alpha;
                    let twice = 2.0 * gamma;
                    let magnitude = delta.hypot(twice);
                    let denominator = if delta >= 0.0 {
                        delta + magnitude
                    } else {
                        delta - magnitude
                    };
                    let t = twice / denominator;
                    if !t.is_finite() || t == 0.0 {
                        return Err(Error::NonFinite);
                    }
                    let c = 1.0 / t.hypot(1.0);
                    let s = t * c;
                    for i in 0..m {
                        ctx.tick()?;
                        let x = b.get(i, p);
                        let y = b.get(i, q);
                        b.set(i, p, (-s).mul_add(y, c * x))?;
                        b.set(i, q, s.mul_add(x, c * y))?;
                    }
                    for i in 0..n {
                        ctx.tick()?;
                        let x = v.get(i, p);
                        let y = v.get(i, q);
                        v.set(i, p, (-s).mul_add(y, c * x))?;
                        v.set(i, q, s.mul_add(x, c * y))?;
                    }
                    changed = true;
                }
            }
            sweeps += 1;
            if !changed {
                break;
            }
            if sweeps >= 100 {
                return Err(Error::NoConvergence);
            }
        }
    }
    let mut norms = vec![0.0_f64; n];
    for (j, value) in norms.iter_mut().enumerate() {
        for i in 0..m {
            ctx.tick()?;
            *value = value.hypot(b.get(i, j));
        }
    }
    let mut order: Vec<_> = (0..n).collect();
    order.sort_by(|i, j| norms[*j].total_cmp(&norms[*i]));
    let mut values = vec![];
    let mut u = Matrix::new(m, m, vec![0.0; m * m])?;
    let mut s = Matrix::new(m, n, vec![0.0; m * n])?;
    let mut sorted_v = Matrix::new(n, n, vec![0.0; n * n])?;
    let mut established = 0;
    for (j, column) in order.iter().enumerate() {
        ctx.tick()?;
        let value = norms[*column] * scale;
        if !value.is_finite() {
            return Err(Error::NonFinite);
        }
        values.push(value);
        s.set(j, j, value)?;
        for i in 0..n {
            ctx.tick()?;
            sorted_v.set(i, j, v.get(i, *column))?;
        }
        if norms[*column] > 0.0 {
            for i in 0..m {
                ctx.tick()?;
                u.set(i, j, b.get(i, *column) / norms[*column])?;
            }
            established = j + 1;
        }
    }
    // Remaining columns correspond to actual zero singular values or the rectangular null basis.
    for j in established..m {
        let mut found = false;
        for axis in 0..m {
            let mut candidate = vec![0.0; m];
            candidate[axis] = 1.0;
            for _ in 0..2 {
                for k in 0..j {
                    let mut dot = 0.0;
                    for (i, x) in candidate.iter().enumerate() {
                        ctx.tick()?;
                        dot = u.get(i, k).mul_add(*x, dot);
                    }
                    for (i, x) in candidate.iter_mut().enumerate() {
                        ctx.tick()?;
                        *x = (-dot).mul_add(u.get(i, k), *x);
                    }
                }
            }
            let norm = candidate.iter().fold(0.0_f64, |a, x| a.hypot(*x));
            if norm > 64.0 * f64::EPSILON {
                for (i, x) in candidate.iter().enumerate() {
                    u.set(i, j, *x / norm)?;
                }
                found = true;
                break;
            }
        }
        if !found {
            return Err(Error::NoConvergence);
        }
    }
    Ok(Svd {
        u,
        s,
        v: sorted_v,
        values,
        sweeps,
    })
}
