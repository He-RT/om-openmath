//! Inspect immutable source snapshots, allocate trusted IDs and freeze whole patches without CAS.
mod patch;
use super::{
    SourceDocument,
    coordinator::{SourceCoordinator, kernel_file},
    request_hash,
};
use crate::{
    protocol::{Nullable, generated::*, validation},
    references::{ReferenceError, ReferenceKind, ReferenceRegistry, ReferenceScope},
};
use om_kernel::source::SourceInvalidation;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

/// A read snapshot records which complete source cells were actually exposed to the requester.
#[derive(Clone, Serialize)]
pub struct FrozenReadSnapshot {
    /// Whole source held locally; not sent to a model by default.
    pub source: NativeSourceSnapshot,
    /// Actual complete source exposure from read_notebook, distinct from an outline/snippet.
    pub complete_cell_ids: BTreeSet<String>,
    /// Trusted real parser settings/bindings at the original read boundary.
    pub coordinator: SourceCoordinator,
}
/// Complete immutable commit data. None of this is reconstructed from later editor drafts.
#[derive(Clone, Serialize)]
pub struct FrozenPreviewPlan {
    /// Exact trusted permission/runtime/document binding.
    pub scope: ReferenceScope,
    /// Stable physical operation and original source/inverse plan.
    pub commit: NativeSourceCommit,
    /// Exact normalized whole-operation sequence.
    pub operations: Vec<NativeSourceOperation>,
    /// Actual graph analysis, not fabricated values or successful evaluation.
    pub invalidation: SourceInvalidation,
    /// Original proposal-local keys to host-assigned cell identities.
    pub assigned: BTreeMap<String, String>,
    /// Hash of the immutable metadata/content above (self hash excluded).
    pub plan_hash: String,
}
#[derive(Clone)]
enum Value {
    Snapshot(Box<FrozenReadSnapshot>),
    Preview(Arc<FrozenPreviewPlan>),
}
impl Serialize for Value {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Snapshot(snapshot) => serializer.serialize_newtype_variant(
                "RetainedPreviewValue",
                0,
                "snapshot",
                snapshot,
            ),
            Self::Preview(plan) => serializer.serialize_newtype_variant(
                "RetainedPreviewValue",
                1,
                "preview",
                plan.as_ref(),
            ),
        }
    }
}
/// Preview failure cannot authorize a different mutation or replay a previous operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewError {
    /// Invalid model argument/schema/selector or source bounds.
    InvalidArgument,
    /// Permission/grant/mode failure.
    PermissionDenied,
    /// No matching original source snapshot.
    StaleSnapshot,
    /// Hash differs from actual original UTF8 bytes.
    SourceHashMismatch,
    /// Full source was not exposed for full replacement.
    IncompleteSource,
    /// References to deleted/new/undeclared cells or duplicate plan-local keys.
    InvalidPatch,
    /// A text replacement is not exactly one original scalar-boundary fragment.
    NonuniqueReplacement,
    /// A source/parse/storage/counter budget or metadata preparation failure.
    LimitExceeded,
    /// Opaque lease failed actual kind/scope/time/signature validation.
    Reference(ReferenceError),
}
impl From<ReferenceError> for PreviewError {
    fn from(value: ReferenceError) -> Self {
        Self::Reference(value)
    }
}
/// Sole ephemeral preview owner. Its secret is supplied only by trusted host secure entropy.
pub struct PreviewService {
    references: ReferenceRegistry<Value>,
}
impl PreviewService {
    /// New runtime-local state, never restored from transcript/JSONL or model bytes.
    pub fn new(host_key: [u8; 32]) -> Self {
        Self {
            references: ReferenceRegistry::new(host_key),
        }
    }
    /// Bind a read snapshot to exact current owner facts and its actually returned full-cell set.
    pub fn snapshot(
        &mut self,
        owner: &SourceDocument,
        coordinator: SourceCoordinator,
        complete_cell_ids: BTreeSet<String>,
        scope: &ReferenceScope,
        now_ms: u64,
    ) -> Result<String, PreviewError> {
        scope.validate()?;
        if !scope.can_read {
            return Err(PreviewError::PermissionDenied);
        }
        Self::matches_owner(scope, owner)?;
        if complete_cell_ids
            .iter()
            .any(|id| !owner.snapshot().file.cells.iter().any(|c| &c.id == id))
        {
            return Err(PreviewError::InvalidArgument);
        }
        if scope.metadata_revision
            != u64::from(om_kernel::capabilities::function_catalog().metadata_version)
        {
            return Err(PreviewError::StaleSnapshot);
        }
        self.references.retain_scope(scope);
        Ok(self.references.issue(
            ReferenceKind::Snapshot,
            scope.clone(),
            Value::Snapshot(Box::new(FrozenReadSnapshot {
                source: owner.snapshot().clone(),
                complete_cell_ids,
                coordinator,
            })),
            now_ms,
            600_000,
        )?)
    }
    /// Decode the same closed union/limits as model tool arguments before any reference lookup.
    pub fn decode(bytes: &[u8]) -> Result<NativePreviewArgs, PreviewError> {
        static SCHEMA: OnceLock<serde_json::Value> = OnceLock::new();
        let root = SCHEMA.get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../docs/design/native-preview.schema.json"
            ))
            .expect("checked-in preview schema")
        });
        validation::decode(bytes, &root["$defs"]["NativePreviewArgs"], root)
            .map_err(|_| PreviewError::InvalidArgument)
    }
    /// Inspect source or patch. `source` never issues a committable reference; only valid complete
    /// patches get one, and even then actual commit requires execution mode/grants/native fence.
    pub fn inspect(
        &mut self,
        bytes: &[u8],
        owner: &SourceDocument,
        current: &ReferenceScope,
        now_ms: u64,
        planned_time: String,
    ) -> Result<NativePreviewData, PreviewError> {
        let arguments = Self::decode(bytes)?;
        if !current.can_preview || !current.can_read {
            return Err(PreviewError::PermissionDenied);
        }
        Self::matches_owner(current, owner)?;
        let resolved = self.references.resolve(
            &arguments.snapshot_ref,
            ReferenceKind::Snapshot,
            current,
            now_ms,
        )?;
        let Value::Snapshot(snapshot) = resolved.as_ref() else {
            return Err(PreviewError::InvalidPatch);
        };
        if snapshot.source.snapshot_hash != owner.snapshot().snapshot_hash {
            return Err(PreviewError::StaleSnapshot);
        }
        match arguments.input {
            NativePreviewInput::PreviewSourceInput(input) => self.source(input, snapshot, current),
            NativePreviewInput::PreviewPatchInput(input) => {
                let (operations, assigned) =
                    patch::normalize(&mut self.references, &input, snapshot)?;
                let operation = self.references.allocate_id("operation")?;
                let transaction = self.references.allocate_id("transaction")?;
                let event = self.references.allocate_id("event")?;
                let mut mutation = snapshot
                    .coordinator
                    .prepare(
                        owner,
                        &operations,
                        operation,
                        transaction,
                        event,
                        planned_time,
                    )
                    .map_err(|_| PreviewError::LimitExceeded)?;
                if let Some(task) = &current.task {
                    mutation.commit.commit.actor = DocumentCommitActor::Agent;
                    mutation.commit.commit.task_id = Nullable(Some(task.clone()));
                    mutation.commit.commit.request_hash = request_hash(&mutation.commit);
                }
                let mut plan = FrozenPreviewPlan {
                    scope: current.clone(),
                    commit: mutation.commit,
                    operations,
                    invalidation: mutation.invalidation,
                    assigned,
                    plan_hash: String::new(),
                };
                let (valid, diagnostics) = patch::diagnostics(&plan);
                if plan.invalidation.affected_cells.len() > 1000 {
                    return Err(PreviewError::LimitExceeded);
                }
                plan.plan_hash = plan_hash(&plan)?;
                let ref_id = if valid {
                    Some(self.references.issue(
                        ReferenceKind::Preview,
                        current.clone(),
                        Value::Preview(Arc::new(plan.clone())),
                        now_ms,
                        300_000,
                    )?)
                } else {
                    None
                };
                checked_output(NativePreviewData {
                    preview_kind: NativePreviewDataPreviewKind::Patch,
                    valid,
                    preview_ref: Nullable(ref_id),
                    plan_hash: plan.plan_hash,
                    diagnostics,
                    affected_cell_ids: plan.invalidation.affected_cells,
                    assigned_cell_ids: plan.assigned,
                })
            }
        }
    }
    fn source(
        &mut self,
        input: PreviewSourceInput,
        snapshot: &FrozenReadSnapshot,
        current: &ReferenceScope,
    ) -> Result<NativePreviewData, PreviewError> {
        let mut file = kernel_file(&snapshot.source.file);
        let id = self.references.allocate_id("sourcecheck")?;
        file.cells.push(om_kernel::protocol::CellInput {
            id: id.clone(),
            kind: om_kernel::protocol::CellKind::Math,
            source: input.source.clone(),
            dialect: match input.dialect {
                Dialect::Modern => om_kernel::protocol::Dialect::Modern,
                Dialect::Wolfram => om_kernel::protocol::Dialect::Wolfram,
                Dialect::Auto => om_kernel::protocol::Dialect::Auto,
            },
        });
        let analysis = om_kernel::source::analyze_source(
            &file,
            snapshot.coordinator.dialect,
            snapshot.coordinator.constants,
            &snapshot.coordinator.known_functions,
        )
        .map_err(|_| PreviewError::LimitExceeded)?;
        let cell = analysis
            .cells
            .iter()
            .find(|c| c.cell_id == id)
            .ok_or(PreviewError::InvalidPatch)?;
        let diagnostics = cell
            .diagnostics
            .iter()
            .map(|d| diagnostic(d, None))
            .collect::<Vec<_>>();
        if diagnostics.len() > 100 {
            return Err(PreviewError::LimitExceeded);
        }
        let valid = !diagnostics
            .iter()
            .any(|d| d.severity == NativePreviewDiagnosticSeverity::Error);
        let raw = serde_json::to_vec(&("source-inspection-v1", current, &input))
            .map_err(|_| PreviewError::InvalidArgument)?;
        checked_output(NativePreviewData {
            preview_kind: NativePreviewDataPreviewKind::Source,
            valid,
            preview_ref: Nullable(None),
            plan_hash: format!("{:x}", Sha256::digest(raw)),
            diagnostics,
            affected_cell_ids: vec![],
            assigned_cell_ids: BTreeMap::new(),
        })
    }
    /// Retrieve the original, protected, byte-identical plan. Neither current source/draft nor a
    /// model-passed operation list can replace it; commit remains a separate owner API.
    pub fn resolve_for_commit(
        &self,
        preview_ref: &str,
        owner: &SourceDocument,
        current: &ReferenceScope,
        now_ms: u64,
    ) -> Result<Arc<FrozenPreviewPlan>, PreviewError> {
        if !current.execution_mode || !current.can_write {
            return Err(PreviewError::PermissionDenied);
        }
        Self::matches_owner(current, owner)?;
        let resolved =
            self.references
                .resolve(preview_ref, ReferenceKind::Preview, current, now_ms)?;
        let Value::Preview(plan) = resolved.as_ref() else {
            return Err(PreviewError::InvalidPatch);
        };
        if plan.plan_hash != plan_hash(plan)? {
            return Err(PreviewError::InvalidPatch);
        }
        Ok(plan.clone())
    }
    /// Host cancellation/document close invalidates an uncommitted token, never a durable receipt.
    pub fn revoke(&mut self, token: &str) -> Result<(), PreviewError> {
        Ok(self.references.revoke(token)?)
    }
    fn matches_owner(scope: &ReferenceScope, owner: &SourceDocument) -> Result<(), PreviewError> {
        let source = owner.snapshot();
        if scope.generation != owner.generation().get()
            || scope.document != source.document_id
            || scope.revision != source.revision.get()
            || scope.execution_epoch != source.execution_epoch.get()
            || scope.snapshot_hash != source.snapshot_hash
        {
            return Err(PreviewError::StaleSnapshot);
        }
        Ok(())
    }
}
fn checked_output(data: NativePreviewData) -> Result<NativePreviewData, PreviewError> {
    let bytes = serde_json::to_vec(&data).map_err(|_| PreviewError::InvalidArgument)?;
    if bytes.len() > 65536 {
        return Err(PreviewError::LimitExceeded);
    }
    Ok(data)
}
fn plan_hash(plan: &FrozenPreviewPlan) -> Result<String, PreviewError> {
    let bytes = serde_json::to_vec(&(
        "frozen-source-plan-v1",
        &plan.scope,
        &plan.commit,
        &plan.operations,
        &plan.invalidation,
        &plan.assigned,
    ))
    .map_err(|_| PreviewError::InvalidArgument)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn diagnostic(
    d: &om_kernel::protocol::Diagnostic,
    cell: Option<String>,
) -> NativePreviewDiagnostic {
    NativePreviewDiagnostic {
        code: d.code.clone(),
        message: d.message.chars().take(4000).collect(),
        severity: match d.severity {
            om_kernel::protocol::Severity::Error => NativePreviewDiagnosticSeverity::Error,
            om_kernel::protocol::Severity::Warning => NativePreviewDiagnosticSeverity::Warning,
            om_kernel::protocol::Severity::Hint => NativePreviewDiagnosticSeverity::Hint,
        },
        cell_id: Nullable(cell),
        start_utf8: Nullable(Some(d.span.start)),
        end_utf8: Nullable(Some(d.span.end)),
    }
}
