//! Generated native contract fields; validation/admission remain separate.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contract enum `NativeKernelCommitKind`.
pub enum NativeKernelCommitKind {
    /// Contract value `bootstrap`.
    #[serde(rename = "bootstrap")]
    Bootstrap,
    /// Contract value `execute_cell`.
    #[serde(rename = "execute_cell")]
    ExecuteCell,
}
