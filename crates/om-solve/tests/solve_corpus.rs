//! Every PLAN §14 authority row executes the public solver, with 200-digit residual checks.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_num::ctx::Interrupt;
use om_parse::{Dialect, parse_expr};
use om_solve::{Domain, SolutionSet, SolveOptions, Verification};
#[path = "../../../tests/corpus/solve_support.rs"]
mod support;
use support::{Case, corpus, e, variables};
fn run(case: &Case) -> Result<(SolutionSet, Expr, Vec<Expr>), String> {
    let call = parse_expr(&case.input, Dialect::Wolfram).map_err(|e| format!("parse {e:?}"))?;
    let vars = variables(&call.args()[1]);
    let ctx = Interrupt::default();
    if call.is_head(B::ELIMINATE) {
        return om_solve::eliminate(&call.args()[0], &vars, &ctx)
            .map(|c| {
                (
                    SolutionSet::Region {
                        cond: c,
                        intervals: vec![],
                    },
                    call.args()[0].clone(),
                    vars,
                )
            })
            .map_err(|e| e.to_string());
    }
    let mut opts = SolveOptions::default();
    for arg in &call.args()[2..] {
        match arg.as_symbol() {
            Some(B::REALS) => opts.domain = Domain::Reals,
            Some(B::INTEGERS) => opts.domain = Domain::Integers,
            Some(B::RATIONALS) => opts.domain = Domain::Rationals,
            _ => {}
        }
        if arg.is_head(B::RULE) && arg.args().len() == 2 {
            let value = arg.args()[1].as_symbol() == Some(B::TRUE);
            match arg.args()[0].as_symbol() {
                Some(B::CUBICS) => opts.cubics = value,
                Some(B::QUARTICS) => opts.quartics = value,
                _ => {}
            }
        }
    }
    let result = if call.is_head(B::REDUCE) {
        om_solve::reduce(&call.args()[0], &vars, opts.domain, &ctx)
    } else {
        om_solve::solve(&call.args()[0], &vars, &opts, &ctx)
    };
    result
        .map(|r| (r.set, call.args()[0].clone(), vars))
        .map_err(|e| e.to_string())
}
fn numeric(case: &Case, set: &SolutionSet, original: &Expr) -> Result<(), String> {
    if let SolutionSet::Finite(roots) = set
        && roots
            .iter()
            .any(|r| r.verification == Verification::Unverified)
    {
        return Err("unverified authority root".into());
    }
    support::numeric(case, set, original)
}
#[test]
fn all_fifty_three_authority_rows_execute_and_validate() {
    let cases = corpus();
    assert_eq!(cases.len(), 53);
    let mut errors = vec![];
    for case in cases {
        let result = (|| {
            let (set, original, _vars) = run(&case)?;
            if case.check == "=" {
                let expected = e(case
                    .expected
                    .as_ref()
                    .ok_or("missing exact authority form")?);
                let actual = canonicalize(&set.to_expr());
                let expected = om_format::input_form(&expected);
                let actual = om_format::input_form(&actual);
                if actual != expected {
                    return Err(format!("exact form {actual}; expected {expected}"));
                }
            } else {
                numeric(&case, &set, &original)?;
            }
            Ok(())
        })();
        if let Err(reason) = result {
            errors.push(format!(
                "#{} {}: {reason}; authority {} {}",
                case.id, case.input, case.authority, case.notes
            ));
        }
    }
    assert!(
        errors.is_empty(),
        "{} corpus failures:\n{}",
        errors.len(),
        errors.join("\n")
    );
}
