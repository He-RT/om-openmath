//! Pure inverse-source merge. Producing a file is not a transaction, a permission or an IO receipt.
use crate::protocol::generated::{
    NativeSourceCell, NativeSourceCommit, NativeSourceFile, NativeSourceSnapshot,
};
use std::collections::BTreeMap;

/// Explicit inverse failure; never discard current source to manufacture a successful undo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndoError {
    /// Invalid stored hashes, versions or document identity.
    InvalidRecord,
    /// Later related content or order cannot be merged safely.
    Conflict,
}
/// Merge one original inverse against current confirmed source. The caller still has to obtain
/// the original durable transaction and apply the result through the native commit port.
pub fn merge_inverse(
    original: &NativeSourceCommit,
    current: &NativeSourceSnapshot,
) -> Result<NativeSourceFile, UndoError> {
    super::validate_commit(original).map_err(|_| UndoError::InvalidRecord)?;
    super::validate_snapshot(current).map_err(|_| UndoError::InvalidRecord)?;
    if original.commit.document_id != current.document_id
        || current.revision < original.after.revision
    {
        return Err(UndoError::InvalidRecord);
    }
    let before = &original.before.file;
    let after = &original.after.file;
    let before_by_id: BTreeMap<_, _> = before.cells.iter().map(|c| (c.id.as_str(), c)).collect();
    let after_by_id: BTreeMap<_, _> = after.cells.iter().map(|c| (c.id.as_str(), c)).collect();
    let current_by_id: BTreeMap<_, _> = current
        .file
        .cells
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect();
    let current_positions: BTreeMap<_, _> = current
        .file
        .cells
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.as_str(), i))
        .collect();
    let original_revisions: BTreeMap<_, _> = original
        .after
        .cell_revisions
        .iter()
        .map(|r| (r.cell_id.as_str(), r.revision))
        .collect();
    let current_revisions: BTreeMap<_, _> = current
        .cell_revisions
        .iter()
        .map(|r| (r.cell_id.as_str(), r.revision))
        .collect();
    let mut restored = BTreeMap::new();
    // Only source actually written by the original operation is inverted. Unchanged neighboring
    // cells retain their current bytes even when an order inverse moves their containing region.
    for (id, old) in &before_by_id {
        match after_by_id.get(id) {
            None => {
                if current_by_id.contains_key(id) {
                    return Err(UndoError::Conflict);
                }
            }
            Some(expected) if !same_cell(old, expected) => {
                if current_by_id
                    .get(id)
                    .is_none_or(|now| !same_cell(now, expected))
                    || original_revisions.get(id) != current_revisions.get(id)
                {
                    return Err(UndoError::Conflict);
                }
                restored.insert(*id, *old);
            }
            _ => {}
        }
    }
    for (id, inserted) in &after_by_id {
        if !before_by_id.contains_key(id)
            && (current_by_id
                .get(id)
                .is_none_or(|now| !same_cell(now, inserted))
                || original_revisions.get(id) != current_revisions.get(id))
        {
            return Err(UndoError::Conflict);
        }
    }
    let mut merged = current.file.clone();
    if before.title != after.title {
        if merged.title != after.title {
            return Err(UndoError::Conflict);
        }
        merged.title = before.title.clone();
    }
    let before_ids: Vec<_> = before.cells.iter().map(|c| c.id.as_str()).collect();
    let after_ids: Vec<_> = after.cells.iter().map(|c| c.id.as_str()).collect();
    if before_ids != after_ids {
        // Unique source identities let us find stable order anchors with an O(n log n) LIS.
        // Validate all changed gaps against the same current source before applying any inverse.
        let mut regions = Vec::new();
        let (mut before_start, mut after_start) = (0, 0);
        let mut left: Option<&str> = None;
        for (next_before, next_after) in stable_anchors(&before_ids, &after_ids)
            .into_iter()
            .chain(std::iter::once((before_ids.len(), after_ids.len())))
        {
            if before_ids[before_start..next_before] != after_ids[after_start..next_after] {
                let start = match left {
                    None => 0,
                    Some(id) => {
                        current_positions
                            .get(id)
                            .copied()
                            .ok_or(UndoError::Conflict)?
                            + 1
                    }
                };
                let end = match before_ids.get(next_before) {
                    None => merged.cells.len(),
                    Some(id) => current_positions
                        .get(id)
                        .copied()
                        .ok_or(UndoError::Conflict)?,
                };
                if start > end
                    || merged.cells[start..end]
                        .iter()
                        .map(|c| c.id.as_str())
                        .ne(after_ids[after_start..next_after].iter().copied())
                {
                    return Err(UndoError::Conflict);
                }
                let replacement = before.cells[before_start..next_before]
                    .iter()
                    .map(|old| {
                        restored
                            .get(old.id.as_str())
                            .copied()
                            .or_else(|| current_by_id.get(old.id.as_str()).copied())
                            .unwrap_or(old)
                            .clone()
                    })
                    .collect::<Vec<_>>();
                regions.push((start..end, replacement));
            }
            left = before_ids.get(next_before).copied();
            before_start = next_before + 1;
            after_start = next_after + 1;
        }
        regions.sort_by_key(|(range, _)| {
            (std::cmp::Reverse(range.start), std::cmp::Reverse(range.end))
        });
        for (range, replacement) in regions {
            merged.cells.splice(range, replacement);
        }
    }
    for cell in &mut merged.cells {
        if let Some(old) = restored.get(cell.id.as_str()) {
            *cell = (*old).clone();
        }
    }
    Ok(merged)
}
fn same_cell(a: &NativeSourceCell, b: &NativeSourceCell) -> bool {
    a.id == b.id && a.source == b.source && a.kind == b.kind && a.dialect == b.dialect
}

// Inputs have unique byte-identical IDs, already verified by the source contract.
fn stable_anchors(before: &[&str], after: &[&str]) -> Vec<(usize, usize)> {
    let positions: BTreeMap<_, _> = before.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut nodes: Vec<(usize, usize, Option<usize>)> = Vec::new();
    let mut tails: Vec<usize> = Vec::new();
    for (j, id) in after.iter().enumerate() {
        let Some(&i) = positions.get(id) else {
            continue;
        };
        let slot = tails.partition_point(|&node| nodes[node].0 < i);
        let previous = slot.checked_sub(1).map(|p| tails[p]);
        let node = nodes.len();
        nodes.push((i, j, previous));
        if slot == tails.len() {
            tails.push(node);
        } else {
            tails[slot] = node;
        }
    }
    let mut anchor = tails.last().copied();
    let mut result = Vec::new();
    while let Some(node) = anchor {
        let (i, j, previous) = nodes[node];
        result.push((i, j));
        anchor = previous;
    }
    result.reverse();
    result
}
