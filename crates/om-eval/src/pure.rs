//! Budgeted, capture-avoiding beta substitution for named and slot pure functions.
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel, Symbol};
use om_num::Number;
use std::collections::{BTreeMap, BTreeSet};
type Bindings = BTreeMap<Symbol, Expr>;
enum Walk {
    Enter(Expr, Bindings, bool, u32),
    Build(usize),
}
pub(crate) fn apply(
    ev: &mut Evaluator,
    function: &Expr,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let (body, bindings) = match function.args() {
        [body] => (body, Bindings::new()),
        [params, body] => {
            let params = if params.is_head(B::LIST) {
                params.args().to_vec()
            } else {
                vec![params.clone()]
            };
            if params.len() != args.len() {
                return Ok(None);
            }
            let mut bindings = Bindings::new();
            for (param, arg) in params.iter().zip(args) {
                let Some(symbol) = param.as_symbol() else {
                    return Ok(None);
                };
                bindings.insert(symbol, arg.clone());
            }
            (body, bindings)
        }
        _ => return Ok(None),
    };
    let mut used = BTreeSet::new();
    let mut pending = vec![function];
    pending.extend(args);
    while let Some(e) = pending.pop() {
        ctx.tick()?;
        if let Some(s) = e.as_symbol() {
            used.insert(s);
        }
        if let ExprKind::Normal(n) = e.kind() {
            pending.push(&n.head);
            pending.extend(n.args.iter());
        }
    }
    let mut fresh = 0u64;
    let mut work = vec![Walk::Enter(body.clone(), bindings, true, 0)];
    let mut values = vec![];
    while let Some(task) = work.pop() {
        ctx.tick()?;
        match task {
            Walk::Enter(e, mut scope, slots, depth) => {
                if depth > ev.settings.recursion_limit {
                    return Err(EvalError::Recursion(ev.settings.recursion_limit));
                }
                if let Some(value) = e.as_symbol().and_then(|s| scope.get(&s)) {
                    values.push(value.clone());
                    continue;
                }
                if e.is_head(B::FUNCTION) {
                    match e.args() {
                        [inner] => {
                            values.push(Expr::sym(B::FUNCTION));
                            work.push(Walk::Build(1));
                            work.push(Walk::Enter(inner.clone(), scope, false, depth + 1));
                            continue;
                        }
                        [params, inner] => {
                            let params = if params.is_head(B::LIST) {
                                params.args().to_vec()
                            } else {
                                vec![params.clone()]
                            };
                            let Some(names) = params
                                .iter()
                                .map(Expr::as_symbol)
                                .collect::<Option<Vec<_>>>()
                            else {
                                values.push(e);
                                continue;
                            };
                            for name in &names {
                                scope.remove(name);
                            }
                            // Conservative symbol scanning may rename more binders, but never captures an inserted value.
                            let mut free = BTreeSet::new();
                            let mut scan: Vec<_> = scope.values().collect();
                            while let Some(value) = scan.pop() {
                                ctx.tick()?;
                                if let Some(s) = value.as_symbol() {
                                    free.insert(s);
                                }
                                if let ExprKind::Normal(n) = value.kind() {
                                    scan.push(&n.head);
                                    scan.extend(n.args.iter());
                                }
                            }
                            let mut renamed = vec![];
                            for name in names {
                                if free.contains(&name) {
                                    let symbol = loop {
                                        ctx.tick()?;
                                        fresh += 1;
                                        let s = Symbol::intern(&format!("$om$lambda${fresh}"));
                                        if used.insert(s) {
                                            break s;
                                        }
                                    };
                                    scope.insert(name, Expr::sym(symbol));
                                    renamed.push(Expr::sym(symbol));
                                } else {
                                    renamed.push(Expr::sym(name));
                                }
                            }
                            let params = if e.args()[0].is_head(B::LIST) {
                                Expr::call(B::LIST, renamed)
                            } else {
                                renamed.remove(0)
                            };
                            values.push(Expr::sym(B::FUNCTION));
                            values.push(params);
                            work.push(Walk::Build(2));
                            work.push(Walk::Enter(inner.clone(), scope, false, depth + 1));
                            continue;
                        }
                        _ => {
                            values.push(e);
                            continue;
                        }
                    }
                }
                if slots && e.is_head(B::SLOT) && e.args().len() == 1 {
                    let index = match e.args()[0].as_number() {
                        Some(Number::Integer(n)) => usize::try_from(n).ok(),
                        _ => None,
                    };
                    let Some(index) = index else {
                        return Ok(None);
                    };
                    if index == 0 {
                        values.push(function.clone());
                    } else if let Some(arg) = args.get(index - 1) {
                        values.push(arg.clone());
                    } else {
                        ev.message(
                            "Function",
                            "slotn",
                            "The requested pure-function slot has no argument.".into(),
                            MsgLevel::Warning,
                        );
                        return Ok(None);
                    }
                    continue;
                }
                if let ExprKind::Normal(n) = e.kind() {
                    work.push(Walk::Build(n.args.len()));
                    for arg in n.args.iter().rev() {
                        work.push(Walk::Enter(arg.clone(), scope.clone(), slots, depth + 1));
                    }
                    work.push(Walk::Enter(n.head.clone(), scope, slots, depth + 1));
                } else {
                    values.push(e);
                }
            }
            Walk::Build(count) => {
                let start = values
                    .len()
                    .checked_sub(count)
                    .expect("invariant: beta children produced values");
                let args = values.split_off(start);
                let head = values.pop().expect("invariant: beta head produced a value");
                values.push(Expr::normal(head, args));
            }
        }
    }
    Ok(values.pop())
}
