//! Normalize model-facing plan-local selectors into host-assigned actual source operations.
use super::{FrozenPreviewPlan, FrozenReadSnapshot, PreviewError, Value, diagnostic};
use crate::{
    protocol::{Nullable, generated::*},
    references::ReferenceRegistry,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
fn selected(
    selector: &PreviewCellSelector,
    original: &FrozenReadSnapshot,
    assigned: &BTreeMap<String, String>,
) -> Result<String, PreviewError> {
    match selector {
        PreviewCellSelector::PreviewExistingCell(target) => {
            existing(&target.cell_id, original).map(|_| target.cell_id.clone())
        }
        PreviewCellSelector::PreviewNewCell(target) => assigned
            .get(&target.client_key)
            .cloned()
            .ok_or(PreviewError::InvalidPatch),
    }
}
fn existing<'a>(
    id: &str,
    snapshot: &'a FrozenReadSnapshot,
) -> Result<&'a NativeSourceCell, PreviewError> {
    snapshot
        .source
        .file
        .cells
        .iter()
        .find(|c| c.id == id)
        .ok_or(PreviewError::InvalidPatch)
}
fn hash_checked<'a>(
    id: &str,
    expected: &str,
    snapshot: &'a FrozenReadSnapshot,
) -> Result<&'a NativeSourceCell, PreviewError> {
    let cell = existing(id, snapshot)?;
    if format!("{:x}", Sha256::digest(cell.source.as_bytes())) != expected {
        return Err(PreviewError::SourceHashMismatch);
    }
    Ok(cell)
}
fn anchor(
    selector: Option<&PreviewCellSelector>,
    original: &FrozenReadSnapshot,
    assigned: &BTreeMap<String, String>,
) -> Result<Option<String>, PreviewError> {
    selector
        .map(|s| selected(s, original, assigned))
        .transpose()
}
pub(super) fn normalize(
    references: &mut ReferenceRegistry<Value>,
    input: &PreviewPatchInput,
    original: &FrozenReadSnapshot,
) -> Result<(Vec<NativeSourceOperation>, BTreeMap<String, String>), PreviewError> {
    if input.operations.is_empty() || input.operations.len() > 64 {
        return Err(PreviewError::LimitExceeded);
    }
    let mut assigned = BTreeMap::new();
    let mut operations = Vec::new();
    let mut temporary = original.source.file.clone();
    let mut content_writes: BTreeMap<String, bool> = BTreeMap::new();
    let mut deleted = BTreeSet::new();
    for proposed in &input.operations {
        let operation = match proposed {
            PreviewPatchOperation::PreviewInsertCell(body) => {
                if assigned.contains_key(&body.client_key) {
                    return Err(PreviewError::InvalidPatch);
                }
                let id = references.allocate_id("cell")?;
                if original.source.file.cells.iter().any(|c| c.id == id) {
                    return Err(PreviewError::InvalidPatch);
                }
                let after = anchor(body.after.0.as_ref(), original, &assigned)?;
                let cell = NativeSourceCell {
                    id: id.clone(),
                    kind: match body.kind {
                        PreviewInsertCellKind::Math => NativeCellKind::Math,
                        PreviewInsertCellKind::Text => NativeCellKind::Text,
                    },
                    source: body.source.clone(),
                    dialect: body.dialect.clone(),
                };
                assigned.insert(body.client_key.clone(), id);
                NativeSourceOperation::InsertSourceCell(InsertSourceCell {
                    kind: InsertSourceCellKind::InsertCell,
                    cell,
                    after_cell_id: Nullable(after),
                })
            }
            PreviewPatchOperation::PreviewUpdateCell(body) => {
                let id = &body.target.cell_id;
                let prior = hash_checked(id, &body.expected_source_hash, original)?;
                if !original.complete_cell_ids.contains(id) {
                    return Err(PreviewError::IncompleteSource);
                }
                if deleted.contains(id) || content_writes.insert(id.clone(), true).is_some() {
                    return Err(PreviewError::InvalidPatch);
                }
                let mut cell = prior.clone();
                cell.source = body.source.clone();
                if let Some(kind) = &body.kind {
                    cell.kind = match kind {
                        PreviewUpdateCellKind::Math => NativeCellKind::Math,
                        PreviewUpdateCellKind::Text => NativeCellKind::Text,
                    };
                }
                if let Some(dialect) = &body.dialect {
                    cell.dialect = dialect.clone();
                }
                NativeSourceOperation::UpdateSourceCell(UpdateSourceCell {
                    kind: UpdateSourceCellKind::UpdateCell,
                    cell,
                })
            }
            PreviewPatchOperation::PreviewReplaceText(body) => {
                let id = &body.target.cell_id;
                hash_checked(id, &body.expected_source_hash, original)?;
                if deleted.contains(id) || content_writes.get(id) == Some(&true) {
                    return Err(PreviewError::InvalidPatch);
                }
                content_writes.insert(id.clone(), false);
                let mut cell = temporary
                    .cells
                    .iter()
                    .find(|c| &c.id == id)
                    .cloned()
                    .ok_or(PreviewError::InvalidPatch)?;
                if body.r#match.is_empty() {
                    return Err(PreviewError::NonuniqueReplacement);
                }
                let positions = cell
                    .source
                    .char_indices()
                    .filter_map(|(i, _)| cell.source[i..].starts_with(&body.r#match).then_some(i))
                    .take(2)
                    .collect::<Vec<_>>();
                if positions.len() != 1 {
                    return Err(PreviewError::NonuniqueReplacement);
                }
                let start = positions[0];
                cell.source
                    .replace_range(start..start + body.r#match.len(), &body.replacement);
                NativeSourceOperation::UpdateSourceCell(UpdateSourceCell {
                    kind: UpdateSourceCellKind::UpdateCell,
                    cell,
                })
            }
            PreviewPatchOperation::PreviewDeleteCell(body) => {
                let id = &body.target.cell_id;
                hash_checked(id, &body.expected_source_hash, original)?;
                if content_writes.contains_key(id) || !deleted.insert(id.clone()) {
                    return Err(PreviewError::InvalidPatch);
                }
                NativeSourceOperation::DeleteSourceCells(DeleteSourceCells {
                    kind: DeleteSourceCellsKind::DeleteCells,
                    cell_ids: vec![id.clone()],
                })
            }
            PreviewPatchOperation::PreviewMoveCell(body) => {
                NativeSourceOperation::MoveSourceCells(MoveSourceCells {
                    kind: MoveSourceCellsKind::MoveCells,
                    cell_ids: vec![selected(&body.target, original, &assigned)?],
                    after_cell_id: Nullable(anchor(body.after.0.as_ref(), original, &assigned)?),
                })
            }
            PreviewPatchOperation::PreviewRenameNotebook(body) => {
                NativeSourceOperation::RenameSourceNotebook(RenameSourceNotebook {
                    kind: RenameSourceNotebookKind::RenameNotebook,
                    title: body.title.clone(),
                })
            }
        };
        apply_one(&mut temporary, &operation)?;
        operations.push(operation);
    }
    Ok((operations, assigned))
}
fn apply_one(
    file: &mut NativeSourceFile,
    operation: &NativeSourceOperation,
) -> Result<(), PreviewError> {
    *file = crate::document::coordinator::apply_operations(file, std::slice::from_ref(operation))
        .map_err(|_| PreviewError::InvalidPatch)?;
    Ok(())
}
pub(super) fn diagnostics(plan: &FrozenPreviewPlan) -> (bool, Vec<NativePreviewDiagnostic>) {
    let relevant = plan
        .commit
        .commit
        .changed_cell_ids
        .iter()
        .chain(&plan.invalidation.affected_cells)
        .collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();
    let mut valid = true;
    for cell in &plan.invalidation.analysis.cells {
        if !relevant.contains(&cell.cell_id) {
            continue;
        }
        for d in &cell.diagnostics {
            let record = diagnostic(d, Some(cell.cell_id.clone()));
            if record.severity == NativePreviewDiagnosticSeverity::Error {
                valid = false;
            }
            diagnostics.push(record);
        }
    }
    for conflict in &plan.invalidation.analysis.conflicts {
        if conflict.cell_ids.iter().any(|id| relevant.contains(id)) {
            valid = false;
            diagnostics.push(NativePreviewDiagnostic {
                code: "MULTIPLE_DEFINITIONS".into(),
                message: format!("重复定义：{}", conflict.symbol)
                    .chars()
                    .take(4000)
                    .collect(),
                severity: NativePreviewDiagnosticSeverity::Error,
                cell_id: Nullable(conflict.cell_ids.first().cloned()),
                start_utf8: Nullable(None),
                end_utf8: Nullable(None),
            });
        }
    }
    for id in &plan.invalidation.analysis.cycles {
        if relevant.contains(id) {
            valid = false;
            diagnostics.push(NativePreviewDiagnostic {
                code: "DEPENDENCY_CYCLE".into(),
                message: "单元格之间存在循环依赖".into(),
                severity: NativePreviewDiagnosticSeverity::Error,
                cell_id: Nullable(Some(id.clone())),
                start_utf8: Nullable(None),
                end_utf8: Nullable(None),
            });
        }
    }
    // Never hide an omitted error by truncating diagnostics into a seemingly valid preview.
    if diagnostics.len() > 100 {
        valid = false;
        diagnostics.truncate(99);
        diagnostics.push(NativePreviewDiagnostic {
            code: "DIAGNOSTIC_LIMIT".into(),
            message: "诊断超出预览上限，请缩小修改范围".into(),
            severity: NativePreviewDiagnosticSeverity::Error,
            cell_id: Nullable(None),
            start_utf8: Nullable(None),
            end_utf8: Nullable(None),
        });
    }
    (valid, diagnostics)
}
