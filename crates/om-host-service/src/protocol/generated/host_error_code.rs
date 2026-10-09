//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `HostErrorCode`.
pub enum HostErrorCode {
    /// Contract value `INVALID_ARGUMENT`.
    #[serde(rename = "INVALID_ARGUMENT")]
    InvalidArgument,
    /// Contract value `INVALID_SOURCE`.
    #[serde(rename = "INVALID_SOURCE")]
    InvalidSource,
    /// Contract value `INVALID_REFERENCE`.
    #[serde(rename = "INVALID_REFERENCE")]
    InvalidReference,
    /// Contract value `PERMISSION_DENIED`.
    #[serde(rename = "PERMISSION_DENIED")]
    PermissionDenied,
    /// Contract value `STALE_DOCUMENT`.
    #[serde(rename = "STALE_DOCUMENT")]
    StaleDocument,
    /// Contract value `EDITING_BUSY`.
    #[serde(rename = "EDITING_BUSY")]
    EditingBusy,
    /// Contract value `NOT_AVAILABLE`.
    #[serde(rename = "NOT_AVAILABLE")]
    NotAvailable,
    /// Contract value `BUDGET_EXCEEDED`.
    #[serde(rename = "BUDGET_EXCEEDED")]
    BudgetExceeded,
    /// Contract value `CANCELLED`.
    #[serde(rename = "CANCELLED")]
    Cancelled,
    /// Contract value `UNKNOWN_OUTCOME`.
    #[serde(rename = "UNKNOWN_OUTCOME")]
    UnknownOutcome,
    /// Contract value `ABI_MISMATCH`.
    #[serde(rename = "ABI_MISMATCH")]
    AbiMismatch,
    /// Contract value `IO_FAILED`.
    #[serde(rename = "IO_FAILED")]
    IoFailed,
    /// Contract value `INTERNAL_ERROR`.
    #[serde(rename = "INTERNAL_ERROR")]
    InternalError,
}
