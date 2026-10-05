//! Structural operations preserve explicit heads and raw replacement values.

use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, MsgLevel, Symbol};
use om_num::{Integer, Number, Rational};

pub(crate) fn dispatch(
    ev: &mut Evaluator,
    head: Symbol,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    ctx.tick()?;
    if args.iter().any(|e| e.is_head(B::DATA_TABLE))
        && matches!(
            head.name(),
            "Length" | "First" | "Last" | "Rest" | "Append" | "Map"
        )
    {
        return crate::science::table_structure(ev, head.name(), args, ctx);
    }
    Ok(match head.name() {
        "List" | "Function" | "Slot" | "Rule" | "Element" => None,
        "Part" => part(ev, args, ctx)?,
        "Length" => Some(Expr::integer(Integer::from(args[0].args().len()))),
        "First" => args[0].args().first().cloned(),
        "Last" => args[0].args().last().cloned(),
        "Rest" => {
            if let ExprKind::Normal(n) = args[0].kind() {
                (!n.args.is_empty())
                    .then(|| Expr::normal(n.head.clone(), n.args[1..].iter().cloned()))
            } else {
                None
            }
        }
        "Append" => {
            if let ExprKind::Normal(n) = args[0].kind() {
                Some(Expr::normal(
                    n.head.clone(),
                    n.args
                        .iter()
                        .cloned()
                        .chain(std::iter::once(args[1].clone())),
                ))
            } else {
                None
            }
        }
        "Map" => {
            if let ExprKind::Normal(n) = args[1].kind() {
                Some(Expr::normal(
                    n.head.clone(),
                    n.args
                        .iter()
                        .map(|e| Expr::normal(args[0].clone(), [e.clone()])),
                ))
            } else {
                Some(args[1].clone())
            }
        }
        "Apply" => {
            if let ExprKind::Normal(n) = args[1].kind() {
                Some(Expr::normal(args[0].clone(), n.args.iter().cloned()))
            } else {
                Some(args[1].clone())
            }
        }
        "Range" => {
            let (start, end, step) = match args {
                [end] => (Expr::int(1), end.clone(), Expr::int(1)),
                [start, end] => (start.clone(), end.clone(), Expr::int(1)),
                [start, end, step] => (start.clone(), end.clone(), step.clone()),
                _ => unreachable!("invariant: Range arity checked"),
            };
            range_values(&start, &end, &step, ctx)?.map(|values| Expr::call(B::LIST, values))
        }
        "ReplaceAll" => replace(ev, &args[0], &args[1], ctx)?,
        "ReplaceRepeated" => {
            let mut value = args[0].clone();
            for i in 0..=ev.settings.iteration_limit {
                ctx.tick()?;
                let Some(next) = replace(ev, &value, &args[1], ctx)? else {
                    return Ok(None);
                };
                if next == value {
                    return Ok(Some(value));
                }
                if i == ev.settings.iteration_limit {
                    ev.message(
                        "ReplaceRepeated",
                        "rrlim",
                        "Repeated replacement exceeded the iteration limit.".into(),
                        MsgLevel::Warning,
                    );
                    return Ok(Some(Expr::call(B::HOLD, [next])));
                }
                value = next;
            }
            unreachable!("invariant: replacement loop returns within its limit")
        }
        // Finite iterators are executed by the heap-frame dispatcher.
        "Table" | "Sum" | "Product" => None,
        _ => None,
    })
}
fn part(ev: &mut Evaluator, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let mut value = args[0].clone();
    for index in &args[1..] {
        let Some(Number::Integer(n)) = index.as_number() else {
            if matches!(index.kind(), ExprKind::String(_)) || index.is_head(B::SPAN) {
                let Some(next) = crate::composition::part(ev, &value, index, ctx)? else {
                    return Ok(None);
                };
                value = next;
                continue;
            }
            return Ok(None);
        };
        if n.is_zero() {
            value = value.head();
            continue;
        }
        let pos = if n < &Integer::ZERO {
            usize::try_from(n.clone().into_parts().1)
                .ok()
                .and_then(|i| value.args().len().checked_sub(i))
        } else {
            usize::try_from(n).ok().and_then(|i| i.checked_sub(1))
        };
        let Some(next) = pos.and_then(|i| value.args().get(i)) else {
            ev.message(
                "Part",
                "partw",
                "The requested part does not exist.".into(),
                MsgLevel::Warning,
            );
            return Ok(None);
        };
        value = next.clone();
    }
    Ok(Some(value))
}
pub(crate) fn range_values(
    start: &Expr,
    end: &Expr,
    step: &Expr,
    ctx: &Interrupt,
) -> Result<Option<Vec<Expr>>, EvalError> {
    let Some((a, b, c)) = start
        .as_number()
        .zip(end.as_number())
        .zip(step.as_number())
        .map(|((a, b), c)| (a, b, c))
    else {
        return Ok(None);
    };
    let Some((aq, bq, cq)) = crate::scalar::rational(a)
        .zip(crate::scalar::rational(b))
        .zip(crate::scalar::rational(c))
        .map(|((a, b), c)| (a, b, c))
    else {
        return Ok(None);
    };
    if cq.is_zero() {
        return Ok(None);
    }
    let intervals = (bq - aq) / cq;
    if intervals < Rational::ZERO {
        return Ok(Some(vec![]));
    }
    let count = intervals.numerator() / Integer::from(intervals.denominator().clone()) + 1;
    let Ok(count) = usize::try_from(count) else {
        return Ok(None);
    };
    if count > 1_000_000 {
        return Ok(None);
    }
    let mut values = Vec::with_capacity(count);
    for i in 0..count {
        ctx.tick()?;
        values.push(Expr::number(a.add(&c.mul(&Number::Integer(i.into())))));
    }
    Ok(Some(values))
}

fn replace(
    ev: &mut Evaluator,
    expr: &Expr,
    rules: &Expr,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let rules = if rules.is_head(B::LIST) {
        rules.args().to_vec()
    } else {
        vec![rules.clone()]
    };
    if rules
        .iter()
        .any(|r| !matches!(r.head_symbol(), Some(B::RULE | B::RULE_DELAYED)) || r.args().len() != 2)
    {
        return Ok(None);
    }
    enum Walk<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut work = vec![Walk::Enter(expr)];
    let mut values = Vec::new();
    while let Some(task) = work.pop() {
        ctx.tick()?;
        match task {
            Walk::Enter(e) => {
                let mut replaced = false;
                for rule in &rules {
                    if let Some(bindings) = crate::pattern::match_rule(&rule.args()[0], e, ev, ctx)?
                    {
                        values.push(crate::pattern::substitute(&rule.args()[1], &bindings));
                        replaced = true;
                        break;
                    }
                }
                if replaced {
                    continue;
                }
                if let ExprKind::Normal(n) = e.kind() {
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
                    .expect("invariant: replaced arguments produced values");
                let args = values.split_off(start);
                let head = values
                    .pop()
                    .expect("invariant: replaced head produced a value");
                values.push(Expr::normal(head, args));
            }
        }
    }
    Ok(values.pop())
}
