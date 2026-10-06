//! Projected local BFGS. A gradient stopping condition never masquerades as certified optimality.
use super::*;
fn projected(
    x: &[f64],
    g: &[f64],
    bounds: Option<&[(f64, f64)]>,
    ctx: &Interrupt,
) -> Result<Vec<f64>, Error> {
    let mut p = g.to_vec();
    if let Some(bounds) = bounds {
        for i in 0..x.len() {
            ctx.tick()?;
            let (low, high) = bounds[i];
            if low == high || (x[i] <= low && g[i] >= 0.) || (x[i] >= high && g[i] <= 0.) {
                p[i] = 0.;
            }
        }
    }
    Ok(p)
}
fn identity(n: usize) -> Vec<Vec<f64>> {
    let mut h = vec![vec![0.; n]; n];
    for (i, row) in h.iter_mut().enumerate() {
        row[i] = 1.;
    }
    h
}
fn norm(g: &[f64]) -> f64 {
    g.iter().fold(0_f64, |n, g| n.max(g.abs()))
}
/// Minimize with an actual value/gradient callback, optionally within finite box bounds.
/// Initial values outside the box are rejected, not silently moved to another problem.
pub fn minimize<F>(
    mut f: F,
    initial: Vec<f64>,
    bounds: Option<&[(f64, f64)]>,
    options: &Options,
    ctx: &Interrupt,
) -> Result<Answer, Failure>
where
    F: FnMut(&[f64], &mut [f64], &Interrupt) -> Result<f64, Error>,
{
    let mut saved: Option<Answer> = None;
    let mut calls = 0;
    let mut evaluate = |x: &[f64], g: &mut [f64]| -> Result<f64, Error> {
        ctx.tick()?;
        calls += 1;
        g.fill(f64::NAN);
        let value = f(x, g, ctx)?;
        ctx.tick()?;
        if value.is_finite() && g.iter().all(|g| g.is_finite()) {
            Ok(value)
        } else {
            Err(Error::NonFinite)
        }
    };
    let result = (|| -> Result<Answer, Error> {
        ctx.tick()?;
        options.validate()?;
        let n = initial.len();
        if !(1..=64).contains(&n) || initial.iter().any(|x| !x.is_finite()) {
            return Err(Error::Input("BFGS需要1..64维有限初值"));
        }
        if let Some(bounds) = bounds
            && (bounds.len() != n
                || bounds.iter().zip(&initial).any(|((lo, hi), x)| {
                    !lo.is_finite() || !hi.is_finite() || lo > hi || x < lo || x > hi
                }))
        {
            return Err(Error::Input("盒约束必须有限有序、等维并包含初值"));
        }
        let mut x = initial;
        let mut g = vec![0.; n];
        let mut value = evaluate(&x, &mut g)?;
        let mut h = identity(n);
        for iteration in 0..=options.max_iterations {
            ctx.tick()?;
            let pg = projected(&x, &g, bounds, ctx)?;
            let answer = Answer {
                point: x.clone(),
                value,
                iterations: iteration,
                evaluations: 0,
                gradient_norm: Some(norm(&pg)),
                bracket_width: None,
                method: Method::Bfgs,
            };
            saved = Some(answer.clone());
            if norm(&pg) <= options.gradient_tol {
                return Ok(answer);
            }
            if iteration == options.max_iterations {
                return Err(Error::NoConvergence);
            }
            let mut p = vec![0.; n];
            for i in 0..n {
                p[i] = -dot(&h[i], &pg, ctx)?;
            }
            if let Some(bounds) = bounds {
                for i in 0..n {
                    if (x[i] <= bounds[i].0 && p[i] < 0.) || (x[i] >= bounds[i].1 && p[i] > 0.) {
                        p[i] = 0.;
                    }
                }
            }
            if dot(&g, &p, ctx)? >= 0. {
                h = identity(n);
                p = pg.iter().map(|g| -g).collect();
            }
            let mut accepted = None;
            let mut alpha = 1.;
            for _ in 0..64 {
                ctx.tick()?;
                let mut candidate = vec![0.; n];
                for i in 0..n {
                    ctx.tick()?;
                    candidate[i] = x[i] + alpha * p[i];
                    if let Some(bounds) = bounds {
                        candidate[i] = candidate[i].clamp(bounds[i].0, bounds[i].1);
                    }
                }
                if candidate.iter().any(|x| !x.is_finite()) {
                    alpha *= 0.5;
                    continue;
                }
                let step: Vec<_> = candidate
                    .iter()
                    .zip(&x)
                    .map(|(new, old)| new - old)
                    .collect();
                if step.iter().all(|s| *s == 0.) {
                    return Err(Error::Input("BFGS步长停滞而投影梯度仍未满足容差"));
                }
                let slope = dot(&g, &step, ctx)?;
                let mut new_g = vec![0.; n];
                let new_value = match evaluate(&candidate, &mut new_g) {
                    Ok(v) => v,
                    Err(Error::NonFinite) => {
                        alpha *= 0.5;
                        continue;
                    }
                    Err(e) => return Err(e),
                };
                if slope < 0. && new_value <= value + 1e-4 * slope {
                    accepted = Some((candidate, new_g, new_value, step));
                    break;
                }
                alpha *= 0.5;
            }
            let (candidate, new_g, new_value, step) =
                accepted.ok_or(Error::Input("BFGS线搜索未找到有限下降步"))?;
            saved = Some(Answer {
                point: candidate.clone(),
                value: new_value,
                iterations: iteration + 1,
                evaluations: 0,
                gradient_norm: Some(norm(&projected(&candidate, &new_g, bounds, ctx)?)),
                bracket_width: None,
                method: Method::Bfgs,
            });
            let y: Vec<_> = new_g.iter().zip(&g).map(|(new, old)| new - old).collect();
            let curvature = dot(&step, &y, ctx)?;
            let curvature_floor = 32. * f64::EPSILON * norm(&step) * norm(&y);
            if curvature.is_finite() && curvature > curvature_floor.max(0.) {
                let mut hy = vec![0.; n];
                for i in 0..n {
                    hy[i] = dot(&h[i], &y, ctx)?;
                }
                let factor = (1. + dot(&y, &hy, ctx)? / curvature) / curvature;
                let mut valid = true;
                for i in 0..n {
                    for j in 0..n {
                        ctx.tick()?;
                        h[i][j] += factor * step[i] * step[j]
                            - (hy[i] * step[j] + step[i] * hy[j]) / curvature;
                        valid &= h[i][j].is_finite();
                    }
                }
                if !valid {
                    h = identity(n);
                }
            } else {
                h = identity(n);
            }
            x = candidate;
            g = new_g;
            value = new_value;
        }
        Err(Error::NoConvergence)
    })();
    match result {
        Ok(mut answer) => {
            answer.evaluations = calls;
            Ok(answer)
        }
        Err(e) => {
            if let Some(answer) = &mut saved {
                answer.evaluations = calls;
            }
            Err(failed(e, saved))
        }
    }
}
