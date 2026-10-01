//! Algebraic relations among reversible kernels provide a bounded second route.
use super::{Engine, State, and, depends};
use crate::{Level, MaxExtra, SolutionSet, SolveError, Step, StepKind, StepSink};
use om_core::{BUILTIN as B, Expr, Symbol, add, pow, sub};
use om_simplify::convert::to_rational_function_with;

fn reversible(e: &Expr) -> bool {
    matches!(
        e.head_symbol(),
        Some(
            B::SIN
                | B::COS
                | B::TAN
                | B::COT
                | B::SEC
                | B::CSC
                | B::EXP
                | B::LOG
                | B::SINH
                | B::COSH
                | B::TANH
                | B::POWER
        )
    )
}
impl<S: StepSink> Engine<'_, S> {
    pub(super) fn lift(&mut self, state: State) -> Result<Option<Vec<State>>, SolveError> {
        let mut kernels = vec![];
        for e in &state.equations {
            let Some(view) = to_rational_function_with(e, &state.vars, self.ctx)? else {
                return Ok(None);
            };
            for g in &view.gens[state.vars.len()..] {
                if depends(g, &state.vars, self.ctx)? {
                    if !reversible(g) {
                        return Ok(None);
                    }
                    if !kernels.contains(g) {
                        kernels.push(g.clone())
                    }
                }
            }
        }
        if kernels.is_empty() || kernels.len() > 16 {
            return Ok(None);
        }
        let mut used = vec![];
        for e in state
            .equations
            .iter()
            .chain(self.branch.original.iter())
            .chain(self.requested.iter())
            .chain(state.root.rules.iter().map(|(_, v)| v))
        {
            used.extend(e.free_symbols());
        }
        let mut replacements = vec![];
        let mut index = 0u32;
        for k in &kernels {
            let axis = loop {
                self.ctx.tick()?;
                let s = Symbol::intern(&format!("om$kernel{index}"));
                index = index
                    .checked_add(1)
                    .ok_or_else(|| SolveError::Unsupported("kernel symbol exhaustion".into()))?;
                if !used.contains(&s) {
                    used.push(s);
                    break Expr::sym(s);
                }
            };
            replacements.push((k.clone(), axis));
        }
        let mut equations = state
            .equations
            .iter()
            .map(|e| e.replace_all(&replacements))
            .collect::<Vec<_>>();
        for (i, k) in kernels.iter().enumerate() {
            if k.is_head(B::SIN)
                && k.args().len() == 1
                && let Some(j) = kernels
                    .iter()
                    .position(|g| g.is_head(B::COS) && g.args() == k.args())
            {
                equations.push(sub(
                    add([
                        pow(replacements[i].1.clone(), Expr::int(2)),
                        pow(replacements[j].1.clone(), Expr::int(2)),
                    ]),
                    Expr::int(1),
                ));
            }
        }
        let mut axes = vec![];
        for v in &state.vars {
            let mut present = false;
            for e in &equations {
                present |= depends(e, std::slice::from_ref(v), self.ctx)?;
            }
            if present {
                axes.push(v.clone())
            }
        }
        axes.extend(replacements.iter().map(|(_, v)| v.clone()));
        let input = Expr::call(
            B::LIST,
            equations
                .iter()
                .map(|e| Expr::call(B::EQUAL, [e.clone(), Expr::int(0)])),
        );
        for (k, v) in &replacements {
            self.sink.record(|| {
                Step::new(
                    StepKind::Substitute {
                        new_var: v.clone(),
                        def: k.clone(),
                    },
                    state.equations.clone(),
                    equations.clone(),
                    Level::Major,
                )
            });
        }
        let mut opts = self.opts.clone();
        opts.domain = crate::Domain::Complexes;
        opts.max_extra_conditions = MaxExtra::Zero;
        let result = match crate::poly_system(&input, &axes, &opts, self.ctx, self.sink) {
            Err(SolveError::Unsupported(_)) => return Ok(None),
            result => result?,
        };
        let SolutionSet::Finite(roots) = result.set else {
            return Ok(None);
        };
        self.messages.extend(result.messages);
        self.allocated = self.allocated.saturating_add(roots.len().saturating_sub(1));
        if self.allocated > 64 {
            return Ok(None);
        }
        let mut complete = vec![];
        for root in roots {
            let auxiliaries = replacements
                .iter()
                .map(|(_, v)| v.clone())
                .collect::<Vec<_>>();
            if root.condition.as_ref().is_some_and(|c| {
                c.free_symbols()
                    .iter()
                    .any(|s| auxiliaries.contains(&Expr::sym(*s)))
            }) {
                return Ok(None);
            }
            let mut recovery = vec![];
            for (k, v) in &replacements {
                let Some((_, value)) = root.rules.iter().find(|(a, _)| a == v) else {
                    return Ok(None);
                };
                if depends(value, &auxiliaries, self.ctx)? {
                    return Ok(None);
                }
                recovery.push(sub(k.clone(), value.clone()));
            }
            let rules = root
                .rules
                .into_iter()
                .filter(|(v, _)| state.vars.contains(v))
                .collect::<Vec<_>>();
            let mut next = state.clone();
            next.equations = recovery.iter().map(|e| e.replace_all(&rules)).collect();
            next.vars.retain(|v| !rules.iter().any(|(a, _)| a == v));
            for (_, v) in &mut next.root.rules {
                *v = v.replace_all(&rules)
            }
            next.root.rules.extend(rules);
            next.root.condition = and(next.root.condition.take(), root.condition);
            next.guards.extend(result.assumptions.iter().cloned());
            let Some(recovered) = self.solve(next, false)? else {
                return Ok(None);
            };
            complete.extend(recovered);
        }
        Ok(Some(complete))
    }
}
