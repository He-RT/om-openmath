//! Byte-framed request identity and closed cross-field validation, shared with Swift physical IO.
use crate::{
    document::{coordinator::math_changed, validate_snapshot},
    protocol::{generated::*, validation},
};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
struct Framer(Sha256);
impl Framer {
    fn text(&mut self, value: &str) {
        self.number(value.len() as u64);
        self.0.update(value.as_bytes());
    }
    fn number(&mut self, n: u64) {
        self.0.update(n.to_be_bytes());
    }
}
fn schema() -> &'static serde_json::Value {
    static ROOT: OnceLock<serde_json::Value> = OnceLock::new();
    ROOT.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../docs/design/native-kernel-store.schema.json"
        ))
        .expect("checked kernel schema")
    })
}
/// Decode closed physical commit shape, then verify actual byte/revision/source relationships.
pub fn decode_commit(bytes: &[u8]) -> Result<NativeKernelCommit, String> {
    let plan = validation::decode(bytes, &schema()["$defs"]["NativeKernelCommit"], schema())?;
    validate_commit(&plan)?;
    Ok(plan)
}
/// Decode only receipt shape. Matching a trusted original frozen plan is a separate requirement.
pub fn decode_receipt(bytes: &[u8]) -> Result<NativeKernelReceipt, String> {
    validation::decode(bytes, &schema()["$defs"]["NativeKernelReceipt"], schema())
}
fn identity(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_alphanumeric() || (i > 0 && b"._:-".contains(&b)))
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// Stable original request identity. Runtime reconnects do not authorize executing it again.
pub fn request_hash(plan: &NativeKernelCommit) -> String {
    let mut f = Framer(Sha256::new());
    f.text("openmath-kernel-acceptance-v1");
    f.text(match plan.kind {
        NativeKernelCommitKind::Bootstrap => "bootstrap",
        NativeKernelCommitKind::ExecuteCell => "execute_cell",
    });
    for text in [
        &plan.store_id,
        &plan.operation_id,
        &plan.checkpoint_id,
        plan.expected_parent_checkpoint_id
            .0
            .as_deref()
            .unwrap_or(""),
        &plan.checkpoint_blob_hash,
    ] {
        f.text(text);
    }
    f.number(plan.expected_kernel_state_revision.get());
    f.number(plan.checkpoint_byte_length.get());
    f.number(plan.codec_version.into());
    let p = &plan.producer;
    f.text(&p.document_id);
    f.number(p.document_generation.get());
    f.number(p.source_revision.get());
    f.number(p.execution_epoch.get());
    f.number(p.kernel_state_revision.get());
    f.text(&p.source_snapshot_hash);
    f.text(&p.kernel_build);
    f.number(p.config_revision.get());
    f.text(match p.calculation.dialect {
        NativeCalculationSettingsDialect::Auto => "auto",
        NativeCalculationSettingsDialect::Modern => "modern",
        NativeCalculationSettingsDialect::Wolfram => "wolfram",
    });
    f.text(match p.calculation.constants {
        NativeCalculationSettingsConstants::Math => "math",
        NativeCalculationSettingsConstants::Strict => "strict",
    });
    for value in [
        p.calculation.reactive,
        p.calculation.auto_run_dependents,
        p.calculation.show_steps,
        p.calculation.auto_plot,
    ] {
        f.number(u64::from(value));
    }
    f.number(p.calculation.eval_timeout_ms.get());
    f.text(&p.general_hash);
    f.text(p.cell_id.0.as_deref().unwrap_or(""));
    f.text(p.cell_source_hash.0.as_deref().unwrap_or(""));
    f.text(match p.terminal_status {
        NativeKernelTerminalStatus::Unexecuted => "unexecuted",
        NativeKernelTerminalStatus::Done => "done",
        NativeKernelTerminalStatus::Error => "error",
    });
    f.number(p.successful_statements.get());
    f.number(p.out_index.0.map_or(0, |n| n.get()));
    f.text(&plan.source.snapshot_hash);
    f.text(&plan.acceptance_source.snapshot_hash);
    f.text(plan.result_id.0.as_deref().unwrap_or(""));
    f.text(&plan.outbox_event_id);
    format!("{:x}", f.0.finalize())
}
/// No malformed role/source/producer/range can be persisted as an accepted mathematical state.
pub fn validate_commit(plan: &NativeKernelCommit) -> Result<(), String> {
    validate_snapshot(&plan.source)?;
    validate_snapshot(&plan.acceptance_source)?;
    let p = &plan.producer;
    if plan.protocol_version != 1
        || plan.codec_version != 1
        || !identity(&plan.store_id)
        || !identity(&plan.runtime_instance_id)
        || !identity(&plan.operation_id)
        || !identity(&plan.checkpoint_id)
        || !identity(&plan.outbox_event_id)
        || !hash(&plan.checkpoint_blob_hash)
        || !hash(&p.general_hash)
        || p.kernel_build.is_empty()
        || p.kernel_build.len() > 128
        || !(21..=64 * 1024 * 1024).contains(&plan.checkpoint_byte_length.get())
        || p.document_generation.get() == 0
        || p.document_id != plan.source.document_id
        || p.document_id != plan.acceptance_source.document_id
        || p.source_revision != plan.source.revision
        || p.source_snapshot_hash != plan.source.snapshot_hash
        || p.execution_epoch != plan.source.execution_epoch
        || p.execution_epoch != plan.acceptance_source.execution_epoch
        || plan.acceptance_source.revision < plan.source.revision
        || math_changed(&plan.source.file, &plan.acceptance_source.file)
        || p.successful_statements.get() > 10000
        || p.out_index
            .0
            .is_some_and(|n| n.get() == 0 || n.get() > 10000)
        || p.calculation.eval_timeout_ms.get() == 0
        || plan.request_hash != request_hash(plan)
    {
        return Err("INVALID_KERNEL_PLAN".into());
    }
    match plan.kind {
        NativeKernelCommitKind::Bootstrap => {
            if plan.expected_parent_checkpoint_id.0.is_some()
                || plan.expected_kernel_state_revision.get() != 0
                || p.kernel_state_revision.get() != 0
                || p.cell_id.0.is_some()
                || p.cell_source_hash.0.is_some()
                || p.terminal_status != NativeKernelTerminalStatus::Unexecuted
                || p.successful_statements.get() != 0
                || p.out_index.0.is_some()
                || plan.result_id.0.is_some()
            {
                return Err("INVALID_KERNEL_BOOTSTRAP".into());
            }
        }
        NativeKernelCommitKind::ExecuteCell => {
            if plan
                .expected_parent_checkpoint_id
                .0
                .as_deref()
                .is_none_or(|id| !identity(id) || id == plan.checkpoint_id)
                || p.kernel_state_revision != plan.expected_kernel_state_revision.checked_next()?
                || plan.result_id.0.as_deref().is_none_or(|id| !identity(id))
                || p.terminal_status == NativeKernelTerminalStatus::Unexecuted
                || (p.successful_statements.get() == 0) != p.out_index.0.is_none()
            {
                return Err("INVALID_KERNEL_CANDIDATE".into());
            }
            let cell = plan
                .source
                .file
                .cells
                .iter()
                .find(|c| Some(&c.id) == p.cell_id.0.as_ref())
                .ok_or("INVALID_KERNEL_CELL")?;
            if cell.kind != NativeCellKind::Math
                || p.cell_source_hash.0.as_deref()
                    != Some(format!("{:x}", Sha256::digest(cell.source.as_bytes())).as_str())
            {
                return Err("INVALID_KERNEL_CELL".into());
            }
        }
    }
    Ok(())
}
/// A matching trusted physical receipt certifies this original plan, not any other staged state.
pub fn validate_receipt(
    plan: &NativeKernelCommit,
    receipt: &NativeKernelReceipt,
) -> Result<(), String> {
    validate_commit(plan)?;
    if receipt.protocol_version != 1
        || receipt.codec_version != 1
        || receipt.store_id != plan.store_id
        || receipt.document_id != plan.producer.document_id
        || receipt.operation_id != plan.operation_id
        || receipt.request_hash != plan.request_hash
        || receipt.checkpoint_id != plan.checkpoint_id
        || receipt.checkpoint_blob_hash != plan.checkpoint_blob_hash
        || receipt.checkpoint_byte_length != plan.checkpoint_byte_length
        || receipt.accepted_source_revision != plan.acceptance_source.revision
        || receipt.accepted_snapshot_hash != plan.acceptance_source.snapshot_hash
        || receipt.kernel_state_revision != plan.producer.kernel_state_revision
        || receipt.result_id != plan.result_id
        || receipt.outbox_event_id != plan.outbox_event_id
        || serde_json::to_value(&receipt.producer).map_err(|_| "INVALID_KERNEL_RECEIPT")?
            != serde_json::to_value(&plan.producer).map_err(|_| "INVALID_KERNEL_RECEIPT")?
        || receipt.committed_at.len() < 20
        || receipt.committed_at.len() > 64
        || !receipt.committed_at.is_ascii()
        || !receipt.committed_at.ends_with('Z')
        || !receipt.committed_at.contains('T')
    {
        return Err("INVALID_KERNEL_RECEIPT".into());
    }
    Ok(())
}
