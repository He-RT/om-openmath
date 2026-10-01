//! Static notebook dependencies include user heads and preserve lexical scopes.
use om_core::{BUILTIN as B, Expr, ExprKind, Symbol};
use om_parse::ParseOutput;
use std::collections::BTreeSet;

type Symbols = BTreeSet<Symbol>;

pub(crate) fn analyze(parsed: &ParseOutput) -> (Symbols, Symbols) {
    let mut defines = Symbols::new();
    let mut uses = Symbols::new();
    for statement in &parsed.statements {
        walk(&statement.expr, &Symbols::new(), &mut defines, &mut uses);
    }
    uses.retain(|s| !defines.contains(s));
    (defines, uses)
}

fn builtin(s: Symbol) -> bool {
    om_core::builtins::names().contains(&s.name()) || om_eval::Evaluator::doc(s).is_some()
}

fn patterns(e: &Expr, locals: &mut Symbols) {
    let mut work = vec![e];
    while let Some(e) = work.pop() {
        if e.is_head(B::PATTERN)
            && e.args().len() == 2
            && let Some(s) = e.args()[0].as_symbol()
        {
            locals.insert(s);
        }
        if let ExprKind::Normal(n) = e.kind() {
            work.push(&n.head);
            work.extend(n.args.iter());
        }
    }
}

fn target(mut e: &Expr) -> Option<Symbol> {
    while e.is_head(B::CONDITION) && e.args().len() == 2 {
        e = &e.args()[0];
    }
    e.as_symbol().or_else(|| e.head_symbol())
}

fn walk(e: &Expr, scope: &Symbols, defines: &mut Symbols, uses: &mut Symbols) {
    if let Some(s) = e.as_symbol() {
        if !scope.contains(&s) && !builtin(s) {
            uses.insert(s);
        }
        return;
    }
    let ExprKind::Normal(n) = e.kind() else {
        return;
    };
    let head = n.head.as_symbol();
    let args = &n.args;
    if matches!(
        head,
        Some(B::SET | B::SET_DELAYED | B::RULE | B::RULE_DELAYED)
    ) && args.len() == 2
    {
        let mut locals = scope.clone();
        patterns(&args[0], &mut locals);
        if matches!(head, Some(B::SET | B::SET_DELAYED))
            && let Some(s) = target(&args[0])
            && !scope.contains(&s)
            && !builtin(s)
        {
            defines.insert(s);
        }
        walk(&args[0], &locals, defines, uses);
        walk(&args[1], &locals, defines, uses);
        return;
    }
    if head == Some(B::CONDITION) && args.len() == 2 {
        let mut locals = scope.clone();
        patterns(&args[0], &mut locals);
        for arg in args {
            walk(arg, &locals, defines, uses);
        }
        return;
    }
    if head == Some(B::PATTERN) && args.len() == 2 {
        walk(&args[1], scope, defines, uses);
        return;
    }
    if head == Some(B::FUNCTION) && args.len() >= 2 {
        let mut locals = scope.clone();
        let params = &args[0];
        if let Some(s) = params.as_symbol() {
            locals.insert(s);
        } else if params.is_head(B::LIST) {
            locals.extend(params.args().iter().filter_map(Expr::as_symbol));
        }
        walk(&args[1], &locals, defines, uses);
        for arg in &args[2..] {
            walk(arg, scope, defines, uses);
        }
        return;
    }
    if matches!(head, Some(B::PLOT | B::CONTOUR_PLOT)) && !args.is_empty() {
        let mut locals = scope.clone();
        let count = if head == Some(B::PLOT) { 1 } else { 2 };
        for iterator in args[1..].iter().take(count) {
            if iterator.is_head(B::LIST)
                && iterator.args().len() == 3
                && let Some(s) = iterator.args()[0].as_symbol()
            {
                locals.insert(s);
                for bound in &iterator.args()[1..] {
                    walk(bound, scope, defines, uses);
                }
            } else {
                walk(iterator, scope, defines, uses);
            }
        }
        for option in args[1..].iter().skip(count) {
            if option.is_head(B::RULE)
                && option.args().len() == 2
                && option.args()[0]
                    .as_symbol()
                    .is_some_and(|s| s.name() == "PlotRange")
            {
                walk(&option.args()[1], scope, defines, uses);
            } else {
                walk(option, scope, defines, uses);
            }
        }
        walk(&args[0], &locals, defines, uses);
        return;
    }
    if head.is_some_and(|s| matches!(s.name(), "Table" | "Sum" | "Product")) && !args.is_empty() {
        let mut locals = scope.clone();
        for iterator in &args[1..] {
            if iterator.is_head(B::LIST)
                && iterator.args().len() >= 2
                && let Some(s) = iterator.args()[0].as_symbol()
            {
                for bound in &iterator.args()[1..] {
                    walk(bound, &locals, defines, uses);
                }
                locals.insert(s);
            } else {
                walk(iterator, &locals, defines, uses);
            }
        }
        walk(&args[0], &locals, defines, uses);
        return;
    }
    walk(&n.head, scope, defines, uses);
    for arg in args {
        walk(arg, scope, defines, uses);
    }
}
