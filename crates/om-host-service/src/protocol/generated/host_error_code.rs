//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `HostErrorCode`.
pub enum HostErrorCode {
    /// Contract value `INVALID_ARGUMENT`.
    #[serde(rename = "INVALID_ARGUMENT")]
    INVALIDARGUMENT,
    /// Contract value `INVALID_SOURCE`.
    #[serde(rename = "INVALID_SOURCE")]
    INVALIDSOURCE,
    /// Contract value `INVALID_REFERENCE`.
    #[serde(rename = "INVALID_REFERENCE")]
    INVALIDREFERENCE,
    /// Contract value `PERMISSION_DENIED`.
    #[serde(rename = "PERMISSION_DENIED")]
    PERMISSIONDENIED,
    /// Contract value `STALE_DOCUMENT`.
    #[serde(rename = "STALE_DOCUMENT")]
    STALEDOCUMENT,
    /// Contract value `EDITING_BUSY`.
    #[serde(rename = "EDITING_BUSY")]
    EDITINGBUSY,
    /// Contract value `NOT_AVAILABLE`.
    #[serde(rename = "NOT_AVAILABLE")]
    NOTAVAILABLE,
    /// Contract value `BUDGET_EXCEEDED`.
    #[serde(rename = "BUDGET_EXCEEDED")]
    BUDGETEXCEEDED,
    /// Contract value `CANCELLED`.
    #[serde(rename = "CANCELLED")]
    CANCELLED,
    /// Contract value `UNKNOWN_OUTCOME`.
    #[serde(rename = "UNKNOWN_OUTCOME")]
    UNKNOWNOUTCOME,
    /// Contract value `ABI_MISMATCH`.
    #[serde(rename = "ABI_MISMATCH")]
    ABIMISMATCH,
    /// Contract value `IO_FAILED`.
    #[serde(rename = "IO_FAILED")]
    IOFAILED,
    /// Contract value `INTERNAL_ERROR`.
    #[serde(rename = "INTERNAL_ERROR")]
    INTERNALERROR,
}
