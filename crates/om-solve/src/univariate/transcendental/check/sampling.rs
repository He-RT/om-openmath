//! Reproducible admissible parameter and independent integer-period probes.
use super::super::super::radical::verify;
use super::condition::{allows, nonzero, residual};
use crate::{SolveError, SolveOptions, Verification};
use om_core::Expr;
use om_num::{Number, Rational, ctx::Interrupt, rng::SplitMix64};
use om_simplify::zero::Tri;
pub(super) fn period_vectors(n: usize, ctx: &Interrupt) -> Result<Vec<Vec<i64>>, SolveError> {
    if n > 64 {
        return Err(SolveError::Unsupported(
            "verification supports at most 64 independent periods".into(),
        ));
    }
    if n == 0 {
        return Ok(vec![vec![]]);
    }
    let mut points = vec![];
    for k in [0, 1, -1, 2, -2] {
        points.push(vec![k; n]);
    }
    for i in 0..n {
        for k in [1, -1, 2, -2] {
            ctx.tick()?;
            let mut v = vec![0; n];
            v[i] = k;
            if !points.contains(&v) {
                points.push(v)
            }
        }
        for j in i + 1..n {
            for a in [1, -1] {
                for b in [1, -1] {
                    ctx.tick()?;
                    let mut v = vec![0; n];
                    v[i] = a;
                    v[j] = b;
                    if !points.contains(&v) {
                        points.push(v)
                    }
                }
            }
        }
    }
    Ok(points)
}
pub(super) fn sample(
    e: &Expr,
    exclusions: &[Expr],
    condition: Option<&Expr>,
    opts: &SolveOptions,
    ctx: &Interrupt,
) -> Result<Option<Verification>, SolveError> {
    let mut vars = e.free_symbols().into_iter().collect::<Vec<_>>();
    for value in exclusions.iter().chain(condition) {
        for s in value.free_symbols() {
            if !vars.contains(&s) {
                vars.push(s)
            }
        }
    }
    vars.sort_by_key(|s| s.name().to_string());
    if vars.is_empty() {
        if let Some(c) = condition
            && allows(c, ctx)? == Some(false)
        {
            return Ok(None);
        }
        for value in exclusions {
            if !nonzero(value, ctx)? {
                return Ok(None);
            }
        }
        let residual = residual(e, ctx)?;
        return Ok(match verify::zero(&residual, ctx)? {
            Tri::Zero => Some(Verification::Exact),
            Tri::NonZero => None,
            Tri::Unknown(_) => {
                verify::numeric(&residual, ctx)?.then_some(Verification::Numeric { digits: 33 })
            }
        });
    }
    let mut rng = SplitMix64::new(opts.seed);
    let mut passed = 0;
    for _ in 0..24 {
        ctx.tick()?;
        let rules = vars
            .iter()
            .map(|s| {
                (
                    Expr::sym(*s),
                    Expr::number(Number::Rational(
                        Rational::from(rng.next_range(0, 7) as i64 - 3)
                            / Rational::from(rng.next_range(1, 5)),
                    )),
                )
            })
            .collect::<Vec<_>>();
        if let Some(c) = condition
            && allows(&c.replace_all(&rules), ctx)? == Some(false)
        {
            continue;
        }
        let mut allowed = true;
        for v in exclusions {
            if !nonzero(&v.replace_all(&rules), ctx)? {
                allowed = false;
                break;
            }
        }
        if !allowed {
            continue;
        }
        let residual = residual(&e.replace_all(&rules), ctx)?;
        match verify::zero(&residual, ctx)? {
            Tri::Zero => {}
            Tri::NonZero => return Ok(None),
            Tri::Unknown(_) => {
                if !verify::numeric(&residual, ctx)? {
                    return Ok(None);
                }
            }
        }
        passed += 1;
        if passed == 3 {
            return Ok(Some(Verification::Numeric { digits: 33 }));
        }
    }
    Ok(None)
}
