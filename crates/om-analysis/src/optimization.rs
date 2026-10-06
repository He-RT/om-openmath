//! Local machine search with genuine objectives, projected gradients and explicit failures.
mod bfgs;
mod brent;
use crate::Error;
pub use bfgs::minimize;
pub use brent::bounded;
use om_num::ctx::Interrupt;
/// Machine search controls; coordinate and gradient tolerances have distinct meanings.
#[derive(Clone, Debug)]
pub struct Options {
    /// Positive absolute coordinate tolerance for bounded Brent.
    pub abs_tol: f64,
    /// Nonnegative relative coordinate tolerance for bounded Brent.
    pub rel_tol: f64,
    /// Positive infinity-norm projected gradient threshold for BFGS.
    pub gradient_tol: f64,
    /// Attempted outer iterations, excluding the separately bounded line search.
    pub max_iterations: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            abs_tol: 1e-10,
            rel_tol: 1e-8,
            gradient_tol: 1e-8,
            max_iterations: 1000,
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
            || !(1..=100000).contains(&self.max_iterations)
        {
            return Err(Error::Input("优化容差或迭代限额无效"));
        }
        Ok(())
    }
}
/// Actual method; neither variant certifies a global optimum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Bounded golden search with safeguarded inverse parabolic steps.
    Brent,
    /// BFGS inverse Hessian updates with optional box projection and Armijo steps.
    Bfgs,
}
/// Successful numerical stopping criterion and actual work, not an optimality certificate.
#[derive(Clone, Debug)]
pub struct Answer {
    /// Best saved actual coordinates.
    pub point: Vec<f64>,
    /// Genuine objective at the saved point.
    pub value: f64,
    /// Actual attempted outer iterations.
    pub iterations: usize,
    /// Actual objective callback evaluations, including rejected steps.
    pub evaluations: usize,
    /// Infinity norm of the saved projected gradient, absent for derivative-free Brent.
    pub gradient_norm: Option<f64>,
    /// Remaining physical bracket width for Brent, absent for BFGS.
    pub bracket_width: Option<f64>,
    /// Executed numerical method.
    pub method: Method,
}
/// Failure retains actually evaluated progress; no synthetic successful optimum.
#[derive(Debug)]
pub struct Failure {
    /// Actual numeric/domain/cancellation reason.
    pub reason: Error,
    /// Genuine saved work when an initial objective succeeded.
    pub partial: Option<Box<Answer>>,
}
fn failed(reason: Error, answer: Option<Answer>) -> Failure {
    Failure {
        reason,
        partial: answer.map(Box::new),
    }
}
fn dot(a: &[f64], b: &[f64], ctx: &Interrupt) -> Result<f64, Error> {
    let mut total = 0.;
    for (a, b) in a.iter().zip(b) {
        ctx.tick()?;
        total += a * b;
    }
    if total.is_finite() {
        Ok(total)
    } else {
        Err(Error::NonFinite)
    }
}
