//! Localize FindRoot axes before parameter and starting-value evaluation.
use super::{capture, error, options, outcome, resolve};
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_num::Number;
use om_solve::{FindRootOptions, SolutionSet};
use std::collections::BTreeMap;
fn invalid(s: &str) -> EvalError {
    EvalError::Other(s.into())
}
fn numeric(
    ev: &mut Evaluator,
    e: &Expr,
    precision: om_num::Precision,
    ctx: &Interrupt,
) -> Result<Number, EvalError> {
    let e = ev.evaluate(e, ctx)?;
    if let Some(n) = e.as_number() {
        return Ok(n.clone());
    }
    om_simplify::numeval::approximate(&e, precision, ctx)?
        .ok_or_else(|| invalid("Starting values must be numerical"))
}
pub(super) fn apply(
    ev: &mut Evaluator,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    ev.last_steps = None;
    let split = args[1..]
        .iter()
        .position(options::option)
        .map_or(args.len(), |i| i + 1);
    let mut specs = vec![];
    for s in &args[1..split] {
        if s.is_head(B::LIST) && s.args().first().is_some_and(|e| e.is_head(B::LIST)) {
            specs.extend_from_slice(s.args());
        } else {
            specs.push(s.clone());
        }
    }
    if specs.is_empty() || specs.len() > 64 {
        return Err(invalid("Expected one to 64 starting specifications"));
    }
    let mut scope = BTreeMap::new();
    for s in &specs {
        if !s.is_head(B::LIST) || !(2..=3).contains(&s.args().len()) {
            return Err(invalid(
                "Expected {variable,start} or one real {variable,a,b} bracket",
            ));
        }
        let v = s.args()[0]
            .as_symbol()
            .filter(|s| !om_core::builtins::names().contains(&s.name()))
            .ok_or_else(|| invalid("FindRoot variables must be user symbols"))?;
        if scope.insert(v, None).is_some() {
            return Err(invalid("Starting variables must be distinct"));
        }
    }
    let previous = ev.scopes.len();
    ev.scopes.push(scope);
    let result = (|| {
        let opts = options::parse(ev, "FindRoot", &args[split..], ctx)?;
        let mut bracket = None;
        let mut starts = vec![];
        for s in &specs {
            let a = numeric(ev, &s.args()[1], opts.precision, ctx)?;
            if s.args().len() == 3 {
                if specs.len() != 1 {
                    return Err(invalid("Brent supports one variable"));
                }
                bracket = Some((a.clone(), numeric(ev, &s.args()[2], opts.precision, ctx)?));
            }
            starts.push((s.args()[0].clone(), a));
        }
        if (opts.method == "Brent" && bracket.is_none())
            || (opts.method == "Newton" && bracket.is_some())
        {
            return Err(invalid("Method does not match the starting specifications"));
        }
        let source = resolve::resolve(ev, &args[0], ctx)?;
        let result = om_solve::find_root(
            &source,
            &starts,
            &FindRootOptions {
                precision: opts.precision,
                max_iterations: opts.iterations,
                bracket,
                record_steps: ev.settings.record_steps,
            },
            ctx,
        )
        .map_err(error)?;
        let set = outcome(ev, "FindRoot", result);
        let value = match &set {
            SolutionSet::Finite(roots) if roots.len() == 1 => Ok(Some(Expr::call(
                B::LIST,
                roots[0]
                    .rules
                    .iter()
                    .map(|(v, a)| Expr::call(B::RULE, [v.clone(), a.clone()])),
            ))),
            SolutionSet::Unevaluated => Ok(None),
            _ => Err(invalid("FindRoot did not produce one complete assignment")),
        }?;
        capture(
            ev,
            "FindRoot",
            source,
            starts.iter().map(|(v, _)| v.clone()).collect(),
            set,
            om_solve::Domain::Complexes,
            &value,
        );
        Ok(value)
    })();
    ev.scopes.truncate(previous);
    result
}
