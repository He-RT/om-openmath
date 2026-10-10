use super::{CheckpointError, CheckpointLimits, wire::*};
use crate::notebook::StatementRecord;
use om_core::{Expr, Interrupt};
use om_solve::{
    Domain,
    checkpoint::{decode_solutions, decode_steps, encode_solutions, encode_steps},
};
pub(crate) fn push(
    roots: &mut Vec<Expr>,
    expr: &Expr,
    limits: CheckpointLimits,
    ctx: &Interrupt,
) -> Result<u32, CheckpointError> {
    ctx.tick()?;
    if roots.len() >= limits.expressions.max_roots {
        return Err(CheckpointError::Limit);
    }
    let id = u32::try_from(roots.len()).map_err(|_| CheckpointError::Limit)?;
    roots.push(expr.clone());
    Ok(id)
}
pub(crate) fn expr(roots: &[Expr], id: u32, ctx: &Interrupt) -> Result<Expr, CheckpointError> {
    ctx.tick()?;
    roots
        .get(id as usize)
        .cloned()
        .ok_or(CheckpointError::Invalid)
}
fn domain(d: Domain) -> u8 {
    match d {
        Domain::Complexes => 0,
        Domain::Reals => 1,
        Domain::Integers => 2,
        Domain::Rationals => 3,
    }
}
fn read_domain(d: u8) -> Result<Domain, CheckpointError> {
    Ok(match d {
        0 => Domain::Complexes,
        1 => Domain::Reals,
        2 => Domain::Integers,
        3 => Domain::Rationals,
        _ => return Err(CheckpointError::Invalid),
    })
}
pub(crate) fn encode_record(
    record: &StatementRecord,
    roots: &mut Vec<Expr>,
    contexts: &mut Vec<ContextData>,
    context_bytes: &mut Vec<u8>,
    build: &str,
    limits: CheckpointLimits,
    ctx: &Interrupt,
) -> Result<RecordData, CheckpointError> {
    if record.out_index == 0 || record.view_id.is_empty() || record.view_id.len() > 256 {
        return Err(CheckpointError::Invalid);
    }
    let input = push(roots, &record.input, limits, ctx)?;
    let value = push(roots, &record.value, limits, ctx)?;
    let mut evidence = limits.evidence;
    evidence.max_expressions = evidence.max_expressions.min(limits.expressions.max_roots);
    let steps = record
        .steps
        .as_ref()
        .map(|s| encode_steps(s, roots, evidence, ctx))
        .transpose()?;
    let solver = record
        .solver
        .as_ref()
        .map(|s| {
            Ok::<_, CheckpointError>(SolverData {
                name: s.name.name().into(),
                source: push(roots, &s.source, limits, ctx)?,
                variables: s
                    .vars
                    .iter()
                    .map(|e| push(roots, e, limits, ctx))
                    .collect::<Result<_, _>>()?,
                value: push(roots, &s.value, limits, ctx)?,
                set: encode_solutions(&s.set, roots, evidence, ctx)?,
                domain: domain(s.domain),
            })
        })
        .transpose()?;
    let scientific = record
        .scientific
        .as_ref()
        .map(|s| {
            Ok::<_, CheckpointError>(ScientificData {
                name: s.name.into(),
                value: push(roots, &s.value, limits, ctx)?,
            })
        })
        .transpose()?;
    let exploration = if let Some(snapshot) = &record.exploration {
        if snapshot.plan.controls.is_empty() || snapshot.plan.controls.len() > 16 {
            return Err(CheckpointError::Invalid);
        }
        let mut evaluator_limits = limits.evaluator;
        evaluator_limits.max_bytes = evaluator_limits.max_bytes.min(limits.max_context_bytes);
        let bytes = snapshot
            .eval
            .encode_readonly_persistent(build, evaluator_limits, ctx)?;
        let offset = u32::try_from(context_bytes.len()).map_err(|_| CheckpointError::Limit)?;
        let length = u32::try_from(bytes.len()).map_err(|_| CheckpointError::Limit)?;
        if context_bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > limits.max_bytes)
        {
            return Err(CheckpointError::Limit);
        }
        context_bytes.extend(bytes);
        let controls = snapshot
            .plan
            .controls
            .iter()
            .map(|c| {
                if !c.range.0.is_finite()
                    || !c.range.1.is_finite()
                    || !c.initial.is_finite()
                    || c.range.0 >= c.range.1
                    || c.initial < c.range.0
                    || c.initial > c.range.1
                {
                    return Err(CheckpointError::Invalid);
                }
                Ok(ControlData {
                    name: c.name.clone(),
                    lower: c.range.0.to_bits(),
                    upper: c.range.1.to_bits(),
                    initial: c.initial.to_bits(),
                })
            })
            .collect::<Result<_, _>>()?;
        let id = u32::try_from(contexts.len()).map_err(|_| CheckpointError::Limit)?;
        contexts.push(ContextData {
            expression: push(roots, &snapshot.plan.expression, limits, ctx)?,
            controls,
            offset,
            length,
        });
        Some(id)
    } else {
        None
    };
    Ok(RecordData {
        input,
        value,
        steps,
        suppressed: record.suppress_output,
        out_index: record.out_index,
        solver,
        scientific,
        view_id: record.view_id.clone(),
        exploration,
    })
}
pub(crate) fn decode_record(
    data: RecordData,
    roots: &[Expr],
    contexts: &mut [Option<crate::explore::Snapshot>],
    limits: CheckpointLimits,
    ctx: &Interrupt,
) -> Result<StatementRecord, CheckpointError> {
    if data.out_index == 0 || data.view_id.is_empty() || data.view_id.len() > 256 {
        return Err(CheckpointError::Invalid);
    }
    let input = expr(roots, data.input, ctx)?;
    let value = expr(roots, data.value, ctx)?;
    let steps = data
        .steps
        .map(|s| decode_steps(s, roots, limits.evidence, ctx))
        .transpose()?;
    let solver = data
        .solver
        .map(|s| {
            if !matches!(
                s.name.as_str(),
                "Solve"
                    | "NSolve"
                    | "FindRoot"
                    | "Reduce"
                    | "Eliminate"
                    | "SolveValues"
                    | "NSolveValues"
                    | "Roots"
            ) {
                return Err(CheckpointError::Invalid);
            }
            Ok::<_, CheckpointError>(om_eval::SolverResult {
                name: om_core::Symbol::intern(&s.name),
                source: expr(roots, s.source, ctx)?,
                vars: s
                    .variables
                    .into_iter()
                    .map(|id| expr(roots, id, ctx))
                    .collect::<Result<_, _>>()?,
                value: expr(roots, s.value, ctx)?,
                set: decode_solutions(s.set, roots, limits.evidence, ctx)?,
                domain: read_domain(s.domain)?,
            })
        })
        .transpose()?;
    let scientific = data
        .scientific
        .map(|s| {
            if !matches!(
                s.name.as_str(),
                "Integrate"
                    | "NIntegrate"
                    | "Ode"
                    | "Optimize"
                    | "Fit"
                    | "Lu"
                    | "Qr"
                    | "Svd"
                    | "Cholesky"
                    | "LeastSquares"
                    | "Eigensystem"
                    | "LinearSolve"
            ) {
                return Err(CheckpointError::Invalid);
            }
            let name = om_eval::Evaluator::all_specs()
                .find(|entry| entry.symbol.name() == s.name)
                .ok_or(CheckpointError::Invalid)?
                .symbol
                .name();
            Ok::<_, CheckpointError>(om_eval::ScientificResult {
                name,
                value: expr(roots, s.value, ctx)?,
            })
        })
        .transpose()?;
    for evidence in [
        solver.as_ref().map(|s| &s.value),
        scientific.as_ref().map(|s| &s.value),
    ]
    .into_iter()
    .flatten()
    {
        if !om_core::checkpoint::same_representation(&value, evidence, limits.expressions, ctx)? {
            return Err(CheckpointError::Invalid);
        }
    }
    let exploration = data
        .exploration
        .map(|id| {
            contexts
                .get_mut(id as usize)
                .ok_or(CheckpointError::Invalid)?
                .take()
                .ok_or(CheckpointError::Invalid)
        })
        .transpose()?;
    Ok(StatementRecord {
        input,
        value,
        steps,
        suppress_output: data.suppressed,
        out_index: data.out_index,
        solver,
        scientific,
        view_id: data.view_id,
        exploration,
    })
}
