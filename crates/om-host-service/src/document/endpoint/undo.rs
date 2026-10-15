//! Native undo adapter uses original ACKed args, current read-issued history and actual IO records.
use super::*;
use crate::agent::ingress::ToolIntent;
pub(super) struct PendingUndo {
    lookup: NativeAgentUndoLookup,
    scope: ReferenceScope,
}
impl SourceEndpoint {
    pub(super) fn validate_undo_records(
        &self,
        records: &[NativeStoredSourceTransaction],
    ) -> Result<(), String> {
        for record in records {
            let receipt = &record.receipt;
            let plan = &record.plan;
            crate::document::validate_commit(plan).map_err(|_| "INVALID_UNDO_RECORD")?;
            if record.store_id != self.store
                || record.transaction_id != plan.commit.transaction_id
                || plan.commit.document_id != self.scope.document
                || plan.calculation_change.0.is_some()
                || receipt.protocol_version != 1
                || receipt.receipt.store_id != self.store
                || receipt.receipt.document_id.0.as_ref() != Some(&self.scope.document)
                || receipt.receipt.transaction_id.0.as_ref() != Some(&record.transaction_id)
                || receipt.receipt.operation_id != plan.commit.operation_id
                || receipt.receipt.request_hash != plan.commit.request_hash
                || receipt.receipt.phase != OperationReceiptPhase::Completed
                || receipt.receipt.committed_revision.0 != Some(plan.after.revision)
                || receipt.receipt.error_code.0.is_some()
                || !receipt.receipt.accepted_result_ids.is_empty()
                || receipt.receipt.operation_kind
                    != if plan.commit.actor == DocumentCommitActor::Undo {
                        OperationReceiptOperationKind::Undo
                    } else {
                        OperationReceiptOperationKind::SourceEdit
                    }
                || receipt.snapshot_hash != plan.after.snapshot_hash
                || receipt.inverse_plan_hash != plan.before.snapshot_hash
                || receipt.execution_epoch != plan.after.execution_epoch
                || receipt.outbox_event_id != plan.commit.outbox_event_id
            {
                return Err("INVALID_UNDO_RECORD".into());
            }
        }
        Ok(())
    }
    pub(super) fn begin_agent_undo(
        &mut self,
        c: SourceBeginAgentUndo,
        reply: &mut NativeSourceHostReply,
    ) -> Result<(), String> {
        let binding = self.binding();
        let agent = self.agent.as_ref().ok_or("AGENT_NOT_ACTIVE")?;
        let (intent, args) = agent.source_effect_input(&c.operation_id, &binding)?;
        if intent.name != AgentToolName::UndoTransaction {
            return Err("INVALID_AGENT_TOOL".into());
        }
        let current =
            agent.source_effect_scope(&c.operation_id, &binding, self.current_source_scope())?;
        if agent.cached_effect(&c.operation_id, &current)?.is_some() {
            return Ok(());
        }
        if let Some(original) = self.patches.get(&c.operation_id) {
            reply.plan = Nullable(Some(original.plan.commit.clone()));
            reply.operation = Nullable(self.controller.operation(&c.operation_id));
            reply.kind = NativeSourceHostReplyKind::CommitPlan;
            return Ok(());
        }
        if !self.undo_lookups.contains_key(&c.operation_id) {
            let lookup = (|| {
                if self.undo_lookups.len() >= 32 || self.patches.len() >= 32 {
                    return Err(("LIMIT_EXCEEDED", "reduce_request"));
                }
                if self.editor_busy {
                    return Err(("EDITING_BUSY", "retry_read"));
                }
                let value = om_agent_contract::json::strict_json_bounded(args.as_bytes(), 65536)
                    .map_err(|_| ("INVALID_ARGUMENT", "none"))?;
                let value = crate::tools::checked_data("undo_transaction_args", value)
                    .map_err(|_| ("INVALID_ARGUMENT", "none"))?;
                let ids = value["transaction_refs"]
                    .as_array()
                    .ok_or(("INVALID_ARGUMENT", "none"))?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map(str::to_owned)
                            .ok_or(("INVALID_ARGUMENT", "none"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if c.allowed_transaction_refs
                    .0
                    .as_ref()
                    .is_some_and(|allowed| ids.iter().any(|id| !allowed.contains(id)))
                {
                    return Err(("PERMISSION_DENIED", "none"));
                }
                let source = self
                    .reads
                    .as_mut()
                    .ok_or(("INVALID_REFERENCE", "refresh_snapshot"))?
                    .undo_transactions(
                        value["snapshot_ref"]
                            .as_str()
                            .ok_or(("INVALID_ARGUMENT", "none"))?,
                        &ids,
                        &current,
                        self.clock.elapsed().as_millis() as u64,
                    )
                    .map_err(|e| match e {
                        crate::tools::read::ReadError::PermissionDenied => {
                            ("PERMISSION_DENIED", "none")
                        }
                        crate::tools::read::ReadError::StaleSnapshot => {
                            ("STALE_SNAPSHOT", "refresh_snapshot")
                        }
                        _ => ("INVALID_REFERENCE", "refresh_snapshot"),
                    })?;
                Ok(NativeAgentUndoLookup {
                    operation_id: intent.operation_id.clone(),
                    arguments_hash: intent.arguments_hash.clone(),
                    document_id: current.document.clone(),
                    store_id: self.store.clone(),
                    snapshot_hash: source.snapshot_hash,
                    source_revision: source.revision,
                    transaction_refs: ids,
                })
            })();
            match lookup {
                Ok(lookup) => {
                    self.undo_lookups.insert(
                        c.operation_id.clone(),
                        PendingUndo {
                            lookup,
                            scope: current.clone(),
                        },
                    );
                }
                Err((code, recovery)) => {
                    return self.undo_failure(&intent, &current, code, recovery);
                }
            }
        }
        let original = self.undo_lookups.get(&c.operation_id).expect("inserted");
        if original.scope != current {
            return self.undo_failure(&intent, &current, "STALE_SNAPSHOT", "refresh_snapshot");
        }
        if c.failure_code.0.is_some() {
            if c.records.0.is_some() {
                return Err("IPC_CALL_SCOPE_MISMATCH".into());
            }
            return self.undo_failure(&intent, &current, "NOT_AVAILABLE", "none");
        }
        let Some(records) = c.records.0 else {
            reply.undo_lookup = Nullable(Some(original.lookup.clone()));
            return Ok(());
        };
        if records.len() != original.lookup.transaction_refs.len()
            || records
                .iter()
                .any(|r| !original.lookup.transaction_refs.contains(&r.transaction_id))
            || records
                .iter()
                .map(|r| &r.transaction_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != records.len()
        {
            return Err("IPC_LEDGER_RECEIPT_MISMATCH".into());
        }
        if self.runs.contains_key(&c.operation_id)
            || self
                .results
                .as_ref()
                .is_some_and(|r| r.has_request(&c.operation_id))
            || self
                .kernel
                .as_ref()
                .is_some_and(|k| k.has_operation(&c.operation_id))
        {
            return Err("DUPLICATE_OPERATION".into());
        }
        self.validate_undo_records(&records)?;
        let prepared = self.controller.undo(
            &records,
            c.operation_id.clone(),
            format!("agent-undo-{}", c.operation_id),
            &current,
            &self.coordinator,
        );
        match prepared {
            Ok(Some(plan)) => {
                let frozen = self
                    .controller
                    .frozen_operation(&c.operation_id)
                    .ok_or("INVALID_OPERATION")?;
                if frozen.scope != current || frozen.commit.commit.operation_id != c.operation_id {
                    return Err("IPC_CALL_SCOPE_MISMATCH".into());
                }
                reply.plan = Nullable(Some(plan));
                reply.operation = Nullable(self.controller.operation(&c.operation_id));
                reply.kind = NativeSourceHostReplyKind::CommitPlan;
                self.patches.insert(
                    c.operation_id,
                    patch::PendingPatch {
                        intent,
                        plan: frozen,
                    },
                );
            }
            Ok(None) => return Err("IPC_CALL_SCOPE_MISMATCH".into()),
            Err(error) => {
                let (code, recovery) = crate::tools::patch::error(error);
                return self.undo_failure(&intent, &current, code, recovery);
            }
        }
        Ok(())
    }
    fn undo_failure(
        &mut self,
        intent: &ToolIntent,
        current: &ReferenceScope,
        code: &str,
        recovery: &str,
    ) -> Result<(), String> {
        let value = patch::envelope(
            intent,
            current,
            None,
            "failed",
            serde_json::Value::Null,
            Some((code, recovery)),
        );
        self.agent
            .as_mut()
            .ok_or("AGENT_NOT_ACTIVE")?
            .register_effect(intent, current, None, value.to_string())?;
        Ok(())
    }
}
