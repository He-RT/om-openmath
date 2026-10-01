//! Factor components use one algebraic root and exact shape-coordinate reductions.
use super::{
    convert::{self, Poly},
    multiplicity,
};
use crate::{
    Domain, Level, Solution, SolutionSet, SolveError, SolveOptions, Step, StepKind, StepSink,
    univariate::poly_uni,
};
use om_core::{BUILTIN as B, Expr, Symbol, add, mul, pow};
use om_num::{Number, Rational, ctx::Interrupt};
use om_poly::{MonoOrder, Monomial, UPoly};

fn shape(
    basis: &[Poly],
    f: &UPoly<Rational>,
    n: usize,
    ctx: &Interrupt,
) -> Result<Option<Vec<UPoly<Rational>>>, SolveError> {
    let mut coordinates = vec![];
    for axis in 0..n - 1 {
        ctx.tick()?;
        let mut coordinate = None;
        for p in basis {
            let mut coefficient = Rational::ZERO;
            let mut rest = vec![];
            let mut valid = true;
            for (m, c) in &p.terms {
                ctx.tick()?;
                if m.exps[axis] == 1 && m.deg == 1 {
                    coefficient += c;
                    continue;
                }
                if m.exps[..n - 1].iter().any(|e| *e != 0) {
                    valid = false;
                    break;
                }
                let degree = m.exps[n - 1] as usize;
                if degree > 4096 {
                    return Ok(None);
                }
                if rest.len() <= degree {
                    rest.resize(degree + 1, Rational::ZERO)
                }
                rest[degree] += c;
            }
            if valid && coefficient != Rational::ZERO {
                let h = UPoly::new(rest).scale(&(-Rational::ONE / coefficient), ctx)?;
                let Some((_, h)) = h.divrem(f, ctx)? else {
                    return Ok(None);
                };
                coordinate = Some(h);
                break;
            }
        }
        let Some(h) = coordinate else { return Ok(None) };
        coordinates.push(h);
    }
    coordinates.push(UPoly::new(vec![Rational::ZERO, Rational::ONE]));
    Ok(Some(coordinates))
}
fn value(p: &UPoly<Rational>, root: &Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    let mut terms = vec![];
    for (i, c) in p.coeffs.iter().enumerate() {
        ctx.tick()?;
        if c != &Rational::ZERO {
            terms.push(mul([
                Expr::number(Number::Rational(c.clone())),
                if i == 0 {
                    Expr::int(1)
                } else {
                    pow(root.clone(), Expr::int(i as i64))
                },
            ]))
        }
    }
    Ok(add(terms))
}
fn fresh(axes: &[Expr], ctx: &Interrupt) -> Result<Expr, SolveError> {
    let mut names = vec![];
    for a in axes {
        ctx.tick()?;
        for s in a.free_symbols() {
            if !names.contains(&s) {
                names.push(s)
            }
        }
    }
    for i in 0u32.. {
        ctx.tick()?;
        let s = Symbol::intern(&format!("om$primitive{i}"));
        if !names.contains(&s) {
            return Ok(Expr::sym(s));
        }
    }
    Err(SolveError::Unsupported(
        "primitive symbol exhaustion".into(),
    ))
}
pub(super) fn solve(
    basis: &[Poly],
    axes: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<Option<Vec<Solution>>, SolveError> {
    if axes.is_empty() {
        return Ok(Some(vec![]));
    }
    let n = axes.len();
    let mut eliminant = None;
    for p in basis {
        if let Some(f) = convert::univariate(p, n - 1, ctx)?
            && f.degree().is_some_and(|d| d > 0)
            && eliminant
                .as_ref()
                .is_none_or(|f0: &UPoly<Rational>| f.degree() < f0.degree())
        {
            eliminant = Some(f)
        }
    }
    let Some(eliminant) = eliminant else {
        return Ok(None);
    };
    let Some(factors) = convert::factors(&eliminant, ctx)? else {
        return Ok(None);
    };
    let mut solutions = vec![];
    for (f, multiplicity) in factors {
        let Some((primary, primary_length)) =
            multiplicity::component(basis, &f, n - 1, multiplicity as usize, axes, ctx, sink)?
        else {
            return Ok(None);
        };
        let fpoly = convert::embed(&f, n - 1, n, ctx)?;
        let expr = convert::render(&fpoly, axes, ctx)?;
        sink.record(|| {
            Step::new(
                StepKind::Eliminant {
                    var: axes[n - 1].clone(),
                    poly: expr.clone(),
                },
                vec![expr.clone()],
                vec![expr.clone()],
                Level::Major,
            )
        });
        let mut inputs = basis.to_vec();
        inputs.push(fpoly);
        let Some(component) = convert::basis(&inputs, axes, MonoOrder::Lex, ctx, sink)? else {
            return Ok(None);
        };
        if let Some(coordinates) = shape(&component, &f, n, ctx)? {
            let mut inner = opts.clone();
            inner.domain = Domain::Complexes;
            let roots = poly_uni(&expr, &axes[n - 1], &inner, ctx, sink)?;
            let SolutionSet::Finite(roots) = roots.set else {
                return Ok(None);
            };
            for mut root in roots {
                let rho = root.rules[0].1.clone();
                let mut rules = vec![];
                for (axis, coordinate) in axes.iter().zip(&coordinates) {
                    let v = value(coordinate, &rho, ctx)?;
                    sink.record(|| {
                        Step::new(
                            StepKind::BackSubstitute {
                                var: axis.clone(),
                                value: v.clone(),
                            },
                            vec![rho.clone()],
                            vec![Expr::call(B::RULE, [axis.clone(), v.clone()])],
                            Level::Major,
                        )
                    });
                    rules.push((axis.clone(), v))
                }
                root.rules = rules;
                root.multiplicity = multiplicity::per_root(
                    primary_length,
                    f.degree().expect("invariant: nonconstant factor"),
                )?;
                solutions.push(root);
            }
        } else {
            let mut found = None;
            let t = fresh(axes, ctx)?;
            let sequence = [1i64, -1, 2, -2, 3, -3, 4, -4, 5];
            for attempt in 0..5 {
                ctx.tick()?;
                let mut extended = vec![];
                for p in &component {
                    let mut terms = vec![];
                    for (m, c) in &p.terms {
                        let mut exps = m.exps.clone();
                        exps.push(0);
                        terms.push((
                            Monomial::new(exps)
                                .expect("invariant: adding a zero axis keeps degree"),
                            c.clone(),
                        ))
                    }
                    extended.push(Poly::new(n + 1, terms, MonoOrder::Lex, ctx)?)
                }
                let def = add(axes.iter().enumerate().map(|(i, a)| {
                    mul([
                        Expr::int(if i == n - 1 {
                            1
                        } else {
                            sequence[attempt + i % 4]
                        }),
                        a.clone(),
                    ])
                }));
                let mut terms = vec![];
                let mut exps = vec![0; n + 1];
                exps[n] = 1;
                terms.push((
                    Monomial::new(exps).expect("invariant: primitive axis degree one"),
                    Rational::ONE,
                ));
                for i in 0..n {
                    let mut exps = vec![0; n + 1];
                    exps[i] = 1;
                    terms.push((
                        Monomial::new(exps).expect("invariant: primitive linear degree"),
                        Rational::from(if i == n - 1 {
                            -1
                        } else {
                            -sequence[attempt + i % 4]
                        }),
                    ))
                }
                extended.push(Poly::new(n + 1, terms, MonoOrder::Lex, ctx)?);
                let primitive_relation = extended
                    .last()
                    .expect("invariant: appended primitive relation")
                    .clone();
                let mut original_extended = vec![];
                for p in &primary {
                    let mut terms = vec![];
                    for (m, c) in &p.terms {
                        ctx.tick()?;
                        let mut exps = m.exps.clone();
                        exps.push(0);
                        terms.push((
                            Monomial::new(exps).expect("invariant: adding a zero axis"),
                            c.clone(),
                        ))
                    }
                    original_extended.push(Poly::new(n + 1, terms, MonoOrder::Lex, ctx)?)
                }
                original_extended.push(primitive_relation);
                let mut new_axes = axes.to_vec();
                new_axes.push(t.clone());
                sink.record(|| {
                    Step::new(
                        StepKind::Substitute {
                            new_var: t.clone(),
                            def: def.clone(),
                        },
                        vec![def],
                        vec![t.clone()],
                        Level::Major,
                    )
                });
                let Some(g) = convert::basis(&extended, &new_axes, MonoOrder::Lex, ctx, sink)?
                else {
                    return Ok(None);
                };
                let mut complete = vec![];
                let mut good = false;
                for p in &g {
                    if let Some(f) = convert::univariate(p, n, ctx)?
                        && f.degree().is_some_and(|d| d > 0)
                        && let Some(coords) = shape(&g, &f, n + 1, ctx)?
                    {
                        let Some(primitive_factors) = convert::factors(&f, ctx)? else {
                            return Ok(None);
                        };
                        let mut inner = opts.clone();
                        inner.domain = Domain::Complexes;
                        for (factor, _) in primitive_factors {
                            let Some((_, length)) = multiplicity::component(
                                &original_extended,
                                &factor,
                                n,
                                primary_length,
                                &new_axes,
                                ctx,
                                sink,
                            )?
                            else {
                                return Ok(None);
                            };
                            let expr = convert::render(
                                &convert::embed(&factor, n, n + 1, ctx)?,
                                &new_axes,
                                ctx,
                            )?;
                            let roots = poly_uni(&expr, &t, &inner, ctx, sink)?;
                            let SolutionSet::Finite(roots) = roots.set else {
                                return Ok(None);
                            };
                            for mut root in roots {
                                let rho = root.rules[0].1.clone();
                                root.rules = axes
                                    .iter()
                                    .zip(&coords)
                                    .map(|(a, h)| Ok((a.clone(), value(h, &rho, ctx)?)))
                                    .collect::<Result<Vec<_>, SolveError>>()?;
                                for (axis, v) in &root.rules {
                                    sink.record(|| {
                                        Step::new(
                                            StepKind::BackSubstitute {
                                                var: axis.clone(),
                                                value: v.clone(),
                                            },
                                            vec![rho.clone()],
                                            vec![Expr::call(B::RULE, [axis.clone(), v.clone()])],
                                            Level::Major,
                                        )
                                    });
                                }
                                root.multiplicity = multiplicity::per_root(
                                    length,
                                    factor
                                        .degree()
                                        .expect("invariant: nonconstant primitive factor"),
                                )?;
                                complete.push(root)
                            }
                        }
                        good = true;
                        break;
                    }
                }
                if good {
                    found = Some(complete);
                    break;
                }
            }
            let Some(component_roots) = found else {
                return Ok(None);
            };
            solutions.extend(component_roots);
        }
    }
    Ok(Some(solutions))
}
