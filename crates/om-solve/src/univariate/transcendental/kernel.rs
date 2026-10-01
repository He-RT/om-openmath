//! Kernels are unified only when every dependent generator shares the same axis.
use super::super::{extract, reductions};
use crate::SolveError;
use om_core::{BUILTIN as B, Expr, add, div, mul, neg, pow, sub};
use om_num::{Integer, Number, Rational, ctx::Interrupt, gcd, perfect_power};
use om_simplify::{algebra::cancel_with, convert::to_rational_function_with};

pub(super) struct Kernel {
    pub definition: Expr,
    pub equation: Expr,
    pub axis: Expr,
}
pub(super) fn real_argument(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<bool, SolveError> {
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if e == x || matches!(e.as_symbol(), Some(B::PI | B::E)) {
            continue;
        }
        if let Some(n) = e.as_number() {
            if matches!(n,Number::Complex(c)if !c.im.is_zero()) {
                return Ok(false);
            }
            continue;
        }
        match e.head_symbol() {
            Some(B::PLUS | B::TIMES) => stack.extend(e.args()),
            Some(B::POWER)
                if e.args().len() == 2
                    && extract::exact(&e.args()[1])
                        .is_some_and(|q| q.denominator() == &1_u8.into()) =>
            {
                stack.push(&e.args()[0])
            }
            Some(
                B::SIN
                | B::COS
                | B::TAN
                | B::COT
                | B::SEC
                | B::CSC
                | B::SINH
                | B::COSH
                | B::TANH
                | B::ARCSINH
                | B::ARCTAN,
            ) if e.args().len() == 1 => stack.push(&e.args()[0]),
            Some(B::ABS) if e.args().len() == 1 => {}
            _ => return Ok(false),
        }
    }
    Ok(true)
}
pub(super) fn log(v: Expr) -> Expr {
    let i = om_core::canonicalize(&Expr::sym(B::I));
    if v == i {
        return mul([Expr::rational(1, 2), Expr::sym(B::PI), i]);
    }
    if v == neg(i.clone()) {
        return mul([Expr::rational(-1, 2), Expr::sym(B::PI), i]);
    }
    if let Some(Number::Integer(n)) = v.as_number()
        && n > &Integer::ONE
        && let Some((b, k)) = perfect_power(n)
    {
        return mul([
            Expr::int(i64::from(k)),
            Expr::call(B::LOG, [Expr::number(Number::Integer(b))]),
        ]);
    }
    let call = Expr::call(B::LOG, [v]);
    om_simplify::special::eval(&call).unwrap_or(call)
}
fn quotient(a: Expr, b: Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    cancel_with(&div(a, b), &[], ctx)?
        .ok_or_else(|| SolveError::Unsupported("kernel coefficient quotient".into()))
}
fn affine(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<Option<(Expr, Expr)>, SolveError> {
    let Some(mut c) = extract::coefficients(e, x, ctx)? else {
        return Ok(None);
    };
    if c.values.len() != 2 || c.values[1].is_zero() {
        return Ok(None);
    }
    let a = c.values.pop().expect("invariant: affine slope");
    Ok(Some((
        a,
        c.values.pop().expect("invariant: affine intercept"),
    )))
}
fn exponential(
    k: &Expr,
    x: &Expr,
    ctx: &Interrupt,
) -> Result<Option<(Expr, Expr, Expr)>, SolveError> {
    if !k.is_head(B::POWER) || k.args().len() != 2 || extract::depends(&k.args()[0], x, ctx)? {
        return Ok(None);
    }
    let Some((a, b)) = affine(&k.args()[1], x, ctx)? else {
        return Ok(None);
    };
    let mut base = k.args()[0].clone();
    let mut a = a;
    let mut b = b;
    if let Some(Number::Integer(n)) = base.as_number()
        && n > &Integer::ONE
        && let Some((g, m)) = perfect_power(n)
    {
        base = Expr::number(Number::Integer(g));
        a = mul([Expr::int(i64::from(m)), a]);
        b = mul([Expr::int(i64::from(m)), b]);
    }
    Ok(Some((base, a, b)))
}
pub(super) fn unify(e: &Expr, x: &Expr, ctx: &Interrupt) -> Result<Option<Kernel>, SolveError> {
    let axis = reductions::fresh(e, x, ctx)?;
    // The defining Lambert relation is a composite kernel rather than an opaque call.
    let terms = if e.is_head(B::PLUS) {
        e.args().to_vec()
    } else {
        vec![e.clone()]
    };
    for term in terms {
        let factors = if term.is_head(B::TIMES) {
            term.args().to_vec()
        } else {
            vec![term.clone()]
        };
        for f in factors {
            ctx.tick()?;
            if f.is_head(B::POWER) && f.args().len() == 2 && f.args()[0].as_symbol() == Some(B::E) {
                let mut arguments = vec![f.args()[1].clone()];
                if let Some((a, _)) = affine(&f.args()[1], x, ctx)? {
                    arguments.push(mul([a, x.clone()]));
                }
                for argument in arguments {
                    let k = mul([argument.clone(), pow(Expr::sym(B::E), argument)]);
                    if extract::depends(&k, x, ctx)? {
                        let coefficient = quotient(term.clone(), k.clone(), ctx)?;
                        if extract::depends(&coefficient, x, ctx)? {
                            continue;
                        }
                        let equation =
                            e.replace_all(&[(term.clone(), mul([coefficient, axis.clone()]))]);
                        if !extract::depends(&equation, x, ctx)? {
                            return Ok(Some(Kernel {
                                definition: k,
                                equation,
                                axis,
                            }));
                        }
                    }
                }
            }
        }
    }
    let Some(view) = to_rational_function_with(e, &[], ctx)? else {
        return Ok(None);
    };
    let mut kernels = vec![];
    for k in &view.gens {
        if extract::depends(k, x, ctx)? {
            kernels.push(k.clone());
        }
    }
    if kernels.is_empty() {
        return Ok(None);
    }
    let mut exponentials = vec![];
    for k in &kernels {
        let Some(data) = exponential(k, x, ctx)? else {
            exponentials.clear();
            break;
        };
        exponentials.push(data);
    }
    if !exponentials.is_empty() {
        let common = exponentials.iter().all(|(b, _, _)| b == &exponentials[0].0);
        let base = if common {
            exponentials[0].0.clone()
        } else {
            Expr::sym(B::E)
        };
        let slopes = exponentials
            .iter()
            .map(|(b, a, _)| {
                if common {
                    a.clone()
                } else {
                    mul([a.clone(), log(b.clone())])
                }
            })
            .collect::<Vec<_>>();
        let mut ratios = vec![];
        let mut d = Integer::ONE;
        for a in &slopes {
            let Some(r) = extract::exact(&quotient(a.clone(), slopes[0].clone(), ctx)?) else {
                ratios.clear();
                break;
            };
            let q = Integer::from(r.denominator().clone());
            d = (&d / gcd(&d, &q)) * q;
            ratios.push(r);
        }
        if !ratios.is_empty() {
            let def = pow(
                base.clone(),
                div(
                    mul([slopes[0].clone(), x.clone()]),
                    Expr::number(Number::Integer(d.clone())),
                ),
            );
            let mut rules = vec![];
            for ((k, (b, _, offset)), r) in kernels.iter().zip(&exponentials).zip(ratios) {
                let exponent = Expr::number(Number::Rational(r * Rational::from(d.clone())));
                rules.push((
                    k.clone(),
                    mul([pow(b.clone(), offset.clone()), pow(axis.clone(), exponent)]),
                ));
            }
            let equation = e.replace_all(&rules);
            if !extract::depends(&equation, x, ctx)? {
                return Ok(Some(Kernel {
                    definition: def,
                    equation,
                    axis,
                }));
            }
        }
    }
    if kernels
        .iter()
        .all(|k| matches!(k.head_symbol(), Some(B::SIN | B::COS)) && k.args().len() == 1)
        && kernels.iter().all(|k| k.args()[0] == kernels[0].args()[0])
        && kernels.iter().any(|k| k.is_head(B::SIN))
        && kernels.iter().any(|k| k.is_head(B::COS))
    {
        let i = om_core::canonicalize(&Expr::sym(B::I));
        let definition = pow(
            Expr::sym(B::E),
            mul([i.clone(), kernels[0].args()[0].clone()]),
        );
        let mut rules = vec![];
        for k in &kernels {
            let value = if k.is_head(B::SIN) {
                div(
                    sub(axis.clone(), pow(axis.clone(), Expr::int(-1))),
                    mul([Expr::int(2), i.clone()]),
                )
            } else {
                div(
                    add([axis.clone(), pow(axis.clone(), Expr::int(-1))]),
                    Expr::int(2),
                )
            };
            rules.push((k.clone(), value));
        }
        let equation = e.replace_all(&rules);
        if !extract::depends(&equation, x, ctx)? {
            return Ok(Some(Kernel {
                definition,
                equation,
                axis,
            }));
        }
    }
    if kernels.len() == 1 && kernels[0] != *x {
        let definition = kernels.remove(0);
        let mut gens = view.gens;
        for g in &mut gens {
            if *g == definition {
                *g = axis.clone();
            }
        }
        let Some(num) = om_simplify::convert::from_mpoly_with(&view.num, &gens, ctx)? else {
            return Ok(None);
        };
        let Some(den) = om_simplify::convert::from_mpoly_with(&view.den, &gens, ctx)? else {
            return Ok(None);
        };
        let equation = div(num, den);
        if !extract::depends(&equation, x, ctx)? {
            return Ok(Some(Kernel {
                definition,
                equation,
                axis,
            }));
        }
    }
    Ok(None)
}
