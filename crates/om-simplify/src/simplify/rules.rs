//! Exact term factoring, elementary trig rules and explicitly assumed positive powers.
use crate::convert::{exact, from_mpoly_with, to_rational_function_with};
use om_core::{BUILTIN as B, Expr, Symbol, add, div, mul, pow, sub};
use om_num::{
    Integer, Number, Rational,
    ctx::{Abort, Interrupt},
    gcd,
};
use om_poly::{Algebraic, MPoly, Monomial, isolate, real_alg};

pub(super) fn factor_terms(e: &Expr, ctx: &Interrupt) -> Result<Expr, Abort> {
    if !e.is_head(B::PLUS) {
        return Ok(e.clone());
    }
    let Some(view) = to_rational_function_with(e, &[], ctx)? else {
        return Ok(e.clone());
    };
    if !view.den.is_one() || view.num.terms.is_empty() {
        return Ok(e.clone());
    }
    let mut common = view.num.terms[0].0.exps.to_vec();
    let mut numerator = Integer::ZERO;
    let mut denominator = Integer::ONE;
    for (m, c) in &view.num.terms {
        ctx.tick()?;
        for (a, b) in common.iter_mut().zip(&m.exps) {
            ctx.tick()?;
            *a = (*a).min(*b);
        }
        numerator = gcd(&numerator, c.numerator());
        let den = Integer::from(c.denominator().clone());
        denominator = (&denominator / gcd(&denominator, &den)) * den;
    }
    let content = Rational::from(numerator) / Rational::from(denominator);
    if content == Rational::ONE && common.iter().all(|p| *p == 0) {
        return Ok(e.clone());
    }
    let mut terms = vec![];
    for (m, c) in &view.num.terms {
        ctx.tick()?;
        let mut powers = vec![];
        for (a, b) in m.exps.iter().zip(&common) {
            ctx.tick()?;
            powers.push(a - b);
        }
        terms.push((
            Monomial::new(powers).expect("invariant: reduced term degree cannot grow"),
            c / &content,
        ));
    }
    let primitive = MPoly::new(view.gens.len(), terms, view.num.order, ctx)?;
    let Some(primitive) = from_mpoly_with(&primitive, &view.gens, ctx)? else {
        return Ok(e.clone());
    };
    let mut factors = vec![Expr::number(Number::Rational(content)), primitive];
    for (g, n) in view.gens.into_iter().zip(common) {
        ctx.tick()?;
        if n != 0 {
            factors.push(pow(g, Expr::integer(Integer::from(n))));
        }
    }
    Ok(mul(factors))
}
pub(super) fn root_reduce(e: &Expr, ctx: &Interrupt) -> Result<Option<Expr>, Abort> {
    if !(e.is_head(B::PLUS) || e.is_head(B::TIMES) || e.is_head(B::POWER)) {
        return Ok(None);
    }
    let Some(a) = crate::root_reduce::to_algebraic(e, ctx)? else {
        return Ok(None);
    };
    if let Algebraic::Rational(q) = a {
        return Ok(Some(Expr::number(Number::Rational(q))));
    }
    let p = a.minimal_polynomial(ctx)?;
    let index = if let Algebraic::Complex(c) = &a {
        c.index
    } else {
        let mut selected = None;
        let Some(roots) = isolate(&p, ctx)? else {
            return Ok(None);
        };
        for (index, root) in roots.into_iter().enumerate() {
            ctx.tick()?;
            let Some(b) = real_alg(&p, (root.lo, root.hi), ctx)? else {
                return Ok(None);
            };
            if a.equals(&b, ctx)? == Some(true) {
                selected = Some(index + 1);
                break;
            }
        }
        let Some(index) = selected else {
            return Ok(None);
        };
        index
    };
    let slot = Expr::call(B::SLOT, [Expr::int(1)]);
    let mut terms = vec![];
    for (i, c) in p.coeffs.into_iter().enumerate() {
        ctx.tick()?;
        if !c.is_zero() {
            terms.push(mul([
                Expr::integer(c),
                pow(slot.clone(), Expr::integer(Integer::from(i))),
            ]));
        }
    }
    Ok(Some(Expr::call(
        B::ROOT,
        [
            Expr::call(B::FUNCTION, [add(terms)]),
            Expr::integer(Integer::from(index)),
        ],
    )))
}
fn positive(e: &Expr, assumptions: &[Symbol], ctx: &Interrupt) -> Result<bool, Abort> {
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if let Some(q) = exact(e) {
            if q <= Rational::ZERO {
                return Ok(false);
            }
        } else if let Some(s) = e.as_symbol() {
            let mut found = false;
            for a in assumptions {
                ctx.tick()?;
                if *a == s {
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(false);
            }
        } else if e.is_head(B::TIMES) {
            stack.extend(e.args().iter());
        } else if e.is_head(B::POWER) && e.args().len() == 2 && exact(&e.args()[1]).is_some() {
            stack.push(&e.args()[0]);
        } else {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn power_expand(
    e: &Expr,
    assumptions: &[Symbol],
    ctx: &Interrupt,
) -> Result<Expr, Abort> {
    ctx.tick()?;
    if !e.is_head(B::POWER) || e.args().len() != 2 || exact(&e.args()[1]).is_none() {
        return Ok(e.clone());
    }
    let base = &e.args()[0];
    let exp = &e.args()[1];
    if base.is_head(B::TIMES) && positive(base, assumptions, ctx)? {
        let mut factors = vec![];
        for e in base.args() {
            ctx.tick()?;
            factors.push(pow(e.clone(), exp.clone()));
        }
        return Ok(mul(factors));
    }
    if base.is_head(B::POWER)
        && base.args().len() == 2
        && exact(&base.args()[1]).is_some()
        && positive(&base.args()[0], assumptions, ctx)?
    {
        return Ok(pow(
            base.args()[0].clone(),
            mul([base.args()[1].clone(), exp.clone()]),
        ));
    }
    Ok(e.clone())
}
fn coefficient(e: &Expr) -> (Rational, Expr) {
    if e.is_head(B::TIMES)
        && let Some(q) = exact(&e.args()[0])
    {
        return (q, mul(e.args()[1..].iter().cloned()));
    }
    (Rational::ONE, e.clone())
}
fn square(e: &Expr, head: Symbol) -> Option<&Expr> {
    if e.is_head(B::POWER)
        && e.args().len() == 2
        && exact(&e.args()[1]) == Some(Rational::from(2))
        && e.args()[0].is_head(head)
        && e.args()[0].args().len() == 1
    {
        Some(&e.args()[0].args()[0])
    } else {
        None
    }
}
fn call(head: Symbol, e: Expr) -> Expr {
    Expr::call(head, [e])
}
fn times(q: Rational, e: Expr) -> Expr {
    mul([Expr::number(Number::Rational(q)), e])
}
fn squared(head: Symbol, e: Expr) -> Expr {
    pow(call(head, e), Expr::int(2))
}
fn half_angle(e: &Expr) -> Option<Expr> {
    if e.is_head(B::TIMES) && e.args().len() >= 2 && exact(&e.args()[0]) == Some(Rational::from(2))
    {
        Some(mul(e.args()[1..].iter().cloned()))
    } else {
        None
    }
}
pub(super) fn trig(e: &Expr, mode: usize, ctx: &Interrupt) -> Result<Expr, Abort> {
    ctx.tick()?;
    if mode <= 1 && e.is_head(B::PLUS) {
        let terms = e.args().iter().map(coefficient).collect::<Vec<_>>();
        for (i, (c, term)) in terms.iter().enumerate() {
            ctx.tick()?;
            let Some(x) = square(term, B::SIN) else {
                continue;
            };
            for (j, (d, other)) in terms.iter().enumerate() {
                ctx.tick()?;
                if square(other, B::COS) != Some(x) {
                    continue;
                }
                let replacement = if mode == 0 && c == d {
                    Expr::number(Number::Rational(c.clone()))
                } else if mode == 1 && c == &(-d) {
                    times(d.clone(), call(B::COS, mul([Expr::int(2), x.clone()])))
                } else {
                    continue;
                };
                let mut out = e
                    .args()
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| *k != i && *k != j)
                    .map(|(_, e)| e.clone())
                    .collect::<Vec<_>>();
                out.push(replacement);
                return Ok(add(out));
            }
        }
    }
    if mode == 2 && e.is_head(B::TIMES) {
        for (i, a) in e.args().iter().enumerate() {
            ctx.tick()?;
            if !a.is_head(B::SIN) || a.args().len() != 1 {
                continue;
            }
            for (j, b) in e.args().iter().enumerate() {
                ctx.tick()?;
                if !b.is_head(B::COS) || b.args() != a.args() {
                    continue;
                }
                let rest = e
                    .args()
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| *k != i && *k != j)
                    .map(|(_, e)| e.clone());
                return Ok(mul(rest.chain([div(
                    call(B::SIN, mul([Expr::int(2), a.args()[0].clone()])),
                    Expr::int(2),
                )])));
            }
        }
    }
    if mode >= 3
        && e.args().len() == 1
        && let Some(x) = half_angle(&e.args()[0])
    {
        if e.is_head(B::SIN) && mode == 3 {
            return Ok(mul([
                Expr::int(2),
                call(B::SIN, x.clone()),
                call(B::COS, x),
            ]));
        }
        if e.is_head(B::COS) {
            return Ok(if mode == 3 {
                sub(squared(B::COS, x.clone()), squared(B::SIN, x))
            } else {
                sub(Expr::int(1), mul([Expr::int(2), squared(B::SIN, x)]))
            });
        }
    }
    Ok(e.clone())
}
