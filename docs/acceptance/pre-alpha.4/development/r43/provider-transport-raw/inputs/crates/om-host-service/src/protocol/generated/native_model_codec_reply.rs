//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeModelCodecReply`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeModelCodecReply {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `codec_ref`.
    pub codec_ref: String,
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
    /// Contract field `phase`.
    pub phase: NativeModelCodecReplyPhase,
    /// Contract field `stream`.
    pub stream: bool,
    /// Contract field `body_json`.
    pub body_json: Nullable<String>,
    /// Contract field `text_deltas`.
    pub text_deltas: Vec<String>,
    /// Contract field `result_json`.
    pub result_json: Nullable<String>,
    /// Contract field `error_code`.
    pub error_code: Nullable<NativeModelCodecReplyErrorCode>,
    /// Contract field `http_status`.
    pub http_status: Nullable<u32>,
}
impl std::fmt::Debug for NativeModelCodecReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeModelCodecReply { redacted }")
    }
}
