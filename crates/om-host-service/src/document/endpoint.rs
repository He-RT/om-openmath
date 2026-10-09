//! Trusted host source bridge. Physical IO stays in Swift; controller methods are short owner steps.
use super::{SourceDocument, commit::DocumentCommitController, coordinator::SourceCoordinator};
use crate::{
    protocol::{Nullable, Serial, generated::*, validation},
    references::ReferenceScope,
};
use sha2::{Digest, Sha256};
use std::{sync::OnceLock, time::Instant};

/// One source owner for the native runtime, separate from CAS/editor/network queues.
pub struct SourceEndpoint {
    controller: DocumentCommitController,
    scope: ReferenceScope,
    store: String,
    clock: Instant,
    coordinator: SourceCoordinator,
}
impl SourceEndpoint {
    /// Restore an actually read source head; host generates a fresh reference key, never the model.
    pub fn open(
        snapshot: NativeSourceSnapshot,
        store: String,
        runtime: String,
        generation: Serial,
        calculation: Option<NativeCalculationSettings>,
        config_revision: Serial,
    ) -> Result<Self, String> {
        if store.is_empty() || store.len() > 256 {
            return Err("INVALID_ARGUMENT".into());
        }
        let owner = SourceDocument::restore(snapshot, generation)?;
        let mut key = [0; 32];
        getrandom::fill(&mut key).map_err(|_| "ENTROPY_UNAVAILABLE")?;
        let s = owner.snapshot();
        let scope = ReferenceScope {
            runtime: runtime.clone(),
            document: s.document_id.clone(),
            generation: generation.get(),
            revision: s.revision.get(),
            execution_epoch: s.execution_epoch.get(),
            snapshot_hash: s.snapshot_hash.clone(),
            task: None,
            task_generation: 0,
            grant_revision: 1,
            config_revision: config_revision.get(),
            definition_revision: 0,
            metadata_revision: u64::from(
                om_kernel::capabilities::function_catalog().metadata_version,
            ),
            editor_state_hash: String::new(),
            execution_mode: true,
            can_read: true,
            can_preview: true,
            can_write: true,
        };
        let mut coordinator = SourceCoordinator::default();
        if let Some(setting) = calculation {
            coordinator.dialect = match setting.dialect {
                NativeCalculationSettingsDialect::Auto => om_kernel::config::ConfigDialect::Auto,
                NativeCalculationSettingsDialect::Modern => {
                    om_kernel::config::ConfigDialect::Modern
                }
                NativeCalculationSettingsDialect::Wolfram => {
                    om_kernel::config::ConfigDialect::Wolfram
                }
            };
            coordinator.constants = match setting.constants {
                NativeCalculationSettingsConstants::Math => om_kernel::config::Constants::Math,
                NativeCalculationSettingsConstants::Strict => om_kernel::config::Constants::Strict,
            };
        }
        Ok(Self {
            controller: DocumentCommitController::new(owner, runtime, key),
            scope,
            store,
            clock: Instant::now(),
            coordinator,
        })
    }
    /// Actual runtime/document binding for host readback and stale request checks.
    pub fn binding(&self) -> DocumentBinding {
        DocumentBinding {
            document_id: self.scope.document.clone(),
            generation: Serial::new(self.scope.generation).expect("checked scope"),
            document_revision: self.controller.owner().snapshot().revision,
            execution_epoch: self.controller.owner().snapshot().execution_epoch,
        }
    }
    /// Direct cancellation does not queue behind SQLite, CAS or SourcePreview's physical IO.
    pub fn cancel(&mut self, operation: &str) -> Option<bool> {
        let previous = self.controller.operation(operation)?;
        let current = self.controller.cancel(operation).ok()?;
        Some(previous.phase != NativeCommitStatePhase::Completed && current.cancel_requested)
    }
    /// Reads/source/draft binding stay available while another operation waits for physical commit.
    pub fn command(
        &mut self,
        command: NativeSourceHostCommand,
    ) -> Result<NativeSourceHostReply, String> {
        let now = self.clock.elapsed().as_millis() as u64;
        let mut reply = NativeSourceHostReply {
            protocol_version: 1,
            kind: NativeSourceHostReplyKind::State,
            document_generation: Serial::new(self.scope.generation)?,
            owner_time_ms: Serial::new(now)?,
            snapshot: Nullable(None),
            plan: Nullable(None),
            preview: Nullable(None),
            reference: Nullable(None),
            operation: Nullable(None),
        };
        let error = |error: super::commit::CommitError| match error {
            super::commit::CommitError::Busy => "OPERATION_IN_PROGRESS".to_owned(),
            super::commit::CommitError::Invalid => "PREVIEW_MISMATCH".to_owned(),
            super::commit::CommitError::Cancelled => "CANCELLED".to_owned(),
            super::commit::CommitError::Preview(_) => "STALE_SNAPSHOT".to_owned(),
        };
        match command {
            NativeSourceHostCommand::SourcePrepareManual(body) => {
                reply.kind = NativeSourceHostReplyKind::CommitPlan;
                reply.plan = Nullable(Some(
                    self.controller
                        .manual(
                            &body.operations,
                            &self.scope,
                            &self.coordinator,
                            "host-manual-time".into(),
                        )
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceRead(_) => {
                reply.kind = NativeSourceHostReplyKind::Source;
                reply.snapshot = Nullable(Some(self.controller.owner().snapshot().clone()));
            }
            NativeSourceHostCommand::SourceSnapshotRef(body) => {
                reply.kind = NativeSourceHostReplyKind::SnapshotRef;
                reply.reference = Nullable(Some(
                    self.controller
                        .snapshot(
                            self.coordinator.clone(),
                            body.complete_cell_ids.into_iter().collect(),
                            &self.scope,
                            now,
                        )
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourcePreview(body) => {
                let data = serde_json::to_vec(&body.arguments).map_err(|_| "INVALID_ARGUMENT")?;
                let result = self
                    .controller
                    .preview(&data, &self.scope, now, "host-preview-time".into())
                    .map_err(error)?;
                reply.kind = NativeSourceHostReplyKind::Preview;
                reply.preview = Nullable(Some(
                    serde_json::to_value(result).map_err(|_| "INTERNAL_ERROR")?,
                ));
            }
            NativeSourceHostCommand::SourceBegin(body) => {
                let plan = self
                    .controller
                    .begin(&body.preview_ref, &self.scope, now)
                    .map_err(error)?;
                reply.kind = if plan.is_some() {
                    NativeSourceHostReplyKind::CommitPlan
                } else {
                    NativeSourceHostReplyKind::State
                };
                reply.plan = Nullable(plan.map(|p| p.commit.clone()));
                reply.operation = Nullable(self.controller.preview_operation(&body.preview_ref));
            }
            NativeSourceHostCommand::SourceAdmitted(body) => {
                if body
                    .admission
                    .receipt
                    .0
                    .as_ref()
                    .is_some_and(|receipt| receipt.receipt.store_id != self.store)
                {
                    return Err("INVALID_RECEIPT".into());
                }
                reply.operation = Nullable(Some(
                    self.controller
                        .admitted(&body.operation_id, &body.admission)
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceFenced(body) => {
                reply.operation = Nullable(Some(
                    self.controller
                        .fenced(&body.operation_id, body.fence, now)
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceBarrier(body) => {
                reply.operation = Nullable(Some(
                    self.controller
                        .enter_commit(&body.operation_id, &body.fence_id, &self.scope, now)
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceUnknown(body) => {
                reply.operation = Nullable(Some(
                    self.controller.unknown(&body.operation_id).map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceCompleted(body) => {
                if body.receipt.receipt.store_id != self.store {
                    return Err("INVALID_RECEIPT".into());
                }
                reply.operation = Nullable(Some(
                    self.controller
                        .accept_receipt(&body.operation_id, &body.receipt)
                        .map_err(error)?,
                ));
                let source = self.controller.owner().snapshot();
                self.scope.revision = source.revision.get();
                self.scope.execution_epoch = source.execution_epoch.get();
                self.scope.snapshot_hash = source.snapshot_hash.clone();
                reply.snapshot = Nullable(Some(source.clone()));
            }
            NativeSourceHostCommand::SourceSettled(body) => {
                reply.operation = Nullable(Some(
                    self.controller
                        .settle_no_commit(&body.operation_id, body.cancelled)
                        .map_err(error)?,
                ));
            }
            NativeSourceHostCommand::SourceStatus(body) => {
                reply.operation = Nullable(self.controller.operation(&body.operation_id));
            }
            NativeSourceHostCommand::SourceEditorChanged(body) => {
                if body.state.runtime_instance_id != self.scope.runtime
                    || body.state.document_id != self.scope.document
                    || body.state.document_generation.get() != self.scope.generation
                    || body.state.source_revision != self.controller.owner().snapshot().revision
                {
                    return Err("STALE_DOCUMENT".into());
                }
                self.scope.editor_state_hash = format!(
                    "{:x}",
                    Sha256::digest(
                        serde_json::to_vec(&body.state).map_err(|_| "INVALID_ARGUMENT")?
                    )
                );
                reply.kind = NativeSourceHostReplyKind::EditorUpdated;
            }
            NativeSourceHostCommand::SourceOpen(_) => return Err("SOURCE_ALREADY_OPEN".into()),
        }
        if reply
            .operation
            .0
            .as_ref()
            .is_some_and(|state| state.phase == NativeCommitStatePhase::Completed)
        {
            let source = self.controller.owner().snapshot();
            self.scope.revision = source.revision.get();
            self.scope.execution_epoch = source.execution_epoch.get();
            self.scope.snapshot_hash = source.snapshot_hash.clone();
            reply.snapshot = Nullable(Some(source.clone()));
        }
        Ok(reply)
    }
}
/// Exact closed host union; this does not authorize a Pi/model to send host-internal commands.
pub fn decode_source_command(bytes: &[u8]) -> Result<NativeSourceHostCommand, String> {
    static SCHEMA: OnceLock<serde_json::Value> = OnceLock::new();
    let root = SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/native-source-store.schema.json"
        ))
        .expect("checked source schema")
    });
    validation::decode(bytes, &root["$defs"]["NativeSourceHostCommand"], root)
}
