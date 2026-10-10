//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeKernelTerminalStatus`.
pub enum NativeKernelTerminalStatus {
    /// Contract value `unexecuted`.
    #[serde(rename = "unexecuted")]
    Unexecuted,
    /// Contract value `done`.
    #[serde(rename = "done")]
    Done,
    /// Contract value `error`.
    #[serde(rename = "error")]
    Error,
}
