//! Commensurate kernels reduce to algebra; explicit inverses retain their periods.
pub(crate) mod check;
mod inverse;
mod kernel;
use super::{PolynomialRoots, extract, poly_uni, radical_path};
use crate::{
    Domain, Level, NoSteps, Solution, SolutionSet, SolveError, SolveOptions, Step, StepKind,
    StepSink, normalize,
};
use om_core::{BUILTIN as B, Expr, ExprKind, Message, MsgLevel, canonical_cmp, sub};
use om_num::ctx::Interrupt;

/// Solve supported transcendental kernels and verify original branch/pole constraints.
/// Generated integer periods remain in Solution::constants; incomplete inverse
/// paths are diagnostic and unevaluated rather than partially returned.
pub fn transcendental_path(
    e: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    let mut inner = opts.clone();
    if matches!(inner.domain, Domain::Integers | Domain::Rationals) {
        inner.domain = Domain::Reals
    }
    if !opts.record_steps {
        let result = run(e, x, &inner, ctx, &mut NoSteps)?;
        return crate::domain::filter(result, &[(x.clone(), opts.domain)], ctx, &mut NoSteps);
    }
    let result = run(e, x, &inner, ctx, sink)?;
    crate::domain::filter(result, &[(x.clone(), opts.domain)], ctx, sink)
}
/// Internal inversion candidates are verified after all system substitutions.
pub(crate) fn candidates(
    e: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    let mut engine = Engine {
        opts,
        ctx,
        sink,
        next: first_constant(e, opts, ctx)?,
        assumptions: vec![],
        messages: vec![],
        real_values: vec![],
    };
    let set = if let Some(mut roots) = engine.solve(e, x, 0)? {
        check::real_filter(&mut roots, &engine.real_values, opts, ctx, engine.sink)?;
        for root in &mut roots {
            root.verification = crate::Verification::Unverified;
        }
        SolutionSet::Finite(roots)
    } else {
        SolutionSet::Unevaluated
    };
    Ok(PolynomialRoots {
        set,
        assumptions: engine.assumptions,
        messages: engine.messages,
    })
}
struct Engine<'a, S> {
    opts: &'a SolveOptions,
    ctx: &'a Interrupt,
    sink: &'a mut S,
    next: u32,
    assumptions: Vec<Expr>,
    messages: Vec<Message>,
    real_values: Vec<Expr>,
}
impl<S: StepSink> Engine<'_, S> {
    fn power_inverse(
        &mut self,
        k: &Expr,
        value: &Expr,
    ) -> Result<Option<inverse::Inversion>, SolveError> {
        let Some(q) = extract::exact(&k.args()[1]) else {
            return Ok(None);
        };
        let p = q.numerator().clone();
        let denominator = om_num::Integer::from(q.denominator().clone());
        let negative = p < om_num::Integer::ZERO;
        let degree = if negative { -p } else { p };
        if degree == om_num::Integer::ZERO || degree > 4096.into() || denominator > 4096.into() {
            return Ok(None);
        }
        let axis = super::reductions::fresh(k, &k.args()[0], self.ctx)?;
        let power = om_core::pow(axis.clone(), Expr::number(om_num::Number::Integer(degree)));
        let target = om_core::pow(
            value.clone(),
            Expr::number(om_num::Number::Integer(denominator)),
        );
        let equation = if negative {
            sub(om_core::mul([power, target]), Expr::int(1))
        } else {
            sub(power, target)
        };
        let mut opts = self.opts.clone();
        opts.domain = Domain::Complexes;
        let result = poly_uni(&equation, &axis, &opts, self.ctx, self.sink)?;
        self.messages.extend(result.messages);
        for guard in result.assumptions {
            if !self.assumptions.contains(&guard) {
                self.assumptions.push(guard)
            }
        }
        let SolutionSet::Finite(roots) = result.set else {
            return Ok(None);
        };
        Ok(Some(inverse::Inversion {
            argument: k.args()[0].clone(),
            head: B::POWER,
            branches: roots
                .into_iter()
                .map(|r| inverse::Branch {
                    target: r.rules[0].1.clone(),
                    condition: r.condition,
                    constants: vec![],
                    known_real: false,
                })
                .collect(),
            ifun: false,
        }))
    }
    fn constant(&mut self) -> Result<Expr, SolveError> {
        self.ctx.tick()?;
        let c = Expr::call(
            self.opts.generated_parameter,
            [Expr::int(i64::from(self.next))],
        );
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| SolveError::Unsupported("generated parameter index exhausted".into()))?;
        Ok(c)
    }
    fn solve(
        &mut self,
        e: &Expr,
        x: &Expr,
        depth: u32,
    ) -> Result<Option<Vec<Solution>>, SolveError> {
        self.ctx.tick()?;
        if depth > 24 {
            return Ok(None);
        }
        let work = extract::special(e, self.ctx)?;
        let prepared = normalize::normalize(
            &Expr::call(B::EQUAL, [work, Expr::int(0)]),
            Some(std::slice::from_ref(x)),
            Domain::Complexes,
            self.ctx,
            self.sink,
        )?;
        let Some(work) = prepared.branches.first().and_then(|b| b.equations.first()) else {
            return Ok(None);
        };
        let mut inner = self.opts.clone();
        inner.domain = Domain::Complexes;
        if extract::coefficients(work, x, self.ctx)?.is_some() {
            let result = poly_uni(work, x, &inner, self.ctx, self.sink)?;
            for a in result.assumptions {
                if !self.assumptions.contains(&a) {
                    self.assumptions.push(a)
                }
            }
            self.messages.extend(result.messages);
            return Ok(match result.set {
                SolutionSet::Finite(r) => Some(r),
                _ => None,
            });
        }
        if !self.opts.inverse_functions {
            return Ok(None);
        }
        let Some(k) = kernel::unify(work, x, self.ctx)? else {
            return Ok(None);
        };
        self.sink.record(|| {
            Step::new(
                StepKind::Substitute {
                    new_var: k.axis.clone(),
                    def: k.definition.clone(),
                },
                vec![work.clone()],
                vec![k.equation.clone()],
                Level::Major,
            )
        });
        let Some(ys) = self.solve(&k.equation, &k.axis, depth + 1)? else {
            return Ok(None);
        };
        let mut roots = vec![];
        let c = if inverse::periodic(&k.definition) {
            Some(self.constant()?)
        } else {
            None
        };
        for y in ys {
            let value = &y.rules[0].1;
            let Some(inv) = (if k.definition.is_head(B::POWER)
                && k.definition.args().len() == 2
                && extract::exact(&k.definition.args()[1]).is_some()
            {
                self.power_inverse(&k.definition, value)?
            } else {
                inverse::invert(
                    &k.definition,
                    value,
                    c.as_ref(),
                    self.opts.domain,
                    x,
                    self.ctx,
                )?
            }) else {
                return Ok(None);
            };
            if inv.ifun {
                let msg=Message{symbol:"Solve".into(),tag:"ifun".into(),text:"Inverse functions are being used by Solve, so some solutions may not be found; use Reduce for complete solution information.".into(),level:MsgLevel::Warning};
                self.sink.record(|| {
                    Step::new(
                        StepKind::Note { msg: msg.clone() },
                        vec![work.clone()],
                        vec![],
                        Level::Minor,
                    )
                });
                self.messages.push(msg);
            }
            self.sink.record(|| {
                Step::new(
                    StepKind::InvertFunction {
                        func: inv.head,
                        branches: inv.branches.iter().map(|b| b.target.clone()).collect(),
                        constants: c.iter().cloned().collect(),
                    },
                    vec![Expr::call(B::EQUAL, [k.definition.clone(), value.clone()])],
                    inv.branches
                        .iter()
                        .map(|b| Expr::call(B::EQUAL, [inv.argument.clone(), b.target.clone()]))
                        .collect(),
                    Level::Major,
                )
            });
            for branch in inv.branches {
                if branch.known_real {
                    self.real_values.push(branch.target.clone())
                }
                let Some(mut recovered) =
                    self.solve(&sub(inv.argument.clone(), branch.target), x, depth + 1)?
                else {
                    return Ok(None);
                };
                for root in &mut recovered {
                    root.condition = inverse::and(
                        root.condition.take(),
                        inverse::and(y.condition.clone(), branch.condition.clone()),
                    );
                    root.constants.extend(y.constants.iter().cloned());
                    root.constants.extend(branch.constants.iter().cloned());
                    root.multiplicity = 1;
                    self.sink.record(|| {
                        Step::new(
                            StepKind::BackSubstitute {
                                var: x.clone(),
                                value: root.rules[0].1.clone(),
                            },
                            vec![k.definition.clone()],
                            vec![root.rules[0].1.clone()],
                            Level::Minor,
                        )
                    });
                }
                roots.extend(recovered);
            }
        }
        Ok(Some(roots))
    }
}
fn first_constant(e: &Expr, opts: &SolveOptions, ctx: &Interrupt) -> Result<u32, SolveError> {
    let mut next = 1;
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if e.head_symbol() == Some(opts.generated_parameter)
            && e.args().len() == 1
            && let Some(q) = extract::exact(&e.args()[0])
            && q.denominator() == &1_u8.into()
            && let Ok(i) = u32::try_from(q.numerator())
        {
            next = next.max(i.checked_add(1).ok_or_else(|| {
                SolveError::Unsupported("generated parameter index exhausted".into())
            })?);
        }
        if let ExprKind::Normal(n) = e.kind() {
            stack.push(&n.head);
            stack.extend(&n.args)
        }
    }
    Ok(next)
}
fn run(
    e: &Expr,
    x: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    ctx.tick()?;
    let prepared = normalize::normalize(
        &Expr::call(B::EQUAL, [e.clone(), Expr::int(0)]),
        Some(std::slice::from_ref(x)),
        opts.domain,
        ctx,
        sink,
    )?;
    if !prepared.unsupported && extract::coefficients(&extract::special(e, ctx)?, x, ctx)?.is_some()
    {
        return radical_path(e, x, opts, ctx, sink);
    }
    let mut engine = Engine {
        opts,
        ctx,
        sink,
        next: first_constant(e, opts, ctx)?,
        assumptions: vec![],
        messages: prepared.messages,
        real_values: vec![],
    };
    let result = if !prepared.unsupported
        && let Some(branch) = prepared.branches.first()
        && let Some(work) = branch.equations.first()
    {
        if let Some(mut roots) = engine.solve(work, x, 0)? {
            check::real_filter(&mut roots, &engine.real_values, opts, ctx, engine.sink)?;
            check::candidates(
                check::Source {
                    original: e,
                    exclusions: &branch.exclusions,
                    guards: &engine.assumptions,
                },
                &mut roots,
                opts,
                ctx,
                engine.sink,
                &mut engine.messages,
            )?;
            let mut unique: Vec<Solution> = vec![];
            for root in roots {
                if !unique.iter().any(|r| {
                    r.rules == root.rules
                        && r.constants == root.constants
                        && r.condition == root.condition
                }) {
                    unique.push(root)
                }
            }
            let mut sorted = vec![];
            for mut root in unique {
                let rules = root
                    .constants
                    .iter()
                    .map(|(c, _)| (c.clone(), Expr::int(0)))
                    .collect::<Vec<_>>();
                let numeric =
                    om_simplify::numeval::evaluate(&root.rules[0].1.replace_all(&rules), 128, ctx)?
                        .map(|n| n.to_complex_f64())
                        .filter(|(r, i)| r.is_finite() && i.is_finite());
                root.numeric = numeric.map(|n| vec![n]);
                sorted.push((root, numeric));
            }
            sorted.sort_by(|(a, an), (b, bn)| match (an, bn) {
                (Some((ar, ai)), Some((br, bi))) => (ai != &0.0)
                    .cmp(&(bi != &0.0))
                    .then_with(|| ar.total_cmp(br))
                    .then_with(|| ai.total_cmp(bi))
                    .then_with(|| canonical_cmp(&a.rules[0].1, &b.rules[0].1)),
                _ => canonical_cmp(&a.rules[0].1, &b.rules[0].1),
            });
            SolutionSet::Finite(sorted.into_iter().map(|(r, _)| r).collect())
        } else {
            SolutionSet::Unevaluated
        }
    } else {
        SolutionSet::Unevaluated
    };
    if matches!(result, SolutionSet::Unevaluated) {
        let msg=Message{symbol:"Solve".into(),tag:"nsmet".into(),text:"This transcendental equation has unsupported independent kernels or inverse branches.".into(),level:MsgLevel::Warning};
        engine.sink.record(|| {
            Step::new(
                StepKind::Note { msg: msg.clone() },
                vec![e.clone()],
                vec![],
                Level::Minor,
            )
        });
        engine.messages.push(msg);
    }
    Ok(PolynomialRoots {
        set: result,
        assumptions: engine.assumptions,
        messages: engine.messages,
    })
}
