//! Generated native contract fields; validation/admission remain separate.
use super::*;
/// Native contract `NativeAgentAcknowledgeRunChild`. Required nullable fields cannot be omitted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAgentAcknowledgeRunChild {
    /// Contract field `type`.
    pub r#type: NativeAgentAcknowledgeRunChildType,
    /// Contract field `scope`.
    pub scope: NativeAgentControlScope,
    /// Contract field `operation_id`.
    pub operation_id: String,
    /// Contract field `child_operation_id`.
    pub child_operation_id: String,
    /// Contract field `group_admission`.
    pub group_admission: NativeRunEventAck,
    /// Contract field `group_started`.
    pub group_started: NativeRunEventAck,
    /// Contract field `child_admission`.
    pub child_admission: NativeRunEventAck,
    /// Contract field `child_started`.
    pub child_started: NativeRunEventAck,
}
impl std::fmt::Debug for NativeAgentAcknowledgeRunChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAgentAcknowledgeRunChild { redacted }")
    }
}
