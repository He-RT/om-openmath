//! Degree-decreasing radical reductions, preserving multiplicities of inverse maps.
use super::{Candidate, extract, formula, numeric, symbolic};
use crate::{Formula, Level, SolveError, SolveOptions, Step, StepKind, StepSink};
use om_core::{Expr, Symbol, add, div, mul, neg, pow};
use om_num::{Integer, Rational, ctx::Interrupt};
use om_poly::UPoly;
use om_simplify::{
    algebra::cancel_with,
    zero::{Tri, is_zero_with},
};

pub(super) fn roots(
    c: &[Expr],
    p: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    ctx.tick()?;
    if c.len() <= 3 {
        return formula(c, p, ctx, sink);
    }
    let degree = c.len() - 1;
    let mut common = 0;
    let mut binomial = true;
    for (i, a) in c.iter().enumerate().skip(1) {
        ctx.tick()?;
        if is_zero_with(a, ctx)? != Tri::Zero {
            common = gcd(common, i);
            if i < degree {
                binomial = false;
            }
        }
    }
    if binomial {
        return Ok(Some(binomial_roots(
            div(neg(c[0].clone()), c[degree].clone()),
            degree,
            p,
            ctx,
            sink,
        )?));
    }
    if common > 1 {
        return power(c, p, x, common, opts, ctx, sink);
    }
    if degree.is_multiple_of(2) {
        let mut reciprocal = true;
        for i in 0..degree / 2 {
            ctx.tick()?;
            if is_zero_with(&om_core::sub(c[i].clone(), c[degree - i].clone()), ctx)? != Tri::Zero {
                reciprocal = false;
                break;
            }
        }
        if reciprocal {
            return palindromic(c, p, x, opts, ctx, sink);
        }
    }
    Ok(None)
}
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b > 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
fn binomial_roots(
    value: Expr,
    n: usize,
    p: &Expr,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Vec<Candidate>, SolveError> {
    ctx.tick()?;
    let value = cancel_with(&value, &[], ctx)?
        .ok_or_else(|| SolveError::Unsupported("binomial quotient failed".into()))?;
    let roots = if is_zero_with(&value, ctx)? == Tri::Zero {
        vec![Candidate {
            value: Expr::int(0),
            multiplicity: u32::try_from(n).expect("invariant: dense degree <=4096"),
            key: None,
        }]
    } else {
        let principal = pow(
            value.clone(),
            Expr::number(om_num::Number::Rational(
                Rational::ONE / Rational::from(Integer::from(n)),
            )),
        );
        let mut roots = vec![];
        for k in 0..n {
            ctx.tick()?;
            let phase = pow(
                Expr::int(-1),
                Expr::number(om_num::Number::Rational(
                    Rational::from(Integer::from(2 * k)) / Rational::from(Integer::from(n)),
                )),
            );
            roots.push(Candidate {
                value: mul([principal.clone(), phase]),
                multiplicity: 1,
                key: None,
            });
        }
        roots
    };
    sink.record(|| {
        Step::new(
            StepKind::ApplyFormula {
                formula: Formula::Binomial,
                bindings: vec![
                    ("n".into(), Expr::integer(Integer::from(n))),
                    ("c".into(), value),
                ],
                results: roots.iter().map(|r| r.value.clone()).collect(),
            },
            vec![p.clone()],
            roots.iter().map(|r| r.value.clone()).collect(),
            Level::Major,
        )
    });
    Ok(roots)
}
pub(super) fn fresh(p: &Expr, x: &Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    for i in 1u64.. {
        ctx.tick()?;
        let y = Expr::sym(Symbol::intern(&format!("OmSolve${i}")));
        if y != *x && !extract::depends(p, &y, ctx)? {
            return Ok(y);
        }
    }
    Err(SolveError::Unsupported(
        "auxiliary variable namespace exhausted".into(),
    ))
}
pub(super) fn polynomial(c: &[Expr], x: &Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    let mut terms = vec![];
    for (i, a) in c.iter().enumerate() {
        ctx.tick()?;
        terms.push(mul([
            a.clone(),
            pow(x.clone(), Expr::integer(Integer::from(i))),
        ]));
    }
    Ok(add(terms))
}
fn reduced(
    c: &[Expr],
    p: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    if let Some(q) = c.iter().map(extract::exact).collect::<Option<Vec<_>>>() {
        numeric(&UPoly::new(q), p, x, opts, ctx, sink)
    } else {
        symbolic::roots(c, p, x, opts, ctx, sink)
    }
}
fn compose(mut root: Candidate, m: u32) -> Result<Candidate, SolveError> {
    root.multiplicity = root
        .multiplicity
        .checked_mul(m)
        .ok_or_else(|| SolveError::Unsupported("inverse-map multiplicity overflow".into()))?;
    root.key = None;
    Ok(root)
}
fn power(
    c: &[Expr],
    p: &Expr,
    x: &Expr,
    n: usize,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    let y = fresh(p, x, ctx)?;
    let coefficients = c.iter().step_by(n).cloned().collect::<Vec<_>>();
    let q = polynomial(&coefficients, &y, ctx)?;
    sink.record(|| {
        Step::new(
            StepKind::Substitute {
                new_var: y.clone(),
                def: pow(x.clone(), Expr::integer(Integer::from(n))),
            },
            vec![p.clone()],
            vec![q.clone()],
            Level::Major,
        )
    });
    let Some(ys) = reduced(&coefficients, &q, &y, opts, ctx, sink)? else {
        return Ok(None);
    };
    if ys.iter().any(|r| contains_root(&r.value)) {
        return Ok(None);
    }
    let mut all = vec![];
    for root in ys {
        ctx.tick()?;
        let inverse = om_core::sub(
            pow(x.clone(), Expr::integer(Integer::from(n))),
            root.value.clone(),
        );
        for r in binomial_roots(root.value, n, &inverse, ctx, sink)? {
            let r = compose(r, root.multiplicity)?;
            sink.record(|| {
                Step::new(
                    StepKind::BackSubstitute {
                        var: x.clone(),
                        value: r.value.clone(),
                    },
                    vec![inverse.clone()],
                    vec![r.value.clone()],
                    Level::Minor,
                )
            });
            all.push(r);
        }
    }
    Ok(Some(all))
}
fn palindromic(
    c: &[Expr],
    p: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Candidate>>, SolveError> {
    let m = (c.len() - 1) / 2;
    let y = fresh(p, x, ctx)?;
    // S_j(z)=x^j+x^-j, S_0=2, S_1=z, S_j=z*S_(j-1)-S_(j-2).
    let mut previous = vec![Integer::from(2)];
    let mut current = vec![Integer::ZERO, Integer::ONE];
    let mut coefficients = vec![Expr::int(0); m + 1];
    coefficients[0] = c[m].clone();
    for j in 1..=m {
        ctx.tick()?;
        for (i, a) in current.iter().enumerate() {
            ctx.tick()?;
            coefficients[i] = add([
                coefficients[i].clone(),
                mul([c[m + j].clone(), Expr::integer(a.clone())]),
            ]);
        }
        if j < m {
            let mut next = vec![Integer::ZERO; current.len() + 1];
            for (i, a) in current.iter().enumerate() {
                ctx.tick()?;
                next[i + 1] = a.clone();
            }
            for (i, a) in previous.iter().enumerate() {
                ctx.tick()?;
                next[i] -= a;
            }
            previous = current;
            current = next;
        }
    }
    let q = polynomial(&coefficients, &y, ctx)?;
    sink.record(|| {
        Step::new(
            StepKind::Substitute {
                new_var: y.clone(),
                def: add([x.clone(), pow(x.clone(), Expr::int(-1))]),
            },
            vec![p.clone()],
            vec![q.clone()],
            Level::Major,
        )
    });
    let Some(ys) = reduced(&coefficients, &q, &y, opts, ctx, sink)? else {
        return Ok(None);
    };
    if ys.iter().any(|r| contains_root(&r.value)) {
        return Ok(None);
    }
    let mut all = vec![];
    for root in ys {
        ctx.tick()?;
        let inverse = [Expr::int(1), neg(root.value), Expr::int(1)];
        let inverse_p = polynomial(&inverse, x, ctx)?;
        let Some(xs) = formula(&inverse, &inverse_p, ctx, sink)? else {
            return Ok(None);
        };
        for r in xs {
            let r = compose(r, root.multiplicity)?;
            sink.record(|| {
                Step::new(
                    StepKind::BackSubstitute {
                        var: x.clone(),
                        value: r.value.clone(),
                    },
                    vec![inverse_p.clone()],
                    vec![r.value.clone()],
                    Level::Minor,
                )
            });
            all.push(r);
        }
    }
    sink.record(|| {
        Step::new(
            StepKind::ApplyFormula {
                formula: Formula::Palindromic,
                bindings: vec![("z".into(), y)],
                results: all.iter().map(|r| r.value.clone()).collect(),
            },
            vec![p.clone()],
            all.iter().map(|r| r.value.clone()).collect(),
            Level::Major,
        )
    });
    Ok(Some(all))
}

fn contains_root(e: &Expr) -> bool {
    e.is_head(om_core::BUILTIN::ROOT) || e.args().iter().any(contains_root)
}
