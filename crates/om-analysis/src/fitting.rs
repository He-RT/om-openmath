//! Pure full-rank machine least squares and scaled damped QR Levenberg–Marquardt.
mod lm;
mod qr;
use crate::Error;
pub use lm::nonlinear;
use om_num::ctx::Interrupt;
pub use qr::{least_squares, rank};
/// Nonlinear residual/gradient controls; parameter step size alone never indicates success.
#[derive(Clone, Debug)]
pub struct Options {
    /// Positive absolute residual-norm tolerance.
    pub abs_tol: f64,
    /// Nonnegative residual-norm tolerance relative to the initial residual.
    pub rel_tol: f64,
    /// Positive maximum column/residual cosine tolerance.
    pub gradient_tol: f64,
    /// Number of attempted LM steps, including rejections.
    pub max_iterations: usize,
    /// Positive dimensionless initial damping of column-normalized steps.
    pub initial_damping: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            abs_tol: 1e-10,
            rel_tol: 1e-8,
            gradient_tol: 1e-8,
            max_iterations: 200,
            initial_damping: 1e-3,
        }
    }
}
impl Options {
    fn validate(&self) -> Result<(), Error> {
        if !self.abs_tol.is_finite()
            || self.abs_tol <= 0.
            || !self.rel_tol.is_finite()
            || self.rel_tol < 0.
            || !self.gradient_tol.is_finite()
            || self.gradient_tol <= 0.
            || self.gradient_tol >= 1.
            || !(1..=100000).contains(&self.max_iterations)
            || !self.initial_damping.is_finite()
            || self.initial_damping <= 0.
        {
            return Err(Error::Input("拟合容差、迭代限额或阻尼无效"));
        }
        Ok(())
    }
}
/// The actually met numerical stopping rule, never a global optimum guarantee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Termination {
    /// Residual norm met the supplied absolute/relative tolerance.
    ResidualTolerance,
    /// All Jacobian column/residual cosines met the tolerance.
    GradientTolerance,
}
/// Complete evaluated fit progress, including actual residuals and parameter values.
#[derive(Clone, Debug)]
pub struct Answer {
    /// Machine parameter values in caller order.
    pub parameters: Vec<f64>,
    /// Actual prediction minus observation residuals.
    pub residuals: Vec<f64>,
    /// Euclidean norm calculated with hypot, without squaring overflow.
    pub residual_norm: f64,
    /// Maximum normalized column/residual inner product.
    pub gradient_cosine: f64,
    /// Actual attempted LM iterations.
    pub iterations: usize,
    /// Accepted LM steps.
    pub accepted_steps: usize,
    /// Rejected LM steps.
    pub rejected_steps: usize,
    /// Calls of the supplied full residual/Jacobian callback.
    pub evaluations: usize,
    /// Current column-normalized damping.
    pub damping: f64,
    /// Actual successful stopping reason (partial failure values do not assert success).
    pub termination: Termination,
}
/// Failure preserves only genuine evaluated progress and never invents convergence.
#[derive(Debug)]
pub struct Failure {
    /// Numeric/domain/cancellation reason.
    pub reason: Error,
    /// Last evaluated parameter/residual state when initialization succeeded.
    pub partial: Option<Box<Answer>>,
}
fn shape(rows: usize, cols: usize, len: usize) -> Result<(), Error> {
    if !(1..=16).contains(&cols)
        || rows < cols
        || rows > 10000
        || rows.saturating_mul(cols) > 100000
        || len != rows * cols
    {
        return Err(Error::Input(
            "拟合需要1..16参数、样本数≥参数数、≤10000样本且≤100000设计标量",
        ));
    }
    Ok(())
}
fn norm(values: &[f64], ctx: &Interrupt) -> Result<f64, Error> {
    let mut n = 0_f64;
    for x in values {
        ctx.tick()?;
        n = n.hypot(*x);
    }
    if n.is_finite() {
        Ok(n)
    } else {
        Err(Error::NonFinite)
    }
}
/// Rescaled product x*b/c; preserve finite/subnormal results without spurious intermediate overflow.
fn product(x: f64, b: f64, c: f64) -> Result<f64, Error> {
    if x == 0. || b == 0. {
        return Ok(0.);
    }
    if !x.is_finite() || !b.is_finite() || !c.is_finite() || b < 0. || c <= 0. {
        return Err(Error::NonFinite);
    }
    fn parts(x: f64) -> (f64, i32) {
        let (x, offset) = if x < f64::MIN_POSITIVE {
            (x * 2f64.powi(54), -54)
        } else {
            (x, 0)
        };
        let bits = x.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023 + offset;
        (
            f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52)),
            exponent,
        )
    }
    let (a, ae) = parts(x.abs());
    let (b, be) = parts(b);
    let (c, ce) = parts(c);
    let mut value = a * b / c;
    let mut exponent = ae + be - ce;
    while value >= 2. {
        value *= 0.5;
        exponent += 1;
    }
    while value < 1. {
        value *= 2.;
        exponent -= 1;
    }
    let result = if !(-1075..=1023).contains(&exponent) {
        f64::INFINITY
    } else if exponent < -1022 {
        (value * 2f64.powi(-1022)) * 2f64.powi(exponent + 1022)
    } else {
        value * 2f64.powi(exponent)
    };
    if !result.is_finite() || result == 0. {
        return Err(Error::NonFinite);
    }
    Ok(result.copysign(x))
}
fn columns(j: &[f64], m: usize, n: usize, ctx: &Interrupt) -> Result<Vec<f64>, Error> {
    let mut out = vec![0_f64; n];
    for i in 0..m {
        for k in 0..n {
            ctx.tick()?;
            out[k] = out[k].hypot(j[i * n + k]);
        }
    }
    if out.iter().any(|c| !c.is_finite() || *c == 0.) {
        return Err(Error::Input(
            "当前Jacobian存在零列或无法表示的列范数，参数局部不可识别",
        ));
    }
    Ok(out)
}
fn cosine(j: &[f64], r: &[f64], cols: &[f64], rn: f64, ctx: &Interrupt) -> Result<f64, Error> {
    if rn == 0. {
        return Ok(0.);
    }
    let n = cols.len();
    let mut maximum = 0_f64;
    for k in 0..n {
        let mut value = 0.;
        for i in 0..r.len() {
            ctx.tick()?;
            value = (j[i * n + k] / cols[k]).mul_add(r[i] / rn, value);
        }
        maximum = maximum.max(value.abs());
    }
    if maximum.is_finite() {
        Ok(maximum)
    } else {
        Err(Error::NonFinite)
    }
}
