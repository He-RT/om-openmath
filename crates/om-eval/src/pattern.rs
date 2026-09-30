//! Iterative ordered matching; sequence alternatives are explored lazily.

use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel, Symbol};
use std::collections::BTreeMap;

/// Named scalar or raw Sequence bindings from one successful match.
pub type Bindings = BTreeMap<Symbol, Expr>;

#[derive(Clone)]
enum Task {
    Expr(Expr, Expr),
    Args {
        lhs: Expr,
        expr: Expr,
        pi: usize,
        ei: usize,
    },
    Choice {
        lhs: Expr,
        expr: Expr,
        pi: usize,
        ei: usize,
        count: usize,
        max: usize,
    },
    Sequence(Expr, Vec<Expr>),
    Bind(Symbol, Expr),
    Condition(Expr),
}
#[derive(Clone)]
struct State {
    work: Vec<Task>,
    bindings: Bindings,
    conditions: Vec<Expr>,
}

/// Match a rule's left side, checking conditions with the supplied evaluator.
/// The evaluator owns any predicate side effects; failed structural branches
/// discard only their local bindings.
pub fn match_rule(
    lhs: &Expr,
    expr: &Expr,
    evaluator: &mut Evaluator,
    ctx: &Interrupt,
) -> Result<Option<Bindings>, EvalError> {
    let Some(mut matcher) = Matcher::new(lhs, expr, evaluator, ctx)? else {
        return Ok(None);
    };
    while let Some(candidate) = matcher.next(ctx)? {
        let mut valid = true;
        for test in candidate.conditions {
            if evaluator
                .evaluate(&substitute(&test, &candidate.bindings), ctx)?
                .as_symbol()
                != Some(B::TRUE)
            {
                valid = false;
                break;
            }
        }
        if valid {
            return Ok(Some(candidate.bindings));
        }
    }
    Ok(None)
}
pub(crate) struct Candidate {
    pub bindings: Bindings,
    pub conditions: Vec<Expr>,
}
pub(crate) struct Matcher {
    pending: Vec<State>,
}
impl Matcher {
    pub(crate) fn new(
        lhs: &Expr,
        expr: &Expr,
        evaluator: &mut Evaluator,
        ctx: &Interrupt,
    ) -> Result<Option<Self>, EvalError> {
        let mut scan = vec![lhs];
        while let Some(e) = scan.pop() {
            ctx.tick()?;
            if let ExprKind::Normal(n) = e.kind() {
                if n.head.as_symbol().is_some_and(|s| {
                    matches!(s.name(), "Optional" | "Alternatives" | "PatternTest")
                }) {
                    evaluator.message(
                        "Pattern",
                        "unsup",
                        "This pattern construct is not supported.".into(),
                        MsgLevel::Warning,
                    );
                    return Ok(None);
                }
                scan.push(&n.head);
                scan.extend(n.args.iter());
            }
        }
        Ok(Some(Self {
            pending: vec![State {
                work: vec![Task::Expr(lhs.clone(), expr.clone())],
                bindings: Bindings::new(),
                conditions: vec![],
            }],
        }))
    }
    pub(crate) fn next(&mut self, ctx: &Interrupt) -> Result<Option<Candidate>, EvalError> {
        while let Some(mut state) = self.pending.pop() {
            let mut matched = true;
            while let Some(task) = state.work.pop() {
                ctx.tick()?;
                match task {
                    Task::Bind(name, value) => {
                        if let Some(old) = state.bindings.get(&name) {
                            matched = *old == value;
                        } else {
                            state.bindings.insert(name, value);
                        }
                    }
                    Task::Condition(test) => state.conditions.push(test),
                    Task::Expr(p, e) => {
                        if p.is_head(B::PATTERN) && p.args().len() == 2 {
                            if let Some(name) = p.args()[0].as_symbol() {
                                state.work.push(Task::Bind(name, e.clone()));
                                state.work.push(Task::Expr(p.args()[1].clone(), e));
                            } else {
                                matched = false;
                            }
                        } else if p.is_head(B::CONDITION) && p.args().len() == 2 {
                            state.work.push(Task::Condition(p.args()[1].clone()));
                            state.work.push(Task::Expr(p.args()[0].clone(), e));
                        } else if matches!(
                            p.head_symbol(),
                            Some(B::BLANK | B::BLANK_SEQUENCE | B::BLANK_NULL_SEQUENCE)
                        ) {
                            matched = blank(&p, &e);
                        } else if let (ExprKind::Normal(pn), ExprKind::Normal(en)) =
                            (p.kind(), e.kind())
                        {
                            state.work.push(Task::Args {
                                lhs: p.clone(),
                                expr: e.clone(),
                                pi: 0,
                                ei: 0,
                            });
                            state
                                .work
                                .push(Task::Expr(pn.head.clone(), en.head.clone()));
                        } else {
                            matched = p == e;
                        }
                    }
                    Task::Args { lhs, expr, pi, ei } => {
                        if pi == lhs.args().len() {
                            matched = ei == expr.args().len();
                        } else if let Some(min) = sequence_min(&lhs.args()[pi]) {
                            let rest = lhs.args()[pi + 1..]
                                .iter()
                                .map(|p| sequence_min(p).unwrap_or(1))
                                .sum::<usize>();
                            if let Some(available) = expr.args().len().checked_sub(ei + rest) {
                                if available >= min {
                                    state.work.push(Task::Choice {
                                        lhs,
                                        expr,
                                        pi,
                                        ei,
                                        count: min,
                                        max: available,
                                    });
                                } else {
                                    matched = false;
                                }
                            } else {
                                matched = false;
                            }
                        } else if ei < expr.args().len() {
                            let p = lhs.args()[pi].clone();
                            let e = expr.args()[ei].clone();
                            state.work.push(Task::Args {
                                lhs,
                                expr,
                                pi: pi + 1,
                                ei: ei + 1,
                            });
                            state.work.push(Task::Expr(p, e));
                        } else {
                            matched = false;
                        }
                    }
                    Task::Choice {
                        lhs,
                        expr,
                        pi,
                        ei,
                        count,
                        max,
                    } => {
                        if count < max {
                            let mut later = state.clone();
                            later.work.push(Task::Choice {
                                lhs: lhs.clone(),
                                expr: expr.clone(),
                                pi,
                                ei,
                                count: count + 1,
                                max,
                            });
                            self.pending.push(later);
                        }
                        let p = lhs.args()[pi].clone();
                        let items = expr.args()[ei..ei + count].to_vec();
                        state.work.push(Task::Args {
                            lhs,
                            expr,
                            pi: pi + 1,
                            ei: ei + count,
                        });
                        state.work.push(Task::Sequence(p, items));
                    }
                    Task::Sequence(p, items) => {
                        if p.is_head(B::PATTERN) && p.args().len() == 2 {
                            if let Some(name) = p.args()[0].as_symbol() {
                                state
                                    .work
                                    .push(Task::Bind(name, Expr::call(B::SEQUENCE, items.clone())));
                                state.work.push(Task::Sequence(p.args()[1].clone(), items));
                            } else {
                                matched = false;
                            }
                        } else if p.is_head(B::CONDITION) && p.args().len() == 2 {
                            state.work.push(Task::Condition(p.args()[1].clone()));
                            state.work.push(Task::Sequence(p.args()[0].clone(), items));
                        } else {
                            for e in items {
                                ctx.tick()?;
                                if !blank(&p, &e) {
                                    matched = false;
                                    break;
                                }
                            }
                        }
                    }
                }
                if !matched {
                    break;
                }
            }
            if matched {
                return Ok(Some(Candidate {
                    bindings: state.bindings,
                    conditions: state.conditions,
                }));
            }
        }
        Ok(None)
    }
}

fn blank(p: &Expr, e: &Expr) -> bool {
    p.args().is_empty() || (p.args().len() == 1 && e.head() == p.args()[0])
}
fn sequence_min(mut p: &Expr) -> Option<usize> {
    loop {
        match p.head_symbol() {
            Some(B::PATTERN | B::CONDITION) if p.args().len() == 2 => {
                p = &p.args()[if p.is_head(B::PATTERN) { 1 } else { 0 }]
            }
            Some(B::BLANK_SEQUENCE) => return Some(1),
            Some(B::BLANK_NULL_SEQUENCE) => return Some(0),
            _ => return None,
        }
    }
}

/// Simultaneously substitute bound symbols, retaining raw held subtrees.
pub fn substitute(expr: &Expr, bindings: &Bindings) -> Expr {
    enum Walk<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut work = vec![Walk::Enter(expr)];
    let mut values = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            Walk::Enter(e) => {
                if let Some(value) = e.as_symbol().and_then(|s| bindings.get(&s)) {
                    values.push(value.clone());
                } else if let ExprKind::Normal(n) = e.kind() {
                    work.push(Walk::Build(n));
                    work.extend(n.args.iter().rev().map(Walk::Enter));
                    work.push(Walk::Enter(&n.head));
                } else {
                    values.push(e.clone());
                }
            }
            Walk::Build(n) => {
                let start = values
                    .len()
                    .checked_sub(n.args.len())
                    .expect("invariant: substituted arguments produced values");
                let args = values.split_off(start);
                let head = values
                    .pop()
                    .expect("invariant: substituted head produced a value");
                values.push(Expr::normal(head, args));
            }
        }
    }
    values
        .pop()
        .expect("invariant: substitution produced one root value")
}

pub(crate) fn definition_head(mut lhs: &Expr) -> Option<Symbol> {
    while lhs.is_head(B::CONDITION) && lhs.args().len() == 2 {
        lhs = &lhs.args()[0];
    }
    lhs.as_symbol().or_else(|| lhs.head_symbol())
}
pub(crate) fn prepare_target(
    ev: &mut Evaluator,
    lhs: &Expr,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    enum Walk<'a> {
        Enter(&'a Expr, bool, u32),
        Build(Expr, usize),
    }
    let mut work = vec![Walk::Enter(lhs, false, ev.depth + 1)];
    let mut values = Vec::new();
    while let Some(task) = work.pop() {
        ctx.tick()?;
        match task {
            Walk::Enter(e, argument, depth) => {
                if depth > ev.settings.recursion_limit {
                    return Err(EvalError::Recursion(ev.settings.recursion_limit));
                }
                if argument {
                    let mut scan = vec![e];
                    let mut patterned = false;
                    while let Some(node) = scan.pop() {
                        ctx.tick()?;
                        if matches!(
                            node.head_symbol(),
                            Some(
                                B::PATTERN
                                    | B::BLANK
                                    | B::BLANK_SEQUENCE
                                    | B::BLANK_NULL_SEQUENCE
                                    | B::CONDITION
                            )
                        ) {
                            patterned = true;
                            break;
                        }
                        scan.extend(node.args());
                    }
                    if !patterned {
                        values.push(ev.evaluate(e, ctx)?);
                        continue;
                    }
                    if matches!(
                        e.head_symbol(),
                        Some(
                            B::PATTERN
                                | B::BLANK
                                | B::BLANK_SEQUENCE
                                | B::BLANK_NULL_SEQUENCE
                                | B::CONDITION
                        )
                    ) {
                        values.push(e.clone());
                        continue;
                    }
                }
                if e.as_symbol().is_some() {
                    values.push(e.clone());
                } else if e.is_head(B::CONDITION) && e.args().len() == 2 {
                    work.push(Walk::Build(Expr::sym(B::CONDITION), 2));
                    // Build expects original argument order; predicate scope is held.
                    values.push(e.args()[1].clone());
                    work.push(Walk::Enter(&e.args()[0], false, depth + 1));
                } else if let ExprKind::Normal(n) = e.kind() {
                    let head = ev.evaluate(&n.head, ctx)?;
                    work.push(Walk::Build(head, n.args.len()));
                    work.extend(
                        n.args
                            .iter()
                            .rev()
                            .map(|arg| Walk::Enter(arg, true, depth + 1)),
                    );
                } else {
                    values.push(e.clone());
                }
            }
            Walk::Build(head, count) => {
                let start = values
                    .len()
                    .checked_sub(count)
                    .expect("invariant: prepared arguments produced values");
                let mut args = values.split_off(start);
                if head.as_symbol() == Some(B::CONDITION) && count == 2 {
                    args.swap(0, 1);
                }
                values.push(Expr::normal(head, args));
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: prepared target produced a root value"))
}
