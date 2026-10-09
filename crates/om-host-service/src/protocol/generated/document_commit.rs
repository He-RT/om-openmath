//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `DocumentCommit`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentCommit {
    /// Contract field `record_type`.
    pub record_type: DocumentCommitRecordType,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `transaction_id`.
    pub transaction_id: String,
    /// Contract field `request_hash`.
    pub request_hash: String,
    /// Contract field `base_revision`.
    pub base_revision: Serial,
    /// Contract field `committed_revision`.
    pub committed_revision: Serial,
    /// Contract field `snapshot_hash`.
    pub snapshot_hash: String,
    /// Contract field `snapshot_blob_hash`.
    pub snapshot_blob_hash: Nullable<String>,
    /// Contract field `inverse_plan_hash`.
    pub inverse_plan_hash: String,
    /// Contract field `execution_epoch`.
    pub execution_epoch: Serial,
    /// Contract field `actor`.
    pub actor: DocumentCommitActor,
    /// Contract field `task_id`.
    pub task_id: Nullable<String>,
    /// Contract field `undo_of`.
    pub undo_of: Nullable<String>,
    /// Contract field `changed_cell_ids`.
    pub changed_cell_ids: Vec<String>,
    /// Contract field `outbox_event_id`.
    pub outbox_event_id: String,
    /// Contract field `committed_at`.
    pub committed_at: String,
}
impl std::fmt::Debug for DocumentCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DocumentCommit { redacted }")
    }
}
