//! Mixed systems eliminate one original variable at each verified substitution.
mod lift;
use crate::{
    Domain, Level, MaxExtra, NoSteps, Solution, SolutionSet, SolveError, SolveOptions, Step,
    StepKind, StepSink, Verification, normalize,
    univariate::{
        PolynomialRoots, radical_candidates, transcendental::check, transcendental_candidates,
    },
};
use om_core::{BUILTIN as B, Expr, ExprKind, Message, MsgLevel};
use om_num::ctx::Interrupt;
use om_simplify::zero::{Tri, is_zero_with};

/// Solve mixed conjunctions by ranked univariate substitution and bounded kernel lifting.
/// Every finite branch must complete; original branch and pole checks are mandatory.
pub fn substitution_system(
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    if !opts.record_steps {
        let result = run(eqs, vars, opts, ctx, &mut NoSteps)?;
        return crate::domain::filter_input(eqs, vars, opts, result, ctx, &mut NoSteps);
    }
    let result = run(eqs, vars, opts, ctx, sink)?;
    crate::domain::filter_input(eqs, vars, opts, result, ctx, sink)
}
fn and(a: Option<Expr>, b: Option<Expr>) -> Option<Expr> {
    match (a, b) {
        (Some(a), Some(b)) if a != b => Some(Expr::call(B::AND, [a, b])),
        (Some(a), _) => Some(a),
        (_, b) => b,
    }
}
fn depends(e: &Expr, vars: &[Expr], ctx: &Interrupt) -> Result<bool, SolveError> {
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if vars.contains(e) {
            return Ok(true);
        }
        if let ExprKind::Normal(n) = e.kind() {
            stack.push(&n.head);
            stack.extend(&n.args);
        }
    }
    Ok(false)
}
fn empty_solution() -> Solution {
    Solution {
        rules: vec![],
        condition: None,
        constants: vec![],
        multiplicity: 1,
        verification: Verification::Exact,
        numeric: None,
    }
}
#[derive(Clone)]
struct State {
    equations: Vec<Expr>,
    vars: Vec<Expr>,
    root: Solution,
    guards: Vec<Expr>,
}
struct Engine<'a, S> {
    opts: &'a SolveOptions,
    ctx: &'a Interrupt,
    sink: &'a mut S,
    branch: &'a normalize::NormalizedBranch,
    requested: &'a [Expr],
    allocated: usize,
    next: u32,
    assumptions: Vec<Expr>,
    messages: Vec<Message>,
}
fn invoke(
    equation: &Expr,
    var: &Expr,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    let radical = radical_candidates(equation, var, opts, ctx, sink);
    match radical {
        Ok(r) if !matches!(r.set, SolutionSet::Unevaluated) => return Ok(r),
        Err(e @ SolveError::Abort(_)) | Err(e @ SolveError::Invalid(_)) => return Err(e),
        _ => {}
    }
    match transcendental_candidates(equation, var, opts, ctx, sink) {
        Err(SolveError::Unsupported(_)) => Ok(PolynomialRoots {
            set: SolutionSet::Unevaluated,
            assumptions: vec![],
            messages: vec![],
        }),
        result => result,
    }
}
impl<S: StepSink> Engine<'_, S> {
    fn note(&mut self, tag: &str, text: &str, input: &Expr) {
        if self.messages.iter().any(|m| m.tag == tag) {
            return;
        }
        let msg = Message {
            symbol: "Solve".into(),
            tag: tag.into(),
            text: text.into(),
            level: MsgLevel::Warning,
        };
        self.sink.record(|| {
            Step::new(
                StepKind::Note { msg: msg.clone() },
                vec![input.clone()],
                vec![],
                Level::Minor,
            )
        });
        self.messages.push(msg);
    }
    fn inner(&self, var: &Expr) -> SolveOptions {
        let mut opts = self.opts.clone();
        opts.domain = self
            .branch
            .domains
            .iter()
            .find(|(v, _)| v == var)
            .map_or(opts.domain, |(_, d)| *d);
        opts.max_extra_conditions = MaxExtra::Zero;
        opts
    }
    fn rename(&mut self, roots: &mut [Solution]) -> Result<(), SolveError> {
        let mut names = vec![];
        for root in roots.iter() {
            for (c, _) in &root.constants {
                if !names.contains(c) {
                    names.push(c.clone())
                }
            }
        }
        let mut rules = vec![];
        for c in names {
            let new = Expr::call(
                self.opts.generated_parameter,
                [Expr::int(i64::from(self.next))],
            );
            self.next = self.next.checked_add(1).ok_or_else(|| {
                SolveError::Unsupported("system parameter index exhaustion".into())
            })?;
            rules.push((c, new));
        }
        for root in roots {
            for (_, v) in &mut root.rules {
                *v = v.replace_all(&rules)
            }
            root.condition = root.condition.take().map(|c| c.replace_all(&rules));
            for (c, _) in &mut root.constants {
                *c = c.replace_all(&rules)
            }
        }
        Ok(())
    }
    fn solve(&mut self, mut state: State, lifting: bool) -> Result<Option<Vec<State>>, SolveError> {
        self.ctx.tick()?;
        let mut equations = vec![];
        for e in state.equations {
            let outcome = is_zero_with(&e, self.ctx)?;
            if outcome == Tri::Zero {
                continue;
            }
            if !depends(&e, &state.vars, self.ctx)? {
                let periods = state
                    .root
                    .constants
                    .iter()
                    .map(|(c, _)| c.clone())
                    .collect::<Vec<_>>();
                if e.free_symbols().is_empty()
                    && !depends(&e, &periods, self.ctx)?
                    && outcome == Tri::NonZero
                {
                    return Ok(Some(vec![]));
                }
                state.root.condition = and(
                    state.root.condition.take(),
                    Some(Expr::call(B::EQUAL, [e, Expr::int(0)])),
                );
            } else {
                equations.push(e)
            }
        }
        state.equations = equations;
        if state.equations.is_empty() {
            return Ok(Some(vec![state]));
        }
        let mut best = None;
        for (i, e) in state.equations.iter().enumerate() {
            for (j, var) in state.vars.iter().enumerate() {
                if !depends(e, std::slice::from_ref(var), self.ctx)? {
                    continue;
                }
                let result = invoke(e, var, &self.inner(var), self.ctx, &mut NoSteps)?;
                let SolutionSet::Finite(roots) = &result.set else {
                    continue;
                };
                let score = (roots.len(), e.leaf_count());
                if best.as_ref().is_none_or(|(_, _, s, _)| score < *s) {
                    best = Some((i, j, score, result))
                }
                if score.0 == 0 {
                    return Ok(Some(vec![]));
                }
            }
        }
        let Some((equation, axis, _, mut result)) = best else {
            return if lifting { self.lift(state) } else { Ok(None) };
        };
        let var = state.vars[axis].clone();
        if self.sink.enabled() {
            result = invoke(
                &state.equations[equation],
                &var,
                &self.inner(&var),
                self.ctx,
                self.sink,
            )?;
        }
        self.messages.extend(result.messages);
        let SolutionSet::Finite(mut roots) = result.set else {
            return Ok(None);
        };
        self.allocated = self.allocated.saturating_add(roots.len().saturating_sub(1));
        if self.allocated > 64 {
            return Ok(None);
        }
        self.rename(&mut roots)?;
        let mut output = vec![];
        for root in roots {
            let Some((_, value)) = root.rules.iter().find(|(v, _)| v == &var) else {
                return Ok(None);
            };
            let value = value.clone();
            let replacement = [(var.clone(), value.clone())];
            let mut next = state.clone();
            next.vars.remove(axis);
            next.equations = state
                .equations
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != equation)
                .map(|(_, e)| e.replace_all(&replacement))
                .collect();
            for (_, v) in &mut next.root.rules {
                *v = v.replace_all(&replacement)
            }
            next.root.rules.extend(root.rules);
            next.root.condition = and(
                next.root
                    .condition
                    .take()
                    .map(|c| c.replace_all(&replacement)),
                root.condition,
            );
            next.root.constants.extend(root.constants);
            next.root.multiplicity = 1;
            next.root.numeric = None;
            next.guards = next
                .guards
                .iter()
                .chain(&result.assumptions)
                .map(|g| g.replace_all(&replacement))
                .collect();
            self.sink.record(|| {
                Step::new(
                    StepKind::BackSubstitute {
                        var: var.clone(),
                        value: value.clone(),
                    },
                    vec![state.equations[equation].clone()],
                    next.equations.clone(),
                    Level::Major,
                )
            });
            let Some(solved) = self.solve(next, lifting)? else {
                return Ok(None);
            };
            output.extend(solved);
        }
        Ok(Some(output))
    }
    fn finish(&mut self, mut state: State) -> Result<Option<Solution>, SolveError> {
        state
            .root
            .rules
            .sort_by_key(|(v, _)| self.requested.iter().position(|a| a == v));
        let rules = state.root.rules.clone();
        state.root.condition = and(
            state.root.condition.take(),
            self.branch
                .conditions
                .iter()
                .map(|c| c.replace_all(&rules))
                .reduce(|a, b| Expr::call(B::AND, [a, b])),
        );
        for (var, domain) in &self.branch.domains {
            if *domain != Domain::Reals {
                continue;
            }
            let value = state
                .root
                .rules
                .iter()
                .find(|(v, _)| v == var)
                .map_or(var, |(_, v)| v);
            let ball = om_simplify::numeval::enclose(value, 512, self.ctx)?;
            if ball.as_ref().is_some_and(|z| z.im.excludes_zero()) {
                return Ok(None);
            }
            if !ball.is_some_and(|z| {
                z.im.mid == om_num::BigFloat::ZERO && z.im.rad == om_num::BigFloat::ZERO
            }) {
                state.root.condition = and(
                    state.root.condition.take(),
                    Some(Expr::call(B::ELEMENT, [value.clone(), Expr::sym(B::REALS)])),
                );
            }
        }
        for guard in &state.guards {
            if !self.assumptions.contains(guard) {
                self.assumptions.push(guard.clone())
            }
        }
        let mut roots = vec![state.root];
        let mut numeric = false;
        let identity = Expr::int(0);
        for original in self
            .branch
            .original
            .iter()
            .chain(std::iter::once(&identity))
        {
            check::candidates(
                check::Source {
                    original,
                    exclusions: &self.branch.exclusions,
                    guards: &state.guards,
                },
                &mut roots,
                self.opts,
                self.ctx,
                self.sink,
                &mut self.messages,
            )?;
            if roots.is_empty() {
                return Ok(None);
            }
            numeric |= matches!(roots[0].verification, Verification::Numeric { .. });
        }
        let mut root = roots.pop().expect("invariant: checked singleton candidate");
        if numeric {
            root.verification = Verification::Numeric { digits: 33 }
        }
        let expose = match self.opts.max_extra_conditions {
            MaxExtra::Zero => false,
            MaxExtra::All => true,
            MaxExtra::Count(n) => state.guards.len() <= n as usize,
        };
        if expose {
            for guard in state.guards {
                root.condition = and(root.condition.take(), Some(guard))
            }
        }
        if !state.vars.is_empty() && !root.rules.is_empty() {
            self.note(
                "svars",
                "The mixed system leaves original free variables.",
                &Expr::call(B::LIST, self.branch.original.iter().cloned()),
            );
        }
        let mut numeric_values = vec![];
        for (_, v) in &root.rules {
            let Some(z) = om_simplify::numeval::enclose(v, 128, self.ctx)? else {
                return Ok(Some(root));
            };
            let value = (z.re.to_f64(), z.im.to_f64());
            if !value.0.is_finite() || !value.1.is_finite() {
                return Ok(Some(root));
            }
            numeric_values.push(value);
        }
        root.numeric = Some(numeric_values);
        Ok(Some(root))
    }
}
fn first_parameter(e: &Expr, opts: &SolveOptions, ctx: &Interrupt) -> Result<u32, SolveError> {
    let mut next = 1;
    let mut stack = vec![e];
    while let Some(e) = stack.pop() {
        ctx.tick()?;
        if e.head_symbol() == Some(opts.generated_parameter)
            && e.args().len() == 1
            && let Some(om_num::Number::Integer(i)) = e.args()[0].as_number()
            && let Ok(i) = u32::try_from(i)
        {
            next = next.max(i.checked_add(1).ok_or_else(|| {
                SolveError::Unsupported("system parameter index exhaustion".into())
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
    eqs: &Expr,
    vars: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<PolynomialRoots, SolveError> {
    if let Some(result) = crate::domain::affine(eqs, vars, opts, ctx, sink)? {
        return Ok(result);
    }
    let prepared = normalize::normalize(eqs, Some(vars), opts.domain, ctx, sink)?;
    let Some(branch) = prepared.branches.first() else {
        return Ok(PolynomialRoots {
            set: if prepared.unsupported {
                SolutionSet::Unevaluated
            } else {
                SolutionSet::Finite(vec![])
            },
            assumptions: vec![],
            messages: prepared.messages,
        });
    };
    let mut engine = Engine {
        opts,
        ctx,
        sink,
        branch,
        requested: &prepared.vars,
        allocated: 1,
        next: first_parameter(eqs, opts, ctx)?,
        assumptions: vec![],
        messages: prepared.messages,
    };
    let states = if prepared.unsupported
        || prepared.branches.len() != 1
        || !branch.inequalities.is_empty()
    {
        None
    } else {
        engine.solve(
            State {
                equations: branch.equations.clone(),
                vars: prepared.vars.clone(),
                root: empty_solution(),
                guards: vec![],
            },
            true,
        )?
    };
    let set = if let Some(states) = states {
        let mut roots = vec![];
        for state in states {
            if let Some(root) = engine.finish(state)?
                && !roots.iter().any(|r: &Solution| {
                    r.rules == root.rules
                        && r.condition == root.condition
                        && r.constants == root.constants
                })
            {
                roots.push(root)
            }
        }
        if roots.len() == 1
            && roots[0].rules.is_empty()
            && roots[0].condition.is_none()
            && roots[0].constants.is_empty()
        {
            SolutionSet::All
        } else {
            SolutionSet::Finite(roots)
        }
    } else {
        engine.note(
            "nsmet",
            "This mixed system has an incomplete substitution branch or exceeds 64 branches.",
            eqs,
        );
        SolutionSet::Unevaluated
    };
    Ok(PolynomialRoots {
        set,
        assumptions: engine.assumptions,
        messages: engine.messages,
    })
}
