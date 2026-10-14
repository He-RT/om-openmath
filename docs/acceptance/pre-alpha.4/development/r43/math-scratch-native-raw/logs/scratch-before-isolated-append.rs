//! Independent writable scratch from the actual current read snapshot/accepted definitions.
mod store;
use crate::{
    agent::ingress::ToolIntent,
    document::coordinator::kernel_file,
    kernel::{KernelState, acceptance::AcceptedKernelState},
    protocol::generated::{AgentToolName, NativeSourceSnapshot},
    references::ReferenceScope,
    tools::read::ReadNotebookService,
};
use om_kernel::{
    CellExecutionScope, KernelConfig, Session,
    checkpoint::{CheckpointBinding, CheckpointLimits, CheckpointRestore},
    config::GeneralConfig,
    protocol::{CellInput, CellKind, Dialect, NotebookFile},
};
use om_num::ctx::{Abort, Interrupt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::Arc};
pub use store::{ResolvedScratch, ScratchResultStore};
fn tick(ctx: &Interrupt) -> Result<(), String> {
    ctx.tick().map_err(|e| {
        match e {
            Abort::Interrupted => "CANCELLED",
            Abort::Timeout => "TIMEOUT",
            Abort::Budget => "LIMIT_EXCEEDED",
        }
        .into()
    })
}
fn default_definitions() -> bool {
    true
}
fn source_file(session: &Session) -> NotebookFile {
    NotebookFile {
        version: 1,
        title: session.notebook.title.clone(),
        cells: session
            .notebook
            .cells
            .iter()
            .map(|c| CellInput {
                id: c.id.clone(),
                kind: c.kind,
                source: c.source.clone(),
                dialect: c.dialect,
            })
            .collect(),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    snapshot_ref: String,
    source: String,
    dialect: Dialect,
    #[serde(default = "default_definitions")]
    use_notebook_definitions: bool,
}
/// Trusted native source/current accepted state; never deserialized from helper/model JSON.
pub struct ScratchSource<'a> {
    /// Current real task-local read-token issuer.
    pub reads: &'a mut ReadNotebookService,
    /// Actual native runtime/document/task/grants/config/editor binding.
    pub scope: &'a ReferenceScope,
    /// Actual nonsecret mathematical settings.
    pub general: GeneralConfig,
    /// Genuinely accepted current checkpoint, if one exists.
    pub accepted: Option<&'a AcceptedKernelState>,
    /// Native monotonic source-owner time.
    pub now: u64,
}
/// Actual parent provenance only. This never claims scratch has a main Kernel receipt/head.
#[derive(Clone, Serialize)]
pub struct ScratchParent {
    checkpoint_id: String,
    checkpoint_hash: String,
    kernel_state_revision: u64,
}
/// Original ACKed argument/source/state capture. Restore/evaluate/encoding run on scratch worker.
pub struct FrozenScratch {
    intent: ToolIntent,
    scope: ReferenceScope,
    source: NativeSourceSnapshot,
    general: GeneralConfig,
    parent: Option<Arc<KernelState>>,
    provenance: Option<ScratchParent>,
    args: Args,
}
/// Actual isolated immutable result; no main acceptance method or fabricated NativeKernelReceipt.
pub struct ScratchEvaluation {
    pub(crate) request: String,
    pub(crate) scope: ReferenceScope,
    pub(crate) source_hash: String,
    pub(crate) parent: Option<ScratchParent>,
    pub(crate) reserved: usize,
    pub(crate) definitions: bool,
    pub(crate) bytes: Vec<u8>,
    pub(crate) binding: CheckpointBinding,
    pub(crate) source: NotebookFile,
    pub(crate) general: GeneralConfig,
}
/// Freeze exact original arguments/current read-issued snapshot and only genuinely accepted state.
pub fn prepare(
    intent: &ToolIntent,
    arguments: &str,
    source: ScratchSource<'_>,
) -> Result<FrozenScratch, String> {
    if intent.name != AgentToolName::EvaluateScratch
        || format!("{:x}", Sha256::digest(arguments.as_bytes())) != intent.arguments_hash
    {
        return Err("IPC_CALL_SCOPE_MISMATCH".into());
    }
    if !source.scope.can_read {
        return Err("PERMISSION_DENIED".into());
    }
    let value = om_agent_contract::json::strict_json_bounded(arguments.as_bytes(), 65536)
        .map_err(|_| "INVALID_ARGUMENT")?;
    let value =
        super::checked_data("evaluate_scratch_args", value).map_err(|_| "INVALID_ARGUMENT")?;
    let args: Args = serde_json::from_value(value).map_err(|_| "INVALID_ARGUMENT")?;
    let snapshot = source
        .reads
        .current_math_snapshot(&args.snapshot_ref, source.scope, source.now)
        .map_err(|e| match e {
            super::read::ReadError::PermissionDenied => "PERMISSION_DENIED",
            super::read::ReadError::StaleSnapshot => "STALE_SNAPSHOT",
            _ => "INVALID_REFERENCE",
        })?;
    if source.general.eval_timeout_ms == 0
        || source.general.eval_timeout_ms > crate::protocol::MAX_SERIAL
    {
        return Err("INVALID_ARGUMENT".into());
    }
    let parent = if args.use_notebook_definitions {
        source.accepted.map(|a| a.state().clone())
    } else {
        None
    };
    if args.use_notebook_definitions
        && (parent.as_ref().is_some_and(|p| {
            p.binding().document_id != source.scope.document
                || p.binding().kernel_state_revision != source.scope.definition_revision
        }) || parent.is_none() && source.scope.definition_revision != 0)
    {
        return Err("CONTEXT_NOT_READY".into());
    }
    let provenance = if args.use_notebook_definitions {
        source.accepted.map(|a| ScratchParent {
            checkpoint_id: a.receipt().checkpoint_id.clone(),
            checkpoint_hash: a.state().hash().into(),
            kernel_state_revision: a.receipt().kernel_state_revision.get(),
        })
    } else {
        None
    };
    Ok(FrozenScratch {
        intent: intent.clone(),
        scope: source.scope.clone(),
        source: snapshot,
        general: source.general,
        parent,
        provenance,
        args,
    })
}
impl FrozenScratch {
    /// Original native identity for cancellation/immutable duplicate readback.
    pub fn id(&self) -> &str {
        &self.intent.operation_id
    }
    /// Actual captured scope, rechecked by native owner before delivering any result.
    pub fn scope(&self) -> &ReferenceScope {
        &self.scope
    }
    pub(crate) fn identity(&self) -> Value {
        json!({"kind":"agent_evaluate_scratch","operation":self.intent.operation_id,"turn":self.intent.turn_id,"call":self.intent.call_id,"proposal":self.intent.proposal_event,"arguments_hash":self.intent.arguments_hash,"parent":self.provenance,"source_hash":self.source.snapshot_hash})
    }
    /// Only the dedicated scratch worker calls this method. It never holds a source/control lock.
    pub fn evaluate(self, ctx: &Interrupt) -> Result<ScratchEvaluation, String> {
        tick(ctx)?;
        let clock = ctx.clock.clone().ok_or("CONTEXT_NOT_READY")?;
        let now = clock.now_ms();
        if !now.is_finite() || now < 0. {
            return Err("CONTEXT_NOT_READY".into());
        }
        let deadline = (now + self.general.eval_timeout_ms.min(5000) as f64)
            .min(ctx.deadline_ms.unwrap_or(f64::INFINITY));
        let bounded = Interrupt {
            flag: ctx.flag.clone(),
            clock: Some(clock),
            deadline_ms: Some(deadline),
            steps_left: std::cell::Cell::new(ctx.steps_left.get().min(4 * 1024 * 1024)),
        };
        let ctx = &bounded;
        let file = kernel_file(&self.source.file);
        let mut session = if let Some(parent) = self.parent {
            let old = parent.binding();
            let changed = om_kernel::source::math_source_changed(parent.source(), &file);
            if self.source.document_id != old.document_id
                || self.source.revision.get() < old.source_revision
                || self.source.execution_epoch.get() < old.execution_epoch
                || changed && self.source.execution_epoch.get() == old.execution_epoch
                || self.source.revision.get() == old.source_revision
                    && self.source.snapshot_hash != old.source_snapshot_hash
            {
                return Err("STALE_SNAPSHOT".into());
            }
            let mut work = parent
                .restore(
                    CheckpointRestore {
                        binding: old,
                        source: parent.source(),
                        general: parent.general(),
                        clock: ctx.clock.clone(),
                        cancel: ctx.flag.clone(),
                    },
                    CheckpointLimits::default(),
                    ctx,
                )
                .map_err(|_| "CONTEXT_NOT_READY")?;
            let delta = self.source.execution_epoch.get() - old.execution_epoch;
            work.reconcile(
                file.clone(),
                self.general.clone(),
                delta > 1 || delta > 0 && !changed,
            )?;
            work.into_session()
        } else {
            let mut config = KernelConfig {
                general: self.general.clone(),
                ..Default::default()
            };
            config.llm.enabled = false;
            config.llm.profiles.clear();
            let mut s = Session::with_cancel_token(config, ctx.clock.clone(), ctx.flag.clone());
            if self.args.use_notebook_definitions {
                s.apply_source_file_without_evaluation(file)?;
            }
            s
        };
        let id = format!("scratch-{}", self.intent.operation_id);
        let mut file = source_file(&session);
        if file.cells.iter().any(|c| c.id == id) {
            return Err("INVALID_ARGUMENT".into());
        }
        file.cells.push(CellInput {
            id: id.clone(),
            kind: CellKind::Math,
            source: self.args.source.clone(),
            dialect: self.args.dialect,
        });
        session.apply_source_file_without_evaluation(file)?;
        let plan = session.plan_cell_execution(
            std::slice::from_ref(&id),
            CellExecutionScope::Selected,
            &BTreeSet::from([id.clone()]),
            ctx,
        )?;
        if !plan.unavailable_prerequisite_cell_ids.is_empty() {
            return Err("DEPENDENCY_NOT_READY".into());
        }
        tick(ctx)?;
        session.config.general.reactive = false;
        session.config.general.eval_timeout_ms = session.config.general.eval_timeout_ms.min(5000);
        session.run_isolated_current_cell(&id, ctx)?;
        tick(ctx)?;
        // Encode this actual independent session solely to enforce/account retention. Its own
        // document identity/build cannot be confused with an accepted main Kernel state/receipt.
        let binding = CheckpointBinding {
            document_id: id.clone(),
            document_generation: 1,
            source_revision: 0,
            execution_epoch: 0,
            kernel_state_revision: 0,
            source_snapshot_hash: format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&source_file(&session)).map_err(|_| "LIMIT_EXCEEDED")?
                )
            ),
            build: "isolated-scratch-retention-1".into(),
        };
        let encoded = session
            .encode_checkpoint(
                &binding,
                CheckpointLimits {
                    max_bytes: 16 * 1024 * 1024,
                    ..Default::default()
                },
                ctx,
            )
            .map_err(|_| tick(ctx).err().unwrap_or_else(|| "LIMIT_EXCEEDED".into()))?;
        let reserved = encoded
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(4096))
            .ok_or("LIMIT_EXCEEDED")?;
        tick(ctx)?;
        Ok(ScratchEvaluation {
            request: self.intent.operation_id,
            scope: self.scope,
            source_hash: format!("{:x}", Sha256::digest(self.args.source.as_bytes())),
            parent: self.provenance,
            reserved,
            definitions: self.args.use_notebook_definitions,
            bytes: encoded,
            binding,
            source: source_file(&session),
            general: session.config.general.clone(),
        })
    }
}
