//! Heap frames keep logical recursion independent of the native call stack.

use crate::{Attributes as A, EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel, with_canonical_messages};

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
                            if let Some(value) = self.defs.own.get(s) {
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
                    let attrs = head
                        .as_symbol()
                        .map_or(A::default(), |s| self.attributes(s));
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
            evaluated: args,
            depth,
            iterations,
            ..
        } = state;
        self.depth = depth;
        let symbol = head.as_symbol();
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
        let down = symbol
            .and_then(|s| self.defs.down.get(&s))
            .and_then(|rules| rules.iter().find(|r| r.lhs == rebuilt))
            .map(|r| r.rhs.clone());
        if let Some(new) = down {
            frames.push(Frame::Rewrite {
                old: rebuilt,
                new,
                depth,
                iterations,
            });
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
}
