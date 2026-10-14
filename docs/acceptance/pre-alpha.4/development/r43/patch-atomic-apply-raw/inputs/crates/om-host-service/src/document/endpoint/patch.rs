//! Agent intent -> existing frozen source plan -> original physical receipt. No IO/peer receipts.
use super::*;
use crate::{agent::ingress::ToolIntent, document::preview::FrozenPreviewPlan};
use serde_json::{Value, json};
use std::sync::Arc;
pub(super) struct PendingPatch {
    intent: ToolIntent,
    pub(super) plan: Arc<FrozenPreviewPlan>,
}
impl SourceEndpoint {
    fn current_source_scope(&self) -> ReferenceScope {
        let mut current = self.scope.clone();
        let s = self.controller.owner().snapshot();
        current.revision = s.revision.get();
        current.snapshot_hash = s.snapshot_hash.clone();
        current.execution_epoch = s.execution_epoch.get();
        current
    }
    pub(super) fn begin_agent_patch(
        &mut self,
        id: &str,
        reply: &mut NativeSourceHostReply,
    ) -> Result<(), String> {
        let binding = self.binding();
        let agent = self.agent.as_ref().ok_or("AGENT_NOT_ACTIVE")?;
        let (intent, args) = agent.apply_input(id, &binding)?;
        let current = agent.readonly_scope(self.current_source_scope());
        if agent.cached_effect(id, &current)?.is_some() {
            return Ok(());
        }
        if let Some(p) = self.patches.get(id) {
            // Same original intent can be read back. No second authority or source regeneration.
            agent.apply_scope(id, &binding, self.current_source_scope())?;
            reply.plan = Nullable(Some(p.plan.commit.clone()));
            reply.operation = Nullable(
                self.controller
                    .operation(&p.plan.commit.commit.operation_id),
            );
            reply.kind = NativeSourceHostReplyKind::CommitPlan;
            return Ok(());
        }
        let prepared = (|| {
            if !current.execution_mode || !current.can_write {
                return Err(("PERMISSION_DENIED", "none"));
            }
            if self.patches.len() >= 32 {
                return Err(("LIMIT_EXCEEDED", "reduce_request"));
            }
            let value = om_agent_contract::json::strict_json_bounded(args.as_bytes(), 65536)
                .map_err(|_| ("INVALID_ARGUMENT", "none"))?;
            let value = crate::tools::checked_data("apply_notebook_patch_args", value)
                .map_err(|_| ("INVALID_ARGUMENT", "none"))?;
            let reference = value["preview_ref"]
                .as_str()
                .ok_or(("INVALID_ARGUMENT", "none"))?;
            let frozen = self
                .controller
                .frozen_preview(reference, &current, self.clock.elapsed().as_millis() as u64)
                .map_err(crate::tools::patch::error)?;
            if self
                .patches
                .values()
                .any(|p| p.plan.commit.commit.operation_id == frozen.commit.commit.operation_id)
            {
                return Err(("PREVIEW_MISMATCH", "query_operation"));
            }
            self.controller
                .begin(reference, &current, self.clock.elapsed().as_millis() as u64)
                .map_err(crate::tools::patch::error)?
                .ok_or(("PREVIEW_MISMATCH", "reprepare"))
        })();
        match prepared {
            Ok(plan) => {
                if self
                    .patches
                    .values()
                    .any(|p| p.plan.commit.commit.operation_id == plan.commit.commit.operation_id)
                {
                    return Err("OPERATION_ALREADY_CLAIMED".into());
                }
                reply.plan = Nullable(Some(plan.commit.clone()));
                reply.operation =
                    Nullable(self.controller.operation(&plan.commit.commit.operation_id));
                reply.kind = NativeSourceHostReplyKind::CommitPlan;
                self.patches
                    .insert(id.into(), PendingPatch { intent, plan });
            }
            Err((code, recovery)) => {
                let envelope = envelope(
                    &intent,
                    &current,
                    None,
                    "failed",
                    Value::Null,
                    Some((code, recovery)),
                );
                self.agent.as_mut().expect("checked").register_effect(
                    &intent,
                    &current,
                    None,
                    envelope.to_string(),
                )?;
            }
        }
        Ok(())
    }
    /// The existing short SourceBarrier still runs under the real physical editor gate lock.
    /// Agent operations derive current grants at that final instant; task=None never substitutes.
    pub(super) fn patch_barrier_scope(&self, operation: &str) -> Result<ReferenceScope, String> {
        let current = self.current_source_scope();
        let Some(p) = self
            .patches
            .values()
            .find(|p| p.plan.commit.commit.operation_id == operation)
        else {
            return Ok(current);
        };
        let agent = self.agent.as_ref().ok_or("AGENT_NOT_ACTIVE")?;
        let scope = agent.apply_scope(&p.intent.operation_id, &self.binding(), current)?;
        if scope.task != p.plan.scope.task || scope.task_generation != p.plan.scope.task_generation
        {
            return Err("STALE_AGENT_SCOPE".into());
        }
        Ok(scope)
    }
    pub(super) fn agent_patch_result(
        &mut self,
        id: &str,
        failure: Option<NativeAgentEffectToolFailureCode>,
        current: ReferenceScope,
    ) -> Result<NativeAgentHostReply, String> {
        let agent = self.agent.as_ref().ok_or("AGENT_NOT_ACTIVE")?;
        agent.apply_input(id, &self.binding())?;
        if let Some(cached) = agent.cached_effect(id, &current)? {
            return Ok(cached);
        }
        let p = self.patches.get(id).ok_or("EFFECT_NOT_PREPARED")?;
        let state = self
            .controller
            .operation(&p.plan.commit.commit.operation_id)
            .ok_or("INVALID_OPERATION")?;
        let operation = state.operation_id.clone();
        let intent = p.intent.clone();
        let (status, data, error) = match state.phase {
            NativeCommitStatePhase::Completed => {
                let receipt = state.receipt.0.as_ref().ok_or("INVALID_RECEIPT")?;
                if receipt.receipt.request_hash != p.plan.commit.commit.request_hash
                    || receipt.receipt.transaction_id.0.as_ref()
                        != Some(&p.plan.commit.commit.transaction_id)
                    || current.snapshot_hash != p.plan.commit.after.snapshot_hash
                {
                    return Err("STALE_SNAPSHOT".into());
                }
                let ref_id = self
                    .controller
                    .snapshot(
                        self.coordinator.clone(),
                        Default::default(),
                        &current,
                        self.clock.elapsed().as_millis() as u64,
                    )
                    .map_err(|_| "SNAPSHOT_NOT_AVAILABLE")?;
                (
                    "completed",
                    json!({"transaction_ref":p.plan.commit.commit.transaction_id,"snapshot_ref":ref_id,"changed_cell_ids":p.plan.commit.commit.changed_cell_ids,"invalidated_cell_ids":p.plan.invalidation.affected_cells,"plan_hash":p.plan.plan_hash,"saved":false}),
                    None,
                )
            }
            NativeCommitStatePhase::Failed => {
                let code = match failure {
                    Some(NativeAgentEffectToolFailureCode::EditingBusy) => "EDITING_BUSY",
                    _ => "PREVIEW_MISMATCH",
                };
                ("failed", Value::Null, Some((code, "refresh_snapshot")))
            }
            NativeCommitStatePhase::Cancelled => {
                ("cancelled", Value::Null, Some(("CANCELLED", "none")))
            }
            NativeCommitStatePhase::Unknown => (
                "unknown",
                Value::Null,
                Some(("UNKNOWN_OUTCOME", "query_operation")),
            ),
            _ => return Ok(agent.pending_readonly()),
        };
        let value = envelope(&intent, &current, Some(&operation), status, data, error);
        self.agent.as_mut().expect("checked").register_effect(
            &intent,
            &current,
            Some(operation),
            value.to_string(),
        )
    }
}
fn envelope(
    intent: &ToolIntent,
    current: &ReferenceScope,
    operation: Option<&str>,
    status: &str,
    data: Value,
    error: Option<(&str, &str)>,
) -> Value {
    json!({"ok":status=="completed","status":status,"request_ref":intent.operation_id,"operation_ref":operation,
        "document":{"id":current.document,"generation":current.generation,"revision":current.revision},"data":data,
        "error":error.map(|(code,recovery)|json!({"code":code,"message":code,"recovery":recovery,"details":{}})),
        "pagination":{"truncated":false,"next_cursor":null}})
}
