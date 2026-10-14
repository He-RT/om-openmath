//! Whole source operation batches and actual non-executing dependency invalidation.
use super::SourceDocument;
use crate::protocol::generated::*;
use om_kernel::{
    config::{ConfigDialect, Constants},
    protocol::{CellInput, CellKind, Dialect as KernelDialect, NotebookFile},
    source::{SourceInvalidation, SourceOwnership},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Typed immutable source plan plus real parser/graph facts. It does not claim IO or CAS success.
pub struct SourceMutationPlan {
    /// Frozen before/after persistence contract.
    pub commit: NativeSourceCommit,
    /// Actual invalidation and diagnostics.
    pub invalidation: SourceInvalidation,
}
/// Source-only owner settings, kept separate from Swift UI draft state.
#[derive(Clone, Serialize, Deserialize)]
pub struct SourceCoordinator {
    /// Actual input dialect setting.
    pub dialect: ConfigDialect,
    /// Actual constant interpretation setting.
    pub constants: Constants,
    /// Parser bindings from a trusted computation state; not made-up executed source.
    pub known_functions: Vec<String>,
    /// Actual old live write ownership from the accepted kernel state.
    pub owned: Vec<SourceOwnership>,
}
impl Default for SourceCoordinator {
    fn default() -> Self {
        Self {
            dialect: ConfigDialect::Auto,
            constants: Constants::Math,
            known_functions: Vec::new(),
            owned: Vec::new(),
        }
    }
}
/// Convert source-only shared DTOs without serializing values/history or rewriting syntax.
pub fn kernel_file(file: &NativeSourceFile) -> NotebookFile {
    NotebookFile {
        version: file.version,
        title: file.title.clone(),
        cells: file
            .cells
            .iter()
            .map(|cell| CellInput {
                id: cell.id.clone(),
                source: cell.source.clone(),
                kind: match cell.kind {
                    NativeCellKind::Math => CellKind::Math,
                    NativeCellKind::Text => CellKind::Text,
                    NativeCellKind::Ask => CellKind::Ask,
                },
                dialect: match cell.dialect {
                    Dialect::Modern => KernelDialect::Modern,
                    Dialect::Wolfram => KernelDialect::Wolfram,
                    Dialect::Auto => KernelDialect::Auto,
                },
            })
            .collect(),
    }
}
/// Math sequence/content changes only; unlike view positions, a Text insertion can shift Math
/// indexes without changing mathematical execution.
pub fn math_changed(before: &NativeSourceFile, after: &NativeSourceFile) -> bool {
    before
        .cells
        .iter()
        .filter(|c| c.kind == NativeCellKind::Math)
        .map(|c| (&c.id, &c.source, &c.dialect))
        .ne(after
            .cells
            .iter()
            .filter(|c| c.kind == NativeCellKind::Math)
            .map(|c| (&c.id, &c.source, &c.dialect)))
}
fn unique(ids: &[String]) -> Result<(), String> {
    if ids.is_empty() || ids.len() > 64 || ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err("INVALID_CELL_SET".into());
    }
    Ok(())
}
fn position(file: &NativeSourceFile, anchor: Option<&str>) -> Result<usize, String> {
    match anchor {
        Some(id) => file
            .cells
            .iter()
            .position(|c| c.id == id)
            .map(|i| i + 1)
            .ok_or("INVALID_CELL_REFERENCE".into()),
        None => Ok(0),
    }
}
/// The shared whole-source operation algorithm, reused by preview normalization at each step.
/// Only temporary data is returned; it does not update a document or invoke CAS/IO.
pub fn apply_operations(
    file: &NativeSourceFile,
    operations: &[NativeSourceOperation],
) -> Result<NativeSourceFile, String> {
    if operations.is_empty() || operations.len() > 64 {
        return Err("PATCH_BUDGET_EXCEEDED".into());
    }
    let mut file = file.clone();
    for operation in operations {
        match operation {
            NativeSourceOperation::InsertSourceCell(body) => {
                if file.cells.iter().any(|c| c.id == body.cell.id) {
                    return Err("DUPLICATE_CELL_ID".into());
                }
                let at = position(&file, body.after_cell_id.0.as_deref())?;
                file.cells.insert(at, body.cell.clone());
            }
            NativeSourceOperation::UpdateSourceCell(body) => {
                let index = file
                    .cells
                    .iter()
                    .position(|c| c.id == body.cell.id)
                    .ok_or("INVALID_CELL_REFERENCE")?;
                file.cells[index] = body.cell.clone();
            }
            NativeSourceOperation::DeleteSourceCells(body) => {
                unique(&body.cell_ids)?;
                if body
                    .cell_ids
                    .iter()
                    .any(|id| !file.cells.iter().any(|c| &c.id == id))
                {
                    return Err("INVALID_CELL_REFERENCE".into());
                }
                let ids = body.cell_ids.iter().collect::<BTreeSet<_>>();
                file.cells.retain(|c| !ids.contains(&c.id));
            }
            NativeSourceOperation::MoveSourceCells(body) => {
                unique(&body.cell_ids)?;
                if body
                    .after_cell_id
                    .0
                    .as_ref()
                    .is_some_and(|id| body.cell_ids.contains(id))
                {
                    return Err("INVALID_MOVE_ANCHOR".into());
                }
                let selected = body
                    .cell_ids
                    .iter()
                    .map(|id| {
                        file.cells
                            .iter()
                            .find(|c| &c.id == id)
                            .cloned()
                            .ok_or("INVALID_CELL_REFERENCE")
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                file.cells.retain(|c| !body.cell_ids.contains(&c.id));
                let at = position(&file, body.after_cell_id.0.as_deref())?;
                file.cells.splice(at..at, selected);
            }
            NativeSourceOperation::RenameSourceNotebook(body) => file.title = body.title.clone(),
        }
    }
    Ok(file)
}
impl SourceCoordinator {
    /// Validate all operations in a temporary source file. No intermediate mutation is published,
    /// no old Upsert/DeleteCell handler is called, and parse/graph analysis never evaluates.
    pub fn prepare(
        &self,
        owner: &SourceDocument,
        operations: &[NativeSourceOperation],
        operation: String,
        transaction: String,
        event: String,
        time: String,
    ) -> Result<SourceMutationPlan, String> {
        if operations.is_empty() || operations.len() > 64 {
            return Err("PATCH_BUDGET_EXCEEDED".into());
        }
        let file = apply_operations(&owner.snapshot().file, operations)?;
        let commit = owner.prepare(file, operation, transaction, event, time)?;
        let invalidation = om_kernel::source::assess_source_change(
            &kernel_file(&commit.before.file),
            &kernel_file(&commit.after.file),
            self.dialect,
            self.constants,
            &self.known_functions,
            &self.owned,
            false,
        )?;
        Ok(SourceMutationPlan {
            commit,
            invalidation,
        })
    }
    /// A trusted metadata change can invalidate computation independently of source revision.
    /// The source mutation caller must persist actual settings in the corresponding settings port.
    pub fn settings_changed(
        &self,
        before: &om_kernel::config::GeneralConfig,
        after: &om_kernel::config::GeneralConfig,
    ) -> bool {
        om_kernel::source::calculation_settings_changed(before, after)
    }
}
/// Exact settings values, independently persisted with the source transaction.
pub fn calculation_values(
    config: &om_kernel::config::GeneralConfig,
) -> Result<NativeCalculationSettings, String> {
    Ok(NativeCalculationSettings {
        dialect: match config.dialect {
            ConfigDialect::Auto => NativeCalculationSettingsDialect::Auto,
            ConfigDialect::Modern => NativeCalculationSettingsDialect::Modern,
            ConfigDialect::Wolfram => NativeCalculationSettingsDialect::Wolfram,
        },
        constants: match config.constants {
            Constants::Math => NativeCalculationSettingsConstants::Math,
            Constants::Strict => NativeCalculationSettingsConstants::Strict,
        },
        reactive: config.reactive,
        auto_run_dependents: config.auto_run_dependents,
        show_steps: config.show_steps,
        auto_plot: config.auto_plot,
        eval_timeout_ms: crate::protocol::Serial::new(config.eval_timeout_ms)?,
    })
}
/// Settings equality never compares a UI language/producer counter to an execution epoch.
pub fn settings_differ(change: &NativeCalculationChange) -> bool {
    let a = &change.before;
    let b = &change.after;
    a.dialect != b.dialect
        || a.constants != b.constants
        || a.reactive != b.reactive
        || a.auto_run_dependents != b.auto_run_dependents
        || a.show_steps != b.show_steps
        || a.auto_plot != b.auto_plot
        || a.eval_timeout_ms != b.eval_timeout_ms
}
