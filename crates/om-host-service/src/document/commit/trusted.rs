//! Trusted inverse preparation and confirmed receipt import; no SQL, UI or model parameters.
use super::*;
impl DocumentCommitController {
    /// Restore an actual durable undo fact into a fresh runtime whose source is already read back.
    /// This changes no source; unknown in-flight operations still use exact plan acceptance.
    pub fn recover_undo(
        &mut self,
        receipt: &NativeDurableSourceReceipt,
    ) -> Result<NativeCommitState, CommitError> {
        if self.pending.contains_key(&receipt.receipt.operation_id) {
            return self.accept_receipt(&receipt.receipt.operation_id, receipt);
        }
        let current = self.owner.snapshot();
        let r = &receipt.receipt;
        if self.pending.len() >= 4096
            || receipt.protocol_version != 1
            || r.document_id.0.as_ref() != Some(&current.document_id)
            || r.phase != OperationReceiptPhase::Completed
            || r.operation_kind != OperationReceiptOperationKind::Undo
            || r.committed_revision
                .0
                .is_none_or(|revision| revision > current.revision)
            || (r.committed_revision.0 == Some(current.revision)
                && receipt.snapshot_hash != current.snapshot_hash)
        {
            return Err(CommitError::Invalid);
        }
        let state = NativeCommitState {
            protocol_version: 1,
            document_id: current.document_id.clone(),
            operation_id: r.operation_id.clone(),
            request_hash: r.request_hash.clone(),
            phase: NativeCommitStatePhase::Completed,
            cancel_requested: false,
            receipt: Nullable(Some(receipt.clone())),
        };
        self.pending.insert(
            r.operation_id.clone(),
            Pending {
                plan: None,
                state: state.clone(),
                fence: None,
                admitted: true,
                manual: false,
            },
        );
        Ok(state)
    }
    /// Trusted physical records prepare one inverse transaction. No model supplies these records.
    pub fn undo(
        &mut self,
        records: &[NativeStoredSourceTransaction],
        operation: String,
        group: String,
        scope: &ReferenceScope,
        coordinator: &crate::document::coordinator::SourceCoordinator,
    ) -> Result<Option<NativeSourceCommit>, CommitError> {
        let requested_ids = records
            .iter()
            .map(|r| r.transaction_id.clone())
            .collect::<Vec<_>>();
        if let Some(pending) = self.pending.get(&operation) {
            if self.undo_requests.get(&operation) != Some(&(group.clone(), requested_ids)) {
                return Err(CommitError::Invalid);
            }
            return Ok(pending.plan.as_ref().map(|p| p.commit.clone()));
        }
        if self.gate.is_some() || self.pending.len() >= 4096 {
            return Err(CommitError::Busy);
        }
        let originals = records.iter().map(|r| r.plan.clone()).collect::<Vec<_>>();
        let file =
            crate::document::undo::merge_group(&originals, self.owner.snapshot()).map_err(|e| {
                match e {
                    crate::document::undo::UndoError::Conflict => CommitError::UndoConflict,
                    _ => CommitError::Invalid,
                }
            })?;
        let mut plan = self
            .owner
            .prepare(
                file,
                operation.clone(),
                format!("transaction-{}", uuid_like_id()?),
                format!("event-{}", uuid_like_id()?),
                "host-undo-time".into(),
            )
            .map_err(|_| CommitError::Invalid)?;
        plan.commit.actor = DocumentCommitActor::Undo;
        let ids = requested_ids;
        plan.commit.undo_of = Nullable(ids.first().cloned());
        plan.undo_group = Some(NativeUndoGroup {
            group_id: group.clone(),
            transaction_ids: ids.clone(),
        });
        plan.commit.request_hash = crate::document::request_hash(&plan);
        crate::document::validate_commit(&plan).map_err(|_| CommitError::Invalid)?;
        let invalidation = om_kernel::source::assess_source_change(
            &crate::document::coordinator::kernel_file(&plan.before.file),
            &crate::document::coordinator::kernel_file(&plan.after.file),
            coordinator.dialect,
            coordinator.constants,
            &coordinator.known_functions,
            &coordinator.owned,
            false,
        )
        .map_err(|_| CommitError::Invalid)?;
        let frozen = Arc::new(FrozenPreviewPlan {
            scope: scope.clone(),
            commit: plan.clone(),
            operations: vec![],
            invalidation,
            assigned: BTreeMap::new(),
            plan_hash: String::new(),
        });
        self.pending.insert(
            operation.clone(),
            Pending {
                plan: Some(frozen),
                state: NativeCommitState {
                    protocol_version: 1,
                    document_id: plan.commit.document_id.clone(),
                    operation_id: operation.clone(),
                    request_hash: plan.commit.request_hash.clone(),
                    phase: NativeCommitStatePhase::AwaitingAdmission,
                    cancel_requested: false,
                    receipt: Nullable(None),
                },
                fence: None,
                admitted: false,
                manual: false,
            },
        );
        self.gate = Some(operation);
        self.undo_requests
            .insert(plan.commit.operation_id.clone(), (group, ids));
        Ok(Some(plan))
    }
}
