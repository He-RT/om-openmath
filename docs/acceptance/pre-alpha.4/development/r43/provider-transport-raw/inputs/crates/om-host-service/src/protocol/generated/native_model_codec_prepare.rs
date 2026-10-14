//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeModelCodecPrepare`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeModelCodecPrepare {
    /// Contract field `type`.
    pub r#type: NativeModelCodecPrepareType,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `request_ref`.
    pub request_ref: String,
    /// Contract field `context_ref`.
    pub context_ref: String,
    /// Contract field `task_id`.
    pub task_id: String,
    /// Contract field `agent_turn_id`.
    pub agent_turn_id: String,
    /// Contract field `message_id`.
    pub message_id: String,
    /// Contract field `request_json`.
    pub request_json: String,
}
impl std::fmt::Debug for NativeModelCodecPrepare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeModelCodecPrepare { redacted }")
    }
}
