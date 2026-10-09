use super::{ExprCodecError, ExprLimits, Writer};
use crate::{Expr, ExprKind, Interrupt, builtins};
use om_num::checkpoint::encode_number;
use std::collections::{BTreeMap, BTreeSet};

/// Encode actual immutable sharing, with raw head order and numeric representation retained.
pub fn encode_expressions(
    roots: &[Expr],
    limits: ExprLimits,
    ctx: &Interrupt,
) -> Result<Vec<u8>, ExprCodecError> {
    ctx.tick()?;
    if roots.len() > limits.max_roots {
        return Err(ExprCodecError::Limit);
    }
    let mut out = Writer {
        bytes: Vec::new(),
        max: limits.max_bytes,
    };
    out.put(b"OMEX\x01")?;
    out.size(roots.len())?;
    out.size(0)?;
    let mut saved: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let mut symbols = BTreeSet::new();
    let mut edges = 0usize;
    for root in roots {
        let mut pending = vec![(root.clone(), false, 1usize)];
        while let Some((expr, exit, depth)) = pending.pop() {
            ctx.tick()?;
            let key = expr.allocation_key();
            if depth > limits.max_depth {
                return Err(ExprCodecError::Limit);
            }
            if let Some((_, height)) = saved.get(&key) {
                if depth
                    .checked_add(*height - 1)
                    .is_none_or(|n| n > limits.max_depth)
                {
                    return Err(ExprCodecError::Limit);
                }
                continue;
            }
            if let ExprKind::Normal(value) = expr.kind()
                && !exit
            {
                edges = edges
                    .checked_add(
                        value
                            .args
                            .len()
                            .checked_add(1)
                            .ok_or(ExprCodecError::Limit)?,
                    )
                    .filter(|&n| n <= limits.max_edges)
                    .ok_or(ExprCodecError::Limit)?;
                pending.push((expr.clone(), true, depth));
                for child in value.args.iter().rev() {
                    pending.push((child.clone(), false, depth + 1));
                }
                pending.push((value.head.clone(), false, depth + 1));
                continue;
            }
            if saved.len() >= limits.max_nodes {
                return Err(ExprCodecError::Limit);
            }
            let mut height = 1;
            match expr.kind() {
                ExprKind::Number(number) => {
                    out.put(&[0])?;
                    out.blob(&encode_number(number, limits.numbers, ctx)?)?;
                }
                ExprKind::Symbol(symbol) => {
                    let name = symbol.name();
                    symbols.insert(name);
                    if symbols.len() > limits.max_symbols || name.len() > limits.max_text_bytes {
                        return Err(ExprCodecError::Limit);
                    }
                    out.put(&[if builtins::names().contains(&name) {
                        1
                    } else {
                        2
                    }])?;
                    out.blob(name.as_bytes())?;
                }
                ExprKind::String(text) => {
                    if text.len() > limits.max_text_bytes {
                        return Err(ExprCodecError::Limit);
                    }
                    out.put(&[3])?;
                    out.blob(text.as_bytes())?;
                }
                ExprKind::Normal(value) => {
                    let reference = |child: &Expr| {
                        saved
                            .get(&child.allocation_key())
                            .copied()
                            .ok_or(ExprCodecError::Invalid)
                    };
                    out.put(&[4])?;
                    let (head, h) = reference(&value.head)?;
                    height = h + 1;
                    out.size(head)?;
                    out.size(value.args.len())?;
                    for child in &value.args {
                        let (id, h) = reference(child)?;
                        height = height.max(h + 1);
                        out.size(id)?;
                    }
                }
            }
            if height > limits.max_depth {
                return Err(ExprCodecError::Limit);
            }
            saved.insert(key, (saved.len(), height));
        }
    }
    let count = u32::try_from(saved.len()).map_err(|_| ExprCodecError::Limit)?;
    out.bytes[9..13].copy_from_slice(&count.to_le_bytes());
    for root in roots {
        out.size(
            saved
                .get(&root.allocation_key())
                .ok_or(ExprCodecError::Invalid)?
                .0,
        )?;
    }
    Ok(out.bytes)
}
