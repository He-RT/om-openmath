//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentHostReply`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentHostReply {
    /// Contract field `protocol_version`.
    pub protocol_version: u32,
    /// Contract field `runtime_instance_id`.
    pub runtime_instance_id: String,
    /// Contract field `document_id`.
    pub document_id: String,
    /// Contract field `document_generation`.
    pub document_generation: Serial,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `phase`.
    pub phase: NativeAgentHostReplyPhase,
    /// Contract field `disposition`.
    pub disposition: NativeAgentHostReplyDisposition,
    /// Contract field `outbound_json`.
    pub outbound_json: Nullable<String>,
    /// Contract field `canonical_json`.
    pub canonical_json: Nullable<String>,
    /// Contract field `peer_message_id`.
    pub peer_message_id: Nullable<String>,
    /// Contract field `event_id`.
    pub event_id: Nullable<String>,
    /// Contract field `source_sequence`.
    pub source_sequence: Nullable<Serial>,
    /// Contract field `tool_intent`.
    pub tool_intent: Nullable<NativeAgentToolIntent>,
    /// Contract field `request_intent`.
    pub request_intent: Nullable<NativeAgentRequestIntent>,
    /// Contract field `readonly_result`.
    pub readonly_result: Nullable<NativeAgentReadonlyResult>,
    /// Contract field `effect_result`.
    pub effect_result: Nullable<NativeAgentEffectResult>,
    /// Contract field `run_progress`.
    pub run_progress: Nullable<NativeRunProgress>,
}
impl std::fmt::Debug for NativeAgentHostReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentHostReply { redacted }")
    }
}
