//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `OperationReceipt`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationReceipt {
    /// Contract field `record_type`.
    pub record_type: OperationReceiptRecordType,
    /// Contract field `store_id`.
    pub store_id: String,
    /// Contract field `document_id`.
    pub document_id: Nullable<String>,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `operation_kind`.
    pub operation_kind: OperationReceiptOperationKind,
    /// Contract field `request_hash`.
    pub request_hash: String,
    /// Contract field `phase`.
    pub phase: OperationReceiptPhase,
    /// Contract field `transaction_id`.
    pub transaction_id: Nullable<String>,
    /// Contract field `committed_revision`.
    pub committed_revision: Nullable<Serial>,
    /// Contract field `accepted_result_ids`.
    pub accepted_result_ids: Vec<String>,
    /// Contract field `details_blob_hash`.
    pub details_blob_hash: Nullable<String>,
    /// Contract field `details_retention`.
    pub details_retention: OperationReceiptDetailsRetention,
    /// Contract field `error_code`.
    pub error_code: Nullable<String>,
    /// Contract field `updated_at`.
    pub updated_at: String,
}
impl std::fmt::Debug for OperationReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OperationReceipt { redacted }")
    }
}
