//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentPrepareRun`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentPrepareRun {
    /// Contract field `type`.
    pub r#type: NativeAgentPrepareRunType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `allowed_cell_ids`.
    pub allowed_cell_ids: Nullable<Vec<String>>,
}
impl std::fmt::Debug for NativeAgentPrepareRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentPrepareRun { redacted }")
    }
}
