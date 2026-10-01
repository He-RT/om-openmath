//! Exact expression/Q-polynomial conversion with shared fractional-power generator axes.
mod arithmetic;
mod generators;
use arithmetic::{Fraction, Poly};
use om_core::{BUILTIN as B, Expr, ExprKind, add, canonicalize, func, mul, pow};
use om_num::{
    Integer, Number, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{MPoly, Monomial};
/// Uncanceled rational function over exact Q coefficients and expression generators.
#[derive(Clone, Debug, PartialEq)]
pub struct PolyView {
    /// Requested variables followed by independent atoms; equal-base roots share one axis.
    pub gens: Vec<Expr>,
    /// Sparse numerator with the same axis count as gens.
    pub num: MPoly<Rational>,
    /// Sparse nonzero denominator; no GCD cancellation is performed here.
    pub den: MPoly<Rational>,
}
/// Total convenience conversion. Unsupported/over-budget expansions retain the complete
/// expression as an opaque axis; use the checked adapter to observe a resource limit.
pub fn to_rational_function(e: &Expr, vars: &[Expr]) -> PolyView {
    if let Ok(Some(view)) = to_rational_function_with(e, vars, &Interrupt::default()) {
        return view;
    }
    let e = canonicalize(e);
    let mut gens = vec![];
    for v in vars {
        let v = canonicalize(v);
        if !gens.contains(&v) {
            gens.push(v);
        }
    }
    let i = gens.iter().position(|g| g == &e).unwrap_or_else(|| {
        gens.push(e);
        gens.len() - 1
    });
    let mut exps = vec![0; gens.len()];
    exps[i] = 1;
    let one = Monomial::new(vec![0; gens.len()]).expect("invariant: constant degree zero");
    PolyView {
        num: Poly {
            nvars: gens.len(),
            terms: vec![(
                Monomial::new(exps).expect("invariant: opaque axis degree one"),
                Rational::ONE,
            )],
            order: om_poly::MonoOrder::Lex,
        },
        den: Poly {
            nvars: gens.len(),
            terms: vec![(one, Rational::ONE)],
            order: om_poly::MonoOrder::Lex,
        },
        gens,
    }
}
/// Interruptible conversion. None denotes an unrepresentable monomial exponent/denominator
/// lcm or a failed exact rational construction; no expanded result is fabricated.
pub fn to_rational_function_with(
    e: &Expr,
    vars: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<PolyView>, Abort> {
    ctx.tick()?;
    let e = normalize(e, ctx)?;
    let mut variables = vec![];
    for v in vars {
        ctx.tick()?;
        variables.push(normalize(v, ctx)?);
    }
    let Some(groups) = generators::discover(&e, &variables, ctx)? else {
        return Ok(None);
    };
    let n = groups.len();
    let mut values: Vec<Fraction> = vec![];
    enum Visit<'a> {
        Enter(&'a Expr),
        Build(&'a Expr),
    }
    let mut stack = vec![Visit::Enter(&e)];
    while let Some(visit) = stack.pop() {
        ctx.tick()?;
        match visit {
            Visit::Enter(e) => {
                if let Some(q) = exact(e) {
                    values.push(arithmetic::constant(q, n, ctx)?);
                } else if let Some((i, exponent)) = generators::axis(e, &groups, ctx)? {
                    let Some(value) = arithmetic::axis(i, &exponent, n, ctx)? else {
                        return Ok(None);
                    };
                    values.push(value);
                } else if e.is_head(B::PLUS) || e.is_head(B::TIMES) {
                    stack.push(Visit::Build(e));
                    stack.extend(e.args().iter().rev().map(Visit::Enter));
                } else if integer_power(e).is_some() {
                    stack.push(Visit::Build(e));
                    stack.push(Visit::Enter(&e.args()[0]));
                } else {
                    return Ok(None);
                }
            }
            Visit::Build(e) => {
                if let Some(exponent) = integer_power(e) {
                    let value = values.pop().expect("invariant: visited integer-power base");
                    let Some(value) = value.power(&exponent, ctx)? else {
                        return Ok(None);
                    };
                    values.push(value);
                } else {
                    let count = e.args().len();
                    let args = values.split_off(values.len() - count);
                    let plus = e.is_head(B::PLUS);
                    let mut value = arithmetic::constant(
                        if plus { Rational::ZERO } else { Rational::ONE },
                        n,
                        ctx,
                    )?;
                    for arg in args {
                        ctx.tick()?;
                        let Some(next) = value.combine(&arg, plus, ctx)? else {
                            return Ok(None);
                        };
                        value = next;
                    }
                    values.push(value);
                }
            }
        }
    }
    let value = values
        .pop()
        .expect("invariant: expression traversal produces one fraction");
    let mut gens = vec![];
    for g in &groups {
        ctx.tick()?;
        gens.push(generators::generator(g));
    }
    Ok(Some(PolyView {
        gens,
        num: value.num,
        den: value.den,
    }))
}
/// Convert a well-formed polynomial to a canonical expression. The variable width/cache
/// must match gens; use from_mpoly_with to reject invalid input without assertion failure.
pub fn from_mpoly(p: &MPoly<Rational>, gens: &[Expr]) -> Expr {
    assert!(
        valid(p, gens, None).expect("invariant: validation without a context cannot abort"),
        "polynomial variable width/cache must match generators"
    );
    render(p, gens, None).expect("invariant: reconstruction without a context cannot abort")
}
/// Interruptible canonical reconstruction; None rejects width/cache mismatch.
pub fn from_mpoly_with(
    p: &MPoly<Rational>,
    gens: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, Abort> {
    ctx.tick()?;
    if !valid(p, gens, Some(ctx))? {
        return Ok(None);
    }
    Ok(Some(render(p, gens, Some(ctx))?))
}
fn valid(p: &Poly, gens: &[Expr], ctx: Option<&Interrupt>) -> Result<bool, Abort> {
    if p.nvars != gens.len() {
        return Ok(false);
    }
    for (m, _) in &p.terms {
        if let Some(ctx) = ctx {
            ctx.tick()?;
        }
        if m.exps.len() != gens.len() {
            return Ok(false);
        }
        let mut degree = 0_u32;
        for exponent in &m.exps {
            if let Some(ctx) = ctx {
                ctx.tick()?;
            }
            let Some(next) = degree.checked_add(*exponent) else {
                return Ok(false);
            };
            degree = next;
        }
        if degree != m.deg {
            return Ok(false);
        }
    }
    Ok(true)
}
fn render(p: &Poly, gens: &[Expr], ctx: Option<&Interrupt>) -> Result<Expr, Abort> {
    let mut terms = vec![];
    for (m, c) in &p.terms {
        if let Some(ctx) = ctx {
            ctx.tick()?;
        }
        let mut factors = vec![Expr::number(Number::Rational(c.clone()))];
        for (generator, exponent) in gens.iter().zip(&m.exps) {
            if let Some(ctx) = ctx {
                ctx.tick()?;
            }
            if *exponent != 0 {
                factors.push(pow(
                    generator.clone(),
                    Expr::integer(Integer::from(*exponent)),
                ));
            }
        }
        terms.push(mul(factors));
    }
    Ok(add(terms))
}
pub(super) fn exact(e: &Expr) -> Option<Rational> {
    match e.as_number() {
        Some(Number::Integer(n)) => Some(Rational::from(n.clone())),
        Some(Number::Rational(q)) => Some(q.clone()),
        _ => None,
    }
}
pub(super) fn integer_power(e: &Expr) -> Option<Integer> {
    if !e.is_head(B::POWER) || e.args().len() != 2 {
        return None;
    }
    let q = exact(&e.args()[1])?;
    (q.denominator() == &1_u8.into()).then(|| q.numerator().clone())
}
fn normalize(e: &Expr, ctx: &Interrupt) -> Result<Expr, Abort> {
    enum Visit<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut stack = vec![Visit::Enter(e)];
    let mut values = vec![];
    while let Some(visit) = stack.pop() {
        ctx.tick()?;
        match visit {
            Visit::Enter(e) => {
                if let ExprKind::Normal(n) = e.kind() {
                    stack.push(Visit::Build(n));
                    stack.extend(n.args.iter().rev().map(Visit::Enter));
                    stack.push(Visit::Enter(&n.head));
                } else {
                    values.push(canonicalize(e));
                }
            }
            Visit::Build(n) => {
                let args = values.split_off(values.len() - n.args.len());
                let head = values.pop().expect("invariant: visited compound head");
                values.push(if let Some(s) = head.as_symbol() {
                    func(s, args)
                } else {
                    Expr::normal(head, args)
                });
            }
        }
    }
    Ok(values.pop().expect("invariant: one canonicalized root"))
}
