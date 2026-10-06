//! Normalized coordinates avoid unstable midpoint arithmetic; no multimodal/global guarantee.
use super::*;
/// Search a finite bounded interval for a local candidate. Endpoints are genuinely evaluated.
pub fn bounded<F>(
    mut f: F,
    left: f64,
    right: f64,
    options: &Options,
    ctx: &Interrupt,
) -> Result<Answer, Failure>
where
    F: FnMut(f64, &Interrupt) -> Result<f64, Error>,
{
    let mut saved: Option<Answer> = None;
    let mut calls = 0;
    let mut evaluate = |u: f64| -> Result<f64, Error> {
        ctx.tick()?;
        calls += 1;
        let x = if u == 0. {
            left
        } else if u == 1. {
            right
        } else {
            left + (right - left) * u
        };
        let value = f(x, ctx)?;
        ctx.tick()?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(Error::NonFinite)
        }
    };
    let result = (|| -> Result<Answer, Error> {
        ctx.tick()?;
        options.validate()?;
        let span = right - left;
        if !left.is_finite() || !right.is_finite() || !span.is_finite() || span <= 0. {
            return Err(Error::Input("Brent需要有限严格递增边界"));
        }
        let golden = (3. - 5f64.sqrt()) / 2.;
        let (mut a, mut b) = (0., 1.);
        let (mut x, mut w, mut v) = (golden, golden, golden);
        let mut fx = evaluate(x)?;
        let mut fw = fx;
        let mut fv = fx;
        saved = Some(Answer {
            point: vec![left + span * x],
            value: fx,
            iterations: 0,
            evaluations: 0,
            gradient_norm: None,
            bracket_width: Some(span),
            method: Method::Brent,
        });
        let mut endpoint = (left, evaluate(0.)?);
        let right_value = evaluate(1.)?;
        if right_value < endpoint.1 {
            endpoint = (right, right_value);
        }
        let (mut d, mut e): (f64, f64) = (0., 0.);
        for iteration in 0..options.max_iterations {
            ctx.tick()?;
            let coordinate = left + span * x;
            let tol = ((options.abs_tol + options.rel_tol * coordinate.abs()) / span)
                .max(8. * f64::EPSILON);
            if !tol.is_finite() {
                return Err(Error::Input("Brent停止容差在当前坐标尺度下无法表示"));
            }
            let midpoint = (a + b) * 0.5;
            let best = if endpoint.1 < fx {
                endpoint
            } else {
                (coordinate, fx)
            };
            let answer = Answer {
                point: vec![best.0],
                value: best.1,
                iterations: iteration,
                evaluations: 0,
                gradient_norm: None,
                bracket_width: Some((b - a) * span),
                method: Method::Brent,
            };
            saved = Some(answer.clone());
            if (x - midpoint).abs() <= 2. * tol - (b - a) * 0.5 {
                return Ok(answer);
            }
            let mut parabolic = false;
            if e.abs() > tol {
                let r = (x - w) * (fx - fv);
                let q = (x - v) * (fx - fw);
                let mut p = (x - v) * q - (x - w) * r;
                let mut q = 2. * (q - r);
                if q > 0. {
                    p = -p;
                }
                q = q.abs();
                let previous = e;
                e = d;
                if p.is_finite()
                    && q.is_finite()
                    && q > 0.
                    && p.abs() < (0.5 * q * previous).abs()
                    && p > q * (a - x)
                    && p < q * (b - x)
                {
                    d = p / q;
                    parabolic = true;
                    let u = x + d;
                    if u - a < 2. * tol || b - u < 2. * tol {
                        d = tol * if midpoint >= x { 1. } else { -1. };
                    }
                }
            }
            if !parabolic {
                e = if x < midpoint { b - x } else { a - x };
                d = golden * e;
            }
            let u = x + if d.abs() >= tol {
                d
            } else {
                tol * if d >= 0. { 1. } else { -1. }
            };
            if u <= a || u >= b || u == x || left + span * u == coordinate {
                return Err(Error::Input("Brent达到机器坐标分辨率，不能满足停止条件"));
            }
            let fu = evaluate(u)?;
            if fu <= fx {
                if u < x {
                    b = x;
                } else {
                    a = x;
                }
                v = w;
                fv = fw;
                w = x;
                fw = fx;
                x = u;
                fx = fu;
            } else {
                if u < x {
                    a = u;
                } else {
                    b = u;
                }
                if fu <= fw || w == x {
                    v = w;
                    fv = fw;
                    w = u;
                    fw = fu;
                } else if fu <= fv || v == x || v == w {
                    v = u;
                    fv = fu;
                }
            }
            let coordinate = left + span * x;
            let best = if endpoint.1 < fx {
                endpoint
            } else {
                (coordinate, fx)
            };
            saved = Some(Answer {
                point: vec![best.0],
                value: best.1,
                iterations: iteration + 1,
                evaluations: 0,
                gradient_norm: None,
                bracket_width: Some((b - a) * span),
                method: Method::Brent,
            });
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
