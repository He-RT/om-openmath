//! Heap iteration frames with scoped OwnValue overrides.

use super::{Dispatch, Frame};
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol};
use std::collections::BTreeMap;

pub(super) struct Iteration {
    head: Symbol,
    body: Expr,
    variable: Option<Symbol>,
    items: Vec<Expr>,
    results: Vec<Expr>,
    next: usize,
    depth: u32,
    waiting: bool,
}
pub(super) struct Logical {
    head: om_core::Symbol,
    args: Vec<Expr>,
    next: usize,
    unknown: Vec<Expr>,
    depth: u32,
}

pub(super) struct Setup {
    state: Dispatch,
    variable: Option<Symbol>,
    raw: Vec<Expr>,
    bounds: Vec<Expr>,
    next: usize,
    waiting: bool,
    repeated: bool,
}
pub(super) struct RuleSyntax {
    head: Symbol,
    rhs: Expr,
    lhs: Option<Expr>,
    depth: u32,
}
impl Evaluator {
    pub(super) fn schedule_iterator(
        &mut self,
        state: Dispatch,
        frames: &mut Vec<Frame>,
        values: &mut Vec<Expr>,
        _: &Interrupt,
    ) -> Result<(), EvalError> {
        let spec = &state.args[1];
        if !spec.is_head(B::LIST) {
            values.push(state.rebuilt);
            return Ok(());
        }
        let (variable, raw, repeated) = match spec.args() {
            [count] => (None, vec![count.clone()], true),
            [var, end] | [var, end, _] | [var, end, _, _] => {
                let Some(variable) = var.as_symbol() else {
                    values.push(state.rebuilt);
                    return Ok(());
                };
                let _ = end;
                (Some(variable), spec.args()[1..].to_vec(), false)
            }
            _ => {
                values.push(state.rebuilt);
                return Ok(());
            }
        };
        frames.push(Frame::IteratorSetup(Setup {
            state,
            variable,
            raw,
            bounds: vec![],
            next: 0,
            waiting: false,
            repeated,
        }));
        Ok(())
    }
    pub(super) fn run_iterator_setup(
        &mut self,
        mut setup: Setup,
        frames: &mut Vec<Frame>,
        values: &mut Vec<Expr>,
        ctx: &Interrupt,
    ) -> Result<(), EvalError> {
        ctx.tick()?;
        if setup.waiting {
            setup.bounds.push(
                values
                    .pop()
                    .expect("invariant: iterator bound produced a value"),
            );
        }
        if setup.next < setup.raw.len() {
            let expr = setup.raw[setup.next].clone();
            let depth = setup.state.depth + 1;
            setup.next += 1;
            setup.waiting = true;
            frames.push(Frame::IteratorSetup(setup));
            frames.push(Frame::Evaluate {
                expr,
                depth,
                iterations: 0,
            });
            return Ok(());
        }
        let items = match setup.bounds.as_slice() {
            [end] => {
                if !setup.repeated && end.is_head(B::LIST) {
                    Some(end.args().to_vec())
                } else {
                    crate::structure::range_values(&Expr::int(1), end, &Expr::int(1), ctx)?
                }
            }
            [start, end] => crate::structure::range_values(start, end, &Expr::int(1), ctx)?,
            [start, end, step] => crate::structure::range_values(start, end, step, ctx)?,
            _ => unreachable!("invariant: iterator bound count was checked"),
        };
        let Dispatch {
            symbol,
            rebuilt,
            args,
            depth,
            ..
        } = setup.state;
        let Some(items) = items else {
            values.push(rebuilt);
            return Ok(());
        };
        let head = symbol.expect("invariant: iterator dispatch has a symbol head");
        let body = if args.len() > 2 {
            Expr::call(
                head,
                std::iter::once(args[0].clone()).chain(args[2..].iter().cloned()),
            )
        } else {
            args[0].clone()
        };
        frames.push(Frame::Iterator(Iteration {
            head,
            body,
            variable: setup.variable,
            items,
            results: vec![],
            next: 0,
            depth,
            waiting: false,
        }));
        Ok(())
    }
    pub(super) fn run_iterator(
        &mut self,
        mut state: Iteration,
        frames: &mut Vec<Frame>,
        values: &mut Vec<Expr>,
        ctx: &Interrupt,
    ) -> Result<(), EvalError> {
        ctx.tick()?;
        if state.waiting {
            state.results.push(
                values
                    .pop()
                    .expect("invariant: iterator body produced a value"),
            );
            self.scopes
                .pop()
                .expect("invariant: iterator body owns one scope");
        }
        if state.next == state.items.len() {
            let (result, messages) = om_core::with_canonical_messages(|| match state.head.name() {
                "Table" => Expr::call(B::LIST, state.results),
                "Sum" => om_core::add(state.results),
                _ => om_core::mul(state.results),
            });
            for message in messages {
                self.messages.push(message);
            }
            values.push(result);
            return Ok(());
        }
        let mut scope = BTreeMap::new();
        if let Some(variable) = state.variable {
            scope.insert(variable, Some(state.items[state.next].clone()));
        }
        self.scopes.push(scope);
        state.next += 1;
        state.waiting = true;
        let expr = state.body.clone();
        let depth = state.depth + 1;
        frames.push(Frame::Iterator(state));
        frames.push(Frame::Evaluate {
            expr,
            depth,
            iterations: 0,
        });
        Ok(())
    }
    pub(super) fn schedule_logical(
        &mut self,
        head: Symbol,
        args: Vec<Expr>,
        depth: u32,
        frames: &mut Vec<Frame>,
    ) {
        frames.push(Frame::Logical(Logical {
            head,
            args,
            depth,
            next: 0,
            unknown: vec![],
        }));
    }
    pub(super) fn run_logical(
        &mut self,
        mut state: Logical,
        frames: &mut Vec<Frame>,
        values: &mut Vec<Expr>,
    ) -> Result<(), EvalError> {
        let decisive = if state.head == B::AND {
            B::FALSE
        } else {
            B::TRUE
        };
        let identity = if state.head == B::AND {
            B::TRUE
        } else {
            B::FALSE
        };
        if state.next > 0 {
            let value = values
                .pop()
                .expect("invariant: logical argument produced a value");
            if value.as_symbol() == Some(decisive) {
                values.push(Expr::sym(decisive));
                return Ok(());
            }
            if value.as_symbol() != Some(identity) {
                state.unknown.push(value);
            }
        }
        if state.next == state.args.len() {
            values.push(match state.unknown.len() {
                0 => Expr::sym(identity),
                1 => state.unknown.remove(0),
                _ => Expr::call(state.head, state.unknown),
            });
        } else {
            let expr = state.args[state.next].clone();
            let depth = state.depth + 1;
            state.next += 1;
            frames.push(Frame::Logical(state));
            frames.push(Frame::Evaluate {
                expr,
                depth,
                iterations: 0,
            });
        }
        Ok(())
    }
    pub(super) fn schedule_rule_syntax(
        &mut self,
        head: Symbol,
        args: Vec<Expr>,
        depth: u32,
        frames: &mut Vec<Frame>,
    ) {
        let mut scope = BTreeMap::new();
        let mut scan = vec![&args[0]];
        while let Some(node) = scan.pop() {
            if node.is_head(B::PATTERN)
                && let Some(name) = node.args().first().and_then(Expr::as_symbol)
            {
                scope.insert(name, None);
            }
            scan.extend(node.args());
        }
        self.scopes.push(scope);
        frames.push(Frame::RuleSyntax(RuleSyntax {
            head,
            rhs: args[1].clone(),
            lhs: None,
            depth,
        }));
        frames.push(Frame::Evaluate {
            expr: args[0].clone(),
            depth: depth + 1,
            iterations: 0,
        });
    }
    pub(super) fn run_rule_syntax(
        &mut self,
        mut state: RuleSyntax,
        frames: &mut Vec<Frame>,
        values: &mut Vec<Expr>,
    ) -> Result<(), EvalError> {
        let value = values
            .pop()
            .expect("invariant: rule expression produced a value");
        if let Some(lhs) = state.lhs {
            self.scopes
                .pop()
                .expect("invariant: rule syntax owns a scope");
            values.push(Expr::call(state.head, [lhs, value]));
        } else if state.head == B::RULE_DELAYED {
            self.scopes
                .pop()
                .expect("invariant: rule syntax owns a scope");
            values.push(Expr::call(state.head, [value, state.rhs]));
        } else {
            let expr = state.rhs.clone();
            let depth = state.depth + 1;
            state.lhs = Some(value);
            frames.push(Frame::RuleSyntax(state));
            frames.push(Frame::Evaluate {
                expr,
                depth,
                iterations: 0,
            });
        }
        Ok(())
    }
}
