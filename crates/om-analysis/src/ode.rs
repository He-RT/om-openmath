//! Self-written real DP5(4), quartic continuous extension and simple sign-crossing event termination.
use crate::{
    Error,
    interpolation::{Interpolation, Method},
};
use om_num::ctx::Interrupt;
/// Numerical controls for nonstiff machine initial-value problems.
#[derive(Clone, Debug)]
pub struct Options {
    /// Absolute component error tolerance.
    pub abs_tol: f64,
    /// Relative component error tolerance.
    pub rel_tol: f64,
    /// Maximum attempted steps, including rejections.
    pub max_steps: usize,
    /// Optional first positive step magnitude.
    pub initial_step: Option<f64>,
    /// Positive maximum step magnitude (infinity means no explicit cap).
    pub max_step: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            abs_tol: 1e-10,
            rel_tol: 1e-8,
            max_steps: 100000,
            initial_step: None,
            max_step: f64::INFINITY,
        }
    }
}
/// Why a successful numerical trajectory ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Termination {
    /// Requested final time reached.
    End,
    /// First detected endpoint zero or sign crossing terminated work.
    Event,
}
/// Actual trajectory and numerical work; tolerances are estimates, not certificates.
#[derive(Clone, Debug)]
pub struct Answer {
    /// Actual continuous interpolation on the computed time domain.
    pub solution: Interpolation,
    /// End or event, distinct from a failed iteration.
    pub termination: Termination,
    /// Actual accepted steps.
    pub accepted_steps: usize,
    /// Actual error-rejected steps.
    pub rejected_steps: usize,
    /// Actual RHS callback evaluations.
    pub evaluations: usize,
}
/// A failure can retain the actually computed partial trajectory without pretending to converge.
#[derive(Debug)]
pub struct Failure {
    /// Real failure or cancellation.
    pub reason: Error,
    /// Saved partial trajectory and work when initialization succeeded.
    pub partial: Option<Box<Answer>>,
}
/// Optional scalar zero-crossing event supplied by the caller.
pub type Event<'a> = dyn FnMut(f64, &[f64], &Interrupt) -> Result<f64, Error> + 'a;
const C: [f64; 7] = [0., 1. / 5., 3. / 10., 4. / 5., 8. / 9., 1., 1.];
const A: [&[f64]; 7] = [
    &[],
    &[1. / 5.],
    &[3. / 40., 9. / 40.],
    &[44. / 45., -56. / 15., 32. / 9.],
    &[
        19372. / 6561.,
        -25360. / 2187.,
        64448. / 6561.,
        -212. / 729.,
    ],
    &[
        9017. / 3168.,
        -355. / 33.,
        46732. / 5247.,
        49. / 176.,
        -5103. / 18656.,
    ],
    &[
        35. / 384.,
        0.,
        500. / 1113.,
        125. / 192.,
        -2187. / 6784.,
        11. / 84.,
    ],
];
const E: [f64; 7] = [
    -71. / 57600.,
    0.,
    71. / 16695.,
    -71. / 1920.,
    17253. / 339200.,
    -22. / 525.,
    1. / 40.,
];
const P: [[f64; 4]; 7] = [
    [
        1.,
        -8048581381. / 2820520608.,
        8663915743. / 2820520608.,
        -12715105075. / 11282082432.,
    ],
    [0.; 4],
    [
        0.,
        131558114200. / 32700410799.,
        -68118460800. / 10900136933.,
        87487479700. / 32700410799.,
    ],
    [
        0.,
        -1754552775. / 470086768.,
        14199869525. / 1410260304.,
        -10690763975. / 1880347072.,
    ],
    [
        0.,
        127303824393. / 49829197408.,
        -318862633887. / 49829197408.,
        701980252875. / 199316789632.,
    ],
    [
        0.,
        -282668133. / 205662961.,
        2019193451. / 616988883.,
        -1453857185. / 822651844.,
    ],
    [
        0.,
        40617522. / 29380423.,
        -110615467. / 29380423.,
        69997945. / 29380423.,
    ],
];
fn rhs<F>(
    f: &mut F,
    t: f64,
    y: &[f64],
    out: &mut [f64],
    count: &mut usize,
    ctx: &Interrupt,
) -> Result<(), Error>
where
    F: FnMut(f64, &[f64], &mut [f64], &Interrupt) -> Result<(), Error>,
{
    ctx.tick()?;
    out.fill(f64::NAN);
    *count += 1;
    f(t, y, out, ctx)?;
    if out.iter().any(|x| !x.is_finite()) {
        Err(Error::NonFinite)
    } else {
        Ok(())
    }
}
fn event_value(event: &mut Event<'_>, t: f64, y: &[f64], ctx: &Interrupt) -> Result<f64, Error> {
    ctx.tick()?;
    let value = event(t, y, ctx)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::NonFinite)
    }
}
/// Solve a 1..64-dimensional nonstiff real IVP in either time direction; no automatic stiff fallback.
pub fn solve<F>(
    mut f: F,
    mut event: Option<&mut Event<'_>>,
    start: f64,
    end: f64,
    initial: Vec<f64>,
    options: &Options,
    ctx: &Interrupt,
) -> Result<Answer, Failure>
where
    F: FnMut(f64, &[f64], &mut [f64], &Interrupt) -> Result<(), Error>,
{
    let initialize = || -> Result<Answer, Error> {
        ctx.tick()?;
        let span = (end - start).abs();
        if !start.is_finite()
            || !end.is_finite()
            || !span.is_finite()
            || !(1..=64).contains(&initial.len())
            || initial.iter().any(|x| !x.is_finite())
            || !options.abs_tol.is_finite()
            || !options.rel_tol.is_finite()
            || options.abs_tol <= 0.
            || options.rel_tol < 0.
            || !(1..=1000000).contains(&options.max_steps)
            || options.max_step.is_nan()
            || options.max_step <= 0.
            || options
                .initial_step
                .is_some_and(|h| !h.is_finite() || h <= 0.)
        {
            return Err(Error::Input("ODE边界、1..64维初值或容差/步数控制无效"));
        }
        Ok(Answer {
            solution: Interpolation::new(
                vec![start],
                vec![initial.clone()],
                vec![],
                Method::DormandPrince,
                ctx,
            )?,
            termination: Termination::End,
            accepted_steps: 0,
            rejected_steps: 0,
            evaluations: 0,
        })
    };
    let mut answer = initialize().map_err(|reason| Failure {
        reason,
        partial: None,
    })?;
    let result = (|| -> Result<(), Error> {
        let direction = if end >= start { 1. } else { -1. };
        let span = (end - start).abs();
        let mut t = start;
        let mut y = initial;
        let mut g = if let Some(e) = event.as_deref_mut() {
            Some(event_value(e, t, &y, ctx)?)
        } else {
            None
        };
        if g == Some(0.) {
            answer.termination = Termination::Event;
            return Ok(());
        }
        if start == end {
            return Ok(());
        }
        let mut h = options
            .initial_step
            .unwrap_or(span * 0.01)
            .min(options.max_step)
            .min(span)
            * direction;
        if h == 0. || t + h == t {
            return Err(Error::Input("ODE初始步无法在机器时间尺度上表示"));
        }
        let d = y.len();
        let mut k = vec![vec![0.; d]; 7];
        rhs(&mut f, t, &y, &mut k[0], &mut answer.evaluations, ctx)?;
        for _ in 0..options.max_steps {
            ctx.tick()?;
            h = h.abs().min(options.max_step).min((end - t).abs()) * direction;
            let next = if h.abs() == (end - t).abs() {
                end
            } else {
                t + h
            };
            if next == t || !next.is_finite() {
                return Err(Error::Input("ODE步长停滞或超出机器分辨率"));
            }
            h = next - t;
            let mut candidate = y.clone();
            for stage in 1..7 {
                for j in 0..d {
                    ctx.tick()?;
                    let sum = A[stage]
                        .iter()
                        .enumerate()
                        .map(|(i, a)| a * k[i][j])
                        .sum::<f64>();
                    candidate[j] = y[j] + h * sum;
                    if !candidate[j].is_finite() {
                        return Err(Error::NonFinite);
                    }
                }
                rhs(
                    &mut f,
                    if stage >= 5 { next } else { t + C[stage] * h },
                    &candidate,
                    &mut k[stage],
                    &mut answer.evaluations,
                    ctx,
                )?;
            }
            let mut norm: f64 = 0.;
            for j in 0..d {
                ctx.tick()?;
                let estimate = h * E.iter().enumerate().map(|(i, e)| e * k[i][j]).sum::<f64>();
                let scale = options.abs_tol + options.rel_tol * y[j].abs().max(candidate[j].abs());
                let ratio = estimate.abs() / scale;
                if !ratio.is_finite() {
                    return Err(Error::NonFinite);
                }
                norm = norm.max(ratio);
            }
            if norm <= 1. {
                if (answer.solution.times.len() + 1).saturating_mul(5 * d + 1) > 100000 {
                    return Err(Error::Input("ODE连续输出超过100000标量资源界限"));
                }
                let mut coefficients = vec![[0.; 4]; d];
                for j in 0..d {
                    for power in 0..4 {
                        ctx.tick()?;
                        coefficients[j][power] =
                            h * (0..7).map(|i| k[i][j] * P[i][power]).sum::<f64>();
                        if !coefficients[j][power].is_finite() {
                            return Err(Error::NonFinite);
                        }
                    }
                }
                let mut event_end = None;
                if let Some(e) = event.as_deref_mut() {
                    let right = event_value(e, next, &candidate, ctx)?;
                    if let Some(left) = g
                        && (right == 0. || left.is_sign_negative() != right.is_sign_negative())
                    {
                        let (mut lo, mut hi, mut low) = (0., 1., left);
                        let mut fraction = 1.;
                        if right != 0. {
                            for _ in 0..64 {
                                ctx.tick()?;
                                let mid = (lo + hi) * 0.5;
                                let time = t + h * mid;
                                let state = Interpolation::segment(&y, &coefficients, mid, ctx)?;
                                let value = event_value(e, time, &state, ctx)?;
                                fraction = mid;
                                if value == 0.
                                    || lo == mid
                                    || hi == mid
                                    || ((hi - lo) * h).abs()
                                        <= 8. * f64::EPSILON * time.abs().max(1.)
                                {
                                    break;
                                }
                                if low.is_sign_negative() == value.is_sign_negative() {
                                    lo = mid;
                                    low = value;
                                } else {
                                    hi = mid;
                                }
                            }
                        }
                        let time = t + h * fraction;
                        let state = Interpolation::segment(&y, &coefficients, fraction, ctx)?;
                        for c in &mut coefficients {
                            for (power, c) in c.iter_mut().enumerate() {
                                ctx.tick()?;
                                *c *= fraction.powi((power + 1) as i32);
                            }
                        }
                        event_end = Some((time, state));
                    }
                    g = Some(right);
                }
                if let Some((time, state)) = event_end {
                    if time == t {
                        return Err(Error::Input("ODE事件时间无法与当前节点区分"));
                    }
                    answer.solution.times.push(time);
                    answer.solution.values.push(state);
                    answer.solution.coefficients.push(coefficients);
                    answer.accepted_steps += 1;
                    answer.termination = Termination::Event;
                    return Ok(());
                }
                answer.solution.times.push(next);
                answer.solution.values.push(candidate.clone());
                answer.solution.coefficients.push(coefficients);
                answer.accepted_steps += 1;
                t = next;
                y = candidate;
                k[0] = k[6].clone();
                if t == end {
                    return Ok(());
                }
                h *= if norm == 0. {
                    5.
                } else {
                    (0.9 * norm.powf(-0.2)).clamp(0.2, 5.)
                };
            } else {
                answer.rejected_steps += 1;
                h *= (0.9 * norm.powf(-0.2)).clamp(0.1, 0.5);
            }
        }
        Err(Error::NoConvergence)
    })();
    if let Err(reason) = result {
        return Err(Failure {
            reason,
            partial: Some(Box::new(answer)),
        });
    }
    if let Err(reason) = answer.solution.validate(ctx) {
        return Err(Failure {
            reason,
            partial: Some(Box::new(answer)),
        });
    }
    Ok(answer)
}
