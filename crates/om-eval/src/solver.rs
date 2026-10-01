//! Public solver outcomes enter evaluation with their source and evidence intact.
#[path = "solver_local.rs"]
mod local;
#[path = "solver_options.rs"]
mod options;
#[path = "solver_resolve.rs"]
mod resolve;
#[path = "solver_root.rs"]
mod root;
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, Interrupt, MsgLevel, Symbol};
use om_solve::{Domain, Solution, SolutionSet, SolveError, SolveOutcome};
/// Actual successful solver callback data, before display formatting loses evidence.
#[derive(Clone, Debug)]
pub struct SolverResult {
    /// Builtin that produced the result.
    pub name: Symbol,
    /// Resolved original source, preserving raw domain restrictions.
    pub source: Expr,
    /// Actual requested or inferred variable order.
    pub vars: Vec<Expr>,
    /// Complete solution set and its verification evidence.
    pub set: SolutionSet,
    /// Actual returned expression, including value-only or boolean formats.
    pub value: Expr,
    /// Actual selected solving domain.
    pub domain: Domain,
}

pub(super) fn capture(
    ev: &mut Evaluator,
    name: &str,
    source: Expr,
    vars: Vec<Expr>,
    set: SolutionSet,
    domain: Domain,
    value: &Option<Expr>,
) {
    ev.last_solver_result = value
        .as_ref()
        .filter(|_| !matches!(set, SolutionSet::Unevaluated))
        .map(|value| SolverResult {
            name: Symbol::intern(name),
            source,
            vars,
            set,
            value: value.clone(),
            domain,
        });
}
pub(crate) fn terminal(s: Symbol) -> bool {
    matches!(
        s.name(),
        "Solve"
            | "NSolve"
            | "FindRoot"
            | "Reduce"
            | "Eliminate"
            | "SolveValues"
            | "NSolveValues"
            | "Roots"
    )
}
pub(super) fn error(e: SolveError) -> EvalError {
    match e {
        SolveError::Abort(e) => e.into(),
        e => EvalError::Other(e.to_string()),
    }
}
pub(crate) fn prepare_numeric(
    ev: &mut Evaluator,
    e: &Expr,
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    resolve::resolve(ev, e, ctx)
}
pub(crate) fn dispatch(
    ev: &mut Evaluator,
    name: &str,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    ctx.tick()?;
    if terminal(Symbol::intern(name)) {
        ev.last_solver_result = None;
    }
    let result = match name {
        "FindRoot" => local::apply(ev, args, ctx),
        "Root" => root::apply(ev, args, ctx),
        "ConditionalExpression" => conditional(ev, args, ctx),
        _ => solve(ev, name, args, ctx),
    };
    match result {
        Err(EvalError::Other(reason)) => {
            ev.message(name, "nsmet", reason, MsgLevel::Warning);
            Ok(None)
        }
        r => r,
    }
}
fn axes(e: &Expr) -> Vec<Expr> {
    if e.is_head(B::LIST) {
        e.args().to_vec()
    } else {
        vec![e.clone()]
    }
}
pub(super) fn outcome(ev: &mut Evaluator, name: &str, result: SolveOutcome) -> SolutionSet {
    ev.last_steps = result.steps;
    for mut m in result.messages {
        m.symbol = name.into();
        ev.messages.push(m);
    }
    result.set
}
fn solve(
    ev: &mut Evaluator,
    name: &str,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    ev.last_steps = None;
    let explicit = args.get(1).filter(|arg| {
        name == "Eliminate" || (!options::option(arg) && options::domain(arg).is_none())
    });
    let tail = if explicit.is_some() {
        &args[2..]
    } else {
        &args[1..]
    };
    let opts = options::parse(ev, name, tail, ctx)?;
    let source = resolve::resolve(ev, &args[0], ctx)?;
    let (vars, single) = if let Some(v) = explicit {
        let v = ev.evaluate(v, ctx)?;
        let single = !v.is_head(B::LIST);
        (axes(&v), single)
    } else {
        let prepared = om_solve::normalize::normalize(
            &source,
            None,
            opts.solve.domain,
            ctx,
            &mut om_solve::NoSteps,
        )
        .map_err(error)?;
        for mut message in prepared.messages {
            message.symbol = name.into();
            ev.messages.push(message);
        }
        let single = prepared.vars.len() == 1;
        (prepared.vars, single)
    };
    if name == "Eliminate" {
        return om_solve::eliminate(&source, &vars, ctx)
            .map(Some)
            .map_err(error);
    }
    let result = match name {
        "NSolve" | "NSolveValues" => {
            om_solve::nsolve_with_options(&source, &vars, opts.precision, &opts.solve, ctx)
        }
        "Reduce" => om_solve::reduce_with_options(&source, &vars, &opts.solve, ctx),
        _ => om_solve::solve(&source, &vars, &opts.solve, ctx),
    }
    .map_err(error)?;
    let set = outcome(ev, name, result);
    let value = match name {
        "Roots" => boolean(&set, ctx)?,
        "SolveValues" | "NSolveValues" => values(&set, &vars, single, ctx)?,
        _ => {
            if matches!(set, SolutionSet::Unevaluated) {
                None
            } else {
                Some(set.to_expr())
            }
        }
    };
    capture(ev, name, source, vars, set, opts.solve.domain, &value);
    Ok(value)
}
fn domain_expr(d: Domain) -> Expr {
    Expr::sym(match d {
        Domain::Complexes => B::COMPLEXES,
        Domain::Reals => B::REALS,
        Domain::Integers => B::INTEGERS,
        Domain::Rationals => B::RATIONALS,
    })
}
fn condition(s: &Solution) -> Option<Expr> {
    let mut cond = s.condition.iter().cloned().collect::<Vec<_>>();
    cond.extend(
        s.constants
            .iter()
            .map(|(c, d)| Expr::call(B::ELEMENT, [c.clone(), domain_expr(*d)])),
    );
    match cond.len() {
        0 => None,
        1 => cond.pop(),
        _ => Some(Expr::call(B::AND, cond)),
    }
}
fn values(
    set: &SolutionSet,
    vars: &[Expr],
    single: bool,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let row = |rules: &[(Expr, Expr)], cond: Option<Expr>| {
        let mut cells = vars
            .iter()
            .map(|v| {
                rules
                    .iter()
                    .find(|(a, _)| a == v)
                    .map_or_else(|| v.clone(), |(_, b)| b.clone())
            })
            .collect::<Vec<_>>();
        let value = if single && cells.len() == 1 {
            cells.remove(0)
        } else {
            Expr::call(B::LIST, cells)
        };
        if let Some(c) = cond {
            Expr::call(B::CONDITIONAL_EXPRESSION, [value, c])
        } else {
            value
        }
    };
    let rows = match set {
        SolutionSet::All => vec![row(&[], None)],
        SolutionSet::Finite(solutions) => {
            let mut rows = vec![];
            for s in solutions {
                ctx.tick()?;
                for _ in 0..s.multiplicity {
                    ctx.tick()?;
                    rows.push(row(&s.rules, condition(s)));
                }
            }
            rows
        }
        SolutionSet::Unevaluated => return Ok(None),
        SolutionSet::Region { .. } => {
            return Err(EvalError::Other(
                "Solution values require explicit assignments".into(),
            ));
        }
    };
    Ok(Some(Expr::call(B::LIST, rows)))
}
fn boolean(set: &SolutionSet, ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    let cond = match set {
        SolutionSet::All => Expr::sym(B::TRUE),
        SolutionSet::Region { cond, .. } => cond.clone(),
        SolutionSet::Unevaluated => return Ok(None),
        SolutionSet::Finite(solutions) => {
            let mut branches = vec![];
            for s in solutions {
                ctx.tick()?;
                let mut terms = s
                    .rules
                    .iter()
                    .map(|(v, a)| Expr::call(B::EQUAL, [v.clone(), a.clone()]))
                    .collect::<Vec<_>>();
                terms.extend(condition(s));
                let branch = match terms.len() {
                    0 => Expr::sym(B::TRUE),
                    1 => terms.remove(0),
                    _ => Expr::call(B::AND, terms),
                };
                if branch.as_symbol() == Some(B::TRUE) {
                    return Ok(Some(branch));
                }
                if !branches.contains(&branch) {
                    branches.push(branch);
                }
            }
            match branches.len() {
                0 => Expr::sym(B::FALSE),
                1 => branches.remove(0),
                _ => Expr::call(B::OR, branches),
            }
        }
    };
    Ok(Some(cond))
}
fn conditional(
    ev: &mut Evaluator,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    let cond = ev.evaluate(&args[1], ctx)?;
    Ok(match cond.as_symbol() {
        Some(B::TRUE) => Some(ev.evaluate(&args[0], ctx)?),
        Some(B::FALSE) => Some(Expr::symbol("Undefined")),
        _ => Some(Expr::call(
            B::CONDITIONAL_EXPRESSION,
            [ev.evaluate(&args[0], ctx)?, cond],
        )),
    })
}
