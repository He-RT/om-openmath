//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `EventEnvelopeEventKind`.
pub enum EventEnvelopeEventKind {
    /// Contract value `host_ready`.
    #[serde(rename = "host_ready")]
    HostReady,
    /// Contract value `host_failed`.
    #[serde(rename = "host_failed")]
    HostFailed,
    /// Contract value `host_snapshot`.
    #[serde(rename = "host_snapshot")]
    HostSnapshot,
    /// Contract value `operation_progress`.
    #[serde(rename = "operation_progress")]
    OperationProgress,
    /// Contract value `operation_finished`.
    #[serde(rename = "operation_finished")]
    OperationFinished,
    /// Contract value `editor_result`.
    #[serde(rename = "editor_result")]
    EditorResult,
    /// Contract value `io_request`.
    #[serde(rename = "io_request")]
    IoRequest,
    /// Contract value `result_accepted`.
    #[serde(rename = "result_accepted")]
    ResultAccepted,
    /// Contract value `result_discarded`.
    #[serde(rename = "result_discarded")]
    ResultDiscarded,
}
