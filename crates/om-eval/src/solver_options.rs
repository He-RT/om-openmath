//! Held option names are checked before invoking any solver.
use crate::{EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_num::{Number, Precision};
use om_solve::{Domain, MaxExtra, SolveOptions, VerifyMode};
pub(super) struct Options {
    pub solve: SolveOptions,
    pub precision: Precision,
    pub iterations: u32,
    pub method: String,
}
fn invalid(s: &str) -> EvalError {
    EvalError::Other(s.into())
}
pub(super) fn domain(e: &Expr) -> Option<Domain> {
    match e.as_symbol()? {
        B::COMPLEXES => Some(Domain::Complexes),
        B::REALS => Some(Domain::Reals),
        B::INTEGERS => Some(Domain::Integers),
        B::RATIONALS => Some(Domain::Rationals),
        _ => None,
    }
}
pub(super) fn option(e: &Expr) -> bool {
    e.is_head(B::RULE)
        || (e.is_head(B::LIST)
            && !e.args().is_empty()
            && e.args().iter().all(|e| e.is_head(B::RULE)))
}
fn boolean(e: &Expr) -> Result<bool, EvalError> {
    match e.as_symbol() {
        Some(B::TRUE) => Ok(true),
        Some(B::FALSE) => Ok(false),
        _ => Err(invalid("Expected True or False")),
    }
}
fn uint(e: &Expr) -> Option<u32> {
    if let Some(Number::Integer(n)) = e.as_number() {
        u32::try_from(n).ok()
    } else {
        None
    }
}
pub(super) fn parse(
    ev: &mut Evaluator,
    name: &str,
    args: &[Expr],
    ctx: &Interrupt,
) -> Result<Options, EvalError> {
    let mut out = Options {
        solve: SolveOptions {
            record_steps: ev.settings.record_steps,
            ..Default::default()
        },
        precision: Precision::Machine,
        iterations: 100,
        method: "Automatic".into(),
    };
    let numeric = matches!(name, "NSolve" | "NSolveValues" | "FindRoot");
    let mut domain_seen = false;
    let mut pending = args.iter().rev().collect::<Vec<_>>();
    while let Some(arg) = pending.pop() {
        ctx.tick()?;
        if arg.is_head(B::LIST) {
            pending.extend(arg.args().iter().rev());
            continue;
        }
        if !arg.is_head(B::RULE) {
            let value = ev.evaluate(arg, ctx)?;
            if domain_seen || name == "FindRoot" {
                return Err(invalid("Unexpected positional argument"));
            }
            out.solve.domain =
                domain(&value).ok_or_else(|| invalid("Unsupported solution domain"))?;
            domain_seen = true;
            continue;
        }
        if arg.args().len() != 2 {
            return Err(invalid("Expected an option rule"));
        }
        let key = arg.args()[0]
            .as_symbol()
            .ok_or_else(|| invalid("Expected an option name"))?
            .name();
        let v = ev.evaluate(&arg.args()[1], ctx)?;
        match key {
            "RecordSteps" => out.solve.record_steps = boolean(&v)?,
            "WorkingPrecision" if numeric => {
                if v.as_symbol()
                    .is_some_and(|s| s.name() == "MachinePrecision")
                {
                    out.precision = Precision::Machine;
                } else {
                    let digits = uint(&v)
                        .ok_or_else(|| invalid("WorkingPrecision expects decimal digits"))?;
                    let bits = (f64::from(digits) * std::f64::consts::LOG2_10).ceil() as u32;
                    if !(16..=8192).contains(&bits) {
                        return Err(invalid("WorkingPrecision requires 5..2466 decimal digits"));
                    }
                    out.precision = Precision::Bits(bits);
                }
            }
            "MaxIterations" if name == "FindRoot" => {
                out.iterations = uint(&v)
                    .filter(|n| *n > 0)
                    .ok_or_else(|| invalid("MaxIterations expects a positive integer"))?;
            }
            "Method" if name == "FindRoot" => {
                out.method = match v.kind() {
                    om_core::ExprKind::String(s) => s.to_string(),
                    _ => v
                        .as_symbol()
                        .ok_or_else(|| invalid("Invalid FindRoot method"))?
                        .name()
                        .into(),
                };
                if !matches!(out.method.as_str(), "Automatic" | "Newton" | "Brent") {
                    return Err(invalid("Supported methods are Automatic, Newton and Brent"));
                }
            }
            "Cubics" if name != "FindRoot" => out.solve.cubics = boolean(&v)?,
            "Quartics" if name != "FindRoot" => out.solve.quartics = boolean(&v)?,
            "VerifySolutions" if name != "FindRoot" => {
                out.solve.verify = match v.as_symbol() {
                    Some(B::TRUE) => VerifyMode::Always,
                    Some(B::FALSE) => VerifyMode::Never,
                    Some(s) if s.name() == "Automatic" => VerifyMode::Auto,
                    _ => return Err(invalid("Invalid VerifySolutions value")),
                }
            }
            "MaxExtraConditions" if name != "FindRoot" => {
                out.solve.max_extra_conditions = if v.as_symbol().is_some_and(|s| s.name() == "All")
                {
                    MaxExtra::All
                } else {
                    match uint(&v).ok_or_else(|| invalid("Invalid MaxExtraConditions value"))? {
                        0 => MaxExtra::Zero,
                        n => MaxExtra::Count(n),
                    }
                }
            }
            "GeneratedParameters" if name != "FindRoot" => {
                let h = v
                    .as_symbol()
                    .ok_or_else(|| invalid("GeneratedParameters expects a symbol head"))?;
                if h != B::C && om_core::builtins::names().contains(&h.name()) {
                    return Err(invalid(
                        "Generated parameter head must be a user symbol or C",
                    ));
                }
                out.solve.generated_parameter = h;
            }
            "InverseFunctions" if name != "FindRoot" => out.solve.inverse_functions = boolean(&v)?,
            _ => return Err(invalid("Unsupported option for this solver")),
        }
    }
    Ok(out)
}
