use super::*;
use crate::{Level, Step, Steps};
use kinds::KindData;
use std::collections::BTreeSet;
/// Opaque flat step data. Deserializing its metadata is not proof of validation or computation.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepsData {
    version: u32,
    nodes: Vec<StepData>,
    roots: Vec<u32>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StepData {
    id: String,
    rule: String,
    kind: KindData,
    before: Vec<u32>,
    after: Vec<u32>,
    major: bool,
    children: Vec<u32>,
}
/// Capture exact event kind/data/ordering with one shared expression pool.
pub fn encode_steps(
    steps: &Steps,
    expressions: &mut Vec<Expr>,
    limits: EvidenceLimits,
    ctx: &Interrupt,
) -> Result<StepsData, EvidenceError> {
    if steps.root.len() > limits.max_steps {
        return Err(EvidenceError::Limit);
    }
    let mut nodes = Vec::new();
    let mut results = Vec::new();
    let mut work = Vec::new();
    let mut seen = BTreeSet::new();
    let mut visits = 0usize;
    for step in steps.root.iter().rev() {
        work.push((step, false, 1usize));
    }
    while let Some((step, exit, depth)) = work.pop() {
        ctx.tick()?;
        if depth > limits.max_depth || nodes.len() >= limits.max_steps {
            return Err(EvidenceError::Limit);
        }
        if !exit {
            visits = visits
                .checked_add(1)
                .filter(|&n| n <= limits.max_steps)
                .ok_or(EvidenceError::Limit)?;
            if step.children.len() > limits.max_steps - visits {
                return Err(EvidenceError::Limit);
            }
            work.push((step, true, depth));
            for child in step.children.iter().rev() {
                work.push((child, false, depth + 1));
            }
            continue;
        }
        texts(&step.id, limits)?;
        if step.id.is_empty()
            || !seen.insert(step.id.as_str())
            || step.rule_id != step.kind.rule_id()
        {
            return Err(EvidenceError::Invalid);
        }
        let at = results
            .len()
            .checked_sub(step.children.len())
            .ok_or(EvidenceError::Invalid)?;
        let children = results.split_off(at);
        let node = u32::try_from(nodes.len()).map_err(|_| EvidenceError::Limit)?;
        nodes.push(StepData {
            id: step.id.clone(),
            rule: step.rule_id.into(),
            kind: KindData::encode(&step.kind, expressions, limits, ctx)?,
            before: put_many(expressions, &step.before, limits, ctx)?,
            after: put_many(expressions, &step.after, limits, ctx)?,
            major: step.level == Level::Major,
            children,
        });
        results.push(node);
    }
    Ok(StepsData {
        version: 1,
        nodes,
        roots: results,
    })
}
/// Restore real event data only; no solver runs and no localized LaTeX is used as source.
pub fn decode_steps(
    data: StepsData,
    expressions: &[Expr],
    limits: EvidenceLimits,
    ctx: &Interrupt,
) -> Result<Steps, EvidenceError> {
    if data.version != 1 {
        return Err(EvidenceError::Invalid);
    }
    if data.nodes.len() > limits.max_steps || data.roots.len() > limits.max_steps {
        return Err(EvidenceError::Limit);
    }
    let mut values: Vec<Option<Step>> = Vec::new();
    let mut heights = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, node) in data.nodes.into_iter().enumerate() {
        ctx.tick()?;
        texts(&node.id, limits)?;
        if node.id.is_empty() || !seen.insert(node.id.clone()) {
            return Err(EvidenceError::Invalid);
        }
        let kind = node.kind.decode(expressions, limits, ctx)?;
        if node.rule != kind.rule_id() {
            return Err(EvidenceError::Invalid);
        }
        let mut children = Vec::new();
        let mut height = 1usize;
        for id in node.children {
            let id = id as usize;
            if id >= index {
                return Err(EvidenceError::Invalid);
            }
            height = height.max(heights[id] + 1);
            children.push(values[id].take().ok_or(EvidenceError::Invalid)?);
        }
        if height > limits.max_depth {
            return Err(EvidenceError::Limit);
        }
        heights.push(height);
        values.push(Some(Step {
            id: node.id,
            rule_id: kind.rule_id(),
            kind,
            before: get_many(expressions, &node.before, ctx)?,
            after: get_many(expressions, &node.after, ctx)?,
            level: if node.major {
                Level::Major
            } else {
                Level::Minor
            },
            children,
        }));
    }
    let mut root = Vec::new();
    for id in data.roots {
        ctx.tick()?;
        root.push(
            values
                .get_mut(id as usize)
                .ok_or(EvidenceError::Invalid)?
                .take()
                .ok_or(EvidenceError::Invalid)?,
        );
    }
    if values.iter().any(Option::is_some) {
        return Err(EvidenceError::Invalid);
    }
    validate_ids(&root, limits, ctx)?;
    Ok(Steps { root })
}
fn validate_ids(
    steps: &[Step],
    limits: EvidenceLimits,
    ctx: &Interrupt,
) -> Result<(), EvidenceError> {
    let mut work = steps
        .iter()
        .enumerate()
        .map(|(n, s)| (s, format!("S{}", n + 1), 1usize))
        .collect::<Vec<_>>();
    while let Some((step, expected, depth)) = work.pop() {
        ctx.tick()?;
        if depth > limits.max_depth {
            return Err(EvidenceError::Limit);
        }
        if step.id != expected {
            return Err(EvidenceError::Invalid);
        }
        for (n, child) in step.children.iter().enumerate() {
            work.push((child, format!("{}.{}", expected, n + 1), depth + 1));
        }
    }
    Ok(())
}
