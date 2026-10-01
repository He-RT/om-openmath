//! Heap frames keep logical recursion independent of the native call stack.

use crate::{Attributes as A, EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel, with_canonical_messages};

#[path = "engine_control.rs"]
mod control;

struct Arguments {
    head: Expr,
    source: Vec<Expr>,
    evaluated: Vec<Expr>,
    attrs: A,
    depth: u32,
    iterations: u32,
    next: usize,
    waiting: bool,
}
struct Thread {
    head: Expr,
    args: Vec<Expr>,
    results: Vec<Expr>,
    len: usize,
    next: usize,
    depth: u32,
    waiting: bool,
}
struct Dispatch {
    symbol: Option<om_core::Symbol>,
    rebuilt: Expr,
    args: Vec<Expr>,
    depth: u32,
    iterations: u32,
}
struct Rules {
    dispatch: Dispatch,
    rules: Vec<crate::Rule>,
    next: usize,
    matcher: Option<crate::pattern::Matcher>,
    rhs: Expr,
    candidate: Option<crate::pattern::Candidate>,
    condition: usize,
    waiting: bool,
}
enum Frame {
    Evaluate {
        expr: Expr,
        depth: u32,
        iterations: u32,
    },
    Head {
        args: Vec<Expr>,
        depth: u32,
        iterations: u32,
    },
    Arguments(Arguments),
    Thread(Thread),
    Rules(Rules),
    Logical(control::Logical),
    Iterator(control::Iteration),
    IteratorSetup(control::Setup),
    RuleSyntax(control::RuleSyntax),
    Sequence {
        args: Vec<Expr>,
        old: Expr,
        next: usize,
        depth: u32,
        iterations: u32,
    },
    Rewrite {
        old: Expr,
        new: Expr,
        depth: u32,
        iterations: u32,
    },
}
impl Evaluator {
    pub(crate) fn run_frames(&mut self, expr: Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
        let mut frames = vec![Frame::Evaluate {
            expr,
            depth: self.depth + 1,
            iterations: 0,
        }];
        let mut values = Vec::new();
        'frames: while let Some(frame) = frames.pop() {
            match frame {
                Frame::Evaluate {
                    expr,
                    depth,
                    iterations,
                } => {
                    ctx.tick()?;
                    if depth > self.settings.recursion_limit {
                        return Err(EvalError::Recursion(self.settings.recursion_limit));
                    }
                    self.depth = depth;
                    match expr.kind() {
                        ExprKind::Number(_) | ExprKind::String(_) => values.push(expr),
                        ExprKind::Symbol(s) => {
                            if let Some(value) = self.own(*s) {
                                frames.push(Frame::Evaluate {
                                    expr: value.clone(),
                                    depth: depth + 1,
                                    iterations,
                                });
                            } else {
                                values.push(om_core::canonicalize(&expr));
                            }
                        }
                        ExprKind::Normal(n) => {
                            frames.push(Frame::Head {
                                args: n.args.to_vec(),
                                depth,
                                iterations,
                            });
                            frames.push(Frame::Evaluate {
                                expr: n.head.clone(),
                                depth: depth + 1,
                                iterations: 0,
                            });
                        }
                    }
                }
                Frame::Head {
                    args,
                    depth,
                    iterations,
                } => {
                    let head = values
                        .pop()
                        .expect("invariant: head evaluation produced one value");
                    let mut attrs = head
                        .as_symbol()
                        .map_or(A::default(), |s| self.attributes(s));
                    if matches!(head.as_symbol(), Some(B::RULE | B::RULE_DELAYED))
                        && args.len() == 2
                    {
                        attrs = attrs | A::HOLD_ALL;
                    }
                    frames.push(Frame::Arguments(Arguments {
                        head,
                        source: args,
                        evaluated: vec![],
                        attrs,
                        depth,
                        iterations,
                        next: 0,
                        waiting: false,
                    }));
                }
                Frame::Arguments(mut state) => {
                    if state.waiting {
                        state.evaluated.push(
                            values
                                .pop()
                                .expect("invariant: argument evaluation produced one value"),
                        );
                        state.waiting = false;
                    }
                    while state.next < state.source.len() {
                        let i = state.next;
                        state.next += 1;
                        if state.attrs.contains(A::HOLD_ALL)
                            || (i == 0 && state.attrs.contains(A::HOLD_FIRST))
                            || (i > 0 && state.attrs.contains(A::HOLD_REST))
                        {
                            state.evaluated.push(state.source[i].clone());
                            continue;
                        }
                        state.waiting = true;
                        let next = Frame::Evaluate {
                            expr: state.source[i].clone(),
                            depth: state.depth + 1,
                            iterations: 0,
                        };
                        frames.push(Frame::Arguments(state));
                        frames.push(next);
                        continue 'frames;
                    }
                    self.finish_arguments(state, &mut frames, &mut values, ctx)?;
                }
                Frame::Sequence {
                    args,
                    old,
                    next,
                    depth,
                    iterations,
                } => {
                    if next > 0 {
                        let value = values
                            .pop()
                            .expect("invariant: compound child produced one value");
                        if next == args.len() {
                            frames.push(Frame::Rewrite {
                                old,
                                new: value,
                                depth,
                                iterations,
                            });
                            continue;
                        }
                    }
                    let child = args[next].clone();
                    frames.push(Frame::Sequence {
                        args,
                        old,
                        next: next + 1,
                        depth,
                        iterations,
                    });
                    frames.push(Frame::Evaluate {
                        expr: child,
                        depth: depth + 1,
                        iterations: 0,
                    });
                }
                Frame::Thread(mut state) => {
                    if state.waiting {
                        state.results.push(
                            values
                                .pop()
                                .expect("invariant: threaded row produced one value"),
                        );
                    }
                    if state.next == state.len {
                        values.push(Expr::call(B::LIST, state.results));
                        continue;
                    }
                    let row = state.args.iter().map(|e| {
                        if e.is_head(B::LIST) {
                            e.args()[state.next].clone()
                        } else {
                            e.clone()
                        }
                    });
                    let expr = Expr::normal(state.head.clone(), row);
                    state.next += 1;
                    state.waiting = true;
                    let depth = state.depth + 1;
                    frames.push(Frame::Thread(state));
                    frames.push(Frame::Evaluate {
                        expr,
                        depth,
                        iterations: 0,
                    });
                }
                Frame::Logical(state) => self.run_logical(state, &mut frames, &mut values)?,
                Frame::RuleSyntax(state) => {
                    self.run_rule_syntax(state, &mut frames, &mut values)?
                }
                Frame::IteratorSetup(state) => {
                    self.run_iterator_setup(state, &mut frames, &mut values, ctx)?
                }
                Frame::Iterator(state) => {
                    self.run_iterator(state, &mut frames, &mut values, ctx)?
                }
                Frame::Rules(state) => self.run_rules(state, &mut frames, &mut values, ctx)?,
                Frame::Rewrite {
                    old,
                    new,
                    depth,
                    iterations,
                } => {
                    if new == old {
                        values.push(old);
                    } else if iterations >= self.settings.iteration_limit {
                        self.message(
                            "$IterationLimit",
                            "itlim",
                            format!(
                                "Iteration limit of {} exceeded.",
                                self.settings.iteration_limit
                            ),
                            MsgLevel::Error,
                        );
                        values.push(Expr::call(B::HOLD, [new]));
                    } else {
                        frames.push(Frame::Evaluate {
                            expr: new,
                            depth,
                            iterations: iterations + 1,
                        });
                    }
                }
            }
        }
        Ok(values
            .pop()
            .expect("invariant: root evaluation produced one value"))
    }
    fn finish_arguments(
        &mut self,
        state: Arguments,
        frames: &mut Vec<Frame>,
        values: &mut Vec<Expr>,
        ctx: &Interrupt,
    ) -> Result<(), EvalError> {
        let Arguments {
            head,
            evaluated: mut args,
            attrs,
            depth,
            iterations,
            ..
        } = state;
        self.depth = depth;
        if !attrs.contains(A::HOLD_ALL) {
            args = args
                .into_iter()
                .flat_map(|e| {
                    if e.is_head(B::SEQUENCE) {
                        e.args().to_vec()
                    } else {
                        vec![e]
                    }
                })
                .collect();
        }
        let symbol = head.as_symbol();
        let pure_head = head.clone();
        let spec = symbol.and_then(|s| self.builtins.get(s));
        if let Some(spec) = spec
            && !spec.arity.accepts(args.len())
        {
            let name = spec.symbol.name();
            let (tag, expected) = match spec.arity {
                crate::Arity::Exactly(n) => (
                    "argx",
                    format!("{n} argument{}", if n == 1 { "" } else { "s" }),
                ),
                crate::Arity::Range(a, b) => ("argrx", format!("between {a} and {b} arguments")),
                crate::Arity::AtLeast(n) => ("argx", format!("at least {n} arguments")),
                crate::Arity::Any => unreachable!("invariant: Any accepts every count"),
            };
            self.message(
                name,
                tag,
                format!(
                    "{name} called with {} argument{}; {expected} are expected.",
                    args.len(),
                    if args.len() == 1 { "" } else { "s" }
                ),
                MsgLevel::Warning,
            );
            values.push(Expr::normal(head, args));
            return Ok(());
        }
        if attrs.contains(A::LISTABLE)
            && let Some(len) = args
                .iter()
                .find(|e| e.is_head(B::LIST))
                .map(|e| e.args().len())
        {
            if args
                .iter()
                .any(|e| e.is_head(B::LIST) && e.args().len() != len)
            {
                self.message(
                    "Thread",
                    "tdlen",
                    "Objects of unequal length cannot be combined.".into(),
                    MsgLevel::Warning,
                );
                values.push(Expr::normal(head, args));
            } else {
                frames.push(Frame::Thread(Thread {
                    head,
                    args,
                    results: Vec::with_capacity(len),
                    len,
                    next: 0,
                    depth,
                    waiting: false,
                }));
            }
            return Ok(());
        }
        let (rebuilt, messages) = with_canonical_messages(|| {
            if let Some(symbol) = symbol {
                om_core::func(symbol, args.clone())
            } else {
                Expr::normal(head, args.clone())
            }
        });
        for message in messages {
            self.messages.push(message);
        }
        if pure_head.is_head(B::FUNCTION) {
            if let Some(new) = crate::structure::apply_function(self, &pure_head, &args) {
                frames.push(Frame::Rewrite {
                    old: rebuilt,
                    new,
                    depth,
                    iterations,
                });
            } else {
                values.push(rebuilt);
            }
            return Ok(());
        }
        let rules = symbol
            .and_then(|s| self.defs.down.get(&s))
            .cloned()
            .unwrap_or_default();
        let dispatch = Dispatch {
            symbol,
            rebuilt,
            args,
            depth,
            iterations,
        };
        if rules.is_empty() {
            self.dispatch(dispatch, frames, values, ctx)?;
        } else {
            frames.push(Frame::Rules(Rules {
                dispatch,
                rules,
                next: 0,
                matcher: None,
                rhs: Expr::sym(B::NULL),
                candidate: None,
                condition: 0,
                waiting: false,
            }));
        }
        Ok(())
    }
    fn dispatch(
        &mut self,
        state: Dispatch,
        frames: &mut Vec<Frame>,
        values: &mut Vec<Expr>,
        ctx: &Interrupt,
    ) -> Result<(), EvalError> {
        let Dispatch {
            symbol,
            rebuilt,
            args,
            depth,
            iterations,
        } = state;
        self.depth = depth;
        let spec = symbol.and_then(|s| self.builtins.get(s));
        if matches!(symbol, Some(B::RULE | B::RULE_DELAYED)) {
            self.schedule_rule_syntax(
                symbol.expect("invariant: rule symbol checked"),
                args,
                depth,
                frames,
            );
        } else if symbol.is_some_and(|s| matches!(s.name(), "Table" | "Sum" | "Product")) {
            self.schedule_iterator(
                Dispatch {
                    symbol,
                    rebuilt,
                    args,
                    depth,
                    iterations,
                },
                frames,
                values,
                ctx,
            )?;
        } else if matches!(symbol, Some(B::AND | B::OR)) {
            self.schedule_logical(
                symbol.expect("invariant: logical head checked"),
                args,
                depth,
                frames,
            );
        } else if symbol == Some(B::COMPOUND_EXPRESSION) {
            frames.push(Frame::Sequence {
                args,
                old: rebuilt,
                next: 0,
                depth,
                iterations,
            });
        } else if let Some(spec) = spec {
            if let Some(new) = (spec.f)(self, &args, ctx)? {
                if crate::solver::terminal(spec.symbol) {
                    values.push(new);
                    return Ok(());
                }
                frames.push(Frame::Rewrite {
                    old: rebuilt,
                    new,
                    depth,
                    iterations,
                });
            } else {
                values.push(rebuilt);
            }
        } else {
            values.push(rebuilt);
        }
        Ok(())
    }
    fn run_rules(
        &mut self,
        mut state: Rules,
        frames: &mut Vec<Frame>,
        values: &mut Vec<Expr>,
        ctx: &Interrupt,
    ) -> Result<(), EvalError> {
        self.depth = state.dispatch.depth;
        if state.waiting {
            let result = values
                .pop()
                .expect("invariant: predicate evaluation produced a value");
            state.waiting = false;
            if result.as_symbol() != Some(B::TRUE) {
                state.candidate = None;
            }
        }
        loop {
            ctx.tick()?;
            if let Some(candidate) = &state.candidate {
                if state.condition < candidate.conditions.len() {
                    let expr = crate::pattern::substitute(
                        &candidate.conditions[state.condition],
                        &candidate.bindings,
                    );
                    let depth = state.dispatch.depth + 1;
                    state.condition += 1;
                    state.waiting = true;
                    frames.push(Frame::Rules(state));
                    frames.push(Frame::Evaluate {
                        expr,
                        depth,
                        iterations: 0,
                    });
                    return Ok(());
                }
                let new = crate::pattern::substitute(&state.rhs, &candidate.bindings);
                frames.push(Frame::Rewrite {
                    old: state.dispatch.rebuilt,
                    new,
                    depth: state.dispatch.depth,
                    iterations: state.dispatch.iterations,
                });
                return Ok(());
            }
            if let Some(matcher) = &mut state.matcher {
                if let Some(candidate) = matcher.next(ctx)? {
                    state.candidate = Some(candidate);
                    state.condition = 0;
                    continue;
                }
                state.matcher = None;
            }
            let Some(rule) = state.rules.get(state.next) else {
                return self.dispatch(state.dispatch, frames, values, ctx);
            };
            state.next += 1;
            let (lhs, rhs) =
                if rule.delayed && rule.rhs.is_head(B::CONDITION) && rule.rhs.args().len() == 2 {
                    (
                        Expr::call(B::CONDITION, [rule.lhs.clone(), rule.rhs.args()[1].clone()]),
                        rule.rhs.args()[0].clone(),
                    )
                } else {
                    (rule.lhs.clone(), rule.rhs.clone())
                };
            state.rhs = rhs;
            state.matcher = crate::pattern::Matcher::new(&lhs, &state.dispatch.rebuilt, self, ctx)?;
        }
    }
}
