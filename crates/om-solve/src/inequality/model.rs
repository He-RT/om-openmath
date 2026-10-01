//! The chart uses exact rational coefficients and certified real algebraic endpoints.
use crate::{Domain, NoSteps, SolutionSet, SolveError, SolveOptions};
use om_core::{BUILTIN as B, Expr, Symbol};
use om_num::{Number, Rational, ctx::Interrupt};
use om_poly::{Algebraic, MPoly, RootInterval, UPoly};
use std::cmp::Ordering;
pub(super) struct Constraint {
    pub n: UPoly<Rational>,
    pub d: UPoly<Rational>,
    pub relation: Symbol,
    pub original: Expr,
}
pub(super) struct Point {
    pub expr: Expr,
    pub value: Algebraic,
}
fn unsupported(reason: &str) -> SolveError {
    SolveError::Unsupported(reason.into())
}
fn dense(p: &MPoly<Rational>, ctx: &Interrupt) -> Result<UPoly<Rational>, SolveError> {
    let degree = p
        .terms
        .iter()
        .map(|(m, _)| m.deg as usize)
        .max()
        .unwrap_or(0);
    if degree > 4096 {
        return Err(unsupported("dense rational chart degree exceeds 4096"));
    }
    let mut values = vec![Rational::ZERO; degree + 1];
    for (m, c) in &p.terms {
        ctx.tick()?;
        values[m.deg as usize] += c;
    }
    Ok(UPoly::new(values))
}
pub(super) fn constraint(
    e: &Expr,
    relation: Symbol,
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Constraint, SolveError> {
    let Some(view) =
        om_simplify::convert::to_rational_function_with(e, std::slice::from_ref(x), ctx)?
    else {
        return Err(unsupported("rational chart conversion unavailable"));
    };
    if view.gens != [x.clone()] {
        return Err(unsupported(
            "inequality coefficients must be exact rationals",
        ));
    }
    Ok(Constraint {
        n: dense(&view.num, ctx)?,
        d: dense(&view.den, ctx)?,
        relation,
        original: e.clone(),
    })
}
fn expression(p: &UPoly<Rational>, x: &Expr) -> Expr {
    om_core::add(
        p.coeffs
            .iter()
            .enumerate()
            .filter(|(_, c)| **c != Rational::ZERO)
            .map(|(i, c)| {
                om_core::mul([
                    Expr::number(Number::Rational(c.clone())),
                    om_core::pow(x.clone(), Expr::int(i as i64)),
                ])
            }),
    )
}
pub(super) fn critical(
    branches: &[Vec<Constraint>],
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Vec<Point>, SolveError> {
    let opts = SolveOptions {
        domain: Domain::Reals,
        record_steps: false,
        ..SolveOptions::default()
    };
    let mut seen = vec![];
    let mut points: Vec<Point> = vec![];
    for p in branches.iter().flatten().flat_map(|c| [&c.n, &c.d]) {
        ctx.tick()?;
        if p.degree().is_none_or(|d| d == 0) || seen.contains(p) {
            continue;
        }
        seen.push(p.clone());
        let result = crate::univariate::poly_uni(&expression(p, x), x, &opts, ctx, &mut NoSteps)?;
        let SolutionSet::Finite(roots) = result.set else {
            return Err(unsupported("incomplete rational chart roots"));
        };
        for root in roots {
            let expr = root.rules[0].1.clone();
            let value = om_simplify::root_reduce::to_algebraic(&expr, ctx)?
                .ok_or_else(|| unsupported("critical point lacks an exact certificate"))?;
            let mut position = points.len();
            let mut duplicate = false;
            for (i, old) in points.iter().enumerate() {
                let order = value
                    .cmp_real(&old.value, ctx)?
                    .ok_or_else(|| unsupported("critical point order unavailable"))?;
                match order {
                    Ordering::Equal => {
                        duplicate = true;
                        break;
                    }
                    Ordering::Less => {
                        position = i;
                        break;
                    }
                    _ => {}
                }
            }
            if !duplicate {
                points.insert(position, Point { expr, value });
            }
        }
    }
    Ok(points)
}
fn bounds(p: &Point, bits: u32, ctx: &Interrupt) -> Result<(Rational, Rational), SolveError> {
    match &p.value {
        Algebraic::Rational(q) => Ok((q.clone(), q.clone())),
        Algebraic::Real(a) => {
            let interval = RootInterval {
                lo: a.iv.0.clone(),
                hi: a.iv.1.clone(),
                exact: false,
            };
            let iv = om_poly::refine(&a.minpoly, &interval, bits, ctx)?
                .ok_or_else(|| unsupported("critical interval refinement unavailable"))?;
            Ok((iv.lo, iv.hi))
        }
        _ => Err(unsupported("nonreal critical point")),
    }
}
pub(super) fn samples(points: &[Point], ctx: &Interrupt) -> Result<Vec<Rational>, SolveError> {
    if points.is_empty() {
        return Ok(vec![Rational::ZERO]);
    }
    let mut bits = 32;
    loop {
        let bounds = points
            .iter()
            .map(|p| bounds(p, bits, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        if bounds.windows(2).all(|w| w[0].1 < w[1].0) {
            let mut samples = vec![&bounds[0].0 - Rational::ONE];
            samples.extend(
                bounds
                    .windows(2)
                    .map(|w| (&w[0].1 + &w[1].0) / Rational::from(2)),
            );
            samples.push(&bounds[bounds.len() - 1].1 + Rational::ONE);
            return Ok(samples);
        }
        ctx.tick()?;
        if bits == 16_352 {
            return Err(unsupported("rational gap separation unavailable"));
        }
        bits = (bits * 2).min(16_352);
    }
}
pub(super) fn sign(p: &UPoly<Rational>, q: &Rational, ctx: &Interrupt) -> Result<i8, SolveError> {
    let mut value = Rational::ZERO;
    for c in p.coeffs.iter().rev() {
        ctx.tick()?;
        value = value * q + c;
    }
    Ok(if value > Rational::ZERO {
        1
    } else if value < Rational::ZERO {
        -1
    } else {
        0
    })
}
pub(super) fn zero_at(p: &UPoly<Rational>, a: &Point, ctx: &Interrupt) -> Result<bool, SolveError> {
    let min = a.value.minimal_polynomial(ctx)?;
    let min = UPoly::new(min.coeffs.into_iter().map(Rational::from).collect());
    let (_, rem) = p
        .divrem(&min, ctx)?
        .ok_or_else(|| unsupported("critical remainder unavailable"))?;
    Ok(rem.is_zero())
}
pub(super) fn holds(rel: Symbol, sign: i8) -> bool {
    match rel {
        B::LESS => sign < 0,
        B::LESS_EQUAL => sign <= 0,
        B::GREATER => sign > 0,
        B::GREATER_EQUAL => sign >= 0,
        B::EQUAL => sign == 0,
        B::UNEQUAL => sign != 0,
        _ => false,
    }
}
