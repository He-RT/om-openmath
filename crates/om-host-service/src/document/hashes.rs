//! Stable length-framed UTF-8 hashes shared with the physical Swift storage port.
use crate::protocol::generated::*;
use sha2::{Digest, Sha256};
struct Framed(Sha256);
impl Framed {
    fn new(domain: &str) -> Self {
        let mut this = Self(Sha256::new());
        this.text(domain);
        this
    }
    fn number(&mut self, n: u64) {
        self.0.update(n.to_be_bytes());
    }
    fn text(&mut self, s: &str) {
        self.number(s.len() as u64);
        self.0.update(s.as_bytes());
    }
    fn finish(self) -> String {
        format!("{:x}", self.0.finalize())
    }
}
fn kind(value: &NativeCellKind) -> &str {
    match value {
        NativeCellKind::Math => "Math",
        NativeCellKind::Text => "Text",
        NativeCellKind::Ask => "Ask",
    }
}
fn dialect(value: &Dialect) -> &str {
    match value {
        Dialect::Modern => "Modern",
        Dialect::Wolfram => "Wolfram",
        Dialect::Auto => "Auto",
    }
}
/// Hash the exact source bytes, ordered IDs, cell revisions and source provenance, never outputs.
pub fn snapshot_hash(snapshot: &NativeSourceSnapshot) -> String {
    let mut hash = Framed::new("openmath-source-v1");
    hash.text(&snapshot.document_id);
    hash.number(snapshot.revision.get());
    hash.number(snapshot.execution_epoch.get());
    hash.number(snapshot.file.version.into());
    hash.text(&snapshot.file.title);
    hash.number(snapshot.file.cells.len() as u64);
    for (index, cell) in snapshot.file.cells.iter().enumerate() {
        hash.text(&cell.id);
        hash.text(kind(&cell.kind));
        hash.text(&cell.source);
        hash.text(dialect(&cell.dialect));
        hash.number(
            snapshot
                .cell_revisions
                .get(index)
                .map_or(u64::MAX, |c| c.revision.get()),
        );
    }
    hash.finish()
}
/// Logical identity excludes transport/generation/commit-time/new receipt IDs, so reconnects can
/// query the original operation. It includes the original before/after frozen bytes and actor.
pub fn request_hash(plan: &NativeSourceCommit) -> String {
    let mut hash = Framed::new("openmath-source-operation-v1");
    hash.text(&plan.commit.document_id);
    hash.text(&plan.commit.operation_id);
    hash.text(&plan.before.snapshot_hash);
    hash.text(&plan.after.snapshot_hash);
    hash.text(match plan.commit.actor {
        DocumentCommitActor::Manual => "manual",
        DocumentCommitActor::Agent => "agent",
        DocumentCommitActor::Undo => "undo",
        DocumentCommitActor::ExternalMerge => "external_merge",
    });
    hash.text(plan.commit.task_id.0.as_deref().unwrap_or(""));
    hash.text(plan.commit.undo_of.0.as_deref().unwrap_or(""));
    if let Some(group) = &plan.undo_group {
        hash.text("undo-group-v1");
        hash.text(&group.group_id);
        hash.number(group.transaction_ids.len() as u64);
        for id in &group.transaction_ids {
            hash.text(id);
        }
    }
    if let Some(group) = &plan.input_group_id {
        hash.text("input-group-v1");
        hash.text(group);
    }
    if let Some(change) = &plan.calculation_change.0 {
        hash.text("calculation-change-v1");
        for setting in [&change.before, &change.after] {
            hash.text(match setting.dialect {
                NativeCalculationSettingsDialect::Auto => "auto",
                NativeCalculationSettingsDialect::Modern => "modern",
                NativeCalculationSettingsDialect::Wolfram => "wolfram",
            });
            hash.text(match setting.constants {
                NativeCalculationSettingsConstants::Math => "math",
                NativeCalculationSettingsConstants::Strict => "strict",
            });
            for value in [
                setting.reactive,
                setting.auto_run_dependents,
                setting.show_steps,
                setting.auto_plot,
            ] {
                hash.number(u64::from(value));
            }
            hash.number(setting.eval_timeout_ms.get());
        }
    }
    hash.finish()
}
