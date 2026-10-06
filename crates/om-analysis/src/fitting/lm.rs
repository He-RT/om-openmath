//! Dimensionless column/residual normalization makes LM gain independent of units and SSE underflow.
use super::*;
/// Fit actual residuals and their Jacobian with full-rank local LM, not MINPACK or a fake linear fallback.
pub fn nonlinear<F>(
    mut f: F,
    initial: Vec<f64>,
    observations: usize,
    options: &Options,
    ctx: &Interrupt,
) -> Result<Answer, Failure>
where
    F: FnMut(&[f64], &mut [f64], &mut [f64], &Interrupt) -> Result<(), Error>,
{
    let n = initial.len();
    let m = observations;
    let mut evaluations = 0;
    let mut saved: Option<Answer> = None;
    let mut evaluate = |point: &[f64]| -> Result<(Vec<f64>, Vec<f64>), Error> {
        ctx.tick()?;
        evaluations += 1;
        let mut r = vec![f64::NAN; m];
        let mut j = vec![f64::NAN; m * n];
        f(point, &mut r, &mut j, ctx)?;
        ctx.tick()?;
        if r.iter().chain(&j).any(|x| !x.is_finite()) {
            return Err(Error::NonFinite);
        }
        Ok((r, j))
    };
    let result = (|| -> Result<Answer, Error> {
        ctx.tick()?;
        options.validate()?;
        shape(m, n, m.saturating_mul(n))?;
        if initial.iter().any(|x| !x.is_finite()) {
            return Err(Error::Input("LM初值必须有限"));
        }
        let mut point = initial;
        let (mut r, mut j) = evaluate(&point)?;
        let initial_norm = norm(&r, ctx)?;
        let threshold = options.abs_tol + options.rel_tol * initial_norm;
        if !threshold.is_finite() {
            return Err(Error::NonFinite);
        }
        let mut damping = options.initial_damping;
        let mut growth = 2.;
        let (mut accepted, mut rejected) = (0, 0);
        for iteration in 0..=options.max_iterations {
            ctx.tick()?;
            let rn = norm(&r, ctx)?;
            let column_norm = columns(&j, m, n, ctx)?;
            let gradient = cosine(&j, &r, &column_norm, rn, ctx)?;
            let state = Answer {
                parameters: point.clone(),
                residuals: r.clone(),
                residual_norm: rn,
                gradient_cosine: gradient,
                iterations: iteration,
                accepted_steps: accepted,
                rejected_steps: rejected,
                evaluations: 0,
                damping,
                termination: Termination::GradientTolerance,
            };
            saved = Some(state.clone());
            if qr::rank(m, n, &j, ctx)? != n {
                return Err(Error::Input(
                    "LM当前Jacobian数值列秩不足，不能宣称参数可识别",
                ));
            }
            if rn <= threshold {
                let mut state = state;
                state.termination = Termination::ResidualTolerance;
                return Ok(state);
            }
            if gradient <= options.gradient_tol {
                return Ok(state);
            }
            if iteration == options.max_iterations {
                return Err(Error::NoConvergence);
            }
            let mut augmented = vec![0.; (m + n) * n];
            let mut rhs = vec![0.; m + n];
            for i in 0..m {
                ctx.tick()?;
                rhs[i] = -r[i] / rn;
                for k in 0..n {
                    ctx.tick()?;
                    augmented[i * n + k] = j[i * n + k] / column_norm[k];
                }
            }
            for k in 0..n {
                augmented[(m + k) * n + k] = damping.sqrt();
            }
            let z = qr::augmented(m + n, n, &augmented, &rhs, ctx)?;
            let step = z
                .iter()
                .zip(&column_norm)
                .map(|(z, c)| product(*z, rn, *c))
                .collect::<Result<Vec<_>, _>>()?;
            let candidate: Vec<_> = point.iter().zip(&step).map(|(x, d)| x + d).collect();
            if candidate == point {
                return Err(Error::Input("LM步长达到机器分辨率，而残差/梯度容差未满足"));
            }
            let mut predicted = vec![0.; m];
            for i in 0..m {
                let mut v = r[i] / rn;
                for k in 0..n {
                    ctx.tick()?;
                    v = (j[i * n + k] / column_norm[k]).mul_add(z[k], v);
                }
                predicted[i] = v;
            }
            let pn = norm(&predicted, ctx)?;
            let predicted_reduction = (1. - pn) * (1. + pn);
            let trial = if candidate.iter().all(|x| x.is_finite()) {
                evaluate(&candidate)
            } else {
                Err(Error::NonFinite)
            };
            let mut keep = None;
            let ratio = match trial {
                Ok((new_r, new_j)) => {
                    let new_norm = norm(&new_r, ctx)?;
                    let ratio = new_norm / rn;
                    let gain = if predicted_reduction > 0. && ratio.is_finite() {
                        (1. - ratio) * (1. + ratio) / predicted_reduction
                    } else {
                        f64::NEG_INFINITY
                    };
                    if gain > 0. && gain.is_finite() && new_norm < rn {
                        keep = Some((new_r, new_j));
                    }
                    gain
                }
                Err(Error::NonFinite) => f64::NEG_INFINITY,
                Err(e) => return Err(e),
            };
            if let Some((new_r, new_j)) = keep {
                accepted += 1;
                point = candidate;
                r = new_r;
                j = new_j;
                damping *= (1. - (2. * ratio - 1.).powi(3)).max(1. / 3.);
                damping = damping.max(f64::MIN_POSITIVE);
                growth = 2.;
            } else {
                rejected += 1;
                damping *= growth;
                growth *= 2.;
            }
            if !damping.is_finite() || !growth.is_finite() {
                return Err(Error::Input("LM阻尼/线性化无法取得有限下降步"));
            }
        }
        Err(Error::NoConvergence)
    })();
    match result {
        Ok(mut a) => {
            a.evaluations = evaluations;
            Ok(a)
        }
        Err(reason) => {
            if let Some(a) = &mut saved {
                a.evaluations = evaluations;
            }
            Err(Failure {
                reason,
                partial: saved.map(Box::new),
            })
        }
    }
}
