//! Freeze original run arguments and plan from a real read-issued snapshot/actual accepted state.
use crate::{
    agent::ingress::ToolIntent,
    document::coordinator::kernel_file,
    kernel::{KernelState, acceptance::AcceptedKernelState},
    protocol::generated::{AgentToolName, NativeSourceSnapshot},
    references::ReferenceScope,
    tools::read::{ReadError, ReadNotebookService},
};
use om_kernel::{
    CellExecutionPlan, CellExecutionScope, KernelConfig, Session,
    checkpoint::{CheckpointLimits, CheckpointRestore},
    config::GeneralConfig,
};
use om_num::ctx::{Abort, Interrupt};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    snapshot_ref: String,
    cell_ids: Vec<String>,
    #[serde(default)]
    scope: CellExecutionScope,
    #[serde(default)]
    continue_on_error: bool,
}
/// Internal frozen planning input. It has no serialized constructor, evaluator or write callback.
pub struct FrozenRunInput {
    scope: ReferenceScope,
    source: NativeSourceSnapshot,
    general: GeneralConfig,
    parent: Option<Arc<KernelState>>,
    args: Args,
    allowed: BTreeSet<String>,
}
/// Real plan and original policy. No state has been evaluated, published or accepted by planning.
#[derive(Debug, serde::Serialize)]
pub struct PreparedRun {
    /// Exact non-evaluating dependency plan from the isolated reconciled Session.
    pub plan: CellExecutionPlan,
    /// Original requested cell-error policy; execution must not silently replace it.
    pub continue_on_error: bool,
}
fn abort(error: Abort) -> String {
    match error {
        Abort::Interrupted => "CANCELLED",
        Abort::Timeout => "TIMEOUT",
        Abort::Budget => "LIMIT_EXCEEDED",
    }
    .into()
}
/// Resolve only the original native intent arguments. The native task owner supplies allowed
/// cells and actual accepted state; neither can come from model JSON or restored transcripts.
pub struct RunSource<'a> {
    /// Actual task-local read token issuer/retention owner.
    pub reads: &'a mut ReadNotebookService,
    /// Current trusted native document/task/editor/config binding.
    pub scope: &'a ReferenceScope,
    /// Actual current nonsecret mathematical configuration.
    pub general: GeneralConfig,
    /// Current genuinely accepted checkpoint, if one exists.
    pub accepted: Option<&'a AcceptedKernelState>,
    /// Native task's execution allowlist; never a model argument.
    pub allowed: BTreeSet<String>,
    /// Native monotonic timestamp.
    pub now: u64,
}
/// Freeze only original arguments and real current source/state, without doing worker decode work.
pub fn prepare(
    intent: &ToolIntent,
    arguments: &[u8],
    source: RunSource<'_>,
) -> Result<FrozenRunInput, String> {
    let RunSource {
        reads,
        scope,
        general,
        accepted,
        allowed,
        now,
    } = source;
    if intent.name != AgentToolName::RunCells
        || format!("{:x}", Sha256::digest(arguments)) != intent.arguments_hash
    {
        return Err("IPC_CALL_SCOPE_MISMATCH".into());
    }
    if !scope.execution_mode || !scope.can_read {
        return Err("PERMISSION_DENIED".into());
    }
    let value = om_agent_contract::json::strict_json_bounded(arguments, 65536)
        .map_err(|_| "INVALID_ARGUMENT")?;
    let value = super::checked_data("run_cells_args", value).map_err(|_| "INVALID_ARGUMENT")?;
    let args: Args = serde_json::from_value(value).map_err(|_| "INVALID_ARGUMENT")?;
    let source = reads
        .current_math_snapshot(&args.snapshot_ref, scope, now)
        .map_err(|e| match e {
            ReadError::PermissionDenied => "PERMISSION_DENIED",
            ReadError::StaleSnapshot => "STALE_SNAPSHOT",
            _ => "INVALID_REFERENCE",
        })?;
    if general.eval_timeout_ms == 0 || general.eval_timeout_ms > crate::protocol::MAX_SERIAL {
        return Err("INVALID_ARGUMENT".into());
    }
    let parent = accepted.map(|a| a.state().clone());
    if parent.as_ref().is_some_and(|p| {
        p.binding().document_id != scope.document
            || p.binding().kernel_state_revision != scope.definition_revision
    }) || parent.is_none() && scope.definition_revision != 0
    {
        return Err("CONTEXT_NOT_READY".into());
    }
    Ok(FrozenRunInput {
        scope: scope.clone(),
        source,
        general,
        parent,
        args,
        allowed,
    })
}
impl FrozenRunInput {
    /// Exact captured native scope; a coordinator must still recheck it before dispatch/acceptance.
    pub fn scope(&self) -> &ReferenceScope {
        &self.scope
    }
    /// Restore/reconcile/graph work belongs on a math worker, outside source/UI/control locks.
    /// The accepted Arc/bytes remain unchanged; this function never invokes a cell evaluator.
    pub fn plan(self, ctx: &Interrupt) -> Result<PreparedRun, String> {
        ctx.tick().map_err(abort)?;
        let source = kernel_file(&self.source.file);
        let session = if let Some(parent) = self.parent {
            let old = parent.binding();
            let changed = om_kernel::source::math_source_changed(parent.source(), &source);
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
            work.reconcile(source, self.general, delta > 1 || delta > 0 && !changed)?;
            work.into_session()
        } else {
            let mut config = KernelConfig {
                general: self.general,
                ..Default::default()
            };
            config.llm.enabled = false;
            config.llm.profiles.clear();
            let mut session =
                Session::with_cancel_token(config, ctx.clock.clone(), ctx.flag.clone());
            session.apply_source_file_without_evaluation(source)?;
            session
        };
        ctx.tick().map_err(abort)?;
        let plan = session.plan_cell_execution(
            &self.args.cell_ids,
            self.args.scope,
            &self.allowed,
            ctx,
        )?;
        Ok(PreparedRun {
            plan,
            continue_on_error: self.args.continue_on_error,
        })
    }
}
