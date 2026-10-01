//! Mathematical action axes exclude function heads and lexical binder names.
use om_core::{BUILTIN as B, Expr, ExprKind, Symbol};
use std::collections::BTreeSet;
fn patterns(e: &Expr, scope: &mut BTreeSet<Symbol>) {
    let mut work = vec![e];
    while let Some(e) = work.pop() {
        if e.is_head(B::PATTERN)
            && e.args().len() == 2
            && let Some(s) = e.args()[0].as_symbol()
        {
            scope.insert(s);
        }
        work.extend(e.args());
    }
}
pub(crate) fn free(expr: &Expr) -> BTreeSet<Symbol> {
    let mut free = BTreeSet::new();
    let mut work = vec![(expr, BTreeSet::new())];
    while let Some((e, scope)) = work.pop() {
        if let Some(s) = e.as_symbol() {
            if !scope.contains(&s)
                && !om_core::builtins::names().contains(&s.name())
                && om_eval::Evaluator::doc(s).is_none()
            {
                free.insert(s);
            }
            continue;
        }
        let ExprKind::Normal(n) = e.kind() else {
            continue;
        };
        let h = n.head.as_symbol();
        let args = &n.args;
        if h == Some(B::FUNCTION) && args.len() >= 2 {
            let mut bound = scope.clone();
            if let Some(s) = args[0].as_symbol() {
                bound.insert(s);
            } else if args[0].is_head(B::LIST) {
                bound.extend(args[0].args().iter().filter_map(Expr::as_symbol));
            }
            work.push((&args[1], bound));
            for arg in &args[2..] {
                work.push((arg, scope.clone()));
            }
            continue;
        }
        if matches!(
            h,
            Some(B::SET | B::SET_DELAYED | B::RULE | B::RULE_DELAYED | B::CONDITION)
        ) && args.len() == 2
        {
            let mut bound = scope.clone();
            patterns(&args[0], &mut bound);
            for arg in args {
                work.push((arg, bound.clone()));
            }
            continue;
        }
        if h == Some(B::PATTERN) && args.len() == 2 {
            work.push((&args[1], scope));
            continue;
        }
        if h.is_some_and(|s| {
            matches!(
                s.name(),
                "Table" | "Sum" | "Product" | "Plot" | "ContourPlot"
            )
        }) && !args.is_empty()
        {
            let mut bound = scope.clone();
            for iterator in &args[1..] {
                if iterator.is_head(B::LIST)
                    && iterator.args().len() >= 2
                    && let Some(s) = iterator.args()[0].as_symbol()
                {
                    for e in &iterator.args()[1..] {
                        work.push((e, bound.clone()));
                    }
                    bound.insert(s);
                } else {
                    work.push((iterator, scope.clone()));
                }
            }
            work.push((&args[0], bound));
            continue;
        }
        if matches!(n.head.kind(), ExprKind::Normal(_)) {
            work.push((&n.head, scope.clone()));
        }
        for arg in args {
            work.push((arg, scope.clone()));
        }
    }
    free
}
