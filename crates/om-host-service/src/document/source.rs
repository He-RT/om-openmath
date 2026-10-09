//! Stage whole source atomically; only a matching trusted durable receipt advances authority.
use super::{request_hash, snapshot_hash};
use crate::protocol::{Nullable, Serial, generated::*};
use std::collections::BTreeSet;
fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .enumerate()
            .all(|(i, c)| c.is_ascii_alphanumeric() || (i > 0 && b"._:-".contains(&c)))
}
fn file_valid(file: &NativeSourceFile) -> Result<(), String> {
    if file.version != 1 || file.title.len() > 16384 || file.cells.len() > 10000 {
        return Err("INVALID_SOURCE".into());
    }
    let mut seen = BTreeSet::new();
    let mut bytes = file.title.len();
    for cell in &file.cells {
        if (cell.id.is_empty() || cell.id.len() > 256) || !seen.insert(&cell.id) {
            return Err("INVALID_CELL_ID".into());
        }
        bytes = bytes
            .checked_add(cell.source.len() + cell.id.len())
            .ok_or("BUDGET_EXCEEDED")?;
    }
    if bytes > 2 * 1024 * 1024 {
        return Err("SOURCE_BLOB_REQUIRED".into());
    }
    Ok(())
}
/// Validate stored source without normalizing Unicode or executing declarations.
pub fn validate_snapshot(snapshot: &NativeSourceSnapshot) -> Result<(), String> {
    file_valid(&snapshot.file)?;
    if snapshot
        .cell_revisions
        .iter()
        .any(|r| r.revision > snapshot.revision)
        || snapshot.codec_version != 1
        || !identity(&snapshot.document_id)
        || snapshot.cell_revisions.len() != snapshot.file.cells.len()
        || snapshot
            .cell_revisions
            .iter()
            .zip(&snapshot.file.cells)
            .any(|(r, c)| r.cell_id != c.id)
        || snapshot.snapshot_hash != snapshot_hash(snapshot)
    {
        return Err("INVALID_SNAPSHOT".into());
    }
    Ok(())
}
/// Check cross-field hashes/revisions of a frozen source plan. Permission/fence/IO still belong
/// to the trusted coordinator, never to this shape/consistency check.
pub fn validate_commit(plan: &NativeSourceCommit) -> Result<(), String> {
    validate_snapshot(&plan.before)?;
    validate_snapshot(&plan.after)?;
    let commit = &plan.commit;
    if plan.protocol_version != 1
        || plan.generation.get() == 0
        || !identity(&commit.operation_id)
        || !identity(&commit.transaction_id)
        || !identity(&commit.outbox_event_id)
        || commit.document_id != plan.before.document_id
        || commit.document_id != plan.after.document_id
        || commit.base_revision != plan.before.revision
        || commit.committed_revision != plan.after.revision
        || plan.after.revision != plan.before.revision.checked_next()?
        || plan.after.execution_epoch
            != if super::coordinator::math_changed(&plan.before.file, &plan.after.file)
                || plan
                    .calculation_change
                    .0
                    .as_ref()
                    .is_some_and(super::coordinator::settings_differ)
            {
                plan.before.execution_epoch.checked_next()?
            } else {
                plan.before.execution_epoch
            }
        || commit.snapshot_hash != plan.after.snapshot_hash
        || commit.inverse_plan_hash != plan.before.snapshot_hash
        || commit.execution_epoch != plan.after.execution_epoch
        || commit.request_hash != request_hash(plan)
        || commit.snapshot_blob_hash.0.is_some()
    {
        return Err("INVALID_COMMIT".into());
    }
    let expected = changed_ids(&plan.before.file, &plan.after.file);
    if commit.changed_cell_ids != expected {
        return Err("INVALID_CHANGE_SET".into());
    }
    for revision in &plan.after.cell_revisions {
        let previous = plan
            .before
            .cell_revisions
            .iter()
            .find(|r| r.cell_id == revision.cell_id);
        let old = plan
            .before
            .file
            .cells
            .iter()
            .find(|c| c.id == revision.cell_id);
        let new = plan
            .after
            .file
            .cells
            .iter()
            .find(|c| c.id == revision.cell_id)
            .ok_or("INVALID_CELL_ID")?;
        let expected = match (previous, old) {
            (Some(r), Some(c)) if cell_same(c, new) => r.revision,
            (Some(r), _) => r.revision.checked_next()?,
            _ => Serial::new(0)?,
        };
        if revision.revision != expected {
            return Err("INVALID_CELL_REVISION".into());
        }
    }
    match commit.actor {
        DocumentCommitActor::Agent
            if commit.task_id.0.as_deref().is_none_or(|id| !identity(id)) =>
        {
            return Err("INVALID_ACTOR".into());
        }
        DocumentCommitActor::Undo if commit.undo_of.0.as_deref().is_none_or(|id| !identity(id)) => {
            return Err("INVALID_ACTOR".into());
        }
        _ => {}
    }
    Ok(())
}
fn cell_same(a: &NativeSourceCell, b: &NativeSourceCell) -> bool {
    a.id == b.id && a.kind == b.kind && a.dialect == b.dialect && a.source == b.source
}
fn changed_ids(before: &NativeSourceFile, after: &NativeSourceFile) -> Vec<String> {
    let mut changed = BTreeSet::new();
    for (index, cell) in before.cells.iter().enumerate() {
        if after
            .cells
            .get(index)
            .is_none_or(|next| !cell_same(cell, next))
        {
            changed.insert(cell.id.clone());
        }
    }
    for (index, cell) in after.cells.iter().enumerate() {
        if before
            .cells
            .get(index)
            .is_none_or(|previous| !cell_same(cell, previous))
        {
            changed.insert(cell.id.clone());
        }
    }
    changed.into_iter().collect()
}
/// The authoritative confirmed source. Plans are separate values until durable acceptance.
pub struct SourceDocument {
    snapshot: NativeSourceSnapshot,
    generation: Serial,
}
impl SourceDocument {
    /// Create revision zero from a source-only notebook. This does not evaluate any cell.
    pub fn new(
        document_id: String,
        generation: Serial,
        file: NativeSourceFile,
    ) -> Result<Self, String> {
        if !identity(&document_id) || generation.get() == 0 {
            return Err("INVALID_DOCUMENT".into());
        }
        file_valid(&file)?;
        let mut snapshot = NativeSourceSnapshot {
            codec_version: 1,
            document_id,
            revision: Serial::new(0)?,
            execution_epoch: Serial::new(0)?,
            cell_revisions: file
                .cells
                .iter()
                .map(|c| NativeCellRevision {
                    cell_id: c.id.clone(),
                    revision: Serial::new(0).expect("zero"),
                })
                .collect(),
            file,
            snapshot_hash: String::new(),
        };
        snapshot.snapshot_hash = snapshot_hash(&snapshot);
        Ok(Self {
            snapshot,
            generation,
        })
    }
    /// Restore source data only, with a new trusted open generation. No Session is constructed.
    pub fn restore(snapshot: NativeSourceSnapshot, generation: Serial) -> Result<Self, String> {
        validate_snapshot(&snapshot)?;
        if generation.get() == 0 {
            return Err("INVALID_DOCUMENT".into());
        }
        Ok(Self {
            snapshot,
            generation,
        })
    }
    /// Read the confirmed source; callers cannot mutate it.
    pub fn snapshot(&self) -> &NativeSourceSnapshot {
        &self.snapshot
    }
    /// Trusted current open lifetime, separate from a stored source revision.
    pub fn generation(&self) -> Serial {
        self.generation
    }
    /// Freeze a replacement and its full inverse; same syntax-invalid source remains editable.
    pub fn prepare(
        &self,
        file: NativeSourceFile,
        operation_id: String,
        transaction_id: String,
        outbox_event_id: String,
        committed_at: String,
    ) -> Result<NativeSourceCommit, String> {
        file_valid(&file)?;
        let mut after = NativeSourceSnapshot {
            codec_version: 1,
            document_id: self.snapshot.document_id.clone(),
            revision: self.snapshot.revision.checked_next()?,
            execution_epoch: if super::coordinator::math_changed(&self.snapshot.file, &file) {
                self.snapshot.execution_epoch.checked_next()?
            } else {
                self.snapshot.execution_epoch
            },
            cell_revisions: Vec::new(),
            file,
            snapshot_hash: String::new(),
        };
        for cell in &after.file.cells {
            let old = self
                .snapshot
                .file
                .cells
                .iter()
                .position(|c| c.id == cell.id);
            let revision = if let Some(index) = old {
                let r = self.snapshot.cell_revisions[index].revision;
                if cell_same(&self.snapshot.file.cells[index], cell) {
                    r
                } else {
                    r.checked_next()?
                }
            } else {
                Serial::new(0)?
            };
            after.cell_revisions.push(NativeCellRevision {
                cell_id: cell.id.clone(),
                revision,
            });
        }
        after.snapshot_hash = snapshot_hash(&after);
        let commit = DocumentCommit {
            record_type: DocumentCommitRecordType::DocumentCommit,
            document_id: after.document_id.clone(),
            operation_id,
            transaction_id,
            request_hash: String::new(),
            base_revision: self.snapshot.revision,
            committed_revision: after.revision,
            snapshot_hash: after.snapshot_hash.clone(),
            snapshot_blob_hash: Nullable(None),
            inverse_plan_hash: self.snapshot.snapshot_hash.clone(),
            execution_epoch: after.execution_epoch,
            actor: DocumentCommitActor::Manual,
            task_id: Nullable(None),
            undo_of: Nullable(None),
            changed_cell_ids: changed_ids(&self.snapshot.file, &after.file),
            outbox_event_id,
            committed_at,
        };
        let mut plan = NativeSourceCommit {
            protocol_version: 1,
            calculation_change: Nullable(None),
            generation: self.generation,
            commit,
            before: self.snapshot.clone(),
            after,
        };
        plan.commit.request_hash = request_hash(&plan);
        validate_commit(&plan)?;
        Ok(plan)
    }
    /// Freeze a real trusted before/after calculation settings transition with the same physical
    /// document transaction; UI language-only changes leave execution epoch untouched.
    pub fn prepare_settings(
        &self,
        before: &om_kernel::config::GeneralConfig,
        after: &om_kernel::config::GeneralConfig,
        operation_id: String,
        transaction_id: String,
        event_id: String,
        time: String,
    ) -> Result<NativeSourceCommit, String> {
        let mut plan = self.prepare(
            self.snapshot.file.clone(),
            operation_id,
            transaction_id,
            event_id,
            time,
        )?;
        let change = NativeCalculationChange {
            before: super::coordinator::calculation_values(before)?,
            after: super::coordinator::calculation_values(after)?,
        };
        if super::coordinator::settings_differ(&change) {
            plan.after.execution_epoch = self.snapshot.execution_epoch.checked_next()?;
        }
        plan.calculation_change = Nullable(Some(change));
        plan.after.snapshot_hash = super::snapshot_hash(&plan.after);
        plan.commit.snapshot_hash = plan.after.snapshot_hash.clone();
        plan.commit.execution_epoch = plan.after.execution_epoch;
        plan.commit.request_hash = super::request_hash(&plan);
        validate_commit(&plan)?;
        Ok(plan)
    }
    /// Advance only on the exact trusted physical receipt. Exposing this function is not a tool
    /// grant: later IOAck scope/fence admission must call it after reading storage itself.
    pub fn accept(
        &mut self,
        plan: &NativeSourceCommit,
        receipt: &NativeDurableSourceReceipt,
    ) -> Result<(), String> {
        validate_commit(plan)?;
        let r = &receipt.receipt;
        if self.generation != plan.generation
            || self.snapshot.snapshot_hash != plan.before.snapshot_hash
            || receipt.protocol_version != 1
            || r.document_id.0.as_ref() != Some(&plan.commit.document_id)
            || r.operation_id != plan.commit.operation_id
            || r.request_hash != plan.commit.request_hash
            || r.phase != OperationReceiptPhase::Completed
            || r.operation_kind
                != if plan.commit.actor == DocumentCommitActor::Undo {
                    OperationReceiptOperationKind::Undo
                } else {
                    OperationReceiptOperationKind::SourceEdit
                }
            || r.error_code.0.is_some()
            || !r.accepted_result_ids.is_empty()
            || r.transaction_id.0.as_ref() != Some(&plan.commit.transaction_id)
            || r.committed_revision.0 != Some(plan.after.revision)
            || receipt.snapshot_hash != plan.after.snapshot_hash
            || receipt.inverse_plan_hash != plan.before.snapshot_hash
            || receipt.execution_epoch != plan.after.execution_epoch
            || receipt.outbox_event_id != plan.commit.outbox_event_id
        {
            return Err("INVALID_RECEIPT".into());
        }
        self.snapshot = plan.after.clone();
        Ok(())
    }
}
