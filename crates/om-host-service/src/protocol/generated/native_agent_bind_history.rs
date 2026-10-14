//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentBindHistory`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentBindHistory {
    /// Contract field `type`.
    pub r#type: NativeAgentBindHistoryType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `expected_snapshot_hash`.
    pub expected_snapshot_hash: String,
    /// Contract field `transaction_refs`.
    pub transaction_refs: Vec<String>,
}
impl std::fmt::Debug for NativeAgentBindHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentBindHistory { redacted }")
    }
}
